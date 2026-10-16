// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
#![cfg(feature = "ledger-transaction")]
mod common;
use common::{witness::*, *};
use compact_rust_effect_plan_jubjub_projections_source_fixture::{
    ledger_contract as c, ledger_slots as slots, runtime as r,
};

#[test]
fn identity_and_nonidentity_coordinates_match_official_curve_and_prepared_bindings() {
    for scalar in [0, 1, 2, 17] {
        let (state, key) = setup();
        let official = point(scalar);
        let value: r::JubjubPoint = official.into();
        let witness = Witness::new(1, 2, Some("first"));
        let native = c::project(state.circuit_context(5), value).unwrap();
        let recorded = c::recorded::project(state.circuit_context(5), value).unwrap();
        let manual = manual_project(state.circuit_context(5), official).unwrap();
        let call = c::Contract::from(witness.clone())
            .recording()
            .project_call(&state, 5, value)
            .unwrap();
        compare(&native, &recorded, &manual);
        compare(&native, call.recorded(), &manual);
        let expected = coordinates(official, official);
        assert_eq!(native.result, expected);
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
        assert_eq!(prepared.input, r::fab::AlignedValue::from((value,)));
        assert_eq!(
            prepared.output,
            r::fab::AlignedValue::from((expected.x, expected.y, expected.repeatedX))
        );
        if scalar == 0 {
            // Jubjub has no point at infinity: the official coordinate
            // accessor returns Some(0), Some(1) even for the identity.
            assert!(official.is_identity());
            assert_eq!(official.x(), Some(r::Field::from(0_u64)));
            assert_eq!(official.y(), Some(r::Field::from(1_u64)));
            assert_eq!(atoms(&prepared.input), vec![vec![], vec![1]]);
            assert_eq!(atoms(&prepared.output), vec![vec![], vec![1], vec![]]);
        } else {
            assert_ne!(expected.x, r::Field::from(0_u64));
            assert_ne!(expected.y, r::Field::from(1_u64));
        }
    }
}
#[test]
fn lazy_point_operands_are_evaluated_once_in_source_order_and_reused() {
    for selected in [false, true] {
        for (first, second) in [(1, 2), (0, 3), (4, 0)] {
            let (a, b, m, w) = (
                Witness::new(first, second, None),
                Witness::new(first, second, None),
                Witness::new(first, second, None),
                Witness::new(first, second, None),
            );
            let (state, key) = setup();
            let native = c::observe(state.circuit_context(5), &a, selected).unwrap();
            let recorded = c::recorded::observe(state.circuit_context(5), &b, selected).unwrap();
            let manual = manual_observe(state.circuit_context(5), &m, selected).unwrap();
            let call = c::Contract::from(w.clone())
                .recording()
                .observe_call(&state, 5, selected)
                .unwrap();
            compare(&native, &recorded, &manual);
            compare(&native, call.recorded(), &manual);
            let expected = if selected {
                coordinates(a.first, a.second)
            } else {
                coordinates(point(0), point(0))
            };
            assert_eq!(native.result, expected);
            assert_eq!(native.context.private_state, if selected { 7 } else { 5 });
            let events = if selected {
                vec![("first", 5, 1), ("second", 6, 1)]
            } else {
                vec![]
            };
            for provider in [&a, &b, &m, &w] {
                assert_eq!(*provider.events.borrow(), events);
            }
            let private: Vec<_> = native.private_transcript_outputs.to_vec();
            let expected_private = if selected {
                vec![
                    r::fab::AlignedValue::from(r::JubjubPoint::from(a.first)),
                    r::fab::AlignedValue::from(r::JubjubPoint::from(a.second)),
                ]
            } else {
                vec![]
            };
            assert_eq!(private, expected_private);
            let prepared = call.prepare(key, r::Field::from(0_u64)).unwrap();
            assert_eq!(
                atoms(&prepared.input),
                vec![if selected { vec![1] } else { vec![] }]
            );
            assert_eq!(
                prepared.output,
                r::fab::AlignedValue::from((expected.x, expected.y, expected.repeatedX))
            );
        }
    }
}
#[test]
fn witness_errors_stop_at_exact_prefix_and_unselected_branch_suppresses_callbacks() {
    for failed in ["first", "second"] {
        for selected in [false, true] {
            let (a, b, m, w) = (
                Witness::new(1, 2, Some(failed)),
                Witness::new(1, 2, Some(failed)),
                Witness::new(1, 2, Some(failed)),
                Witness::new(1, 2, Some(failed)),
            );
            let (state, _) = setup();
            let native = c::observe(state.circuit_context(5), &a, selected);
            let recorded = c::recorded::observe(state.circuit_context(5), &b, selected);
            let manual = manual_observe(state.circuit_context(5), &m, selected);
            let call = c::Contract::from(w.clone())
                .recording()
                .observe_call(&state, 5, selected);
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
            }
            let expected = if !selected {
                vec![]
            } else if failed == "first" {
                vec![("first", 5, 1)]
            } else {
                vec![("first", 5, 1), ("second", 6, 1)]
            };
            for provider in [&a, &b, &m, &w] {
                assert_eq!(*provider.events.borrow(), expected);
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
