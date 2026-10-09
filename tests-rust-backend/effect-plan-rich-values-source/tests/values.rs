// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

use compact_rust_effect_plan_rich_values_source_fixture::{
    ledger_contract as c, ledger_slots, runtime as r,
    support::{Witness, WitnessInput, initial_value, reference},
    types::Phase,
};
use midnight_compact_testkit::{ArtifactIdentity, ContractLab, Environment, LabError};
use r::context::{CircuitContext, ConstructorContext};
use r::{CompactError, Field, FixedBytes};
fn context() -> CircuitContext<u64> {
    c::initial_state(ConstructorContext::new(7))
        .unwrap()
        .into_circuit_context(Default::default())
}
#[test]
fn rich_values_survive_branch_witness_cells_helpers_and_repeated_use() {
    for selected in [false, true] {
        for seed in [0_u64, 1, 19] {
            let seed = Field::from(seed);
            let a = Witness::default();
            let b = Witness::default();
            let w = Witness::default();
            let native = c::run(context(), &a, selected, seed).unwrap();
            let recorded = c::recorded::run(context(), &b, selected, seed).unwrap();
            let manual = reference(context(), selected, seed, &w).unwrap();
            let expected_input = vec![WitnessInput {
                private: 7,
                value: initial_value(selected, seed),
                tag: FixedBytes::new(*b"tag"),
                phase: Phase::hot,
            }];
            assert_eq!(*a.calls.borrow(), expected_input);
            assert_eq!(*b.calls.borrow(), expected_input);
            assert_eq!(*w.calls.borrow(), expected_input);
            let amount = if selected { seed } else { Field::from(0_u64) } + Field::from(7_u64);
            assert_eq!(native.result, amount + amount + amount);
            assert_eq!(native.context.private_state, 8);
            assert_eq!(native.private_transcript_outputs.len(), 1);
            for execution in [&recorded.execution, &manual.execution] {
                assert_eq!(execution.result, native.result);
                assert_eq!(execution.context.query.state, native.context.query.state);
                assert_eq!(
                    execution.context.query.effects,
                    native.context.query.effects
                );
                assert_eq!(execution.gas_cost, native.gas_cost);
                assert_eq!(
                    execution.context.private_state,
                    native.context.private_state
                );
                assert_eq!(
                    execution.private_transcript_outputs,
                    native.private_transcript_outputs
                );
            }
            let stored = ledger_slots::stored
                .inspect(native.context.query.state.get_ref())
                .unwrap();
            assert_eq!(stored.inner.amount, amount);
            assert_eq!(stored.inner.tag, FixedBytes::new(*b"gat"));
            assert_eq!(
                stored.phase,
                if selected { Phase::cold } else { Phase::hot }
            );
            assert_eq!(
                ledger_slots::tag
                    .inspect(native.context.query.state.get_ref())
                    .unwrap(),
                stored.inner.tag
            );
            assert_eq!(
                ledger_slots::phase
                    .inspect(native.context.query.state.get_ref())
                    .unwrap(),
                stored.phase
            );
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
    }
}
#[test]
fn rich_witness_failure_preserves_witness_calls_and_rolls_back_lab() {
    for selected in [false, true] {
        let make = || Witness {
            refuse: true,
            ..Witness::default()
        };
        let a = make();
        let b = make();
        let w = make();
        let seed = Field::from(3_u64);
        let error = reference(context(), selected, seed, &w).err().unwrap();
        assert_eq!(
            error,
            CompactError::AssertionFailed("rich witness refused".into())
        );
        assert_eq!(c::run(context(), &a, selected, seed).err().unwrap(), error);
        assert_eq!(
            c::recorded::run(context(), &b, selected, seed)
                .err()
                .unwrap(),
            error
        );
        assert_eq!(*a.calls.borrow(), *w.calls.borrow());
        assert_eq!(*b.calls.borrow(), *w.calls.borrow());
        assert_eq!(w.calls.borrow().len(), 1);
        // Rollback-only identity; proof source binding is separately checked.
        let mut lab = ContractLab::from_constructor(
            ArtifactIdentity {
                source_sha256: [0; 32],
                generated_sha256: [0; 32],
            },
            Environment::new(Default::default(), Default::default(), [0; 32]),
            c::initial_state(ConstructorContext::new(7)).unwrap(),
        )
        .unwrap();
        let before = lab.snapshot().public_state().clone();
        let witness = make();
        let result = lab.recorded(|ctx| c::recorded::run(ctx, &witness, selected, seed));
        assert!(matches!(result,Err(LabError::Execution(ref e)) if *e == error));
        assert_eq!(lab.snapshot().public_state(), &before);
        assert_eq!(*lab.private_state(), 7);
    }
}
