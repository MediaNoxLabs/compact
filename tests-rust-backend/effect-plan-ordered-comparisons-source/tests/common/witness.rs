// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use compact_rust_effect_plan_ordered_comparisons_source_fixture::{
    ledger_contract as c, ledger_slots as slots, runtime as r, types::Outcome,
};
use r::context::{CircuitContext, WitnessContext};
use r::recording::{RecordedCircuitResult, RecordingFrame};
use std::{cell::RefCell, rc::Rc};

pub type Small = r::BoundedUint<255>;
pub type Large = r::BoundedUint<{ u64::MAX as u128 }>;
pub type Event = (&'static str, u64, u128);
pub fn failure(name: &str) -> r::CompactError {
    r::CompactError::InvalidLedgerCell(format!("{name} ordering witness refused"))
}
#[derive(Clone)]
pub struct Witness {
    pub left: Small,
    pub right: Large,
    pub fail: Option<&'static str>,
    pub events: Rc<RefCell<Vec<Event>>>,
}
impl Witness {
    pub fn new(left: u128, right: u128, fail: Option<&'static str>) -> Self {
        Self {
            left: Small::new(left).unwrap(),
            right: Large::new(right).unwrap(),
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
    ) -> Result<(u64, Small), r::CompactError> {
        Ok((
            self.take("left", *ctx.private_state, ctx.ledger.calls()?.value())?,
            self.left,
        ))
    }
    fn right(
        &self,
        ctx: WitnessContext<'_, u64, c::LedgerView<'_>>,
    ) -> Result<(u64, Large), r::CompactError> {
        Ok((
            self.take("right", *ctx.private_state, ctx.ledger.calls()?.value())?,
            self.right,
        ))
    }
}
/// Reproduce the public query and witness schedule independently of emitted code.
/// The expected comparison is supplied from a literal truth table, not recomputed.
pub fn manual_witnessed(
    ctx: CircuitContext<u64>,
    w: &Witness,
    selected: bool,
    expected: bool,
) -> Result<RecordedCircuitResult<u64, Outcome>, r::CompactError> {
    let frame = slots::calls.record_increment(RecordingFrame::new(ctx), 1)?;
    if !selected {
        return Ok(frame.finish(Outcome { less: false }));
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
    Ok(frame.finish(Outcome { less: expected }))
}
