// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use super::*;
use midnight_serialize::{Deserializable, Serializable};
use runtime::fab::AlignedValue;
use runtime::recording::RecordingFrame;
use runtime::transaction::{Observation, ObservedContractState, VerifierKey};

fn setup(empty: bool) -> (ObservedContractState, VerifierKey) {
    let initial = initial_state(ConstructorContext::new(7_u64))
        .unwrap()
        .into_circuit_context(Default::default());
    let state = if empty {
        ledger_contract::clear_items(initial)
            .unwrap()
            .context
            .query
            .state
            .get_ref()
            .clone()
    } else {
        initial.query.state.get_ref().clone()
    };
    let mut bytes = Vec::new();
    Vec::<u8>::new().serialize(&mut bytes).unwrap();
    let verifier = VerifierKey::deserialize(&mut bytes.as_slice(), 0).unwrap();
    let mut operations = HashMap::new();
    for name in ["first_item", "prepend", "drop_first"] {
        operations = operations.insert(
            EntryPointBuf(name.as_bytes().to_vec()),
            ContractOperation::new(Some(verifier.clone())),
        );
    }
    (
        ObservedContractState::new(
            Default::default(),
            ContractState::new(state, operations, Default::default()),
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
fn observed_chunked_head_prepares_the_concrete_maybe_on_empty_and_populated_state() {
    for empty in [false, true] {
        let (state, key) = setup(empty);
        let contract = Contract::default();
        let native = ledger_contract::first_item(state.circuit_context(7)).unwrap();
        let recorded = ledger_contract::recorded::first_item(state.circuit_context(7)).unwrap();
        let call = contract.recording().first_item_call(&state, 7).unwrap();
        let (frame, result) = items
            .record_head::<Maybe, _, _>(RecordingFrame::new(state.circuit_context(7)))
            .unwrap();
        let manual = frame.finish(result);
        assert_eq!(
            native.result,
            Maybe {
                is_some: !empty,
                value: Field::from(u64::from(!empty))
            }
        );
        for actual in [&recorded, call.recorded(), &manual] {
            compare(&native, actual, &manual);
        }
        // The populated branch is the original TS row; the empty supplied-state
        // branch is an independent runtime control, not a fresh TS capture.
        if !empty {
            assert_eq!(shape(&recorded), oracle()["head"]["publicTranscriptShape"]);
        }
        let expected = AlignedValue::from(native.result);
        let prepared = call.prepare(key, Field::from(0_u64)).unwrap();
        assert_eq!(prepared.input, AlignedValue::from(()));
        assert_eq!(prepared.output, expected);
    }
}
#[test]
fn observed_chunked_push_prepares_input_and_matches_manual_slot() {
    let (state, key) = setup(false);
    let value = Field::from(7_u64);
    let contract = Contract::default();
    let native = ledger_contract::prepend(state.circuit_context(7), value).unwrap();
    let recorded = ledger_contract::recorded::prepend(state.circuit_context(7), value).unwrap();
    let call = contract.recording().prepend_call(&state, 7, value).unwrap();
    let manual = items
        .record_push_front(RecordingFrame::new(state.circuit_context(7)), value)
        .unwrap()
        .finish(());
    for actual in [&recorded, call.recorded(), &manual] {
        compare(&native, actual, &manual);
    }
    assert_eq!(
        shape(&recorded),
        oracle()["prepend"]["publicTranscriptShape"]
    );
    assert_eq!(
        state_hex(native.context.query.state.get_ref().clone()),
        oracle()["prepend"]["afterStateHex"]
    );
    let prepared = call.prepare(key, Field::from(0_u64)).unwrap();
    assert_eq!(prepared.input, AlignedValue::from(value));
    assert_eq!(prepared.output, AlignedValue::from(()));
}
#[test]
fn empty_pop_follows_the_official_slot_outcome_without_inventing_a_refusal() {
    let (state, _) = setup(true);
    let contract = Contract::default();
    let native = ledger_contract::drop_first(state.circuit_context(7));
    let recorded = ledger_contract::recorded::drop_first(state.circuit_context(7));
    let observed = contract.recording().drop_first_call(&state, 7);
    let manual = items
        .record_pop_front(RecordingFrame::new(state.circuit_context(7)))
        .map(|f| f.finish(()));
    match manual {
        Ok(manual) => {
            let native = native.unwrap();
            let recorded = recorded.unwrap();
            let observed = observed.unwrap();
            for actual in [&recorded, observed.recorded(), &manual] {
                compare(&native, actual, &manual);
            }
        }
        Err(error) => {
            assert_eq!(native.err().unwrap(), error);
            assert_eq!(recorded.err().unwrap(), error);
            assert_eq!(observed.err().unwrap(), error);
        }
    }
}
