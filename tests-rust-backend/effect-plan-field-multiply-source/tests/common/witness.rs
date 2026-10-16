// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use compact_rust_effect_plan_field_multiply_source_fixture::{
    ledger_contract as c, ledger_slots as slots, runtime as r, types::Product,
};
use r::context::{CircuitContext, WitnessContext};
use r::recording::{RecordedCircuitResult, RecordingFrame};
use std::{cell::RefCell, rc::Rc};

pub type Event = (&'static str, u64, u128);
pub fn failure(name: &str) -> r::CompactError {
    r::CompactError::InvalidLedgerCell(format!("{name} multiplication witness refused"))
}
#[derive(Clone)]
pub struct Witness {
    pub left: r::Field,
    pub right: r::Field,
    pub fail: Option<&'static str>,
    pub events: Rc<RefCell<Vec<Event>>>,
}
impl Witness {
    pub fn new(left: r::Field, right: r::Field, fail: Option<&'static str>) -> Self {
        Self {
            left,
            right,
            fail,
            events: Rc::default(),
        }
    }
    fn take(&self, name: &'static str, private: u64, count: u128) -> Result<u64, r::CompactError> {
        self.events.borrow_mut().push((name, private, count));
        if self.fail == Some(name) {
            return Err(failure(name));
        }
        Ok(private + 1)
    }
}
impl c::TryWitnesses<u64> for Witness {
    fn left(
        &self,
        ctx: WitnessContext<'_, u64, c::LedgerView<'_>>,
    ) -> Result<(u64, r::Field), r::CompactError> {
        Ok((
            self.take("left", *ctx.private_state, ctx.ledger.calls()?.value())?,
            self.left,
        ))
    }
    fn right(
        &self,
        ctx: WitnessContext<'_, u64, c::LedgerView<'_>>,
    ) -> Result<(u64, r::Field), r::CompactError> {
        Ok((
            self.take("right", *ctx.private_state, ctx.ledger.calls()?.value())?,
            self.right,
        ))
    }
}
/// Reproduce the public query and witness schedule independently of emitted code.
/// The expected product is supplied independently; it is not recomputed here.
pub fn manual_witnessed(
    ctx: CircuitContext<u64>,
    w: &Witness,
    selected: bool,
    expected: r::Field,
) -> Result<RecordedCircuitResult<u64, Product>, r::CompactError> {
    let frame = slots::calls.record_increment(RecordingFrame::new(ctx), 1)?;
    if !selected {
        return Ok(frame.finish(Product {
            value: r::Field::from(0_u64),
        }));
    }
    let (frame, _left) = frame.try_witness_metered(|ctx, meter| {
        Ok((
            w.take(
                "left",
                ctx.private_state,
                slots::calls.witness_read(meter)? as u128,
            )?,
            w.left,
        ))
    })?;
    let (frame, _right) = frame.try_witness_metered(|ctx, meter| {
        Ok((
            w.take(
                "right",
                ctx.private_state,
                slots::calls.witness_read(meter)? as u128,
            )?,
            w.right,
        ))
    })?;
    Ok(frame.finish(Product { value: expected }))
}
