// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
#![cfg(feature = "ledger-transaction")]
mod common;
use common::*;
use compact_rust_effect_plan_nested_map_member_source_fixture::{
    ledger_contract as c, ledger_slots as slots, runtime as r, types::Present,
};
use midnight_onchain_state::state::{ContractOperation, EntryPointBuf};
use r::context::ConstructorContext;
use r::ledger::{DefaultDB, LedgerHashMap, StateValue};
use r::recording::RecordingFrame;
use r::transaction::{Observation, ObservedContractState, VerifierKey};
use rand::{Rng, SeedableRng, rngs::StdRng};
fn setup(supplied: bool) -> (ObservedContractState, VerifierKey) {
    let key: VerifierKey = StdRng::seed_from_u64(579).r#gen();
    let operations = midnight_storage::storage::HashMap::new().insert(
        EntryPointBuf(b"member".to_vec()),
        ContractOperation::new(Some(key.clone())),
    );
    let initial = c::initial_state(ConstructorContext::new(5_u64)).unwrap();
    let state = if supplied {
        // This positive state is provided by the test using official ledger types.
        // It does not claim support for generated nested insertDefault constructors.
        let inner: StateValue<DefaultDB> = StateValue::Map(LedgerHashMap::new());
        let outer =
            LedgerHashMap::new().insert(r::fab::AlignedValue::from(r::Field::from(1_u64)), inner);
        StateValue::Array(vec![StateValue::Map(outer)].into())
    } else {
        initial.ledger_state.get_ref().clone()
    };
    (
        ObservedContractState::new(
            Default::default(),
            r::ledger::ContractState::new(state, operations, Default::default()),
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
fn key_only_membership_handles_generated_empty_and_supplied_nested_map_state() {
    // MapNode intentionally has no scalar CellValue codec. Membership only needs K.
    let _: r::slots::MapSlot<
        r::Field,
        r::slots::MapNode<r::Field, r::BoundedUint<{ u64::MAX as u128 }>>,
    > = slots::outer;
    assert_eq!(slots::outer.path(), &[0]);
    for supplied in [false, true] {
        for number in [1_u64, 2] {
            let (state, key) = setup(supplied);
            let input = r::Field::from(number);
            let expected = supplied && number == 1;
            let native = c::member(state.circuit_context(5), input).unwrap();
            let recorded = c::recorded::member(state.circuit_context(5), input).unwrap();
            let (frame, value) = slots::outer
                .record_member(RecordingFrame::new(state.circuit_context(5)), input)
                .unwrap();
            let manual = frame.finish(Present { value });
            let contract = c::Contract::default();
            let call = contract.recording().member_call(&state, 5, input).unwrap();
            compare(&native, &recorded, &manual);
            compare(&native, call.recorded(), &manual);
            assert_eq!(native.result, Present { value: expected });
            assert_eq!(native.context.private_state, 5);
            assert!(native.private_transcript_outputs.is_empty());
            assert_eq!(
                native.context.query.state,
                state.circuit_context(5).query.state
            );
            let prepared = call.prepare(key, r::Field::from(0_u64)).unwrap();
            assert_eq!(atoms(&prepared.input), vec![vec![number as u8]]);
            assert_eq!(
                atoms(&prepared.output),
                vec![if expected { vec![1] } else { vec![] }]
            );
            let reference = row(&format!(
                "{}-{number}",
                if supplied {
                    "supplied"
                } else {
                    "generated-empty"
                }
            ));
            assert_eq!(reference["supplied"], supplied);
            assert_eq!(reference["args"], serde_json::json!([number.to_string()]));
            assert_eq!(reference["result"], serde_json::json!({"value":expected}));
            assert_ts(&reference, &prepared.input, &prepared.output, &native);
        }
    }
}
