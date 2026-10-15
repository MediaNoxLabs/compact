// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
#![cfg(feature = "ledger-transaction")]
use compact_rust_equality_fixture::ledger_contract as c;
use midnight_compact_runtime as r;
use std::{cell::RefCell, rc::Rc};
#[path = "../../support/witness_only_recording.rs"]
mod support;
type Value = r::Field;
fn value(n: u64) -> Value {
    r::Field::from(n)
}
#[derive(Clone, Default)]
struct Echo {
    events: Rc<RefCell<Vec<(u64, Value)>>>,
    fail: Option<u64>,
}
impl c::TryWitnesses<u64> for Echo {
    fn echo(
        &self,
        ctx: r::context::WitnessContext<'_, u64, c::LedgerView<'_>>,
        value: Value,
    ) -> Result<(u64, Value), r::CompactError> {
        self.events.borrow_mut().push((*ctx.private_state, value));
        if self.fail == Some(*ctx.private_state) {
            return Err(r::CompactError::InvalidLedgerCell(
                "provider refused".into(),
            ));
        }
        Ok((
            *ctx.private_state + 1,
            value + r::Field::from(*ctx.private_state),
        ))
    }
}
#[test]
fn recorded_comparison_preserves_ts_values_order_and_empty_preparation() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/equality.json"
    ))
    .unwrap();
    let initial = c::initial_state(r::context::ConstructorContext::new(7_u64)).unwrap();
    let (state, key) =
        support::observation(initial.ledger_state.get_ref().clone(), &["equal_echo"]);
    for (id, left, right) in [("witnessEqual", 2, 1), ("witnessDifferent", 2, 2)] {
        let (left, right) = (value(left), value(right));
        let a = Echo::default();
        let b = Echo::default();
        let w = Echo::default();
        let native = c::equal_echo(state.circuit_context(7), &a, left, right).unwrap();
        let recorded = c::recorded::equal_echo(state.circuit_context(7), &b, left, right).unwrap();
        let contract = c::Contract::from(w.clone());
        let call = contract
            .recording()
            .equal_echo_call(&state, 7, left, right)
            .unwrap();
        assert_eq!(native.result, oracle[id]["result"].as_bool().unwrap());
        assert_eq!(native.context.private_state, 9);
        support::private_oracle(&native, &oracle[id]);
        for result in [&recorded, call.recorded()] {
            support::parity(result, &native);
        }
        for provider in [&a, &b, &w] {
            assert_eq!(*provider.events.borrow(), vec![(7, left), (8, right)]);
        }
        support::refuses_empty(call, key.clone());
    }
}
#[test]
fn comparison_provider_failure_preserves_first_and_second_callback_prefix() {
    let initial = c::initial_state(r::context::ConstructorContext::new(7_u64)).unwrap();
    let (state, _) = support::observation(initial.ledger_state.get_ref().clone(), &["equal_echo"]);
    for fail in [7, 8] {
        for lane in 0..3 {
            let w = Echo {
                fail: Some(fail),
                ..Default::default()
            };
            let contract = c::Contract::from(w.clone());
            let result = match lane {
                0 => c::equal_echo(state.circuit_context(7), &w, value(2), value(3)).map(|_| ()),
                1 => c::recorded::equal_echo(state.circuit_context(7), &w, value(2), value(3))
                    .map(|_| ()),
                _ => contract
                    .recording()
                    .equal_echo_call(&state, 7, value(2), value(3))
                    .map(|_| ()),
            };
            assert!(
                matches!(result,Err(r::CompactError::InvalidLedgerCell(ref reason)) if reason=="provider refused")
            );
            let expected = if fail == 7 {
                vec![(7, value(2))]
            } else {
                vec![(7, value(2)), (8, value(3))]
            };
            assert_eq!(*w.events.borrow(), expected);
        }
    }
}
