// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use compact_rust_effect_plan_owned_values_source_fixture::{
    ledger_contract as c, runtime as r,
    support::{Witness, WitnessInput, conversion_reference, reference},
    types,
};
use r::context::{CircuitContext, CircuitResult, ConstructorContext};
use r::recording::RecordedCircuitResult;
use r::{CompactError, Field};
fn context() -> CircuitContext<u64> {
    c::initial_state(ConstructorContext::new(7))
        .unwrap()
        .into_circuit_context(Default::default())
}
fn compare(
    native: &CircuitResult<u64, types::Receipt>,
    recorded: &RecordedCircuitResult<u64, types::Receipt>,
    manual: &RecordedCircuitResult<u64, types::Receipt>,
) {
    for execution in [&recorded.execution, &manual.execution] {
        assert_eq!(execution.result, native.result);
        assert_eq!(execution.context.query.state, native.context.query.state);
        assert_eq!(
            execution.context.query.effects,
            native.context.query.effects
        );
        assert_eq!(
            execution.context.query.call_context.com_indices,
            native.context.query.call_context.com_indices
        );
        assert_eq!(
            execution.context.private_state,
            native.context.private_state
        );
        assert_eq!(
            execution.private_transcript_outputs,
            native.private_transcript_outputs
        );
        assert_eq!(execution.gas_cost, native.gas_cost);
    }
    assert_eq!(
        serde_json::to_value(recorded.public.verify_ops()).unwrap(),
        serde_json::to_value(manual.public.verify_ops()).unwrap()
    );
    let replay = recorded
        .public
        .initial()
        .query(
            recorded.public.verify_ops(),
            None,
            &recorded.execution.context.cost_model,
        )
        .unwrap();
    let expected = manual
        .public
        .initial()
        .query(
            manual.public.verify_ops(),
            None,
            &manual.execution.context.cost_model,
        )
        .unwrap();
    assert_eq!(replay.context.state, native.context.query.state);
    assert_eq!(replay.context.effects, native.context.query.effects);
    assert_eq!(replay.gas_cost, expected.gas_cost);
}
#[test]
fn owned_locals_defaults_branches_and_reuse_preserve_public_and_witness_order() {
    for selected in [false, true] {
        for seed in [0_u64, 1, 29] {
            let seed = Field::from(seed);
            let a = Witness::default();
            let b = Witness::default();
            let m = Witness::default();
            let native = c::run(context(), &a, selected, seed, true).unwrap();
            let recorded = c::recorded::run(context(), &b, selected, seed, true).unwrap();
            let manual = reference(context(), selected, seed, true, &m).unwrap();
            compare(&native, &recorded, &manual);
            let total = Field::from(if selected { 18_u64 } else { 0 });
            let expected = vec![WitnessInput {
                private: 7,
                seed,
                stored: total,
            }];
            assert_eq!(*a.calls.borrow(), expected);
            assert_eq!(*b.calls.borrow(), expected);
            assert_eq!(*m.calls.borrow(), expected);
            assert_eq!(native.result.value, seed + Field::from(7_u64) + total);
            assert!(native.result.matched);
            assert_eq!(native.context.private_state, 8);
            assert_eq!(native.private_transcript_outputs.len(), 1);
        }
    }
}
#[test]
fn pure_aggregate_coercions_and_repeated_vector_arguments_match_reference() {
    for seed in [0_u64, 1, 29] {
        let seed = Field::from(seed);
        let native = c::conversions(context(), seed).unwrap();
        let recorded = c::recorded::conversions(context(), seed).unwrap();
        let manual = conversion_reference(context(), seed).unwrap();
        compare(&native, &recorded, &manual);
        assert_eq!(native.result.value, seed + Field::from(26_u64));
        assert!(native.result.matched);
        assert_eq!(native.context.private_state, 7);
        assert!(native.private_transcript_outputs.is_empty());
    }
}
#[test]
fn pure_and_witness_failure_preserve_observed_prefix_write() {
    for selected in [false, true] {
        for witness_refuses in [false, true] {
            let a = Witness {
                refuse: witness_refuses,
                ..Default::default()
            };
            let b = Witness {
                refuse: witness_refuses,
                ..Default::default()
            };
            let m = Witness {
                refuse: witness_refuses,
                ..Default::default()
            };
            let seed = Field::from(4_u64);
            let expected = CompactError::AssertionFailed(
                if witness_refuses {
                    "owned witness refused"
                } else {
                    "owned helper refused"
                }
                .into(),
            );
            assert_eq!(
                c::run(context(), &a, selected, seed, false).err().unwrap(),
                expected
            );
            assert_eq!(
                c::recorded::run(context(), &b, selected, seed, false)
                    .err()
                    .unwrap(),
                expected
            );
            assert_eq!(
                reference(context(), selected, seed, false, &m)
                    .err()
                    .unwrap(),
                expected
            );
            assert_eq!(*a.calls.borrow(), *m.calls.borrow());
            assert_eq!(*b.calls.borrow(), *m.calls.borrow());
            assert_eq!(
                m.calls.borrow().as_slice(),
                [WitnessInput {
                    private: 7,
                    seed,
                    stored: Field::from(if selected { 18_u64 } else { 0 })
                }]
            );
        }
    }
}
