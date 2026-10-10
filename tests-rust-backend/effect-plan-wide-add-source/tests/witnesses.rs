// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
#![cfg(feature = "ledger-transaction")]
mod common;
use common::*;
use compact_rust_effect_plan_wide_add_source_fixture::{
    ledger_contract as c, ledger_slots as slots, runtime as r, types,
};
use r::context::{CircuitContext, WitnessContext};
use r::recording::{RecordedCircuitResult, RecordingFrame};
use std::{cell::RefCell, rc::Rc};
type Wide = r::WideUint<1, { u128::MAX }>;
type Events = Rc<RefCell<Vec<(&'static str, u64, u128)>>>;
#[derive(Clone)]
struct Witness {
    left: Wide,
    right: r::BoundedUint<255>,
    fail: Option<&'static str>,
    events: Events,
}
fn failure(name: &str) -> r::CompactError {
    r::CompactError::InvalidLedgerCell(format!("{name} witness refused"))
}
impl Witness {
    fn new(high: u128, low: u128, right: u128, fail: Option<&'static str>) -> Self {
        Self {
            left: Wide::from_le_bytes(&bytes(high, low)).unwrap(),
            right: r::BoundedUint::new(right).unwrap(),
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
    ) -> Result<(u64, Wide), r::CompactError> {
        self.observe("left", *ctx.private_state, ctx.ledger.calls()?.value())?;
        Ok((*ctx.private_state + 1, self.left))
    }
    fn right(
        &self,
        ctx: WitnessContext<'_, u64, c::LedgerView<'_>>,
    ) -> Result<(u64, r::BoundedUint<255>), r::CompactError> {
        self.observe("right", *ctx.private_state, ctx.ledger.calls()?.value())?;
        Ok((*ctx.private_state + 1, self.right))
    }
}
fn manual(
    ctx: CircuitContext<u64>,
    w: &Witness,
    selected: bool,
) -> Result<RecordedCircuitResult<u64, types::Wide>, r::CompactError> {
    let frame = slots::calls.record_increment(RecordingFrame::new(ctx), 1)?;
    if !selected {
        return Ok(frame.finish(types::Wide {
            value: Wide::from_le_bytes(&[]).unwrap(),
        }));
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
    // Independent two-limb reference, rather than reusing the emitter's addition helper.
    let raw = left.as_le_bytes();
    let low = u128::from_le_bytes(raw[..16].try_into().unwrap());
    let high = u128::from(raw[16]);
    let (sum, carry) = low.overflowing_add(right.value());
    let high = high + u128::from(carry);
    if high > 1 {
        return Err(r::CompactError::InvalidUnsignedValue);
    }
    Ok(frame.finish(types::Wide {
        value: Wide::from_le_bytes(&bytes(high, sum)).unwrap(),
    }))
}
#[test]
fn lazy_add_operands_evaluate_left_then_right_once_and_preserve_bound_failures() {
    for (high, low, right) in [
        (0, u128::MAX, 1),
        (1, u128::MAX - 1, 1),
        (1, u128::MAX, 1),
        (1, u128::MAX, 255),
    ] {
        for selected in [false, true] {
            let (a, b, m, w) = (
                Witness::new(high, low, right, None),
                Witness::new(high, low, right, None),
                Witness::new(high, low, right, None),
                Witness::new(high, low, right, None),
            );
            let (state, key) = setup();
            let native = c::witnessed(state.circuit_context(5), &a, selected);
            let recorded = c::recorded::witnessed(state.circuit_context(5), &b, selected);
            let manual = manual(state.circuit_context(5), &m, selected);
            let contract = c::Contract::from(w.clone());
            let call = contract.recording().witnessed_call(&state, 5, selected);
            for w in [&a, &b, &m, &w] {
                assert_eq!(
                    *w.events.borrow(),
                    if selected {
                        vec![("left", 5, 1), ("right", 6, 1)]
                    } else {
                        vec![]
                    }
                );
            }
            if selected && high == 1 && low == u128::MAX {
                assert_eq!(native.err().unwrap(), r::CompactError::InvalidUnsignedValue);
                assert_eq!(
                    recorded.err().unwrap(),
                    r::CompactError::InvalidUnsignedValue
                );
                assert_eq!(manual.err().unwrap(), r::CompactError::InvalidUnsignedValue);
                assert_eq!(call.err().unwrap(), r::CompactError::InvalidUnsignedValue);
                continue;
            }
            let (native, recorded, manual, call) = (
                native.unwrap(),
                recorded.unwrap(),
                manual.unwrap(),
                call.unwrap(),
            );
            compare(&native, &recorded, &manual);
            compare(&native, call.recorded(), &manual);
            let expected = if !selected {
                vec![]
            } else if high == 0 {
                bytes(1, 0)
            } else {
                bytes(1, u128::MAX)
            };
            let prepared = call.prepare(key, r::Field::from(0_u64)).unwrap();
            assert_eq!(
                atoms(&prepared.input),
                vec![if selected { vec![1] } else { vec![] }]
            );
            assert_eq!(atoms(&prepared.output), vec![expected]);
            assert_ts("witnessed", &prepared.input, &prepared.output, &native);
        }
    }
}
#[test]
fn a_failed_operand_does_not_evaluate_the_following_witness() {
    for name in ["left", "right"] {
        let (a, b, m, w) = (
            Witness::new(0, 1, 1, Some(name)),
            Witness::new(0, 1, 1, Some(name)),
            Witness::new(0, 1, 1, Some(name)),
            Witness::new(0, 1, 1, Some(name)),
        );
        let (state, _) = setup();
        let contract = c::Contract::from(w.clone());
        assert_eq!(
            c::witnessed(state.circuit_context(5), &a, true)
                .err()
                .unwrap(),
            failure(name)
        );
        assert_eq!(
            c::recorded::witnessed(state.circuit_context(5), &b, true)
                .err()
                .unwrap(),
            failure(name)
        );
        assert_eq!(
            manual(state.circuit_context(5), &m, true).err().unwrap(),
            failure(name)
        );
        assert_eq!(
            contract
                .recording()
                .witnessed_call(&state, 5, true)
                .err()
                .unwrap(),
            failure(name)
        );
        for w in [&a, &b, &m, &w] {
            assert_eq!(
                *w.events.borrow(),
                if name == "left" {
                    vec![("left", 5, 1)]
                } else {
                    vec![("left", 5, 1), ("right", 6, 1)]
                }
            );
        }
    }
}
