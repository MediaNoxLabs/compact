// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use midnight_compact_runtime as r;
use r::context::{CircuitContext, CircuitFrame, ConstructorContext, ConstructorResult};
use r::ledger::{ChargedState, ContractAddress, StateValue};
use r::recording::RecordingFrame;
fn context() -> CircuitContext<u64> {
    ConstructorResult::new(
        ConstructorContext::new(5),
        ChargedState::new(StateValue::Null),
    )
    .into_circuit_context(ContractAddress::default())
}
#[test]
fn exact_frames_preserve_original_witness_bytes_and_adopt_private_state_once() {
    let calls = std::cell::RefCell::new(Vec::new());
    let callback = |context: &CircuitContext<u64>, _: &r::context::WitnessReadMeter<'_>| {
        calls.borrow_mut().push(context.private_state);
        Ok((context.private_state + 1, r::OpaqueBytes(vec![1, 0])))
    };
    let (native, value) = CircuitFrame::new(context())
        .try_witness_metered_exact(callback)
        .unwrap();
    assert_eq!(value.0, vec![1, 0]);
    let native = native.finish(value);
    let (recorded, value) = RecordingFrame::new(context())
        .try_witness_metered_exact(callback)
        .unwrap();
    let recorded = recorded.finish(value);
    assert_eq!(*calls.borrow(), vec![5, 5]);
    assert_eq!(native.context.private_state, 6);
    assert_eq!(recorded.execution.context.private_state, 6);
    assert_eq!(native.result, recorded.execution.result);
    assert_eq!(
        native.private_transcript_outputs,
        recorded.execution.private_transcript_outputs
    );
    assert_eq!(
        native.private_transcript_outputs[0].value.0[0].0,
        vec![1, 0]
    );
    assert_eq!(native.gas_cost, recorded.execution.gas_cost);
}
#[test]
fn exact_frame_provider_failures_are_preserved_without_encoding_a_value() {
    let calls = std::cell::Cell::new(0);
    let callback = |_: &CircuitContext<u64>,
                    _: &r::context::WitnessReadMeter<'_>|
     -> Result<(u64, r::OpaqueBytes), r::CompactError> {
        calls.set(calls.get() + 1);
        Err(r::CompactError::AssertionFailed("provider failed".into()))
    };
    let native = CircuitFrame::new(context()).try_witness_metered_exact(callback);
    let recorded = RecordingFrame::new(context()).try_witness_metered_exact(callback);
    assert_eq!(
        native.err(),
        Some(r::CompactError::AssertionFailed("provider failed".into()))
    );
    assert_eq!(
        recorded.err(),
        Some(r::CompactError::AssertionFailed("provider failed".into()))
    );
    assert_eq!(calls.get(), 2);
}
