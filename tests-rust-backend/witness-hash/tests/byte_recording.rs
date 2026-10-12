// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
#![cfg(feature = "ledger-transaction")]
use compact_rust_witness_hash_fixture::ledger_contract as c;
use midnight_compact_runtime as r;
use std::{cell::RefCell, rc::Rc};
#[path = "../../support/witness_only_recording.rs"]
mod support;
#[derive(Clone, Default)]
struct Echo {
    events: Rc<RefCell<Vec<(u64, r::Field)>>>,
    fail: bool,
}
impl c::TryWitnesses<u64> for Echo {
    fn echo(
        &self,
        ctx: r::context::WitnessContext<'_, u64, c::LedgerView<'_>>,
        value: r::Field,
    ) -> Result<(u64, r::Field), r::CompactError> {
        self.events.borrow_mut().push((*ctx.private_state, value));
        if self.fail {
            return Err(r::CompactError::InvalidLedgerCell(
                "provider refused".into(),
            ));
        }
        Ok((
            *ctx.private_state + 1,
            value + r::Field::from(*ctx.private_state),
        ))
    }
    fn echo_bytes(
        &self,
        _: r::context::WitnessContext<'_, u64, c::LedgerView<'_>>,
        _: r::FixedBytes<32>,
    ) -> Result<(u64, r::FixedBytes<32>), r::CompactError> {
        panic!("these roots must only call echo")
    }
}
#[test]
fn recorded_bytes_match_retained_ts_answers_and_empty_preparation() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/witness-hash.json"
    ))
    .unwrap();
    let initial = c::initial_state(r::context::ConstructorContext::new(7_u64)).unwrap();
    let (state, key) = support::observation(
        initial.ledger_state.get_ref().clone(),
        &["persistent_hash_echo", "upgrade_echo"],
    );
    let value = r::Field::from(2_u64);
    for id in ["persistentHash", "upgrade"] {
        let a = Echo::default();
        let b = Echo::default();
        let w = Echo::default();
        let contract = c::Contract::from(w.clone());
        let handle = contract.recording();
        let (native, recorded, call) = match id {
            "persistentHash" => (
                c::persistent_hash_echo(state.circuit_context(7), &a, value).unwrap(),
                c::recorded::persistent_hash_echo(state.circuit_context(7), &b, value).unwrap(),
                handle.persistent_hash_echo_call(&state, 7, value).unwrap(),
            ),
            "upgrade" => (
                c::upgrade_echo(state.circuit_context(7), &a, value).unwrap(),
                c::recorded::upgrade_echo(state.circuit_context(7), &b, value).unwrap(),
                handle.upgrade_echo_call(&state, 7, value).unwrap(),
            ),
            _ => unreachable!(),
        };
        assert_eq!(
            hex::encode(native.result.0),
            oracle[id]["result"].as_str().unwrap()
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
fn provider_refusal_precedes_hash_and_preserves_one_callback() {
    let initial = c::initial_state(r::context::ConstructorContext::new(7_u64)).unwrap();
    let (state, _) = support::observation(
        initial.ledger_state.get_ref().clone(),
        &["persistent_hash_echo", "upgrade_echo"],
    );
    for id in ["persistentHash", "upgrade"] {
        for lane in 0..3 {
            let value = r::Field::from(2_u64);
            let w = Echo {
                fail: true,
                ..Default::default()
            };
            let contract = c::Contract::from(w.clone());
            let handle = contract.recording();
            let result = match (id, lane) {
                ("persistentHash", 0) => {
                    c::persistent_hash_echo(state.circuit_context(7), &w, value).map(|_| ())
                }
                ("persistentHash", 1) => {
                    c::recorded::persistent_hash_echo(state.circuit_context(7), &w, value)
                        .map(|_| ())
                }
                ("persistentHash", 2) => handle
                    .persistent_hash_echo_call(&state, 7, value)
                    .map(|_| ()),
                ("upgrade", 0) => c::upgrade_echo(state.circuit_context(7), &w, value).map(|_| ()),
                ("upgrade", 1) => {
                    c::recorded::upgrade_echo(state.circuit_context(7), &w, value).map(|_| ())
                }
                ("upgrade", 2) => handle.upgrade_echo_call(&state, 7, value).map(|_| ()),
                _ => unreachable!(),
            };
            assert!(
                matches!(result,Err(r::CompactError::InvalidLedgerCell(ref reason)) if reason=="provider refused")
            );
            assert_eq!(*w.events.borrow(), vec![(7, value)]);
        }
    }
}
