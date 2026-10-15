// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
#![cfg(feature = "ledger-transaction")]

use compact_rust_opaque_string_set_oracle_fixture::{
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
    let seeded = c::addName(
        initial.into_circuit_context(Default::default()),
        seed.into(),
    )
    .unwrap();
    let key: VerifierKey = StdRng::seed_from_u64(603).r#gen();
    let operations = midnight_storage::storage::HashMap::new().insert(
        EntryPointBuf(b"hasName".to_vec()),
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
fn canonical_membership_has_exact_observed_input_output_and_manual_replay() {
    // These are codec/observed API laws. The sibling test file binds the original
    // registry-key cases to retained TypeScript evidence.
    for seed in ["registry-key", "", "λ", "a\0b"] {
        for present in [true, false] {
            let (state, verifier) = state(seed);
            let key = r::OpaqueString::from(if present { seed } else { "absent-key" });
            let native = c::hasName(state.circuit_context(7_u64), key.clone()).unwrap();
            let recorded = c::recorded::hasName(state.circuit_context(7_u64), key.clone()).unwrap();
            let contract = c::Contract::default();
            let call = contract
                .recording()
                .hasName_call(&state, 7_u64, key.clone())
                .unwrap();
            let (frame, value) = slots::names
                .record_member(
                    RecordingFrame::new(state.circuit_context(7_u64)),
                    key.clone(),
                )
                .unwrap();
            let manual = frame.finish(value);
            let manual_replay = manual
                .public
                .initial()
                .query(manual.public.verify_ops(), None, &native.context.cost_model)
                .unwrap();
            assert_eq!(native.result, present);
            for actual in [&recorded, call.recorded(), &manual] {
                assert_eq!(actual.execution.result, present);
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
            assert_eq!(prepared.output, AlignedValue::from(present));
        }
    }
}

#[test]
fn noncanonical_membership_keys_are_refused_before_observation() {
    let (state, _) = state("registry-key");
    let contract = c::Contract::default();
    for key in ["registry-key\0", "a\0b\0"] {
        let error = c::hasName(state.circuit_context(7_u64), key.into())
            .err()
            .unwrap();
        assert!(
            matches!(&error, r::CompactError::InvalidLedgerCell(detail) if detail.contains("trailing zero"))
        );
        assert_eq!(
            c::recorded::hasName(state.circuit_context(7_u64), key.into())
                .err()
                .unwrap(),
            error
        );
        assert_eq!(
            contract
                .recording()
                .hasName_call(&state, 7_u64, key.into())
                .err()
                .unwrap(),
            error
        );
    }
}
