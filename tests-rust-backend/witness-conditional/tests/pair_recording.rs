// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
#![cfg(feature = "ledger-transaction")]
use compact_rust_witness_conditional_fixture::ledger_contract as c;
use midnight_compact_runtime as r;
use std::{cell::RefCell, rc::Rc};
#[path = "../../support/witness_only_recording.rs"]
mod support;
#[derive(Clone, Default)]
struct Secret {
    events: Rc<RefCell<Vec<(u64, r::Field)>>>,
    fail: Option<u64>,
}
impl c::TryWitnesses<u64> for Secret {
    fn secret(
        &self,
        ctx: r::context::WitnessContext<'_, u64, c::LedgerView<'_>>,
        seed: r::Field,
    ) -> Result<(u64, r::Field), r::CompactError> {
        self.events.borrow_mut().push((*ctx.private_state, seed));
        if self.fail == Some(*ctx.private_state) {
            return Err(r::CompactError::InvalidLedgerCell(
                "provider refused".into(),
            ));
        }
        Ok((
            *ctx.private_state + 1,
            seed + r::Field::from(*ctx.private_state),
        ))
    }
}
#[test]
fn recorded_pair_preserves_ts_tuple_and_sequential_private_outputs() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/witness-conditional-ts-output.json"
    ))
    .unwrap();
    let initial = c::initial_state(r::context::ConstructorContext::new(7_u64)).unwrap();
    let (state, key) =
        support::observation(initial.ledger_state.get_ref().clone(), &["pair_secret"]);
    let value = r::Field::from(2_u64);
    let a = Secret::default();
    let b = Secret::default();
    let w = Secret::default();
    let native = c::pair_secret(state.circuit_context(7), &a, value).unwrap();
    let recorded = c::recorded::pair_secret(state.circuit_context(7), &b, value).unwrap();
    let contract = c::Contract::from(w.clone());
    let call = contract
        .recording()
        .pair_secret_call(&state, 7, value)
        .unwrap();
    let expected = oracle["pair"]["result"].as_array().unwrap();
    assert_eq!(
        native.result,
        (
            r::Field::from(expected[0].as_str().unwrap().parse::<u64>().unwrap()),
            r::Field::from(expected[1].as_str().unwrap().parse::<u64>().unwrap())
        )
    );
    assert_eq!(native.context.private_state, 9);
    support::private_oracle(&native, &oracle["pair"]);
    for result in [&recorded, call.recorded()] {
        support::parity(result, &native);
    }
    for provider in [&a, &b, &w] {
        assert_eq!(*provider.events.borrow(), vec![(7, value), (8, value)]);
    }
    support::refuses_empty(call, key);
}
#[test]
fn pair_provider_failure_stops_at_the_exact_element() {
    let initial = c::initial_state(r::context::ConstructorContext::new(7_u64)).unwrap();
    let (state, _) = support::observation(initial.ledger_state.get_ref().clone(), &["pair_secret"]);
    for fail in [7, 8] {
        for lane in 0..3 {
            let value = r::Field::from(2_u64);
            let w = Secret {
                fail: Some(fail),
                ..Default::default()
            };
            let contract = c::Contract::from(w.clone());
            let result = match lane {
                0 => c::pair_secret(state.circuit_context(7), &w, value).map(|_| ()),
                1 => c::recorded::pair_secret(state.circuit_context(7), &w, value).map(|_| ()),
                _ => contract
                    .recording()
                    .pair_secret_call(&state, 7, value)
                    .map(|_| ()),
            };
            assert!(
                matches!(result,Err(r::CompactError::InvalidLedgerCell(ref reason)) if reason=="provider refused")
            );
            assert_eq!(
                *w.events.borrow(),
                if fail == 7 {
                    vec![(7, value)]
                } else {
                    vec![(7, value), (8, value)]
                }
            );
        }
    }
}
