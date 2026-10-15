// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
#![cfg(feature = "ledger-transaction")]
mod common;
use common::witness::*;
use common::*;
use compact_rust_effect_plan_pure_unit_statements_source_fixture::{
    ledger_contract as c, ledger_slots as slots, runtime as r,
};

fn event(name: &'static str, private: u64) -> Event {
    (name, private, 1, r::Field::from(0_u64))
}
#[test]
fn transitive_unit_guard_has_lazy_callbacks_and_exact_sequential_replay() {
    for selected in [false, true] {
        let (a, b, m, w) = (
            Witness::new(selected, selected, None),
            Witness::new(selected, selected, None),
            Witness::new(selected, selected, None),
            Witness::new(selected, selected, None),
        );
        let (state, key) = setup();
        let native = c::run(state.circuit_context(5), &a, selected).unwrap();
        let recorded = c::recorded::run(state.circuit_context(5), &b, selected).unwrap();
        let manual = manual_run(state.circuit_context(5), &m, selected).unwrap();
        let call = c::Contract::from(w.clone())
            .recording()
            .run_call(&state, 5, selected)
            .unwrap();
        compare(&native, &recorded, &manual);
        compare(&native, call.recorded(), &manual);
        assert_eq!(
            slots::calls
                .inspect(native.context.query.state.get_ref())
                .unwrap(),
            2
        );
        assert_eq!(
            slots::last
                .inspect(native.context.query.state.get_ref())
                .unwrap(),
            r::Field::from(42_u64)
        );
        assert_eq!(native.context.private_state, if selected { 8 } else { 6 });
        let expected = if selected {
            vec![event("left", 5), event("right", 6), event("marker", 7)]
        } else {
            vec![event("marker", 5)]
        };
        for provider in [&a, &b, &m, &w] {
            assert_eq!(*provider.events.borrow(), expected);
        }
        let outputs: Vec<_> = native
            .private_transcript_outputs
            .iter()
            .map(atoms)
            .collect();
        assert_eq!(
            outputs,
            if selected {
                vec![vec![vec![1]], vec![vec![1]], vec![vec![42]]]
            } else {
                vec![vec![vec![42]]]
            }
        );
        let prepared = call.prepare(key, r::Field::from(0_u64)).unwrap();
        assert_eq!(
            atoms(&prepared.input),
            vec![if selected { vec![1] } else { vec![] }]
        );
        assert!(atoms(&prepared.output).is_empty());
    }
}
#[test]
fn callback_failures_stop_at_the_exact_prefix_and_lazy_branch_suppresses_them() {
    for failed in ["left", "right", "marker"] {
        for selected in [false, true] {
            let (a, b, m, w) = (
                Witness::new(true, true, Some(failed)),
                Witness::new(true, true, Some(failed)),
                Witness::new(true, true, Some(failed)),
                Witness::new(true, true, Some(failed)),
            );
            let (state, _) = setup();
            let native = c::run(state.circuit_context(5), &a, selected);
            let recorded = c::recorded::run(state.circuit_context(5), &b, selected);
            let manual = manual_run(state.circuit_context(5), &m, selected);
            let call = c::Contract::from(w.clone())
                .recording()
                .run_call(&state, 5, selected);
            if selected || failed == "marker" {
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
            }
            let expected = if !selected {
                vec![event("marker", 5)]
            } else {
                match failed {
                    "left" => vec![event("left", 5)],
                    "right" => vec![event("left", 5), event("right", 6)],
                    _ => vec![event("left", 5), event("right", 6), event("marker", 7)],
                }
            };
            for provider in [&a, &b, &m, &w] {
                assert_eq!(*provider.events.borrow(), expected);
            }
        }
    }
}
#[test]
fn pure_guard_failure_occurs_after_both_arguments_before_marker_or_write() {
    for (left, right, message) in [
        (false, false, "left guard"),
        (false, true, "left guard"),
        (true, false, "right guard"),
    ] {
        let (a, b, m, w) = (
            Witness::new(left, right, None),
            Witness::new(left, right, None),
            Witness::new(left, right, None),
            Witness::new(left, right, None),
        );
        let (state, _) = setup();
        assert_eq!(
            c::run(state.circuit_context(5), &a, true).err().unwrap(),
            assertion(message)
        );
        assert_eq!(
            c::recorded::run(state.circuit_context(5), &b, true)
                .err()
                .unwrap(),
            assertion(message)
        );
        assert_eq!(
            manual_run(state.circuit_context(5), &m, true)
                .err()
                .unwrap(),
            assertion(message)
        );
        assert_eq!(
            c::Contract::from(w.clone())
                .recording()
                .run_call(&state, 5, true)
                .err()
                .unwrap(),
            assertion(message)
        );
        for provider in [&a, &b, &m, &w] {
            assert_eq!(
                *provider.events.borrow(),
                vec![event("left", 5), event("right", 6)]
            );
        }
        let unchanged = state.circuit_context(5);
        assert_eq!(
            slots::calls
                .inspect(unchanged.query.state.get_ref())
                .unwrap(),
            0
        );
        assert_eq!(
            slots::last
                .inspect(unchanged.query.state.get_ref())
                .unwrap(),
            r::Field::from(0_u64)
        );
    }
}
#[test]
fn fallible_pure_arguments_fail_left_to_right_before_the_callee() {
    for (a, b) in [(false, false), (false, true), (true, false), (true, true)] {
        let (state, key) = setup();
        let witness = Witness::new(false, false, Some("left"));
        let native = c::argument_order(state.circuit_context(5), a, b);
        let recorded = c::recorded::argument_order(state.circuit_context(5), a, b);
        let manual = manual_arguments(state.circuit_context(5), a, b);
        let call = c::Contract::from(witness.clone())
            .recording()
            .argument_order_call(&state, 5, a, b);
        if !a || !b {
            let error = assertion(if !a {
                "first pure argument"
            } else {
                "second pure argument"
            });
            assert_eq!(native.err().unwrap(), error);
            assert_eq!(recorded.err().unwrap(), error);
            assert_eq!(manual.err().unwrap(), error);
            assert_eq!(call.err().unwrap(), error);
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
                slots::calls
                    .inspect(native.context.query.state.get_ref())
                    .unwrap(),
                2
            );
            assert!(native.private_transcript_outputs.is_empty());
            assert_eq!(native.context.private_state, 5);
            let prepared = call.prepare(key, r::Field::from(0_u64)).unwrap();
            assert_eq!(atoms(&prepared.input), vec![vec![1], vec![1]]);
            assert!(atoms(&prepared.output).is_empty());
        }
        assert!(witness.events.borrow().is_empty());
    }
}
