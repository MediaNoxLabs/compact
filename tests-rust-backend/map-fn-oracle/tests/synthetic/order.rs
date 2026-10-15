// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use super::common::*;
#[test]
fn fallible_pure_source_precedes_left_to_right_body_errors() {
    let (state, key) = setup();
    let contract = c::Contract::from(Offset::default());
    for (input, source_ok, message) in [
        ([0, 2, 3], false, "source refused"),
        ([0, 2, 3], true, "zero item"),
        ([1, 2, 0], true, "two item"),
        ([1, 3, 0], true, "zero item"),
    ] {
        let values = vector(input);
        let native = c::fallible(state.circuit_context(7), values.clone(), source_ok)
            .err()
            .unwrap();
        let recorded = c::recorded::fallible(state.circuit_context(7), values.clone(), source_ok)
            .err()
            .unwrap();
        let observed = contract
            .recording()
            .fallible_call(&state, 7, values, source_ok)
            .err()
            .unwrap();
        assert_eq!(native, r::CompactError::AssertionFailed(message.into()));
        assert_eq!(recorded, native);
        assert_eq!(observed, native);
    }
    let values = vector([1, 3, 4]);
    let native = c::fallible(state.circuit_context(7), values.clone(), true).unwrap();
    let recorded = c::recorded::fallible(state.circuit_context(7), values.clone(), true).unwrap();
    let call = contract
        .recording()
        .fallible_call(&state, 7, values, true)
        .unwrap();
    for actual in [&recorded, call.recorded()] {
        pure_parity(&state, &native, actual, vector([11, 13, 14]));
    }
    crate::support::refuses_empty(call, key);
}
#[test]
fn outer_witness_is_called_once_and_captured_by_all_three_elements() {
    let (state, key) = setup();
    let a = Offset::default();
    let b = Offset::default();
    let w = Offset::default();
    let native = c::witnessed(state.circuit_context(7), &a).unwrap();
    let recorded = c::recorded::witnessed(state.circuit_context(7), &b).unwrap();
    let contract = c::Contract::from(w.clone());
    let call = contract.recording().witnessed_call(&state, 7).unwrap();
    let (frame, offset) = r::recording::RecordingFrame::new(state.circuit_context(7))
        .try_witness_metered(|ctx, _| Ok((ctx.private_state + 1, field(10))))
        .unwrap();
    let manual = frame.finish(r::FixedVector::new([1, 2, 3].map(|n| field(n) + offset)));
    assert_eq!(native.result, vector([11, 12, 13]));
    assert_eq!(native.context.private_state, 8);
    assert_eq!(
        native.private_transcript_outputs,
        vec![r::fab::AlignedValue::from(field(10))]
    );
    for actual in [&recorded, call.recorded(), &manual] {
        crate::support::parity(actual, &native);
    }
    for provider in [&a, &b, &w] {
        assert_eq!(*provider.events.borrow(), vec![7]);
    }
    crate::support::refuses_empty(call, key);
}
#[test]
fn refused_outer_witness_stops_before_mapping_and_preserves_callback_prefix() {
    let (state, _) = setup();
    for lane in 0..3 {
        let provider = Offset {
            fail: true,
            ..Default::default()
        };
        let contract = c::Contract::from(provider.clone());
        let error = match lane {
            0 => c::witnessed(state.circuit_context(7), &provider)
                .err()
                .unwrap(),
            1 => c::recorded::witnessed(state.circuit_context(7), &provider)
                .err()
                .unwrap(),
            _ => contract
                .recording()
                .witnessed_call(&state, 7)
                .err()
                .unwrap(),
        };
        assert_eq!(
            error,
            r::CompactError::InvalidLedgerCell("offset refused".into())
        );
        assert_eq!(*provider.events.borrow(), vec![7]);
    }
}
