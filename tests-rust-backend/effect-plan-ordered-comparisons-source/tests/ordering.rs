// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
#![cfg(feature = "ledger-transaction")]
mod common;
use common::{witness::*, *};
use compact_rust_effect_plan_ordered_comparisons_source_fixture::{
    ledger_contract as c, ledger_slots as slots, runtime as r,
    types::{Outcome, Relations},
};
use r::recording::RecordingFrame;

// Literal truth vectors in source field order: <, <=, >, >=.
const EQUAL: [bool; 4] = [false, true, false, true];
const LESS: [bool; 4] = [true, true, false, false];
const GREATER: [bool; 4] = [false, false, true, true];
fn relations(v: [bool; 4]) -> Relations {
    Relations {
        less: v[0],
        lessEqual: v[1],
        greater: v[2],
        greaterEqual: v[3],
    }
}
fn output(v: [bool; 4]) -> r::fab::AlignedValue {
    (v[0], v[1], v[2], v[3]).into()
}

#[test]
fn mixed_public_bounds_preserve_frontend_promotion_and_all_four_relations() {
    // Genuine Compact normalizes the Uint8 operand to Uint64. Unequal typed-IR
    // maxima are qualified separately by compiler tests, not claimed here.
    for (left, right, expected) in [
        (0, 0, EQUAL),
        (0, 1, LESS),
        (1, 0, GREATER),
        (255, 255, EQUAL),
        (254, 255, LESS),
        (255, 254, GREATER),
        (255, 256, LESS),
        (255, u64::MAX as u128, LESS),
    ] {
        let (state, key) = setup();
        let (a, b) = (Small::new(left).unwrap(), Large::new(right).unwrap());
        let native = c::compare(state.circuit_context(5), a, b).unwrap();
        let recorded = c::recorded::compare(state.circuit_context(5), a, b).unwrap();
        let manual = slots::calls
            .record_increment(RecordingFrame::new(state.circuit_context(5)), 1)
            .unwrap()
            .finish(relations(expected));
        let call = c::Contract::from(Witness::new(0, 0, Some("left")))
            .recording()
            .compare_call(&state, 5, a, b)
            .unwrap();
        compare(&native, &recorded, &manual);
        compare(&native, call.recorded(), &manual);
        assert_eq!(native.result, relations(expected));
        assert_eq!(native.context.private_state, 5);
        assert!(native.private_transcript_outputs.is_empty());
        assert_eq!(
            slots::calls
                .inspect(native.context.query.state.get_ref())
                .unwrap(),
            1
        );
        let prepared = call.prepare(key, r::Field::from(0_u64)).unwrap();
        assert_eq!(prepared.input, r::fab::AlignedValue::from((a, b)));
        assert_eq!(prepared.output, output(expected));
    }
}
#[test]
fn maximum_width_ordering_has_no_signed_conversion_or_overflow() {
    type Full = r::BoundedUint<{ u128::MAX }>;
    for (left, right, expected) in [
        (0, 0, EQUAL),
        (0, 1, LESS),
        (1, 0, GREATER),
        (u128::MAX, u128::MAX, EQUAL),
        (u128::MAX - 1, u128::MAX, LESS),
        (u128::MAX, u128::MAX - 1, GREATER),
        (0, u128::MAX, LESS),
        (u128::MAX, 0, GREATER),
        ((1_u128 << 127) - 1, 1_u128 << 127, LESS),
    ] {
        let (state, key) = setup();
        let (a, b) = (Full::new(left).unwrap(), Full::new(right).unwrap());
        let native = c::compare128(state.circuit_context(5), a, b).unwrap();
        let recorded = c::recorded::compare128(state.circuit_context(5), a, b).unwrap();
        let manual = slots::calls
            .record_increment(RecordingFrame::new(state.circuit_context(5)), 1)
            .unwrap()
            .finish(relations(expected));
        let call = c::Contract::from(Witness::new(0, 0, Some("left")))
            .recording()
            .compare128_call(&state, 5, a, b)
            .unwrap();
        compare(&native, &recorded, &manual);
        compare(&native, call.recorded(), &manual);
        assert_eq!(native.result, relations(expected));
        assert_eq!(native.context.private_state, 5);
        assert!(native.private_transcript_outputs.is_empty());
        let prepared = call.prepare(key, r::Field::from(0_u64)).unwrap();
        assert_eq!(prepared.input, r::fab::AlignedValue::from((a, b)));
        assert_eq!(prepared.output, output(expected));
    }
}
#[test]
fn lazy_comparisons_preserve_left_to_right_witness_schedule_and_failure_prefix() {
    for selected in [false, true] {
        for (left, right, less) in [(0, 0, false), (1, 2, true), (2, 1, false), (255, 256, true)] {
            for fail in [None, Some("left"), Some("right")] {
                let (a, b, m, w) = (
                    Witness::new(left, right, fail),
                    Witness::new(left, right, fail),
                    Witness::new(left, right, fail),
                    Witness::new(left, right, fail),
                );
                let (state, key) = setup();
                let native = c::witnessed(state.circuit_context(5), &a, selected);
                let recorded = c::recorded::witnessed(state.circuit_context(5), &b, selected);
                let manual = manual_witnessed(state.circuit_context(5), &m, selected, less);
                let call = c::Contract::from(w.clone())
                    .recording()
                    .witnessed_call(&state, 5, selected);
                if let Some(failed) = fail.filter(|_| selected) {
                    let expected = failure(failed);
                    assert_eq!(native.err().unwrap(), expected);
                    assert_eq!(recorded.err().unwrap(), expected);
                    assert_eq!(manual.err().unwrap(), expected);
                    assert_eq!(call.err().unwrap(), expected);
                } else {
                    let (native, recorded, manual, call) = (
                        native.unwrap(),
                        recorded.unwrap(),
                        manual.unwrap(),
                        call.unwrap(),
                    );
                    compare(&native, &recorded, &manual);
                    compare(&native, call.recorded(), &manual);
                    let expected = selected && less;
                    assert_eq!(native.result, Outcome { less: expected });
                    assert_eq!(native.context.private_state, if selected { 7 } else { 5 });
                    let private = if selected {
                        vec![
                            r::fab::AlignedValue::from(a.left),
                            r::fab::AlignedValue::from(a.right),
                        ]
                    } else {
                        vec![]
                    };
                    assert_eq!(native.private_transcript_outputs, private);
                    let prepared = call.prepare(key, r::Field::from(0_u64)).unwrap();
                    assert_eq!(
                        atoms(&prepared.input),
                        vec![if selected { vec![1] } else { vec![] }]
                    );
                    assert_eq!(prepared.output, r::fab::AlignedValue::from((expected,)));
                }
                let events = if !selected {
                    vec![]
                } else if fail == Some("left") {
                    vec![("left", 5, 1)]
                } else {
                    vec![("left", 5, 1), ("right", 6, 1)]
                };
                for witness in [&a, &b, &m, &w] {
                    assert_eq!(*witness.events.borrow(), events);
                }
                assert_eq!(
                    slots::calls
                        .inspect(state.circuit_context(5).query.state.get_ref())
                        .unwrap(),
                    0
                );
            }
        }
    }
}
