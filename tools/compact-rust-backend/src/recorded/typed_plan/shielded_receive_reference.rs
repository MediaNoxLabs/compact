// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Frozen whole-family receive admission oracle; production uses checked composition.
use super::*;

pub(super) fn lower<'a>(
    circuit: &StatefulCircuit,
    ledger: &'a HashMap<&'a str, &'a LedgerField>,
    witnesses: &'a HashMap<&'a str, &'a WitnessDeclaration>,
    pure: &'a HashMap<&'a str, &'a PureCircuit>,
    circuits: &'a HashMap<&'a str, &'a StatefulCircuit>,
) -> Option<TypedPlan> {
    if !shielded_unit_signature(circuit)
        || !matches!(
            circuit.actions.as_slice(),
            [StateAction::CircuitCall { .. }]
        )
        || !circuit.actions.iter().all(|action| {
            shielded_receive_action(
                action,
                pure,
                circuits,
                &mut HashSet::from([circuit.name.clone()]),
            )
        })
    {
        return None;
    }
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
            unit_actions: true,
            phase_reset: false,
            composite_domain: CompositeDomain::ShieldedReceive,
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
    let parameter = &circuit.parameters[0];
    let scope = Scope::from([(
        parameter.name.clone(),
        TypedValue {
            ty: parameter.ty.clone(),
            value: syn::parse_quote!(__compact_param_0),
        },
    )]);
    let mut steps = Vec::new();
    for action in &circuit.actions {
        plan.action(action, &scope, &mut steps)?;
    }
    (plan.evidence.kernel_self_reads == 1
        && plan.evidence.intent_effects == 1
        && plan.evidence.intent_queries == 1
        && plan.evidence.witness_calls == 0
        && plan.evidence.cell_reads == 0
        && plan.evidence.cell_writes == 0
        && plan.evidence.tree_writes == 0
        && plan.evidence.set_writes == 0
        && plan.evidence.counter_reads == 0
        && plan.evidence.counter_writes == 0)
        .then_some(TypedPlan {
            steps,
            result: syn::parse_quote!(()),
        })
}
