// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Source-bound Unit interfaces; no fresh TS oracle claim.
#![cfg(feature = "ledger-transaction")]
mod common;
use compact_rust_opaque_cell_inputs_source_fixture::{
    ledger_contract as c, ledger_slots as slots, runtime as r,
};
use r::fab::AlignedValue;
use r::recording::RecordingFrame;

#[test]
fn byte_inputs_preserve_execution_replay_and_prepared_public_binding() {
    let (state, verifier) = common::setup();
    let original = state.contract().clone();
    for bytes in [vec![], vec![1], vec![0, 1, 0, 255]] {
        let value = r::OpaqueBytes::from(bytes.clone());
        let native = c::setBytes(state.circuit_context(7), value.clone()).unwrap();
        let recorded = c::recorded::setBytes(state.circuit_context(7), value.clone()).unwrap();
        let manual = slots::blob
            .record_write(RecordingFrame::new(state.circuit_context(7)), value.clone())
            .unwrap()
            .finish(());
        let call = c::recorded::Contract
            .setBytes_call(&state, 7, value.clone())
            .unwrap();
        common::compare(&native, &recorded, call.recorded(), &manual);
        assert_eq!(
            slots::blob
                .inspect(native.context.query.state.get_ref())
                .unwrap(),
            value
        );
        let prepared = call.prepare(verifier.clone(), 0_u64.into()).unwrap();
        assert_eq!(prepared.input, AlignedValue::from(value));
        assert_eq!(common::atoms(&prepared.input), vec![bytes]);
        assert_eq!(prepared.output, AlignedValue::from(()));
        assert_eq!(state.contract(), &original);
    }
}

#[test]
fn nested_inputs_preserve_all_fields_and_prepared_argument_order() {
    let (state, verifier) = common::setup();
    let original = state.contract().clone();
    for (text, bytes) in [
        ("", vec![]),
        ("λ\0key", vec![1]),
        ("stored ✓", vec![0, 1, 0, 255]),
    ] {
        let value = common::envelope(text, &bytes);
        let native = c::setEnvelope(state.circuit_context(7), value.clone()).unwrap();
        let recorded = c::recorded::setEnvelope(state.circuit_context(7), value.clone()).unwrap();
        let manual = slots::envelope
            .record_write(RecordingFrame::new(state.circuit_context(7)), value.clone())
            .unwrap()
            .finish(());
        let call = c::recorded::Contract
            .setEnvelope_call(&state, 7, value.clone())
            .unwrap();
        common::compare(&native, &recorded, call.recorded(), &manual);
        assert_eq!(
            slots::envelope
                .inspect(native.context.query.state.get_ref())
                .unwrap(),
            value
        );
        let prepared = call.prepare(verifier.clone(), 0_u64.into()).unwrap();
        assert_eq!(prepared.input, AlignedValue::from(value));
        assert_eq!(
            common::atoms(&prepared.input),
            vec![text.as_bytes().to_vec(), bytes, vec![23]]
        );
        assert_eq!(prepared.output, AlignedValue::from(()));
        assert_eq!(state.contract(), &original);
    }
}
