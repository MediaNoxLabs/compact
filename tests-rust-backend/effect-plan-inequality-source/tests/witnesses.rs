// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
#![cfg(feature = "ledger-transaction")]
mod common;
use common::*;
use compact_rust_effect_plan_inequality_source_fixture::{
    ledger_contract as c, ledger_slots as slots, runtime as r, types,
};
use r::context::{CircuitContext, WitnessContext};
use r::recording::{RecordedCircuitResult, RecordingFrame};
use std::{cell::RefCell, rc::Rc};
type Uint = r::BoundedUint<{ u64::MAX as u128 }>;
type Events = Rc<RefCell<Vec<(&'static str, u64, u128)>>>;
#[derive(Clone)]
struct Witness {
    left: Uint,
    right: Uint,
    fail: Option<&'static str>,
    events: Events,
}
fn failure(name: &str) -> r::CompactError {
    r::CompactError::InvalidLedgerCell(format!("{name} witness refused"))
}
impl Witness {
    fn new(left: u64, right: u64, fail: Option<&'static str>) -> Self {
        Self {
            left: Uint::new(left.into()).unwrap(),
            right: Uint::new(right.into()).unwrap(),
            fail,
            events: Rc::default(),
        }
    }
    fn observe(
        &self,
        name: &'static str,
        private: u64,
        counter: u128,
    ) -> Result<(), r::CompactError> {
        self.events.borrow_mut().push((name, private, counter));
        if self.fail == Some(name) {
            Err(failure(name))
        } else {
            Ok(())
        }
    }
}
impl c::TryWitnesses<u64> for Witness {
    fn left(
        &self,
        ctx: WitnessContext<'_, u64, c::LedgerView<'_>>,
    ) -> Result<(u64, Uint), r::CompactError> {
        self.observe("left", *ctx.private_state, ctx.ledger.calls()?.value())?;
        Ok((*ctx.private_state + 1, self.left))
    }
    fn right(
        &self,
        ctx: WitnessContext<'_, u64, c::LedgerView<'_>>,
    ) -> Result<(u64, Uint), r::CompactError> {
        self.observe("right", *ctx.private_state, ctx.ledger.calls()?.value())?;
        Ok((*ctx.private_state + 1, self.right))
    }
}
fn manual(
    ctx: CircuitContext<u64>,
    w: &Witness,
    selected: bool,
) -> Result<RecordedCircuitResult<u64, types::Outcome>, r::CompactError> {
    let frame = slots::calls.record_increment(RecordingFrame::new(ctx), 1)?;
    if !selected {
        return Ok(frame.finish(types::Outcome { different: false }));
    }
    let (frame, left) = frame.try_witness_metered(|ctx, meter| {
        w.observe(
            "left",
            ctx.private_state,
            slots::calls.witness_read(meter)? as u128,
        )?;
        Ok((ctx.private_state + 1, w.left))
    })?;
    let (frame, right) = frame.try_witness_metered(|ctx, meter| {
        w.observe(
            "right",
            ctx.private_state,
            slots::calls.witness_read(meter)? as u128,
        )?;
        Ok((ctx.private_state + 1, w.right))
    })?;
    Ok(frame.finish(types::Outcome {
        different: left.value() != right.value(),
    }))
}
#[test]
fn operands_are_eager_left_to_right_once_only_inside_selected_branch() {
    for (left, right) in [
        (0, 0),
        (0, 1),
        (u64::MAX, u64::MAX),
        (u64::MAX, 0),
        (0, u64::MAX),
    ] {
        for selected in [false, true] {
            let (a, b, m, w) = (
                Witness::new(left, right, None),
                Witness::new(left, right, None),
                Witness::new(left, right, None),
                Witness::new(left, right, None),
            );
            let (state, key) = setup();
            let native = c::witnessed(state.circuit_context(5), &a, selected).unwrap();
            let recorded = c::recorded::witnessed(state.circuit_context(5), &b, selected).unwrap();
            let manual = manual(state.circuit_context(5), &m, selected).unwrap();
            let call = c::Contract::from(w.clone())
                .recording()
                .witnessed_call(&state, 5, selected)
                .unwrap();
            compare(&native, &recorded, &manual);
            compare(&native, call.recorded(), &manual);
            assert_eq!(native.result.different, selected && left != right);
            assert_eq!(native.context.private_state, if selected { 7 } else { 5 });
            for provider in [&a, &b, &m, &w] {
                assert_eq!(
                    *provider.events.borrow(),
                    if selected {
                        vec![("left", 5, 1), ("right", 6, 1)]
                    } else {
                        vec![]
                    }
                );
            }
            let prepared = call.prepare(key, r::Field::from(0_u64)).unwrap();
            assert_eq!(
                atoms(&prepared.input),
                vec![if selected { vec![1] } else { vec![] }]
            );
            assert_eq!(
                atoms(&prepared.output),
                vec![if selected && left != right {
                    vec![1]
                } else {
                    vec![]
                }]
            );
        }
    }
}
#[test]
fn failure_prefix_is_eager_and_unselected_failures_are_suppressed() {
    for name in ["left", "right"] {
        for selected in [false, true] {
            let (a, b, m, w) = (
                Witness::new(1, 2, Some(name)),
                Witness::new(1, 2, Some(name)),
                Witness::new(1, 2, Some(name)),
                Witness::new(1, 2, Some(name)),
            );
            let (state, _) = setup();
            let native = c::witnessed(state.circuit_context(5), &a, selected);
            let recorded = c::recorded::witnessed(state.circuit_context(5), &b, selected);
            let manual = manual(state.circuit_context(5), &m, selected);
            let call = c::Contract::from(w.clone())
                .recording()
                .witnessed_call(&state, 5, selected);
            if selected {
                assert_eq!(native.err().unwrap(), failure(name));
                assert_eq!(recorded.err().unwrap(), failure(name));
                assert_eq!(manual.err().unwrap(), failure(name));
                assert_eq!(call.err().unwrap(), failure(name));
            } else {
                let (native, recorded, manual, call) = (
                    native.unwrap(),
                    recorded.unwrap(),
                    manual.unwrap(),
                    call.unwrap(),
                );
                compare(&native, &recorded, &manual);
                compare(&native, call.recorded(), &manual);
                assert!(!native.result.different);
            }
            let expected = if !selected {
                vec![]
            } else if name == "left" {
                vec![("left", 5, 1)]
            } else {
                vec![("left", 5, 1), ("right", 6, 1)]
            };
            for provider in [&a, &b, &m, &w] {
                assert_eq!(*provider.events.borrow(), expected);
            }
        }
    }
}
