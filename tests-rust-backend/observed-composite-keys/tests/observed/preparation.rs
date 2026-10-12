// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use super::*;
use compact_rust_observed_composite_keys_fixture::ledger_slots;
use midnight_onchain_state::state::{ContractOperation, EntryPointBuf};
use midnight_serialize::{Deserializable, Serializable};
use runtime::recording::RecordingFrame;
use runtime::transaction::{Observation, ObservedContractState, VerifierKey};

fn key(marker: u64) -> CompositeKey {
    CompositeKey {
        vector: FixedVector::new([Field::from(3_u64), Field::from(5_u64)]),
        pair: (Field::from(marker), true),
    }
}
fn setup() -> (ObservedContractState, VerifierKey) {
    let initial = initial_state(ConstructorContext::new(7_u64)).unwrap();
    // The extra key is supplied test state, so both operations must preserve a
    // populated sibling. Frozen TS rows below authenticate only the input FAB.
    let seeded = ledger_slots::structKeys
        .insert(initial.into_circuit_context(Default::default()), key(43))
        .unwrap();
    let mut bytes = Vec::new();
    Vec::<u8>::new().serialize(&mut bytes).unwrap();
    let verifier = VerifierKey::deserialize(&mut bytes.as_slice(), 0).unwrap();
    let mut operations = runtime::ledger::LedgerHashMap::new();
    for name in ["insert_struct", "roundtrip_struct"] {
        operations = operations.insert(
            EntryPointBuf(name.as_bytes().to_vec()),
            ContractOperation::new(Some(verifier.clone())),
        );
    }
    (
        ObservedContractState::new(
            Default::default(),
            runtime::ledger::ContractState::new(
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
        verifier,
    )
}
fn compare<T: std::fmt::Debug + PartialEq>(
    native: &CircuitResult<u64, T>,
    actual: &RecordedCircuitResult<u64, T>,
    manual: &RecordedCircuitResult<u64, T>,
) {
    assert_eq!(actual.execution.result, native.result);
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
    let reference = manual
        .public
        .initial()
        .query(manual.public.verify_ops(), None, &native.context.cost_model)
        .unwrap();
    assert_eq!(replay.context.state, native.context.query.state);
    assert_eq!(replay.context.effects, native.context.query.effects);
    assert_eq!(replay.gas_cost, reference.gas_cost);
    assert!(
        PublicStateView::from(&actual.execution)
            .structKeys()
            .unwrap()
            .try_member(key(43))
            .unwrap()
    );
}
fn input(captured: &serde_json::Value, actual: &AlignedValue) {
    let expected = AlignedValue::from(key(42));
    check_input(captured, actual.clone(), expected.alignment);
    assert_eq!(actual, &AlignedValue::from(key(42)));
}
#[test]
fn observed_nested_key_insert_prepares_exact_ts_input_and_preserves_sibling() {
    let (state, verifier) = setup();
    let value = key(42);
    let native = ledger_contract::insert_struct(state.circuit_context(7), value.clone()).unwrap();
    let recorded =
        ledger_contract::recorded::insert_struct(state.circuit_context(7), value.clone()).unwrap();
    let contract = Contract::default();
    let call = contract
        .recording()
        .insert_struct_call(&state, 7, value.clone())
        .unwrap();
    let manual = ledger_slots::structKeys
        .record_insert(RecordingFrame::new(state.circuit_context(7)), value.clone())
        .unwrap()
        .finish(());
    for actual in [&recorded, call.recorded(), &manual] {
        compare(&native, actual, &manual);
    }
    let view = PublicStateView::from(&native).structKeys().unwrap();
    assert!(view.try_member(value).unwrap());
    assert_eq!(view.size().unwrap().value(), 2);
    assert_eq!(native.context.private_state, 7);
    assert!(native.private_transcript_outputs.is_empty());
    let prepared = call.prepare(verifier, Field::from(0_u64)).unwrap();
    input(&oracle()["struct"], &prepared.input);
    assert_eq!(prepared.output, AlignedValue::from(()));
}
#[test]
fn observed_nested_key_roundtrip_prepares_boolean_and_preserves_sibling() {
    let (state, verifier) = setup();
    let value = key(42);
    let native =
        ledger_contract::roundtrip_struct(state.circuit_context(7), value.clone()).unwrap();
    let recorded =
        ledger_contract::recorded::roundtrip_struct(state.circuit_context(7), value.clone())
            .unwrap();
    let contract = Contract::default();
    let call = contract
        .recording()
        .roundtrip_struct_call(&state, 7, value.clone())
        .unwrap();
    let frame = ledger_slots::structKeys
        .record_insert(RecordingFrame::new(state.circuit_context(7)), value.clone())
        .unwrap();
    let frame = ledger_slots::structKeys
        .record_remove(frame, value.clone())
        .unwrap();
    let (frame, present) = ledger_slots::structKeys
        .record_member(frame, value.clone())
        .unwrap();
    let manual = frame.finish(present);
    assert!(!native.result);
    for actual in [&recorded, call.recorded(), &manual] {
        compare(&native, actual, &manual);
    }
    let view = PublicStateView::from(&native).structKeys().unwrap();
    assert!(!view.try_member(value).unwrap());
    assert_eq!(view.size().unwrap().value(), 1);
    assert_eq!(native.context.private_state, 7);
    assert!(native.private_transcript_outputs.is_empty());
    let prepared = call.prepare(verifier, Field::from(0_u64)).unwrap();
    input(&oracle()["structRoundtrip"], &prepared.input);
    assert_eq!(prepared.output, AlignedValue::from(false));
}
