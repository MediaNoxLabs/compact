// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Frozen context-query selector reference; never compiled in production.
use super::*;

pub(super) fn lower<'a>(
    circuit: &StatefulCircuit,
    ledger: &'a HashMap<&'a str, &'a LedgerField>,
    witnesses: &'a HashMap<&'a str, &'a WitnessDeclaration>,
    pure: &'a HashMap<&'a str, &'a PureCircuit>,
) -> Option<TypedPlan> {
    if !circuit.parameters.is_empty()
        || !circuit.actions.is_empty()
        || circuit.result != (Type::Bytes { length: 32 })
    {
        return None;
    }
    let StateReturn::Expression { value } = &circuit.return_value else {
        return None;
    };
    if !context_query_value(value, pure, &mut HashSet::new(), true) {
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
            effectful_field_cells: false,
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
            context_query: true,
            scalar_arguments: false,
            scalar_body_depth: 0,
            active_calls: HashSet::new(),
        },
    };
    let mut steps = Vec::new();
    let result = plan.expression(value, &Scope::new(), &mut steps)?;
    (result.ty == circuit.result
        && plan.evidence.kernel_self_reads > 0
        && plan.evidence.witness_calls == 0)
        .then_some(TypedPlan {
            steps,
            result: result.value,
        })
}
