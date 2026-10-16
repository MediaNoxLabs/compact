// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
#![cfg(feature = "ledger-transaction")]

use compact_rust_opaque_string_map_query_oracle_fixture::{
    ledger_contract as c, ledger_slots as slots, runtime as r,
};
use midnight_onchain_state::state::{ContractOperation, EntryPointBuf};
use r::context::ConstructorContext;
use r::fab::AlignedValue;
use r::recording::RecordingFrame;
use r::transaction::{Observation, ObservedContractState, VerifierKey};
use rand::{Rng, SeedableRng, rngs::StdRng};

fn state(seed: &str) -> (ObservedContractState, VerifierKey) {
    let initial = c::initial_state(ConstructorContext::new(7_u64)).unwrap();
    let seeded = c::put(
        initial.into_circuit_context(Default::default()),
        seed.into(),
        r::Field::from(42_u64),
    )
    .unwrap();
    let key: VerifierKey = StdRng::seed_from_u64(604).r#gen();
    let operations = midnight_storage::storage::HashMap::new().insert(
        EntryPointBuf(b"ensure".to_vec()),
        ContractOperation::new(Some(key.clone())),
    );
    (
        ObservedContractState::new(
            Default::default(),
            r::ledger::ContractState::new(
                seeded.context.query.state.get_ref().clone(),
                operations,
                Default::default(),
            ),
            Observation {
                transaction_hash: [0; 32],
                block_hash: [0; 32],
                block_height: 0,
            },
        ),
        key,
    )
}

#[test]
fn guarded_lookup_has_exact_observed_input_output_and_ordered_manual_replay() {
    // These are codec/observed API laws. The sibling test file binds the original
    // asset-1 cases to retained TypeScript evidence.
    for seed in ["asset-1", "", "λ", "a\0b"] {
        {
            let (state, verifier) = state(seed);
            let key = r::OpaqueString::from(seed);
            let native = c::ensure(state.circuit_context(7_u64), key.clone()).unwrap();
            let recorded = c::recorded::ensure(state.circuit_context(7_u64), key.clone()).unwrap();
            let contract = c::Contract::default();
            let call = contract
                .recording()
                .ensure_call(&state, 7_u64, key.clone())
                .unwrap();
            let (frame, value) = slots::entries
                .record_member(
                    RecordingFrame::new(state.circuit_context(7_u64)),
                    key.clone(),
                )
                .unwrap();
            assert!(value, "manual guard precedes lookup");
            let (frame, value) = slots::entries.record_lookup(frame, key.clone()).unwrap();
            let manual = frame.finish(value);
            let manual_replay = manual
                .public
                .initial()
                .query(manual.public.verify_ops(), None, &native.context.cost_model)
                .unwrap();
            assert_eq!(native.result, r::Field::from(42_u64));
            for actual in [&recorded, call.recorded(), &manual] {
                assert_eq!(actual.execution.result, r::Field::from(42_u64));
                assert_eq!(
                    actual.execution.context.query.state,
                    native.context.query.state
                );
                assert_eq!(
                    actual.execution.context.query.effects,
                    native.context.query.effects
                );
                assert_eq!(actual.execution.gas_cost, native.gas_cost);
                assert_eq!(actual.execution.context.private_state, 7);
                assert!(actual.execution.private_transcript_outputs.is_empty());
                assert_eq!(actual.public.verify_ops(), manual.public.verify_ops());
                let replay = actual
                    .public
                    .initial()
                    .query(actual.public.verify_ops(), None, &native.context.cost_model)
                    .unwrap();
                assert_eq!(replay.context.state, native.context.query.state);
                assert_eq!(replay.context.effects, native.context.query.effects);
                assert_eq!(replay.gas_cost, manual_replay.gas_cost);
            }
            let prepared = call.prepare(verifier, r::Field::from(0_u64)).unwrap();
            assert_eq!(prepared.input, AlignedValue::from(key));
            assert_eq!(prepared.output, AlignedValue::from(r::Field::from(42_u64)));
        }
    }
}

#[test]
fn absent_and_noncanonical_keys_preserve_guard_and_codec_refusals() {
    let (state, _) = state("registry-key");
    let contract = c::Contract::default();
    let missing = c::ensure(state.circuit_context(7_u64), "absent-key".into())
        .err()
        .unwrap();
    assert_eq!(
        missing,
        r::CompactError::AssertionFailed("missing key".into())
    );
    assert_eq!(
        c::recorded::ensure(state.circuit_context(7_u64), "absent-key".into())
            .err()
            .unwrap(),
        missing
    );
    assert_eq!(
        contract
            .recording()
            .ensure_call(&state, 7_u64, "absent-key".into())
            .err()
            .unwrap(),
        missing
    );
    for key in ["registry-key\0", "a\0b\0"] {
        let error = c::ensure(state.circuit_context(7_u64), key.into())
            .err()
            .unwrap();
        assert!(
            matches!(&error, r::CompactError::InvalidLedgerCell(detail) if detail.contains("trailing zero"))
        );
        assert_eq!(
            c::recorded::ensure(state.circuit_context(7_u64), key.into())
                .err()
                .unwrap(),
            error
        );
        assert_eq!(
            contract
                .recording()
                .ensure_call(&state, 7_u64, key.into())
                .err()
                .unwrap(),
            error
        );
    }
}
