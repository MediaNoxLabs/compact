// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
#![cfg(feature = "ledger-transaction")]
mod common;
use common::*;
use compact_rust_effect_plan_inequality_source_fixture::{
    ledger_contract as c, ledger_slots as slots, runtime as r, types,
};
use r::context::WitnessContext;
use r::recording::RecordingFrame;
type Uint = r::BoundedUint<{ u64::MAX as u128 }>;
struct NoWitness;
impl c::TryWitnesses<u64> for NoWitness {
    fn left(
        &self,
        _: WitnessContext<'_, u64, c::LedgerView<'_>>,
    ) -> Result<(u64, Uint), r::CompactError> {
        panic!("compare/owned must not call witnesses")
    }
    fn right(
        &self,
        _: WitnessContext<'_, u64, c::LedgerView<'_>>,
    ) -> Result<(u64, Uint), r::CompactError> {
        panic!("compare/owned must not call witnesses")
    }
}
fn uint_atom(value: u64) -> Vec<u8> {
    let mut bytes = value.to_le_bytes().to_vec();
    while bytes.last() == Some(&0) {
        bytes.pop();
    }
    bytes
}
#[test]
fn uint_boundaries_have_literal_public_bindings_and_manual_replay() {
    for (a, b) in [
        (0, 0),
        (0, 1),
        (1, 0),
        (u64::MAX, u64::MAX),
        (u64::MAX, 0),
        (u64::MAX - 1, u64::MAX),
    ] {
        let (state, key) = setup();
        let (left, right) = (Uint::new(a.into()).unwrap(), Uint::new(b.into()).unwrap());
        let native = c::compare(state.circuit_context(5), left, right).unwrap();
        let recorded = c::recorded::compare(state.circuit_context(5), left, right).unwrap();
        let manual = slots::calls
            .record_increment(RecordingFrame::new(state.circuit_context(5)), 1)
            .unwrap()
            .finish(types::Outcome { different: a != b });
        let call = c::Contract::from(NoWitness)
            .recording()
            .compare_call(&state, 5, left, right)
            .unwrap();
        compare(&native, &recorded, &manual);
        compare(&native, call.recorded(), &manual);
        assert_eq!(native.context.private_state, 5);
        assert!(native.private_transcript_outputs.is_empty());
        let prepared = call.prepare(key, r::Field::from(0_u64)).unwrap();
        assert_eq!(atoms(&prepared.input), vec![uint_atom(a), uint_atom(b)]);
        assert_eq!(
            atoms(&prepared.output),
            vec![if a != b { vec![1] } else { vec![] }]
        );
    }
}
fn payload(text: &str, bytes: &[u8]) -> types::Payload {
    types::Payload {
        text: r::OpaqueString::from(text),
        bytes: r::OpaqueBytes::from(bytes.to_vec()),
    }
}
#[test]
fn owned_products_are_compared_then_reused_without_moving_or_normalizing_values() {
    for (a, b) in [
        (payload("", &[]), payload("", &[])),
        (payload("é", &[1, 2]), payload("é", &[1, 2])),
        (payload("é", &[1, 2]), payload("e", &[1, 2])),
        (payload("same", &[1, 2]), payload("same", &[1, 3])),
    ] {
        let (state, key) = setup();
        let native = c::owned(state.circuit_context(5), a.clone(), b.clone()).unwrap();
        let recorded = c::recorded::owned(state.circuit_context(5), a.clone(), b.clone()).unwrap();
        let different = a.text.0 != b.text.0 || a.bytes.0 != b.bytes.0;
        let manual = slots::calls
            .record_increment(RecordingFrame::new(state.circuit_context(5)), 1)
            .unwrap()
            .finish(types::OwnedResult {
                different,
                original: a.clone(),
            });
        let call = c::Contract::from(NoWitness)
            .recording()
            .owned_call(&state, 5, a.clone(), b.clone())
            .unwrap();
        compare(&native, &recorded, &manual);
        compare(&native, call.recorded(), &manual);
        assert_eq!(native.result.original, a);
        assert!(native.private_transcript_outputs.is_empty());
        let prepared = call.prepare(key, r::Field::from(0_u64)).unwrap();
        assert_eq!(
            atoms(&prepared.input),
            vec![
                a.text.0.as_bytes().to_vec(),
                a.bytes.0.clone(),
                b.text.0.as_bytes().to_vec(),
                b.bytes.0.clone()
            ]
        );
        assert_eq!(
            atoms(&prepared.output),
            vec![
                if different { vec![1] } else { vec![] },
                a.text.0.as_bytes().to_vec(),
                a.bytes.0.clone()
            ]
        );
    }
}
#[test]
fn raw_owned_comparison_retains_terminal_zeros_while_observed_boundary_refuses_them() {
    for (a, b) in [
        (payload("same", &[1, 0]), payload("same", &[1])),
        (payload("x\0", &[1]), payload("x", &[1])),
        (payload("same", &[1]), payload("same", &[1, 0])),
    ] {
        let (state, _) = setup();
        let native = c::owned(state.circuit_context(5), a.clone(), b.clone()).unwrap();
        let recorded = c::recorded::owned(state.circuit_context(5), a.clone(), b.clone()).unwrap();
        let manual = slots::calls
            .record_increment(RecordingFrame::new(state.circuit_context(5)), 1)
            .unwrap()
            .finish(types::OwnedResult {
                different: true,
                original: a.clone(),
            });
        compare(&native, &recorded, &manual);
        assert!(native.result.different);
        assert_eq!(native.result.original, a);
        let error = c::Contract::from(NoWitness)
            .recording()
            .owned_call(&state, 5, a, b)
            .err()
            .unwrap();
        assert_eq!(
            error,
            r::CompactError::InvalidLedgerCell(
                "opaque ledger input has trailing zero bytes".into()
            )
        );
    }
}
