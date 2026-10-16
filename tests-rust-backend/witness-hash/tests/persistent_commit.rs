// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
#![cfg(feature = "ledger-transaction")]
use compact_rust_witness_hash_fixture::ledger_contract as c;
use midnight_compact_runtime as r;
use std::{cell::RefCell, rc::Rc};
#[path = "../../support/witness_only_recording.rs"]
mod support;
#[derive(Debug, PartialEq)]
enum Event {
    Value(u64, r::Field),
    Opening(u64, r::FixedBytes<32>),
}
#[derive(Clone, Default)]
struct Echo {
    events: Rc<RefCell<Vec<Event>>>,
    fail_at: Option<u64>,
}
impl Echo {
    fn check(&self, private: u64) -> Result<(), r::CompactError> {
        if self.fail_at == Some(private) {
            Err(r::CompactError::InvalidLedgerCell(format!(
                "provider refused at {private}"
            )))
        } else {
            Ok(())
        }
    }
}
impl c::TryWitnesses<u64> for Echo {
    fn echo(
        &self,
        ctx: r::context::WitnessContext<'_, u64, c::LedgerView<'_>>,
        value: r::Field,
    ) -> Result<(u64, r::Field), r::CompactError> {
        self.events
            .borrow_mut()
            .push(Event::Value(*ctx.private_state, value));
        self.check(*ctx.private_state)?;
        Ok((
            *ctx.private_state + 1,
            value + r::Field::from(*ctx.private_state),
        ))
    }
    fn echo_bytes(
        &self,
        ctx: r::context::WitnessContext<'_, u64, c::LedgerView<'_>>,
        value: r::FixedBytes<32>,
    ) -> Result<(u64, r::FixedBytes<32>), r::CompactError> {
        self.events
            .borrow_mut()
            .push(Event::Opening(*ctx.private_state, value));
        self.check(*ctx.private_state)?;
        Ok((*ctx.private_state + 1, value))
    }
}
fn inputs() -> (r::Field, r::FixedBytes<32>) {
    (
        r::Field::from(2_u64),
        r::FixedBytes::new(std::array::from_fn(|i| (i + 1) as u8)),
    )
}
#[test]
fn persistent_commit_records_value_then_opening_and_matches_original_ts() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/witness-hash.json"
    ))
    .unwrap();
    let initial = c::initial_state(r::context::ConstructorContext::new(7_u64)).unwrap();
    let (state, key) = support::observation(
        initial.ledger_state.get_ref().clone(),
        &["persistent_commit_echo"],
    );
    let (value, opening) = inputs();
    let a = Echo::default();
    let b = Echo::default();
    let w = Echo::default();
    let native = c::persistent_commit_echo(state.circuit_context(7), &a, value, opening).unwrap();
    let recorded =
        c::recorded::persistent_commit_echo(state.circuit_context(7), &b, value, opening).unwrap();
    let contract = c::Contract::from(w.clone());
    let call = contract
        .recording()
        .persistent_commit_echo_call(&state, 7, value, opening)
        .unwrap();
    assert_eq!(
        hex::encode(native.result.0),
        oracle["persistentCommit"]["result"].as_str().unwrap()
    );
    assert_eq!(native.context.private_state, 9);
    assert_eq!(
        native.private_transcript_outputs,
        vec![
            r::fab::AlignedValue::from(r::Field::from(9_u64)),
            r::fab::AlignedValue::from(opening)
        ]
    );
    support::private_oracle(&native, &oracle["persistentCommit"]);
    for actual in [&recorded, call.recorded()] {
        support::parity(actual, &native);
    }
    for provider in [&a, &b, &w] {
        assert_eq!(
            *provider.events.borrow(),
            vec![Event::Value(7, value), Event::Opening(8, opening)]
        );
    }
    support::refuses_empty(call, key);
}
#[test]
fn first_and_second_provider_refusals_preserve_the_exact_callback_prefix() {
    let initial = c::initial_state(r::context::ConstructorContext::new(7_u64)).unwrap();
    let (state, _) = support::observation(
        initial.ledger_state.get_ref().clone(),
        &["persistent_commit_echo"],
    );
    let (value, opening) = inputs();
    for fail_at in [7, 8] {
        for lane in 0..3 {
            let w = Echo {
                fail_at: Some(fail_at),
                ..Default::default()
            };
            let contract = c::Contract::from(w.clone());
            let result = match lane {
                0 => c::persistent_commit_echo(state.circuit_context(7), &w, value, opening)
                    .map(|_| ()),
                1 => c::recorded::persistent_commit_echo(
                    state.circuit_context(7),
                    &w,
                    value,
                    opening,
                )
                .map(|_| ()),
                _ => contract
                    .recording()
                    .persistent_commit_echo_call(&state, 7, value, opening)
                    .map(|_| ()),
            };
            assert_eq!(
                result,
                Err(r::CompactError::InvalidLedgerCell(format!(
                    "provider refused at {fail_at}"
                )))
            );
            let mut expected = vec![Event::Value(7, value)];
            if fail_at == 8 {
                expected.push(Event::Opening(8, opening));
            }
            assert_eq!(*w.events.borrow(), expected);
            assert_eq!(
                state.contract().data.get_ref(),
                initial.ledger_state.get_ref()
            );
        }
    }
}
