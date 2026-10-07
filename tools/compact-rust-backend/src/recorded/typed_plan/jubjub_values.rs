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
//! Closed typed Jubjub values with one checked point Cell write.
//! Admission is structural; Plan owns evaluation, lexical scope and recording.
use super::*;

fn value_type(ty: &Type) -> bool {
    matches!(ty, Type::Field | Type::JubjubPoint)
}
fn value(v: &Expr) -> bool {
    match v {
        Expr::Parameter { .. } | Expr::FieldLiteral { .. } => true,
        Expr::JubjubScalarFromNative { value: v } | Expr::EcMulGenerator { scalar: v } => value(v),
        Expr::EcMul { point, scalar } => value(point) && value(scalar),
        Expr::EcAdd { left, right } => value(left) && value(right),
        Expr::Let { bindings: b, body } => bindings(b) && value(body),
        _ => false,
    }
}
fn bindings(b: &[LocalBinding]) -> bool {
    b.iter().all(|b| value_type(&b.ty) && value(&b.value))
}
struct Audit<'a> {
    ledger: &'a HashMap<&'a str, &'a LedgerField>,
    writes: usize,
}
impl Audit<'_> {
    fn action(&mut self, a: &StateAction) -> bool {
        match a {
            StateAction::Sequence { actions } => actions.iter().all(|a| self.action(a)),
            StateAction::Let {
                bindings: b,
                action,
            } => bindings(b) && self.action(action),
            StateAction::CellWrite {
                field,
                index,
                value: v,
            } => {
                self.writes += 1;
                self.writes == 1
                    && self.ledger.get(field.as_str()).is_some_and(|f| {
                        f.index == *index
                            && f.physical_path() == [*index]
                            && f.declaration
                                == (LedgerFieldKind::Cell {
                                    ty: Type::JubjubPoint,
                                })
                    })
                    && value(v)
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
    let StateReturn::Expression { value: returned } = &circuit.return_value else {
        return None;
    };
    let mut audit = Audit { ledger, writes: 0 };
    if circuit.result != Type::JubjubPoint
        || !circuit.parameters.iter().all(|p| value_type(&p.ty))
        || !circuit.actions.iter().all(|a| audit.action(a))
        || audit.writes != 1
        || !value(returned)
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
        composite_domain: CompositeDomain::JubjubValues,
        composition_calls: None,
        composition_lookup_sites: Default::default(),
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
        .map(|(index, p)| {
            let name = syn::Ident::new(&format!("__compact_param_{index}"), Span::call_site());
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
    let body = terminal_returns::adapt(&circuit.actions, returned);
    let result = plan.return_plan(&body, &scope, &mut steps)?;
    (result.ty == Type::JubjubPoint && plan.cell_writes == 1).then_some(TypedPlan {
        steps,
        result: result.value,
    })
}

/// Each child is evaluated once, left to right, before shared syntax lowering.
pub(super) fn expression(
    plan: &mut Plan<'_>,
    expr: &Expr,
    scope: &Scope,
    steps: &mut Vec<syn::Stmt>,
) -> Option<TypedValue> {
    if plan.composite_domain != CompositeDomain::JubjubValues {
        return None;
    }
    let mut operand = |v: &Expr| {
        let v = plan.expression(v, scope, steps)?;
        Some((v.value, v.ty))
    };
    use crate::jubjub_value::Operation;
    let operation = match expr {
        Expr::JubjubScalarFromNative { value } => Operation::Reduce(operand(value)?),
        Expr::EcMulGenerator { scalar } => Operation::Generator(operand(scalar)?),
        Expr::EcMul { point, scalar } => Operation::Multiply(operand(point)?, operand(scalar)?),
        Expr::EcAdd { left, right } => Operation::Add(operand(left)?, operand(right)?),
        _ => return None,
    };
    let (value, ty) = operation.lower().ok()?;
    plan.bind(value, ty, steps)
}

#[cfg(test)]
mod tests;
