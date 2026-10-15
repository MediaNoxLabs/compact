// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Frozen whole-composite admission oracle, retained only for inclusion tests.
use super::*;

fn composite_type(ty: &Type, intents: bool) -> bool {
    match ty {
        Type::Unit if intents => true,
        Type::Boolean | Type::Bytes { length: 32 } => true,
        Type::Unsigned { max } => matches!(
            crate::unsigned_maximum(max),
            Ok(crate::UnsignedMaximum::Small(_))
        ),
        Type::Struct { fields, .. } => {
            !fields.is_empty()
                && fields
                    .iter()
                    .all(|field| composite_type(&field.ty, intents))
        }
        _ => false,
    }
}
fn composite_value(
    value: &Expr,
    witnesses: &HashMap<&str, &WitnessDeclaration>,
    circuits: &HashMap<&str, &StatefulCircuit>,
    visiting: &mut HashSet<String>,
    intents: bool,
) -> bool {
    match value {
        Expr::CreateZswapInput { coin } if intents => {
            composite_value(coin, witnesses, circuits, visiting, intents)
        }
        Expr::CreateZswapOutput { coin, recipient } if intents => [coin, recipient]
            .iter()
            .all(|v| composite_value(v, witnesses, circuits, visiting, intents)),
        Expr::KernelClaim { value, .. } if intents => {
            composite_value(value, witnesses, circuits, visiting, intents)
        }
        Expr::Parameter { .. } | Expr::Boolean { .. } => true,
        Expr::UnsignedLiteral { max, .. } => {
            composite_type(&Type::Unsigned { max: max.clone() }, intents)
        }
        Expr::KernelSelf { ty } => *ty == contract_address_type(),
        Expr::Default {
            ty: ty @ Type::Struct { .. },
        } => composite_type(ty, intents),
        Expr::StructLiteral { ty, fields } => {
            composite_type(ty, intents)
                && fields
                    .iter()
                    .all(|field| composite_value(field, witnesses, circuits, visiting, intents))
        }
        Expr::Coerce { value, ty } => {
            composite_type(ty, intents)
                && composite_value(value, witnesses, circuits, visiting, intents)
        }
        Expr::UnsignedCast { value, max } => {
            composite_type(&Type::Unsigned { max: max.clone() }, intents)
                && composite_value(value, witnesses, circuits, visiting, intents)
        }
        Expr::If {
            condition,
            then,
            otherwise,
        } => [condition, then, otherwise]
            .iter()
            .all(|v| composite_value(v, witnesses, circuits, visiting, intents)),
        Expr::Let { bindings, body } => {
            bindings.iter().all(|binding| {
                composite_type(&binding.ty, intents)
                    && composite_value(&binding.value, witnesses, circuits, visiting, intents)
            }) && composite_value(body, witnesses, circuits, visiting, intents)
        }
        Expr::WitnessCall { name, arguments } => witnesses.get(name.as_str()).is_some_and(|w| {
            w.result
                == (Type::Unsigned {
                    max: u64::MAX.to_string(),
                })
                && w.parameters
                    .iter()
                    .all(|p| p.ty == (Type::Unsigned { max: "255".into() }))
                && arguments
                    .iter()
                    .all(|a| composite_value(a, witnesses, circuits, visiting, intents))
        }),
        Expr::Call { name, arguments } => {
            if !arguments
                .iter()
                .all(|a| composite_value(a, witnesses, circuits, visiting, intents))
                || !visiting.insert(name.clone())
            {
                return false;
            }
            let valid = circuits.get(name.as_str()).is_some_and(|callee| {
                composite_circuit(callee, witnesses, circuits, visiting, intents)
            });
            visiting.remove(name);
            valid
        }
        _ => false,
    }
}
fn composite_circuit(
    circuit: &StatefulCircuit,
    witnesses: &HashMap<&str, &WitnessDeclaration>,
    circuits: &HashMap<&str, &StatefulCircuit>,
    visiting: &mut HashSet<String>,
    intents: bool,
) -> bool {
    circuit.actions.is_empty()
        && matches!(circuit.result, Type::Struct { .. })
        && composite_type(&circuit.result, intents)
        && circuit.parameters.iter().all(|p| {
            p.ty == Type::Boolean
                || (intents
                    && (p.ty == shielded_coin_type()
                        || p.ty == qualified_coin_type()
                        || p.ty == shielded_recipient_type()
                        || p.ty == (Type::Bytes { length: 32 })))
        })
        && matches!(&circuit.return_value, StateReturn::Expression { value } if composite_value(value,witnesses,circuits,visiting,intents))
}

pub(super) fn lower<'a>(
    circuit: &StatefulCircuit,
    ledger: &'a HashMap<&'a str, &'a LedgerField>,
    witnesses: &'a HashMap<&'a str, &'a WitnessDeclaration>,
    pure: &'a HashMap<&'a str, &'a PureCircuit>,
    circuits: &'a HashMap<&'a str, &'a StatefulCircuit>,
) -> Option<TypedPlan> {
    let audit = |intents| {
        composite_circuit(
            circuit,
            witnesses,
            circuits,
            &mut HashSet::from([circuit.name.clone()]),
            intents,
        )
    };
    let domain = if audit(false) {
        CompositeDomain::Values
    } else if audit(true) {
        CompositeDomain::Intents
    } else if field_observations::audit(circuit, ledger, pure, circuits) {
        CompositeDomain::FieldObservations
    } else {
        return None;
    };
    let mut plan = Plan {
        declarations: Declarations {
            ledger,
            witnesses,
            pure,
            stateful_circuits: Some(circuits),
        },
        evidence: EffectEvidence::default(),
        admission: AdmissionPolicy {
            field_cell_slot: None,
            effectful_field_cells: false,
            read_only_assertions: false,
            unit_actions: false,
            phase_reset: false,
            composite_domain: domain,
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
    let StateReturn::Expression { value } = &circuit.return_value else {
        return None;
    };
    let mut steps = Vec::new();
    let result = plan.expression(value, &scope, &mut steps)?;
    (result.ty == circuit.result
        && (domain != CompositeDomain::Intents
            || (plan.evidence.intent_effects > 0
                && plan.evidence.kernel_self_reads + plan.evidence.intent_queries > 0))
        && (domain != CompositeDomain::FieldObservations || plan.evidence.cell_reads > 0))
        .then_some(TypedPlan {
            steps,
            result: result.value,
        })
}
