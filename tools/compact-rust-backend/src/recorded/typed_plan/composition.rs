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
    ReadOnlyBoolean(&'a StatefulCircuit),
}
pub(super) type Calls<'a> = HashMap<String, AuditedCall<'a>>;

// The frontend may retain a Boolean Map membership as a specialized return
// rather than Expr::MapMember. Both forms use the one shared typed evaluator.
pub(super) fn boolean_result(circuit: &StatefulCircuit) -> Option<Expr> {
    match &circuit.return_value {
        StateReturn::Expression { value } => Some(value.clone()),
        StateReturn::MapMember { field, index, key } => Some(Expr::MapMember {
            field: field.clone(),
            index: *index,
            key: Box::new(key.clone()),
        }),
        _ => None,
    }
}

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
// A declared, key-only membership query never reads or constructs the Map value.
// Keep this independent from the narrower value mutation domain.
pub(super) fn string_key_map(declaration: &LedgerFieldKind) -> bool {
    matches!(
        declaration,
        LedgerFieldKind::Map {
            key: Type::OpaqueString,
            ..
        }
    )
}

// Mutation accepts nonempty flat named products of previously audited scalar
// values. The declared struct name/field order remain exact typed data, never
// an admission dispatch key.
pub(super) fn flat_string_point_product(ty: &Type) -> bool {
    matches!(ty, Type::Struct { fields, .. }
        if !fields.is_empty() && fields.iter().all(|field|
            matches!(field.ty, Type::OpaqueString | Type::JubjubPoint)))
}

pub(super) fn flat_string_point_map(declaration: &LedgerFieldKind) -> bool {
    matches!(declaration, LedgerFieldKind::Map { key: Type::OpaqueString, value }
        if flat_string_point_product(value))
}

// Map mutation serializes the declared value. Reuse the generated CellValue
// implementations for checked named products, including nested products and
// enums, while excluding container and unreviewed scalar leaves. Key-only
// membership of nested products outside the read-only Boolean helper is
// allowed only on the same declared Map mutated by this composition.
fn checked_product_value(ty: &Type) -> bool {
    match ty {
        Type::Struct { fields, .. } => {
            !fields.is_empty() && fields.iter().all(|field| checked_product_value(&field.ty))
        }
        Type::OpaqueString | Type::JubjubPoint | Type::Enum { .. } => true,
        _ => false,
    }
}

pub(super) fn checked_product_map(declaration: &LedgerFieldKind) -> bool {
    matches!(declaration, LedgerFieldKind::Map { key: Type::OpaqueString, value }
        if matches!(value, Type::Struct { .. }) && checked_product_value(value))
}
pub(super) fn slot_path(field: &LedgerField) -> bool {
    let path = field.physical_path();
    // The contract-level ledger validator owns physical layout/uniqueness.
    // Retain that complete declared path rather than reconstructing an index.
    path.first() == Some(&field.index) && (1..=2).contains(&path.len())
}
/// Failure evidence stays private: capability JSON retains its existing schema.
#[derive(Debug)]
pub(in crate::recorded) struct CompositionRejection {
    pub(super) gap: RecordingGap,
    root_action: Option<usize>,
    precision: RejectionPrecision,
    phase: RejectionPhase,
}
#[derive(Debug, PartialEq, Eq)]
enum RejectionPrecision {
    ConcreteNode,
    EnclosingAction,
    Obligation,
}
#[derive(Debug, PartialEq, Eq)]
enum RejectionPhase {
    PolicyAudit,
    TypedLowering,
    FinalObligation,
}

impl CompositionRejection {
    /// Refine only the same root action's coarse legacy refusal. Never inspect
    /// diagnostic path/detail text, or override an earlier concrete error.
    pub(in crate::recorded) fn refine_action(
        &self,
        ordinal: usize,
        legacy_precision: super::super::LegacyActionPrecision,
        gap: RecordingGap,
    ) -> RecordingGap {
        if legacy_precision == super::super::LegacyActionPrecision::Coarse
            && self.root_action == Some(ordinal)
            && self.precision == RejectionPrecision::ConcreteNode
            && self.phase == RejectionPhase::PolicyAudit
            && gap.code == super::super::RecordingGapCode::UnsupportedAction
        {
            self.gap.clone()
        } else {
            gap
        }
    }
    fn obligation(detail: &str, no_effect: bool) -> Self {
        let mut gap = RecordingGap::no_effect();
        if !no_effect {
            gap.code = super::super::RecordingGapCode::RecordingUnavailable;
        }
        gap.detail = detail.to_owned();
        Self {
            gap,
            root_action: None,
            precision: RejectionPrecision::Obligation,
            phase: RejectionPhase::FinalObligation,
        }
    }
}

struct Audit<'a> {
    ledger: &'a HashMap<&'a str, &'a LedgerField>,
    witnesses: &'a HashMap<&'a str, &'a WitnessDeclaration>,
    pure: &'a HashMap<&'a str, &'a PureCircuit>,
    circuits: &'a HashMap<&'a str, &'a StatefulCircuit>,
    active: HashSet<String>,
    calls: Calls<'a>,
    public: usize,
    product_map_writes: usize,
    nested_product_map_members: HashSet<(String, u8)>,
    product_map_written_fields: HashSet<(String, u8)>,
    read_only_boolean_depth: usize,
    root_action: usize,
    failure: Option<CompositionRejection>,
}
impl Audit<'_> {
    fn slot(&self, name: &str, index: u8) -> Option<&LedgerField> {
        self.ledger
            .get(name)
            .copied()
            .filter(|f| f.index == index && slot_path(f))
    }
    fn reject(&mut self, gap: RecordingGap) {
        if self.failure.is_none() {
            self.failure = Some(CompositionRejection {
                gap,
                root_action: Some(self.root_action),
                precision: RejectionPrecision::ConcreteNode,
                phase: RejectionPhase::PolicyAudit,
            });
        }
    }
    fn value(&mut self, value: &Expr, pure: bool, path: &str) -> bool {
        let accepted = self.value_inner(value, pure, path);
        if !accepted {
            self.reject(RecordingGap::expression(value, path.to_owned()));
        }
        accepted
    }
    fn value_inner(&mut self, value: &Expr, pure: bool, path: &str) -> bool {
        match value {
            Expr::Parameter { .. }
            | Expr::Boolean { .. }
            | Expr::FieldLiteral { .. }
            | Expr::BytesLiteral { .. }
            | Expr::UnsignedLiteral { .. }
            | Expr::EnumVariant { .. } => true,
            Expr::Coerce { value, ty } => {
                value_type(ty) && self.value(value, pure, &format!("{path}.value"))
            }
            Expr::StructField { value, .. }
            | Expr::JubjubPointX { value }
            | Expr::JubjubPointY { value } => self.value(value, pure, &format!("{path}.value")),
            Expr::StructLiteral { ty, fields } => {
                value_type(ty)
                    && fields
                        .iter()
                        .enumerate()
                        .all(|(i, v)| self.value(v, pure, &format!("{path}.fields[{i}]")))
            }
            Expr::Equal { left, right } => {
                self.value(left, pure, &format!("{path}.left"))
                    && self.value(right, pure, &format!("{path}.right"))
            }
            // This profile admits Field and declared Enum inequality in
            // stateful guards. The shared typed leaf checks both operands.
            Expr::NotEqual { left, right } if !pure => {
                self.value(left, false, &format!("{path}.left"))
                    && self.value(right, false, &format!("{path}.right"))
            }
            Expr::If {
                condition,
                then,
                otherwise,
            } => {
                self.value(condition, pure, &format!("{path}.condition"))
                    && self.value(then, pure, &format!("{path}.then"))
                    && self.value(otherwise, pure, &format!("{path}.otherwise"))
            }
            Expr::Let { bindings, body } => {
                bindings.iter().enumerate().all(|(i, b)| {
                    value_type(&b.ty)
                        && self.value(&b.value, pure, &format!("{path}.bindings[{i}].value"))
                }) && self.value(body, pure, &format!("{path}.body"))
            }
            // Pure helpers use their existing typed renderer, not a second hash evaluator.
            Expr::TransientHash { value } if pure => {
                self.value(value, true, &format!("{path}.value"))
            }
            Expr::Vector { elements, .. } | Expr::Tuple { elements } if pure => elements
                .iter()
                .enumerate()
                .all(|(i, v)| self.value(v, true, &format!("{path}.elements[{i}]"))),
            Expr::CellRead { field, index } if !pure && self.read_only_boolean_depth == 0 => {
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
            } if !pure && self.read_only_boolean_depth == 0 => {
                if !matches!(
                    self.slot(field, *index).map(|f| &f.declaration),
                    Some(LedgerFieldKind::Set {
                        ty: Type::OpaqueString
                    })
                ) {
                    return false;
                }
                self.public += 1;
                self.value(value, false, &format!("{path}.value"))
            }
            Expr::MapMember { field, index, key } if !pure => {
                let Some(slot) = self.slot(field, *index) else {
                    return false;
                };
                if !string_key_map(&slot.declaration)
                    || !(self.read_only_boolean_depth > 0
                        || flat_string_point_map(&slot.declaration)
                        || checked_product_map(&slot.declaration))
                {
                    return false;
                }
                if self.read_only_boolean_depth == 0 && !flat_string_point_map(&slot.declaration) {
                    self.nested_product_map_members
                        .insert((field.clone(), *index));
                }
                self.public += 1;
                self.value(key, false, &format!("{path}.key"))
            }
            Expr::CounterRead { field, index } if !pure && self.read_only_boolean_depth == 0 => {
                if !matches!(
                    self.slot(field, *index).map(|f| &f.declaration),
                    Some(LedgerFieldKind::Counter)
                ) {
                    return false;
                }
                self.public += 1;
                true
            }
            Expr::WitnessCall { name, arguments } if !pure && self.read_only_boolean_depth == 0 => {
                self.witnesses.get(name.as_str()).is_some_and(|w| {
                    observed_value_type(&w.result)
                        && w.parameters.iter().all(|p| observed_value_type(&p.ty))
                        && w.parameters.len() == arguments.len()
                }) && arguments
                    .iter()
                    .enumerate()
                    .all(|(i, v)| self.value(v, false, &format!("{path}.arguments[{i}]")))
            }
            Expr::Call { name, arguments } => {
                arguments
                    .iter()
                    .enumerate()
                    .all(|(i, v)| self.value(v, pure, &format!("{path}.arguments[{i}]")))
                    && self.call(name, pure, path)
            }
            _ => false,
        }
    }
    // Pure Unit guards have a statement-shaped grammar. Keep that capability
    // separate from value helpers; the existing pure renderer checks all types.
    fn unit_guard(&mut self, value: &Expr, path: &str) -> bool {
        let accepted = self.unit_guard_inner(value, path);
        if !accepted {
            self.reject(RecordingGap::expression(value, path.to_owned()));
        }
        accepted
    }
    fn unit_guard_inner(&mut self, value: &Expr, path: &str) -> bool {
        match value {
            Expr::Unit => true,
            Expr::Sequence { steps, value } => {
                steps
                    .iter()
                    .enumerate()
                    .all(|(i, step)| self.unit_guard(step, &format!("{path}.steps[{i}]")))
                    && self.unit_guard(value, &format!("{path}.value"))
            }
            Expr::Assert { condition, .. } => {
                self.value(condition, true, &format!("{path}.condition"))
            }
            Expr::If {
                condition,
                then,
                otherwise,
            } => {
                self.value(condition, true, &format!("{path}.condition"))
                    && self.unit_guard(then, &format!("{path}.then"))
                    && self.unit_guard(otherwise, &format!("{path}.otherwise"))
            }
            Expr::Let { bindings, body } => {
                bindings.iter().enumerate().all(|(i, binding)| {
                    value_type(&binding.ty)
                        && self.value(&binding.value, true, &format!("{path}.bindings[{i}].value"))
                }) && self.unit_guard(body, &format!("{path}.body"))
            }
            Expr::Call { name, arguments } => {
                arguments.iter().enumerate().all(|(i, argument)| {
                    self.value(argument, true, &format!("{path}.arguments[{i}]"))
                }) && self.call(name, true, path)
                    && matches!(self.calls.get(name), Some(AuditedCall::PureUnitGuard(_)))
            }
            _ => false,
        }
    }
    fn call(&mut self, name: &str, pure_only: bool, path: &str) -> bool {
        if let Some(call) = self.calls.get(name) {
            return if pure_only {
                matches!(
                    call,
                    AuditedCall::PureValue(_) | AuditedCall::PureUnitGuard(_)
                )
            } else if self.read_only_boolean_depth > 0 {
                matches!(
                    call,
                    AuditedCall::PureValue(_) | AuditedCall::ReadOnlyBoolean(_)
                )
            } else {
                true
            };
        }
        if !self.active.insert(name.to_owned()) {
            self.declaration_rejection(name, path, "recursive helper call");
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
                        self.unit_guard(&c.body, &format!("{path}.callee[{name:?}].body"))
                    } else {
                        self.value(&c.body, true, &format!("{path}.callee[{name:?}].body"))
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
                    && self.read_only_boolean_depth == 0
                    && c.result == Type::Unit
                    && c.return_value == StateReturn::Unit
                    && c.parameters.iter().all(|p| value_type(&p.ty)) =>
            {
                if super::super::audited_local::unit_helper(c, self.witnesses, self.circuits) {
                    self.calls
                        .insert(name.to_owned(), AuditedCall::LocalUnit(c));
                    true
                } else if c
                    .actions
                    .iter()
                    .enumerate()
                    .all(|(i, a)| self.action(a, &format!("{path}.callee[{name:?}].actions[{i}]")))
                {
                    self.calls
                        .insert(name.to_owned(), AuditedCall::RecordedUnit(c));
                    true
                } else {
                    false
                }
            }
            (None, Some(c))
                if !pure_only
                    && c.result == Type::Boolean
                    && c.actions.is_empty()
                    && c.parameters.iter().all(|p| value_type(&p.ty))
                    && c.parameters
                        .iter()
                        .map(|p| &p.name)
                        .collect::<HashSet<_>>()
                        .len()
                        == c.parameters.len() =>
            {
                let Some(value) = boolean_result(c) else {
                    self.active.remove(name);
                    return false;
                };
                self.read_only_boolean_depth += 1;
                let accepted = self.value(
                    &value,
                    false,
                    &format!("{path}.callee[{name:?}].return_value"),
                );
                self.read_only_boolean_depth -= 1;
                if accepted {
                    self.calls
                        .insert(name.to_owned(), AuditedCall::ReadOnlyBoolean(c));
                }
                accepted
            }
            _ => false,
        };
        self.active.remove(name);
        if !accepted {
            self.declaration_rejection(
                name,
                path,
                "helper declaration or signature is outside Unit composition",
            );
        }
        accepted
    }
    fn declaration_rejection(&mut self, name: &str, path: &str, detail: &str) {
        self.reject(RecordingGap {
            code: super::super::RecordingGapCode::RecordingUnavailable,
            ir_node: "CircuitDeclaration".to_owned(),
            path: path.to_owned(),
            detail: format!("{detail}: {name:?}"),
        });
    }
    fn action(&mut self, a: &StateAction, path: &str) -> bool {
        let accepted = self.action_inner(a, path);
        if !accepted {
            self.reject(RecordingGap::action(a, path.to_owned()));
        }
        accepted
    }
    fn action_inner(&mut self, a: &StateAction, path: &str) -> bool {
        match a {
            StateAction::Sequence { actions } => actions
                .iter()
                .enumerate()
                .all(|(i, a)| self.action(a, &format!("{path}.actions[{i}]"))),
            StateAction::Let { bindings, action } => {
                bindings.iter().enumerate().all(|(i, b)| {
                    value_type(&b.ty)
                        && self.value(&b.value, false, &format!("{path}.bindings[{i}].value"))
                }) && self.action(action, &format!("{path}.action"))
            }
            StateAction::If {
                condition,
                then,
                otherwise,
            } => {
                self.value(condition, false, &format!("{path}.condition"))
                    && self.action(then, &format!("{path}.then"))
                    && self.action(otherwise, &format!("{path}.otherwise"))
            }
            StateAction::Assert { condition, .. } => {
                self.value(condition, false, &format!("{path}.condition"))
            }
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
                self.value(value, false, &format!("{path}.value"))
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
                self.value(value, false, &format!("{path}.value"))
            }
            StateAction::MapInsert {
                field,
                index,
                key,
                value,
            } => {
                if !self
                    .slot(field, *index)
                    .is_some_and(|slot| checked_product_map(&slot.declaration))
                {
                    return false;
                }
                self.public += 1;
                self.product_map_writes += 1;
                self.product_map_written_fields
                    .insert((field.clone(), *index));
                self.value(key, false, &format!("{path}.key"))
                    && self.value(value, false, &format!("{path}.value"))
            }
            StateAction::MapRemove { field, index, key } => {
                if !self
                    .slot(field, *index)
                    .is_some_and(|slot| checked_product_map(&slot.declaration))
                {
                    return false;
                }
                self.public += 1;
                self.product_map_writes += 1;
                self.product_map_written_fields
                    .insert((field.clone(), *index));
                self.value(key, false, &format!("{path}.key"))
            }
            StateAction::PureCall { name, arguments } => {
                arguments
                    .iter()
                    .enumerate()
                    .all(|(i, value)| self.value(value, false, &format!("{path}.arguments[{i}]")))
                    && self.call(name, true, path)
                    && matches!(self.calls.get(name), Some(AuditedCall::PureUnitGuard(_)))
            }
            StateAction::CircuitCall { name, arguments } => {
                arguments
                    .iter()
                    .enumerate()
                    .all(|(i, v)| self.value(v, false, &format!("{path}.arguments[{i}]")))
                    && self.call(name, false, path)
                    && matches!(
                        self.calls.get(name),
                        Some(AuditedCall::LocalUnit(_) | AuditedCall::RecordedUnit(_))
                    )
            }
            _ => false,
        }
    }
}

pub(super) fn lower_checked<'a>(
    circuit: &StatefulCircuit,
    ledger: &'a HashMap<&'a str, &'a LedgerField>,
    witnesses: &'a HashMap<&'a str, &'a WitnessDeclaration>,
    pure: &'a HashMap<&'a str, &'a PureCircuit>,
    circuits: &'a HashMap<&'a str, &'a StatefulCircuit>,
) -> ProfileAttempt<TypedPlan, CompositionRejection> {
    if circuit.result != Type::Unit
        || circuit.return_value != StateReturn::Unit
        || !circuit.parameters.iter().all(|p| value_type(&p.ty))
    {
        return ProfileAttempt::NotApplicable;
    }
    let mut audit = Audit {
        ledger,
        witnesses,
        pure,
        circuits,
        active: HashSet::from([circuit.name.clone()]),
        calls: HashMap::new(),
        public: 0,
        product_map_writes: 0,
        nested_product_map_members: HashSet::new(),
        product_map_written_fields: HashSet::new(),
        read_only_boolean_depth: 0,
        root_action: 0,
        failure: None,
    };
    for (index, action) in circuit.actions.iter().enumerate() {
        audit.root_action = index;
        if !audit.action(action, &format!("actions[{index}]")) {
            return ProfileAttempt::Rejected(
                audit.failure.expect("failed audit records its location"),
            );
        }
    }
    if audit.public == 0 {
        return ProfileAttempt::Rejected(CompositionRejection::obligation(
            "Unit composition has no replayable ledger read or write",
            true,
        ));
    }
    if !audit.calls.values().any(|call| {
        matches!(
            call,
            AuditedCall::LocalUnit(_) | AuditedCall::RecordedUnit(_)
        )
    }) {
        return ProfileAttempt::Rejected(CompositionRejection::obligation(
            "Unit composition requires an audited stateful Unit helper",
            false,
        ));
    }
    if (audit
        .calls
        .values()
        .any(|call| matches!(call, AuditedCall::ReadOnlyBoolean(_)))
        && audit.product_map_writes == 0)
        || !audit
            .nested_product_map_members
            .is_subset(&audit.product_map_written_fields)
    {
        return ProfileAttempt::Rejected(CompositionRejection::obligation(
            "Map membership requires the declared Map mutation domain",
            false,
        ));
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
        return ProfileAttempt::Rejected(CompositionRejection::obligation(
            "duplicate Unit composition parameter binding",
            false,
        ));
    }
    let mut steps = Vec::new();
    for (index, action) in circuit.actions.iter().enumerate() {
        if plan.action(action, &scope, &mut steps).is_none() {
            return ProfileAttempt::Rejected(CompositionRejection {
                gap: RecordingGap::action(action, format!("actions[{index}]")),
                root_action: Some(index),
                precision: RejectionPrecision::EnclosingAction,
                phase: RejectionPhase::TypedLowering,
            });
        }
    }
    ProfileAttempt::Admitted(TypedPlan {
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

    pub(super) fn composition_scalar_not_equal(
        &mut self,
        left: &Expr,
        right: &Expr,
        scope: &Scope,
        steps: &mut Vec<syn::Stmt>,
    ) -> Option<TypedValue> {
        let left = self.expression(left, scope, steps)?;
        let right = self.expression(right, scope, steps)?;
        if left.ty != right.ty || !matches!(left.ty, Type::Field | Type::Enum { .. }) {
            return None;
        }
        let (left, right) = (left.value, right.value);
        self.bind(syn::parse_quote!(#left != #right), Type::Boolean, steps)
    }
}

#[cfg(test)]
mod tests;
