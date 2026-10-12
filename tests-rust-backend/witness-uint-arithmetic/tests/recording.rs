// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
#![cfg(feature = "ledger-transaction")]
use compact_rust_witness_uint_arithmetic_fixture::ledger_contract as c;
use midnight_compact_runtime as r;
use std::{cell::RefCell, rc::Rc};
#[path = "../../support/witness_only_recording.rs"]
mod support;
type Uint16 = r::BoundedUint<65535>;
#[derive(Clone, Default)]
struct Echo {
    events: Rc<RefCell<Vec<(u64, Uint16)>>>,
    fail: bool,
}
impl c::TryWitnesses<u64> for Echo {
    fn echo(
        &self,
        ctx: r::context::WitnessContext<'_, u64, c::LedgerView<'_>>,
        value: Uint16,
    ) -> Result<(u64, Uint16), r::CompactError> {
        self.events.borrow_mut().push((*ctx.private_state, value));
        if self.fail {
            return Err(r::CompactError::InvalidLedgerCell(
                "provider refused".into(),
            ));
        }
        Ok((*ctx.private_state + 1, value))
    }
}
#[test]
fn recorded_unsigned_arithmetic_matches_ts_and_empty_preparation() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/witness-uint-arithmetic-ts-output.json"
    ))
    .unwrap();
    let initial = c::initial_state(r::context::ConstructorContext::new(7_u64)).unwrap();
    let (state, key) = support::observation(
        initial.ledger_state.get_ref().clone(),
        &["add_echo", "subtract_echo", "multiply_echo"],
    );
    let value = Uint16::new(2).unwrap();
    for id in ["add", "subtract", "multiply"] {
        let a = Echo::default();
        let b = Echo::default();
        let w = Echo::default();
        let contract = c::Contract::from(w.clone());
        let handle = contract.recording();
        let (native, recorded, call) = match id {
            "add" => (
                c::add_echo(state.circuit_context(7), &a, value).unwrap(),
                c::recorded::add_echo(state.circuit_context(7), &b, value).unwrap(),
                handle.add_echo_call(&state, 7, value).unwrap(),
            ),
            "subtract" => (
                c::subtract_echo(state.circuit_context(7), &a, value).unwrap(),
                c::recorded::subtract_echo(state.circuit_context(7), &b, value).unwrap(),
                handle.subtract_echo_call(&state, 7, value).unwrap(),
            ),
            "multiply" => (
                c::multiply_echo(state.circuit_context(7), &a, value).unwrap(),
                c::recorded::multiply_echo(state.circuit_context(7), &b, value).unwrap(),
                handle.multiply_echo_call(&state, 7, value).unwrap(),
            ),
            _ => unreachable!(),
        };
        assert_eq!(
            native.result.value(),
            oracle[id]["result"]
                .as_str()
                .unwrap()
                .parse::<u128>()
                .unwrap()
        );
        assert_eq!(native.context.private_state, 8);
        support::private_oracle(&native, &oracle[id]);
        for result in [&recorded, call.recorded()] {
            support::parity(result, &native);
        }
        for provider in [&a, &b, &w] {
            assert_eq!(*provider.events.borrow(), vec![(7, value)]);
        }
        support::refuses_empty(call, key.clone());
    }
}
#[test]
fn arithmetic_errors_happen_after_one_witness_and_provider_errors_are_preserved() {
    let initial = c::initial_state(r::context::ConstructorContext::new(7_u64)).unwrap();
    let (state, _) = support::observation(
        initial.ledger_state.get_ref().clone(),
        &["add_echo", "subtract_echo", "multiply_echo"],
    );
    for (operation, input) in [("add", 65535), ("subtract", 0), ("multiply", 65535)] {
        for fail in [false, true] {
            let mut errors = Vec::new();
            for lane in 0..3 {
                let value = Uint16::new(input).unwrap();
                let w = Echo {
                    fail,
                    ..Default::default()
                };
                let contract = c::Contract::from(w.clone());
                let handle = contract.recording();
                let outcome = match (operation, lane) {
                    ("add", 0) => c::add_echo(state.circuit_context(7), &w, value).map(|_| ()),
                    ("add", 1) => {
                        c::recorded::add_echo(state.circuit_context(7), &w, value).map(|_| ())
                    }
                    ("add", 2) => handle.add_echo_call(&state, 7, value).map(|_| ()),
                    ("subtract", 0) => {
                        c::subtract_echo(state.circuit_context(7), &w, value).map(|_| ())
                    }
                    ("subtract", 1) => {
                        c::recorded::subtract_echo(state.circuit_context(7), &w, value).map(|_| ())
                    }
                    ("subtract", 2) => handle.subtract_echo_call(&state, 7, value).map(|_| ()),
                    ("multiply", 0) => {
                        c::multiply_echo(state.circuit_context(7), &w, value).map(|_| ())
                    }
                    ("multiply", 1) => {
                        c::recorded::multiply_echo(state.circuit_context(7), &w, value).map(|_| ())
                    }
                    ("multiply", 2) => handle.multiply_echo_call(&state, 7, value).map(|_| ()),
                    _ => unreachable!(),
                };
                let error = outcome.unwrap_err();
                if fail {
                    assert!(
                        matches!(&error,r::CompactError::InvalidLedgerCell(reason) if reason=="provider refused")
                    );
                } else {
                    assert!(!matches!(&error, r::CompactError::InvalidLedgerCell(_)));
                }
                errors.push(error.to_string());
                assert_eq!(*w.events.borrow(), vec![(7, value)]);
            }
            assert!(errors.windows(2).all(|pair| pair[0] == pair[1]));
        }
    }
}
