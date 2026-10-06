// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//  	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
//! Closed typed Cell/Counter Unit composition; shared Plan owns evaluation.
use super::*;

#[derive(Clone, Copy)]
pub(super) enum AuditedCall<'a> {
    PureValue(&'a PureCircuit),
    PureUnitGuard(&'a PureCircuit),
    LocalUnit(&'a StatefulCircuit),
    RecordedUnit(&'a StatefulCircuit),
}
pub(super) type Calls<'a> = HashMap<String, AuditedCall<'a>>;

// Preserve the Cell and witness signature domain independently from argument
// and pure-helper values admitted by collection composition.
pub(super) fn observed_value_type(ty: &Type) -> bool {
    match ty {
        Type::Boolean | Type::Field | Type::JubjubPoint | Type::Bytes { .. } => true,
        Type::Unsigned { max } => max.parse::<u64>().is_ok(),
        Type::Struct { fields, .. } => fields.iter().all(|f| observed_value_type(&f.ty)),
        Type::Tuple { elements } => {
            !elements.is_empty() && elements.len() <= 8 && elements.iter().all(observed_value_type)
        }
        Type::Vector { element, .. } => observed_value_type(element),
        _ => false,
    }
}
pub(super) fn value_type(ty: &Type) -> bool {
    match ty {
        Type::OpaqueString | Type::Enum { .. } => true,
        Type::Struct { fields, .. } => fields.iter().all(|f| value_type(&f.ty)),
        Type::Tuple { elements } => {
            !elements.is_empty() && elements.len() <= 8 && elements.iter().all(value_type)
        }
        Type::Vector { element, .. } => value_type(element),
        _ => observed_value_type(ty),
    }
}
pub(super) fn write_type(ty: &Type) -> bool {
    matches!(ty, Type::Boolean | Type::JubjubPoint)
        || *ty
            == (Type::Unsigned {
                max: u64::MAX.to_string(),
            })
}
pub(super) fn slot_path(field: &LedgerField) -> bool {
    let path = field.physical_path();
    // The contract-level ledger validator owns physical layout/uniqueness.
    // Retain that complete declared path rather than reconstructing an index.
    path.first() == Some(&field.index) && (1..=2).contains(&path.len())
}
struct Audit<'a> {
    ledger: &'a HashMap<&'a str, &'a LedgerField>,
    witnesses: &'a HashMap<&'a str, &'a WitnessDeclaration>,
    pure: &'a HashMap<&'a str, &'a PureCircuit>,
    circuits: &'a HashMap<&'a str, &'a StatefulCircuit>,
    active: HashSet<String>,
    calls: Calls<'a>,
    public: usize,
}
impl Audit<'_> {
    fn slot(&self, name: &str, index: u8) -> Option<&LedgerField> {
        self.ledger
            .get(name)
            .copied()
            .filter(|f| f.index == index && slot_path(f))
    }
    fn value(&mut self, value: &Expr, pure: bool) -> bool {
        match value {
            Expr::Parameter { .. }
            | Expr::Boolean { .. }
            | Expr::FieldLiteral { .. }
            | Expr::BytesLiteral { .. }
            | Expr::UnsignedLiteral { .. }
            | Expr::EnumVariant { .. } => true,
            Expr::Coerce { value, ty } => value_type(ty) && self.value(value, pure),
            Expr::StructField { value, .. }
            | Expr::JubjubPointX { value }
            | Expr::JubjubPointY { value } => self.value(value, pure),
            Expr::StructLiteral { ty, fields } => {
                value_type(ty) && fields.iter().all(|v| self.value(v, pure))
            }
            Expr::Equal { left, right } => self.value(left, pure) && self.value(right, pure),
            // This profile only needs Field inequality in stateful guards.
            // The shared typed leaf checks both actual operand types.
            Expr::NotEqual { left, right } if !pure => {
                self.value(left, false) && self.value(right, false)
            }
            Expr::If {
                condition,
                then,
                otherwise,
            } => {
                self.value(condition, pure) && self.value(then, pure) && self.value(otherwise, pure)
            }
            Expr::Let { bindings, body } => {
                bindings
                    .iter()
                    .all(|b| value_type(&b.ty) && self.value(&b.value, pure))
                    && self.value(body, pure)
            }
            // Pure helpers use their existing typed renderer, not a second hash evaluator.
            Expr::TransientHash { value } if pure => self.value(value, true),
            Expr::Vector { elements, .. } | Expr::Tuple { elements } if pure => {
                elements.iter().all(|v| self.value(v, true))
            }
            Expr::CellRead { field, index } if !pure => {
                if !matches!(self.slot(field,*index).map(|f| &f.declaration), Some(LedgerFieldKind::Cell {ty}) if observed_value_type(ty))
                {
                    return false;
                }
                self.public += 1;
                true
            }
            Expr::SetMember {
                field,
                index,
                value,
            } if !pure => {
                if !matches!(
                    self.slot(field, *index).map(|f| &f.declaration),
                    Some(LedgerFieldKind::Set {
                        ty: Type::OpaqueString
                    })
                ) {
                    return false;
                }
                self.public += 1;
                self.value(value, false)
            }
            Expr::CounterRead { field, index } if !pure => {
                if !matches!(
                    self.slot(field, *index).map(|f| &f.declaration),
                    Some(LedgerFieldKind::Counter)
                ) {
                    return false;
                }
                self.public += 1;
                true
            }
            Expr::WitnessCall { name, arguments } if !pure => {
                self.witnesses.get(name.as_str()).is_some_and(|w| {
                    observed_value_type(&w.result)
                        && w.parameters.iter().all(|p| observed_value_type(&p.ty))
                        && w.parameters.len() == arguments.len()
                }) && arguments.iter().all(|v| self.value(v, false))
            }
            Expr::Call { name, arguments } => {
                arguments.iter().all(|v| self.value(v, pure)) && self.call(name, pure)
            }
            _ => false,
        }
    }
    // Pure Unit guards have a statement-shaped grammar. Keep that capability
    // separate from value helpers; the existing pure renderer checks all types.
    fn unit_guard(&mut self, value: &Expr) -> bool {
        match value {
            Expr::Unit => true,
            Expr::Sequence { steps, value } => {
                steps.iter().all(|step| self.unit_guard(step)) && self.unit_guard(value)
            }
            Expr::Assert { condition, .. } => self.value(condition, true),
            Expr::If {
                condition,
                then,
                otherwise,
            } => self.value(condition, true) && self.unit_guard(then) && self.unit_guard(otherwise),
            Expr::Let { bindings, body } => {
                bindings
                    .iter()
                    .all(|binding| value_type(&binding.ty) && self.value(&binding.value, true))
                    && self.unit_guard(body)
            }
            Expr::Call { name, arguments } => {
                arguments.iter().all(|argument| self.value(argument, true))
                    && self.call(name, true)
                    && matches!(self.calls.get(name), Some(AuditedCall::PureUnitGuard(_)))
            }
            _ => false,
        }
    }
    fn call(&mut self, name: &str, pure_only: bool) -> bool {
        if let Some(call) = self.calls.get(name) {
            return !pure_only
                || matches!(
                    call,
                    AuditedCall::PureValue(_) | AuditedCall::PureUnitGuard(_)
                );
        }
        if !self.active.insert(name.to_owned()) {
            return false;
        }
        let accepted = match (
            self.pure.get(name).copied(),
            self.circuits.get(name).copied(),
        ) {
            (Some(c), None) => {
                let parameters: HashMap<_, _> = c
                    .parameters
                    .iter()
                    .enumerate()
                    .map(|(i, p)| {
                        (
                            p.name.as_str(),
                            (
                                &p.ty,
                                syn::Ident::new(
                                    &format!("__compact_composition_{i}"),
                                    Span::call_site(),
                                ),
                            ),
                        )
                    })
                    .collect();
                if parameters.len() != c.parameters.len()
                    || !(value_type(&c.result) || c.result == Type::Unit)
                    || !c.parameters.iter().all(|p| value_type(&p.ty))
                    || !(if c.result == Type::Unit {
                        self.unit_guard(&c.body)
                    } else {
                        self.value(&c.body, true)
                    })
                    || !matches!(expression_with_calls(&c.body,&parameters,self.pure),Ok((_,ty)) if ty==c.result)
                {
                    false
                } else {
                    let call = if c.result == Type::Unit {
                        AuditedCall::PureUnitGuard(c)
                    } else {
                        AuditedCall::PureValue(c)
                    };
                    self.calls.insert(name.to_owned(), call);
                    true
                }
            }
            (None, Some(c))
                if !pure_only
                    && c.result == Type::Unit
                    && c.return_value == StateReturn::Unit
                    && c.parameters.iter().all(|p| value_type(&p.ty)) =>
            {
                if super::super::audited_local::unit_helper(c, self.witnesses, self.circuits) {
                    self.calls
                        .insert(name.to_owned(), AuditedCall::LocalUnit(c));
                    true
                } else if c.actions.iter().all(|a| self.action(a)) {
                    self.calls
                        .insert(name.to_owned(), AuditedCall::RecordedUnit(c));
                    true
                } else {
                    false
                }
            }
            _ => false,
        };
        self.active.remove(name);
        accepted
    }
    fn action(&mut self, a: &StateAction) -> bool {
        match a {
            StateAction::Sequence { actions } => actions.iter().all(|a| self.action(a)),
            StateAction::Let { bindings, action } => {
                bindings
                    .iter()
                    .all(|b| value_type(&b.ty) && self.value(&b.value, false))
                    && self.action(action)
            }
            StateAction::If {
                condition,
                then,
                otherwise,
            } => self.value(condition, false) && self.action(then) && self.action(otherwise),
            StateAction::Assert { condition, .. } => self.value(condition, false),
            StateAction::CellWrite {
                field,
                index,
                value,
            } => {
                if !matches!(self.slot(field,*index).map(|f|&f.declaration),Some(LedgerFieldKind::Cell {ty}) if write_type(ty))
                {
                    return false;
                }
                self.public += 1;
                self.value(value, false)
            }
            StateAction::CounterIncrement { field, index, .. } => {
                if !matches!(
                    self.slot(field, *index).map(|f| &f.declaration),
                    Some(LedgerFieldKind::Counter)
                ) {
                    return false;
                }
                self.public += 1;
                true // Literal/parameter operands remain checked by the shared typed Plan.
            }
            StateAction::SetInsert {
                field,
                index,
                value,
            }
            | StateAction::SetRemove {
                field,
                index,
                value,
            } => {
                if !matches!(
                    self.slot(field, *index).map(|f| &f.declaration),
                    Some(LedgerFieldKind::Set {
                        ty: Type::OpaqueString
                    })
                ) {
                    return false;
                }
                self.public += 1;
                self.value(value, false)
            }
            StateAction::PureCall { name, arguments } => {
                arguments.iter().all(|value| self.value(value, false))
                    && self.call(name, true)
                    && matches!(self.calls.get(name), Some(AuditedCall::PureUnitGuard(_)))
            }
            StateAction::CircuitCall { name, arguments } => {
                arguments.iter().all(|v| self.value(v, false))
                    && self.call(name, false)
                    && matches!(
                        self.calls.get(name),
                        Some(AuditedCall::LocalUnit(_) | AuditedCall::RecordedUnit(_))
                    )
            }
            _ => false,
        }
    }
}

pub(super) fn lower<'a>(
    circuit: &StatefulCircuit,
    ledger: &'a HashMap<&'a str, &'a LedgerField>,
    witnesses: &'a HashMap<&'a str, &'a WitnessDeclaration>,
    pure: &'a HashMap<&'a str, &'a PureCircuit>,
    circuits: &'a HashMap<&'a str, &'a StatefulCircuit>,
) -> Option<TypedPlan> {
    if circuit.result != Type::Unit
        || circuit.return_value != StateReturn::Unit
        || !circuit.parameters.iter().all(|p| value_type(&p.ty))
    {
        return None;
    }
    let mut audit = Audit {
        ledger,
        witnesses,
        pure,
        circuits,
        active: HashSet::from([circuit.name.clone()]),
        calls: HashMap::new(),
        public: 0,
    };
    if !circuit.actions.iter().all(|a| audit.action(a))
        || audit.public == 0
        || !audit.calls.values().any(|call| {
            matches!(
                call,
                AuditedCall::LocalUnit(_) | AuditedCall::RecordedUnit(_)
            )
        })
    {
        return None;
    }
    let mut plan = Plan {
        ledger,
        witnesses,
        pure,
        next: 0,
        witness_calls: 0,
        kernel_self_reads: 0,
        context_query: false,
        root_observations: 0,
        tree_writes: 0,
        set_writes: 0,
        counter_writes: 0,
        counter_reads: 0,
        counter_comparisons: 0,
        cell_reads: 0,
        cell_writes: 0,
        field_cell_writes: 0,
        field_cell_slot: None,
        effectful_field_cells: false,
        read_only_assertions: false,
        unit_actions: false,
        phase_reset: false,
        composite_domain: CompositeDomain::UnitComposition,
        composition_calls: Some(audit.calls),
        intent_effects: 0,
        intent_queries: 0,
        zswap_inputs: 0,
        zswap_outputs: 0,
        counter_hash_helpers: false,
        scalar_arguments: false,
        scalar_body_depth: 0,
        scalar_helper_calls: 0,
        scalar_counter_reads: 0,
        active_calls: HashSet::new(),
        stateful_circuits: Some(circuits),
        optional_cells: 0,
        opaque_cells: 0,
        historic_roots: 0,
        historic_writes: 0,
        qualified_set_reads: 0,
        qualified_set_writes: 0,
        qualified_cell_writes: 0,
    };
    let scope: Scope = circuit
        .parameters
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let name = syn::Ident::new(&format!("__compact_param_{i}"), Span::call_site());
            (
                p.name.clone(),
                TypedValue {
                    ty: p.ty.clone(),
                    value: syn::parse_quote!(#name),
                },
            )
        })
        .collect();
    if scope.len() != circuit.parameters.len() {
        return None;
    }
    let mut steps = Vec::new();
    for action in &circuit.actions {
        plan.action(action, &scope, &mut steps)?;
    }
    Some(TypedPlan {
        steps,
        result: syn::parse_quote!(()),
    })
}

impl Plan<'_> {
    pub(super) fn composition_point_coordinate(
        &mut self,
        expression: &Expr,
        input: &Expr,
        scope: &Scope,
        steps: &mut Vec<syn::Stmt>,
    ) -> Option<TypedValue> {
        let point = self.expression(input, scope, steps)?;
        if point.ty != Type::JubjubPoint {
            return None;
        }
        let point = point.value;
        let value = match expression {
            Expr::JubjubPointX { .. } => syn::parse_quote!(runtime::jubjub_point_x(#point)),
            Expr::JubjubPointY { .. } => syn::parse_quote!(runtime::jubjub_point_y(#point)),
            _ => return None,
        };
        self.bind(value, Type::Field, steps)
    }

    pub(super) fn composition_field_not_equal(
        &mut self,
        left: &Expr,
        right: &Expr,
        scope: &Scope,
        steps: &mut Vec<syn::Stmt>,
    ) -> Option<TypedValue> {
        let left = self.expression(left, scope, steps)?;
        let right = self.expression(right, scope, steps)?;
        if left.ty != Type::Field || right.ty != Type::Field {
            return None;
        }
        let (left, right) = (left.value, right.value);
        self.bind(syn::parse_quote!(#left != #right), Type::Boolean, steps)
    }
}
