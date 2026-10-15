// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use compact_rust_effect_plan_pure_unit_statements_source_fixture::{
    ledger_contract as c, ledger_slots as slots, runtime as r,
};
use r::context::{CircuitContext, WitnessContext};
use r::recording::{RecordedCircuitResult, RecordingFrame};
use std::{cell::RefCell, rc::Rc};

pub type Event = (&'static str, u64, u128, r::Field);
#[derive(Clone)]
pub struct Witness {
    pub left: bool,
    pub right: bool,
    pub fail: Option<&'static str>,
    pub events: Rc<RefCell<Vec<Event>>>,
}
pub fn assertion(message: &str) -> r::CompactError {
    r::CompactError::AssertionFailed(message.into())
}
pub fn failure(name: &str) -> r::CompactError {
    r::CompactError::InvalidLedgerCell(format!("{name} witness refused"))
}
impl Witness {
    pub fn new(left: bool, right: bool, fail: Option<&'static str>) -> Self {
        Self {
            left,
            right,
            fail,
            events: Rc::default(),
        }
    }
    fn observe(
        &self,
        name: &'static str,
        private: u64,
        counter: u128,
        last: r::Field,
    ) -> Result<(), r::CompactError> {
        self.events
            .borrow_mut()
            .push((name, private, counter, last));
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
    ) -> Result<(u64, bool), r::CompactError> {
        self.observe(
            "left",
            *ctx.private_state,
            ctx.ledger.calls()?.value(),
            ctx.ledger.last()?,
        )?;
        Ok((*ctx.private_state + 1, self.left))
    }
    fn right(
        &self,
        ctx: WitnessContext<'_, u64, c::LedgerView<'_>>,
    ) -> Result<(u64, bool), r::CompactError> {
        self.observe(
            "right",
            *ctx.private_state,
            ctx.ledger.calls()?.value(),
            ctx.ledger.last()?,
        )?;
        Ok((*ctx.private_state + 1, self.right))
    }
    fn marker(
        &self,
        ctx: WitnessContext<'_, u64, c::LedgerView<'_>>,
    ) -> Result<(u64, r::Field), r::CompactError> {
        self.observe(
            "marker",
            *ctx.private_state,
            ctx.ledger.calls()?.value(),
            ctx.ledger.last()?,
        )?;
        Ok((*ctx.private_state + 1, r::Field::from(42_u64)))
    }
}

/// Independent statement semantics: no call to generated pure helpers.
pub fn manual_run(
    ctx: CircuitContext<u64>,
    w: &Witness,
    selected: bool,
) -> Result<RecordedCircuitResult<u64, ()>, r::CompactError> {
    let mut frame = slots::calls.record_increment(RecordingFrame::new(ctx), 1)?;
    if selected {
        let (next, left) = frame.try_witness_metered(|ctx, meter| {
            w.observe(
                "left",
                ctx.private_state,
                slots::calls.witness_read(meter)? as u128,
                slots::last.witness_read(meter)?,
            )?;
            Ok((ctx.private_state + 1, w.left))
        })?;
        let (next, right) = next.try_witness_metered(|ctx, meter| {
            w.observe(
                "right",
                ctx.private_state,
                slots::calls.witness_read(meter)? as u128,
                slots::last.witness_read(meter)?,
            )?;
            Ok((ctx.private_state + 1, w.right))
        })?;
        // Source arguments are evaluated before either pure assertion.
        if !left {
            return Err(assertion("left guard"));
        }
        if !right {
            return Err(assertion("right guard"));
        }
        frame = next;
    }
    let (frame, marker) = frame.try_witness_metered(|ctx, meter| {
        w.observe(
            "marker",
            ctx.private_state,
            slots::calls.witness_read(meter)? as u128,
            slots::last.witness_read(meter)?,
        )?;
        Ok((ctx.private_state + 1, r::Field::from(42_u64)))
    })?;
    let frame = slots::last.record_write(frame, marker)?;
    Ok(slots::calls.record_increment(frame, 1)?.finish(()))
}
pub fn manual_arguments(
    ctx: CircuitContext<u64>,
    a: bool,
    b: bool,
) -> Result<RecordedCircuitResult<u64, ()>, r::CompactError> {
    let frame = slots::calls.record_increment(RecordingFrame::new(ctx), 1)?;
    // These failures occur in the two argument expressions, before the callee.
    if !a {
        return Err(assertion("first pure argument"));
    }
    if !b {
        return Err(assertion("second pure argument"));
    }
    Ok(slots::calls.record_increment(frame, 1)?.finish(()))
}
