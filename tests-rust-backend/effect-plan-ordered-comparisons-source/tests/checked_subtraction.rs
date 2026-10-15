// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Retained source: an explicit comparison guard precedes Counter mutation.
#![cfg(feature = "ledger-transaction")]
include!(concat!(env!("OUT_DIR"), "/checked_subtraction.rs"));

use ledger_contract as c;
use midnight_onchain_state::state::{ContractOperation, EntryPointBuf};
use r::context::{CircuitContext, CircuitResult, ConstructorContext};
use r::recording::{RecordedCircuitResult, RecordingFrame};
use r::transaction::{
    Observation, ObservedCallError, ObservedContractState, PrepareCallError, VerifierKey,
};
use rand::{Rng, SeedableRng, rngs::StdRng};
use runtime as r;
type Value = r::BoundedUint<65535>;

fn assertion() -> r::CompactError {
    r::CompactError::AssertionFailed("explicit guard before mutation".into())
}
fn context() -> CircuitContext<u64> {
    c::initial_state(ConstructorContext::new(7))
        .unwrap()
        .into_circuit_context(Default::default())
}
fn observed() -> (ObservedContractState, VerifierKey) {
    let key: VerifierKey = StdRng::seed_from_u64(57602).r#gen();
    let operations = midnight_storage::storage::HashMap::new().insert(
        EntryPointBuf(b"explicit_guard_then_subtract".to_vec()),
        ContractOperation::new(Some(key.clone())),
    );
    (
        ObservedContractState::new(
            Default::default(),
            r::ledger::ContractState::new(
                context().query.state.get_ref().clone(),
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
fn manual(
    context: CircuitContext<u64>,
    selected: bool,
    input: u128,
    expected: Option<u128>,
) -> Result<RecordedCircuitResult<u64, Value>, r::CompactError> {
    let frame = RecordingFrame::new(context);
    if !selected {
        return Ok(frame.finish(Value::new(input).unwrap()));
    }
    let Some(expected) = expected else {
        return Err(assertion());
    };
    Ok(ledger_slots::count
        .record_increment(frame, 1)?
        .finish(Value::new(expected).unwrap()))
}
fn compare(
    native: &CircuitResult<u64, Value>,
    recorded: &RecordedCircuitResult<u64, Value>,
    manual: &RecordedCircuitResult<u64, Value>,
) {
    for actual in [&recorded.execution, &manual.execution] {
        assert_eq!(actual.result, native.result);
        assert_eq!(actual.context.query.state, native.context.query.state);
        assert_eq!(actual.context.query.effects, native.context.query.effects);
        assert_eq!(actual.context.private_state, 7);
        assert_eq!(actual.gas_cost, native.gas_cost);
        assert!(actual.private_transcript_outputs.is_empty());
    }
    assert_eq!(native.context.private_state, 7);
    assert!(native.private_transcript_outputs.is_empty());
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
            &native.context.cost_model,
        )
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
fn explicit_guard_boundaries_match_native_manual_replay_and_observed_preparation() {
    // The manual oracle uses literal expected results instead of generated arithmetic.
    for (selected, input, expected) in [
        (false, 0, Some(0)),
        (false, 1, Some(1)),
        (false, 2, Some(2)),
        (false, 65535, Some(65535)),
        (true, 0, None),
        (true, 1, None),
        (true, 2, Some(1)),
        (true, 65535, Some(65534)),
    ] {
        let value = Value::new(input).unwrap();
        let (state, key) = observed();
        let native = c::explicit_guard_then_subtract(state.circuit_context(7), selected, value);
        let recorded =
            c::recorded::explicit_guard_then_subtract(state.circuit_context(7), selected, value);
        let manual = manual(state.circuit_context(7), selected, input, expected);
        let call = c::Contract::default()
            .recording()
            .explicit_guard_then_subtract_call(&state, 7, selected, value);
        if let Some(result) = expected {
            let (native, recorded, manual, call) = (
                native.unwrap(),
                recorded.unwrap(),
                manual.unwrap(),
                call.unwrap(),
            );
            compare(&native, &recorded, &manual);
            compare(&native, call.recorded(), &manual);
            assert_eq!(native.result.value(), result);
            assert_eq!(
                ledger_slots::count
                    .inspect(native.context.query.state.get_ref())
                    .unwrap(),
                u64::from(selected)
            );
            if selected {
                let prepared = call.prepare(key, r::Field::from(0_u64)).unwrap();
                assert_eq!(
                    prepared.input,
                    r::fab::AlignedValue::from((selected, value))
                );
                assert_eq!(
                    prepared.output,
                    r::fab::AlignedValue::from((Value::new(result).unwrap(),))
                );
            } else {
                assert!(recorded.public.verify_ops().is_empty());
                assert!(matches!(
                    call.prepare(key, r::Field::from(0_u64)),
                    Err(ObservedCallError::Prepare(
                        PrepareCallError::EmptyTranscript
                    ))
                ));
            }
        } else {
            assert_eq!(native.err().unwrap(), assertion());
            assert_eq!(recorded.err().unwrap(), assertion());
            assert_eq!(manual.err().unwrap(), assertion());
            assert_eq!(call.err().unwrap(), assertion());
        }
        // Observation snapshots remain immutable; this does not inspect a consumed failure frame.
        assert_eq!(
            ledger_slots::count
                .inspect(state.circuit_context(7).query.state.get_ref())
                .unwrap(),
            0
        );
    }
}

#[test]
fn explicit_assertion_precedes_any_access_to_an_invalid_counter() {
    let missing_counter = || {
        let mut context = context();
        context.query.state = r::ledger::contract_state(vec![]);
        context
    };
    for value in [0, 1] {
        let value = Value::new(value).unwrap();
        assert_eq!(
            c::explicit_guard_then_subtract(missing_counter(), true, value)
                .err()
                .unwrap(),
            assertion()
        );
        assert_eq!(
            c::recorded::explicit_guard_then_subtract(missing_counter(), true, value)
                .err()
                .unwrap(),
            assertion()
        );
    }
    // The counter is observably invalid: passing the guard reaches and rejects it.
    let two = Value::new(2).unwrap();
    let native = c::explicit_guard_then_subtract(missing_counter(), true, two)
        .err()
        .unwrap();
    let recorded = c::recorded::explicit_guard_then_subtract(missing_counter(), true, two)
        .err()
        .unwrap();
    assert_ne!(native, assertion());
    assert_eq!(recorded, native);
    // The unselected branch must not consult that invalid ledger slot at all.
    let zero = Value::new(0).unwrap();
    assert_eq!(
        c::explicit_guard_then_subtract(missing_counter(), false, zero)
            .unwrap()
            .result,
        zero
    );
    let recorded =
        c::recorded::explicit_guard_then_subtract(missing_counter(), false, zero).unwrap();
    assert_eq!(recorded.execution.result, zero);
    assert!(recorded.public.verify_ops().is_empty());
}
