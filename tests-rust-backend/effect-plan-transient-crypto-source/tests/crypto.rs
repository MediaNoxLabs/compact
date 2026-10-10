// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use compact_rust_effect_plan_transient_crypto_source_fixture::{
    ledger_contract as c, runtime as r,
    support::{Witness, context, reference},
};
use r::{CompactError, Field, FixedBytes};
#[test]
fn official_crypto_composes_with_ordered_ledger_witnesses_and_replay() {
    for selected in [false, true] {
        for byte in [0, 1, 255] {
            let input = FixedBytes::new([byte; 32]);
            let seed = Field::from(19_u64);
            let (a, b, m) = (Witness::default(), Witness::default(), Witness::default());
            let native = c::run(context(), &a, input, seed, selected).unwrap();
            let recorded = c::recorded::run(context(), &b, input, seed, selected).unwrap();
            let manual = reference(context(), input, seed, selected, &m).unwrap();
            let hash = r::transient_hash((r::degrade_to_transient(input), seed));
            let committed = if selected {
                r::transient_commit(hash, seed + Field::from(1_u64))
            } else {
                hash
            };
            assert_eq!(native.result.hash, hash);
            assert_eq!(native.result.commitment, committed);
            assert_eq!(native.result.digest, r::upgrade_from_transient(committed));
            for execution in [&recorded.execution, &manual.execution] {
                assert_eq!(execution.result, native.result);
                assert_eq!(execution.context.query.state, native.context.query.state);
                assert_eq!(
                    execution.context.query.effects,
                    native.context.query.effects
                );
                assert_eq!(execution.gas_cost, native.gas_cost);
                assert_eq!(
                    execution.private_transcript_outputs,
                    native.private_transcript_outputs
                );
                assert_eq!(execution.context.private_state, 7 + u64::from(selected));
            }
            let expected = if selected {
                vec![(7, seed, input)]
            } else {
                vec![]
            };
            for witness in [&a, &b, &m] {
                assert_eq!(*witness.calls.borrow(), expected);
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
            let reference = manual
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
            assert_eq!(replay.gas_cost, reference.gas_cost);
        }
    }
}
#[test]
fn reused_vector_values_have_official_hash_and_commitment_encodings() {
    for n in [0, 1, 999] {
        let seed = Field::from(n);
        let native = c::products(context(), seed).unwrap();
        let recorded = c::recorded::products(context(), seed).unwrap();
        let values = r::FixedVector::new([seed, seed + Field::from(1_u64)]);
        assert_eq!(native.result.hash, r::transient_hash(values.clone()));
        assert_eq!(native.result.commitment, r::transient_commit(values, seed));
        assert_eq!(recorded.execution.result, native.result);
        assert_eq!(
            recorded.execution.context.query.state,
            native.context.query.state
        );
        assert_eq!(
            recorded.execution.context.query.effects,
            native.context.query.effects
        );
        assert_eq!(recorded.execution.gas_cost, native.gas_cost);
        assert_eq!(
            recorded.execution.private_transcript_outputs,
            native.private_transcript_outputs
        );
        assert_eq!(
            recorded.execution.context.private_state,
            native.context.private_state
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
        assert_eq!(replay.context.state, native.context.query.state);
        assert_eq!(replay.context.effects, native.context.query.effects);
    }
}
#[test]
fn refused_opening_is_observed_only_in_selected_branch_after_the_stored_write() {
    let input = FixedBytes::new([77; 32]);
    let seed = Field::from(5_u64);
    for selected in [false, true] {
        let a = Witness {
            refuse: true,
            ..Default::default()
        };
        let b = Witness {
            refuse: true,
            ..Default::default()
        };
        let native = c::run(context(), &a, input, seed, selected);
        let recorded = c::recorded::run(context(), &b, input, seed, selected);
        if selected {
            let error = CompactError::AssertionFailed("opening refused".into());
            assert_eq!(native.err().unwrap(), error);
            assert_eq!(recorded.err().unwrap(), error);
            assert_eq!(*a.calls.borrow(), vec![(7, seed, input)]);
            assert_eq!(*b.calls.borrow(), *a.calls.borrow());
        } else {
            assert_eq!(native.unwrap().result, recorded.unwrap().execution.result);
            assert!(a.calls.borrow().is_empty());
            assert!(b.calls.borrow().is_empty());
        }
    }
}
