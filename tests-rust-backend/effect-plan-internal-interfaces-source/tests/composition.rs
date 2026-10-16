// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
#![cfg(feature = "ledger-transaction")]
mod common;
use common::*;
use compact_rust_effect_plan_internal_interfaces_source_fixture::{
    ledger_contract as c, runtime as r, support::*, types,
};
use r::fab::{Alignment, AlignmentAtom};
fn events(selected: bool) -> Vec<(u64, u128)> {
    if selected {
        vec![(5, 1), (6, 2), (7, 3)]
    } else {
        vec![(5, 1), (6, 2)]
    }
}
#[test]
fn public_wrapper_retains_owned_tuples_and_lazy_call_order() {
    for bytes in [vec![], vec![1, 0, 255]] {
        for selected in [false, true] {
            let (state, key) = setup();
            let input: r::OpaqueBytes = bytes.clone().into();
            let (a, b, m, w) = (
                Witness::default(),
                Witness::default(),
                Witness::default(),
                Witness::default(),
            );
            let native = c::run(state.circuit_context(5), &a, input.clone(), selected).unwrap();
            let recorded =
                c::recorded::run(state.circuit_context(5), &b, input.clone(), selected).unwrap();
            let manual = manual(state.circuit_context(5), &m, input.clone(), selected).unwrap();
            assert_eq!(
                native.result,
                types::Result {
                    chosen: input.clone(),
                    original: input.clone(),
                    value: r::Field::from(11_u64)
                }
            );
            compare(&native, &recorded, &manual);
            let contract = c::Contract::from(w.clone());
            let call = contract
                .recording()
                .run_call(&state, 5, input, selected)
                .unwrap();
            compare(&native, call.recorded(), &manual);
            for witness in [&a, &b, &m, &w] {
                assert_eq!(*witness.events.borrow(), events(selected));
            }
            assert_eq!(native.context.private_state, if selected { 8 } else { 7 });
            let private = (1..=if selected { 3 } else { 2 })
                .map(|n| vec![vec![n]])
                .collect::<Vec<_>>();
            assert_eq!(
                native
                    .private_transcript_outputs
                    .iter()
                    .map(atoms)
                    .collect::<Vec<_>>(),
                private
            );
            let prepared = call.prepare(key, r::Field::from(0_u64)).unwrap();
            assert_eq!(
                atoms(&prepared.input),
                vec![bytes.clone(), if selected { vec![1] } else { vec![] }]
            );
            assert_eq!(
                atoms(&prepared.output),
                vec![bytes.clone(), bytes.clone(), vec![11]]
            );
            let opaque = Alignment::singleton(AlignmentAtom::Compress);
            let boolean = Alignment::singleton(AlignmentAtom::Bytes { length: 1 });
            let field = Alignment::singleton(AlignmentAtom::Field);
            assert_eq!(
                prepared.input.alignment,
                Alignment::concat([&opaque, &boolean])
            );
            assert_eq!(
                prepared.output.alignment,
                Alignment::concat([&opaque, &opaque, &field])
            );
        }
    }
}
#[test]
fn raw_owned_values_survive_local_calls_but_observed_public_validation_precedes_callbacks() {
    for selected in [false, true] {
        let (state, _) = setup();
        let input: r::OpaqueBytes = vec![9, 0].into();
        let (a, b, m, w) = (
            Witness::default(),
            Witness::default(),
            Witness::default(),
            Witness::default(),
        );
        let native = c::run(state.circuit_context(5), &a, input.clone(), selected).unwrap();
        let recorded =
            c::recorded::run(state.circuit_context(5), &b, input.clone(), selected).unwrap();
        let manual = manual(state.circuit_context(5), &m, input.clone(), selected).unwrap();
        compare(&native, &recorded, &manual);
        assert_eq!(native.result.chosen, input);
        assert_eq!(native.result.original, input);
        for witness in [&a, &b, &m] {
            assert_eq!(*witness.events.borrow(), events(selected));
        }
        let contract = c::Contract::from(w.clone());
        assert_eq!(
            contract
                .recording()
                .run_call(&state, 5, input, selected)
                .err()
                .unwrap(),
            r::CompactError::InvalidLedgerCell(
                "opaque ledger input has trailing zero bytes".into()
            )
        );
        assert!(w.events.borrow().is_empty());
    }
}
#[test]
fn failure_stops_at_the_same_callback_and_inactive_call_does_not_fail() {
    for selected in [false, true] {
        for fail_at in [1, 2, 3] {
            let (state, _) = setup();
            let before = state.circuit_context(5).query.state;
            let (a, b, m, w) = (
                Witness::new(Some(fail_at)),
                Witness::new(Some(fail_at)),
                Witness::new(Some(fail_at)),
                Witness::new(Some(fail_at)),
            );
            let native = c::run(state.circuit_context(5), &a, vec![1].into(), selected);
            let recorded = c::recorded::run(state.circuit_context(5), &b, vec![1].into(), selected);
            let manual = manual(state.circuit_context(5), &m, vec![1].into(), selected);
            let contract = c::Contract::from(w.clone());
            let call = contract
                .recording()
                .run_call(&state, 5, vec![1].into(), selected);
            let expected = events(selected)
                .into_iter()
                .take(fail_at as usize)
                .collect::<Vec<_>>();
            for witness in [&a, &b, &m, &w] {
                assert_eq!(*witness.events.borrow(), expected);
            }
            if !selected && fail_at == 3 {
                let (native, recorded, manual, call) = (
                    native.unwrap(),
                    recorded.unwrap(),
                    manual.unwrap(),
                    call.unwrap(),
                );
                compare(&native, &recorded, &manual);
                compare(&native, call.recorded(), &manual);
            } else {
                assert_eq!(native.err().unwrap(), failure());
                assert_eq!(recorded.err().unwrap(), failure());
                assert_eq!(manual.err().unwrap(), failure());
                assert_eq!(call.err().unwrap(), failure());
            }
            assert_eq!(state.circuit_context(5).query.state, before);
        }
    }
}
