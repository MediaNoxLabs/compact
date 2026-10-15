// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use super::*;
use midnight_serialize::{Deserializable, Serializable};
use runtime::fab::AlignedValue;
use runtime::recording::RecordingFrame;
use runtime::transaction::{Observation, ObservedContractState, VerifierKey};

// The serialized empty verifier is a preparation-order carrier, not proof material.
fn setup(entries: u8) -> (ObservedContractState, VerifierKey) {
    let initial = initial_state(ConstructorContext::new(7_u64))
        .unwrap()
        .into_circuit_context(Default::default());
    let mut context = table.reset(initial).unwrap().context;
    if entries > 0 {
        context = table
            .insert(context, true, Field::from(42_u64))
            .unwrap()
            .context;
    }
    if entries > 1 {
        context = table
            .insert(context, false, Field::from(90_u64))
            .unwrap()
            .context;
    }
    let mut bytes = Vec::new();
    Vec::<u8>::new().serialize(&mut bytes).unwrap();
    let verifier = VerifierKey::deserialize(&mut bytes.as_slice(), 0).unwrap();
    let mut operations = HashMap::new();
    for name in ["table_size", "table_is_empty", "put_default", "reset_table"] {
        operations = operations.insert(
            EntryPointBuf(name.as_bytes().to_vec()),
            ContractOperation::new(Some(verifier.clone())),
        );
    }
    (
        ObservedContractState::new(
            Default::default(),
            ContractState::new(
                context.query.state.get_ref().clone(),
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
    assert_eq!(actual.execution.gas_cost, native.gas_cost);
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
    let reference = manual
        .public
        .initial()
        .query(manual.public.verify_ops(), None, &native.context.cost_model)
        .unwrap();
    assert_eq!(replay.context.state, native.context.query.state);
    assert_eq!(replay.context.effects, native.context.query.effects);
    assert_eq!(replay.gas_cost, reference.gas_cost);
}
#[test]
fn observed_map_metadata_matches_manual_on_empty_and_populated_states() {
    for entries in 0..=2 {
        let (state, key) = setup(entries);
        let contract = Contract::default();
        let native = ledger_contract::table_size(state.circuit_context(7)).unwrap();
        let recorded = ledger_contract::recorded::table_size(state.circuit_context(7)).unwrap();
        let call = contract.recording().table_size_call(&state, 7).unwrap();
        let (frame, size) = table
            .record_size(RecordingFrame::new(state.circuit_context(7)))
            .unwrap();
        let manual = frame
            .finish(runtime::BoundedUint::<{ u64::MAX as u128 }>::new(u128::from(size)).unwrap());
        assert_eq!(native.result.value(), u128::from(entries));
        for actual in [&recorded, call.recorded(), &manual] {
            compare(&native, actual, &manual);
        }
        assert_eq!(
            native.context.query.state.get_ref(),
            state.contract().data.get_ref()
        );
        let expected = AlignedValue::from(native.result);
        let prepared = call.prepare(key.clone(), Field::from(0_u64)).unwrap();
        assert_eq!(prepared.input, AlignedValue::from(()));
        assert_eq!(prepared.output, expected);

        let native = ledger_contract::table_is_empty(state.circuit_context(7)).unwrap();
        let recorded = ledger_contract::recorded::table_is_empty(state.circuit_context(7)).unwrap();
        let call = contract.recording().table_is_empty_call(&state, 7).unwrap();
        let (frame, empty) = table
            .record_is_empty(RecordingFrame::new(state.circuit_context(7)))
            .unwrap();
        let manual = frame.finish(empty);
        assert_eq!(native.result, entries == 0);
        for actual in [&recorded, call.recorded(), &manual] {
            compare(&native, actual, &manual);
        }
        let prepared = call.prepare(key, Field::from(0_u64)).unwrap();
        assert_eq!(prepared.input, AlignedValue::from(()));
        assert_eq!(prepared.output, AlignedValue::from(entries == 0));
    }
}
#[test]
fn observed_default_insertion_overwrites_and_preserves_the_other_key() {
    for entries in [0, 2] {
        let (state, key) = setup(entries);
        let contract = Contract::default();
        let native = ledger_contract::put_default(state.circuit_context(7), true).unwrap();
        let recorded =
            ledger_contract::recorded::put_default(state.circuit_context(7), true).unwrap();
        let call = contract
            .recording()
            .put_default_call(&state, 7, true)
            .unwrap();
        let manual = table
            .record_insert_default(RecordingFrame::new(state.circuit_context(7)), true)
            .unwrap()
            .finish(());
        for actual in [&recorded, call.recorded(), &manual] {
            compare(&native, actual, &manual);
        }
        let view = table.inspect(native.context.query.state.get_ref()).unwrap();
        assert_eq!(view.lookup(true).unwrap(), Field::from(0_u64));
        assert_eq!(
            view.size().unwrap().value(),
            if entries == 0 { 1 } else { 2 }
        );
        if entries == 2 {
            assert_eq!(view.lookup(false).unwrap(), Field::from(90_u64));
        }
        let prepared = call.prepare(key, Field::from(0_u64)).unwrap();
        assert_eq!(prepared.input, AlignedValue::from(true));
        assert_eq!(prepared.output, AlignedValue::from(()));
    }
}
#[test]
fn observed_reset_clears_only_the_chunked_map_in_empty_and_populated_states() {
    for entries in [0, 2] {
        let (state, key) = setup(entries);
        let (empty, _) = setup(0);
        let contract = Contract::default();
        let native = ledger_contract::reset_table(state.circuit_context(7)).unwrap();
        let recorded = ledger_contract::recorded::reset_table(state.circuit_context(7)).unwrap();
        let call = contract.recording().reset_table_call(&state, 7).unwrap();
        let manual = table
            .record_reset(RecordingFrame::new(state.circuit_context(7)))
            .unwrap()
            .finish(());
        for actual in [&recorded, call.recorded(), &manual] {
            compare(&native, actual, &manual);
        }
        assert_eq!(
            native.context.query.state.get_ref(),
            empty.contract().data.get_ref()
        );
        let prepared = call.prepare(key, Field::from(0_u64)).unwrap();
        assert_eq!(prepared.input, AlignedValue::from(()));
        assert_eq!(prepared.output, AlignedValue::from(()));
    }
}
