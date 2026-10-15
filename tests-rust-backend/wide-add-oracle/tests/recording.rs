// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
#![cfg(feature = "ledger-transaction")]
use compact_rust_wide_add_oracle_fixture::ledger_contract as c;
use midnight_compact_runtime as r;
use std::{cell::RefCell, rc::Rc};
#[path = "../../support/witness_only_recording.rs"]
mod support;
type Events = Rc<RefCell<Vec<(Vec<u8>, u8)>>>;
#[derive(Clone)]
struct Values {
    values: [u128; 2],
    fail: Option<u8>,
    events: Events,
}
impl Values {
    fn new(values: [u128; 2], fail: Option<u8>) -> Self {
        Self {
            values,
            fail,
            events: Default::default(),
        }
    }
}
impl c::TryWitnesses<Vec<u8>> for Values {
    fn next_value(
        &self,
        ctx: r::context::WitnessContext<'_, Vec<u8>, c::LedgerView<'_>>,
        tag: r::BoundedUint<255>,
    ) -> Result<(Vec<u8>, r::BoundedUint<{ u128::MAX }>), r::CompactError> {
        let tag = tag.value() as u8;
        self.events
            .borrow_mut()
            .push((ctx.private_state.clone(), tag));
        if self.fail == Some(tag) {
            return Err(r::CompactError::InvalidLedgerCell(
                "provider refused".into(),
            ));
        }
        let mut private = ctx.private_state.clone();
        private.push(tag);
        Ok((
            private,
            r::BoundedUint::new(self.values[usize::from(tag - 1)]).unwrap(),
        ))
    }
}
#[test]
fn wide_recording_matches_ts_full_width_and_lazy_outputs() {
    let rows: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/wide-add-oracle.json"
    ))
    .unwrap();
    let initial = c::initial_state(r::context::ConstructorContext::new(Vec::<u8>::new())).unwrap();
    let (state, key) = support::observation(initial.ledger_state.get_ref().clone(), &["witnessed"]);
    let rows: Vec<_> = rows
        .as_array()
        .unwrap()
        .iter()
        .filter(|row| row["name"] == "witnessed")
        .collect();
    assert_eq!(rows.len(), 3);
    for row in rows {
        let values = [
            row["a"].as_str().unwrap().parse().unwrap(),
            row["b"].as_str().unwrap().parse().unwrap(),
        ];
        let selected = row["selected"].as_bool().unwrap();
        let a = Values::new(values, None);
        let b = Values::new(values, None);
        let w = Values::new(values, None);
        let native = c::witnessed(state.circuit_context(vec![]), &a, selected).unwrap();
        let recorded = c::recorded::witnessed(state.circuit_context(vec![]), &b, selected).unwrap();
        let contract = c::Contract::from(w.clone());
        let call = contract
            .recording()
            .witnessed_call(&state, vec![], selected)
            .unwrap();
        assert_eq!(
            serde_json::json!(r::fab::AlignedValue::from(native.result)),
            row["output"]
        );
        assert_eq!(
            serde_json::json!(native.private_transcript_outputs),
            row["privateOutputs"]
        );
        assert_eq!(
            serde_json::json!(native.context.private_state),
            row["privateState"]
        );
        let private: Vec<_> = row["privateOutputs"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| serde_json::json!({"valueAtoms":v["value"],"alignment":v["alignment"]}))
            .collect();
        support::private_oracle(
            &native,
            &serde_json::json!({"privateTranscriptOutputs":private}),
        );
        for actual in [&recorded, call.recorded()] {
            support::parity(actual, &native);
        }
        let expected = if selected {
            vec![(vec![], 1), (vec![1], 2)]
        } else {
            vec![]
        };
        for provider in [&a, &b, &w] {
            assert_eq!(*provider.events.borrow(), expected);
        }
        support::refuses_empty(call, key.clone());
    }
}
#[test]
fn wide_callbacks_fail_in_order_and_unselected_branch_never_calls_provider() {
    let initial = c::initial_state(r::context::ConstructorContext::new(Vec::<u8>::new())).unwrap();
    let (state, _) = support::observation(initial.ledger_state.get_ref().clone(), &["witnessed"]);
    for fail in [1, 2] {
        for selected in [true, false] {
            for lane in 0..3 {
                let w = Values::new([u128::MAX, 1], Some(fail));
                let contract = c::Contract::from(w.clone());
                let result = match lane {
                    0 => c::witnessed(state.circuit_context(vec![]), &w, selected).map(|_| ()),
                    1 => c::recorded::witnessed(state.circuit_context(vec![]), &w, selected)
                        .map(|_| ()),
                    _ => contract
                        .recording()
                        .witnessed_call(&state, vec![], selected)
                        .map(|_| ()),
                };
                if selected {
                    assert!(
                        matches!(result,Err(r::CompactError::InvalidLedgerCell(ref reason)) if reason=="provider refused")
                    );
                } else {
                    assert!(result.is_ok());
                }
                let expected = if !selected {
                    vec![]
                } else if fail == 1 {
                    vec![(vec![], 1)]
                } else {
                    vec![(vec![], 1), (vec![1], 2)]
                };
                assert_eq!(*w.events.borrow(), expected);
            }
        }
    }
}
