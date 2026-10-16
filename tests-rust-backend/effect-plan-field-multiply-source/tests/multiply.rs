// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
#![cfg(feature = "ledger-transaction")]
mod common;
use common::{witness::*, *};
use compact_rust_effect_plan_field_multiply_source_fixture::{
    ledger_contract as c, ledger_slots as slots, runtime as r, types::Product,
};
use r::recording::RecordingFrame;

/// Independent modular identities using official field negation, never the
/// multiplication expression emitted by the compiler to compute expectations.
fn cases() -> Vec<(r::Field, r::Field, r::Field)> {
    let f = r::Field::from;
    vec![
        (f(0_u64), f(0_u64), f(0_u64)),
        (f(0_u64), -f(1_u64), f(0_u64)),
        (-f(1_u64), f(0_u64), f(0_u64)),
        (f(1_u64), -f(1_u64), -f(1_u64)),
        (-f(1_u64), f(1_u64), -f(1_u64)),
        (f(3_u64), f(7_u64), f(21_u64)),
        (-f(1_u64), -f(1_u64), f(1_u64)),
        (-f(1_u64), f(2_u64), -f(2_u64)),
        (-f(2_u64), -f(3_u64), f(6_u64)),
    ]
}
#[test]
fn field_products_preserve_zero_one_and_modular_wrap_in_prepared_calls() {
    for (a, b, expected) in cases() {
        let (state, key) = setup();
        let native = c::multiply(state.circuit_context(5), a, b).unwrap();
        let recorded = c::recorded::multiply(state.circuit_context(5), a, b).unwrap();
        let manual = slots::calls
            .record_increment(RecordingFrame::new(state.circuit_context(5)), 1)
            .unwrap()
            .finish(Product { value: expected });
        let witness = Witness::new(a, b, Some("left"));
        let call = c::Contract::from(witness.clone())
            .recording()
            .multiply_call(&state, 5, a, b)
            .unwrap();
        compare(&native, &recorded, &manual);
        compare(&native, call.recorded(), &manual);
        assert_eq!(native.result, Product { value: expected });
        assert_eq!(native.context.private_state, 5);
        assert!(native.private_transcript_outputs.is_empty());
        assert!(witness.events.borrow().is_empty());
        assert_eq!(
            slots::calls
                .inspect(native.context.query.state.get_ref())
                .unwrap(),
            1
        );
        let prepared = call.prepare(key, r::Field::from(0_u64)).unwrap();
        assert_eq!(prepared.input, r::fab::AlignedValue::from((a, b)));
        assert_eq!(prepared.output, r::fab::AlignedValue::from((expected,)));
    }
}
#[test]
fn lazy_field_operands_execute_once_in_left_to_right_order() {
    for selected in [false, true] {
        for (left, right, product) in cases() {
            let (a, b, m, w) = (
                Witness::new(left, right, None),
                Witness::new(left, right, None),
                Witness::new(left, right, None),
                Witness::new(left, right, None),
            );
            let (state, key) = setup();
            let native = c::witnessed(state.circuit_context(5), &a, selected).unwrap();
            let recorded = c::recorded::witnessed(state.circuit_context(5), &b, selected).unwrap();
            let manual = manual_witnessed(state.circuit_context(5), &m, selected, product).unwrap();
            let call = c::Contract::from(w.clone())
                .recording()
                .witnessed_call(&state, 5, selected)
                .unwrap();
            compare(&native, &recorded, &manual);
            compare(&native, call.recorded(), &manual);
            let expected = if selected {
                product
            } else {
                r::Field::from(0_u64)
            };
            assert_eq!(native.result, Product { value: expected });
            assert_eq!(native.context.private_state, if selected { 7 } else { 5 });
            let private = if selected {
                vec![
                    r::fab::AlignedValue::from(left),
                    r::fab::AlignedValue::from(right),
                ]
            } else {
                vec![]
            };
            assert_eq!(native.private_transcript_outputs, private);
            let events = if selected {
                vec![("left", 5, 1), ("right", 6, 1)]
            } else {
                vec![]
            };
            for witness in [&a, &b, &m, &w] {
                assert_eq!(*witness.events.borrow(), events);
            }
            let prepared = call.prepare(key, r::Field::from(0_u64)).unwrap();
            assert_eq!(
                atoms(&prepared.input),
                vec![if selected { vec![1] } else { vec![] }]
            );
            assert_eq!(prepared.output, r::fab::AlignedValue::from((expected,)));
        }
    }
}
#[test]
fn multiplication_witness_failures_stop_at_the_exact_prefix() {
    for failed in ["left", "right"] {
        for selected in [false, true] {
            let (left, right, product) = (
                r::Field::from(3_u64),
                r::Field::from(7_u64),
                r::Field::from(21_u64),
            );
            let (a, b, m, w) = (
                Witness::new(left, right, Some(failed)),
                Witness::new(left, right, Some(failed)),
                Witness::new(left, right, Some(failed)),
                Witness::new(left, right, Some(failed)),
            );
            let (state, _) = setup();
            let native = c::witnessed(state.circuit_context(5), &a, selected);
            let recorded = c::recorded::witnessed(state.circuit_context(5), &b, selected);
            let manual = manual_witnessed(state.circuit_context(5), &m, selected, product);
            let call = c::Contract::from(w.clone())
                .recording()
                .witnessed_call(&state, 5, selected);
            if selected {
                assert_eq!(native.err().unwrap(), failure(failed));
                assert_eq!(recorded.err().unwrap(), failure(failed));
                assert_eq!(manual.err().unwrap(), failure(failed));
                assert_eq!(call.err().unwrap(), failure(failed));
            } else {
                let (native, recorded, manual, call) = (
                    native.unwrap(),
                    recorded.unwrap(),
                    manual.unwrap(),
                    call.unwrap(),
                );
                compare(&native, &recorded, &manual);
                compare(&native, call.recorded(), &manual);
                assert_eq!(
                    native.result,
                    Product {
                        value: r::Field::from(0_u64)
                    }
                );
                assert_eq!(native.context.private_state, 5);
                assert!(native.private_transcript_outputs.is_empty());
            }
            let events = if !selected {
                vec![]
            } else if failed == "left" {
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
