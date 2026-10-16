// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use super::*;
use compact_rust_call_arg_declared_type_fixture::ledger_contract as c;
use midnight_compact_runtime as r;
use midnight_serialize::{Deserializable, Serializable};
use r::fab::AlignedValue;
use r::recording::RecordingFrame;
use r::transaction::{Observation, ObservedContractState, VerifierKey};

#[test]
fn observed_declared_field_commitments_match_manual_and_original_ts() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../runtime-rs/tests/fixtures/call-arg-declared-type.json"
    ))
    .unwrap();
    let mut bytes = Vec::new();
    Vec::<u8>::new().serialize(&mut bytes).unwrap();
    // Preparation carrier only; source-bound real proofs run separately.
    let key = VerifierKey::deserialize(&mut bytes.as_slice(), 0).unwrap();
    let mut operations = HashMap::new();
    for name in CIRCUITS {
        operations = operations.insert(
            EntryPointBuf(name.as_bytes().to_vec()),
            ContractOperation::new(Some(key.clone())),
        );
    }
    let initial = initial_state(ConstructorContext::new(7_u64)).unwrap();
    let state = ObservedContractState::new(
        Default::default(),
        ContractState::new(
            initial.ledger_state.get_ref().clone(),
            operations,
            Default::default(),
        ),
        Observation {
            transaction_hash: [0; 32],
            block_hash: [0; 32],
            block_height: 0,
        },
    );
    let field_only = Field::from_le_bytes(&[
        151, 229, 222, 218, 203, 225, 18, 90, 16, 2, 153, 121, 18, 4, 205, 20, 96, 135, 38, 32, 96,
        231, 204, 32, 245, 117, 166, 76, 157, 182, 207, 1,
    ])
    .unwrap();
    for (name, value) in [
        ("commitSmall", Field::from(5_u64)),
        ("commitU128", Field::from(1_u128 << 100)),
        ("commitFieldOnly", field_only),
    ] {
        let contract = c::Contract::default();
        let recording = contract.recording();
        let (native, recorded, call) = match name {
            "commitSmall" => (
                c::commitSmall(state.circuit_context(7)).unwrap(),
                c::recorded::commitSmall(state.circuit_context(7)).unwrap(),
                recording.commitSmall_call(&state, 7).unwrap(),
            ),
            "commitU128" => (
                c::commitU128(state.circuit_context(7)).unwrap(),
                c::recorded::commitU128(state.circuit_context(7)).unwrap(),
                recording.commitU128_call(&state, 7).unwrap(),
            ),
            _ => (
                c::commitFieldOnly(state.circuit_context(7)).unwrap(),
                c::recorded::commitFieldOnly(state.circuit_context(7)).unwrap(),
                recording.commitFieldOnly_call(&state, 7).unwrap(),
            ),
        };
        let (frame, opening) = ledger_slots::opening
            .record_read(RecordingFrame::new(state.circuit_context(7)))
            .unwrap();
        let commitment = r::persistent_commit(value, opening);
        let manual = ledger_slots::commitCell
            .record_write(frame, commitment)
            .unwrap()
            .finish(());
        assert_eq!(
            ledger_slots::commitCell
                .inspect(native.context.query.state.get_ref())
                .unwrap(),
            commitment
        );
        let expected = manual
            .public
            .initial()
            .query(manual.public.verify_ops(), None, &native.context.cost_model)
            .unwrap();
        for actual in [&recorded, call.recorded(), &manual] {
            boolean_observation_assertions::assert_ts_trace(
                name,
                &native,
                actual,
                &oracle["circuits"][name]["trace"],
            );
            assert_eq!(
                actual.execution.context.query.state,
                native.context.query.state
            );
            assert_eq!(
                actual.execution.context.query.effects,
                native.context.query.effects
            );
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
            assert_eq!(replay.gas_cost, expected.gas_cost);
            assert_eq!(
                state_hex(actual.execution.context.query.state.get_ref().clone()),
                oracle["circuits"][name]["stateHex"]
            );
        }
        let prepared = call.prepare(key.clone(), Field::from(0_u64)).unwrap();
        assert_eq!(prepared.input, AlignedValue::from(()));
        assert_eq!(prepared.output, AlignedValue::from(()));
    }
}
