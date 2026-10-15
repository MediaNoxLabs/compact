// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Retired effectful-return admission oracle, compiled only for regression tests.
//! Production recording never calls this recognizer. Keep its original behavior
//! while finite inclusion and generated-runtime comparisons use it as evidence.
use super::*;

// Bounded admission is separate from the existing single-slot root-Let profile.
fn effectful_value(value: &Expr, witnesses: &HashMap<&str, &WitnessDeclaration>) -> bool {
    match value {
        Expr::Parameter { .. }
        | Expr::FieldLiteral { .. }
        | Expr::Boolean { .. }
        | Expr::UnsignedLiteral { .. }
        | Expr::CellRead { .. } => true,
        Expr::Add { left, right } | Expr::Equal { left, right } => {
            effectful_value(left, witnesses) && effectful_value(right, witnesses)
        }
        Expr::If {
            condition,
            then,
            otherwise,
        } => {
            effectful_value(condition, witnesses)
                && effectful_value(then, witnesses)
                && effectful_value(otherwise, witnesses)
        }
        Expr::Let { bindings, body } => {
            effectful_bindings(bindings, witnesses) && effectful_value(body, witnesses)
        }
        Expr::Coerce {
            value,
            ty: Type::Field | Type::Boolean,
        } => effectful_value(value, witnesses),
        Expr::FieldCast { value } => effectful_value(value, witnesses),
        Expr::WitnessCall { name, arguments } => witnesses.get(name.as_str()).is_some_and(|w| {
            w.result == Type::Field
                && w.parameters.iter().all(|p| p.ty == Type::Field)
                && arguments.iter().all(|a| effectful_value(a, witnesses))
        }),
        _ => false,
    }
}
fn effectful_bindings(
    bindings: &[LocalBinding],
    witnesses: &HashMap<&str, &WitnessDeclaration>,
) -> bool {
    bindings.iter().all(|b| {
        (matches!(b.ty, Type::Field | Type::Boolean)
            || matches!(
                (&b.ty, &b.value),
                (Type::Unsigned { .. }, Expr::UnsignedLiteral { .. })
            ))
            && effectful_value(&b.value, witnesses)
    })
}
fn effectful_action(action: &StateAction, witnesses: &HashMap<&str, &WitnessDeclaration>) -> bool {
    match action {
        StateAction::CellWrite { value, .. } => effectful_value(value, witnesses),
        StateAction::Sequence { actions } => actions.iter().all(|a| effectful_action(a, witnesses)),
        StateAction::Let { bindings, action } => {
            effectful_bindings(bindings, witnesses) && effectful_action(action, witnesses)
        }
        _ => false,
    }
}
fn effectful_body(
    body: &ReturnPlan,
    witnesses: &HashMap<&str, &WitnessDeclaration>,
    branches: &mut usize,
) -> bool {
    match body {
        ReturnPlan::Value { value } => effectful_value(value, witnesses),
        ReturnPlan::Let { bindings, result } => {
            effectful_bindings(bindings, witnesses) && effectful_body(result, witnesses, branches)
        }
        ReturnPlan::Sequence { actions, result } => {
            actions.iter().all(|a| effectful_action(a, witnesses))
                && effectful_body(result, witnesses, branches)
        }
        ReturnPlan::Conditional {
            condition,
            then,
            otherwise,
        } => {
            *branches += 1;
            effectful_value(condition, witnesses)
                && effectful_body(then, witnesses, branches)
                && effectful_body(otherwise, witnesses, branches)
        }
    }
}

pub(in crate::recorded) fn lower_effectful<'a>(
    circuit: &StatefulCircuit,
    ledger: &'a HashMap<&'a str, &'a LedgerField>,
    witnesses: &'a HashMap<&'a str, &'a WitnessDeclaration>,
    pure: &'a HashMap<&'a str, &'a PureCircuit>,
) -> Option<TypedPlan> {
    let StateReturn::Effectful { body } = &circuit.return_value else {
        return None;
    };
    let mut branches = 0;
    if !circuit.actions.is_empty()
        || circuit.result != Type::Field
        || !circuit.parameters.iter().all(|p| p.ty == Type::Field)
        || !effectful_body(body, witnesses, &mut branches)
        || branches == 0
    {
        return None;
    }
    let mut plan = Plan {
        declarations: Declarations {
            ledger,
            witnesses,
            pure,
            stateful_circuits: None,
        },
        evidence: EffectEvidence::default(),
        admission: AdmissionPolicy {
            field_cell_slot: None,
            effectful_field_cells: true,
            read_only_assertions: false,
            unit_actions: false,
            phase_reset: false,
            composite_domain: CompositeDomain::None,
            composition_calls: None,
            composition_lookup_sites: HashSet::new(),
            counter_hash_helpers: false,
        },
        evaluation: EvaluationState {
            next: 0,
            context_query: false,
            scalar_arguments: false,
            scalar_body_depth: 0,
            active_calls: HashSet::new(),
        },
    };
    let scope = circuit
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
    let mut steps = Vec::new();
    let result = plan.return_plan(body, &scope, &mut steps)?;
    if result.ty != circuit.result
        || plan.evidence.cell_reads == 0
        || plan.evidence.cell_writes == 0
        || plan.evidence.cell_writes != plan.evidence.field_cell_writes
        || plan.evidence.optional_cells != 0
        || plan.evidence.opaque_cells != 0
    {
        return None;
    }
    Some(TypedPlan {
        steps,
        result: result.value,
    })
}
