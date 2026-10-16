// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use compact_rust_effect_plan_jubjub_projections_source_fixture::{
    ledger_contract as c, ledger_slots as slots, runtime as r, types::Coordinates,
};
use midnight_transient_crypto::curve::{EmbeddedFr, EmbeddedGroupAffine};
use r::context::{CircuitContext, WitnessContext};
use r::recording::{RecordedCircuitResult, RecordingFrame};
use std::{cell::RefCell, rc::Rc};

pub type Event = (&'static str, u64, u128);
/// Independent official curve carrier, not the runtime projection helpers under test.
pub fn point(scalar: u64) -> EmbeddedGroupAffine {
    EmbeddedGroupAffine::generator() * EmbeddedFr::from(scalar)
}
pub fn coordinates(first: EmbeddedGroupAffine, second: EmbeddedGroupAffine) -> Coordinates {
    let x = first.x().unwrap_or_else(|| r::Field::from(0_u64));
    Coordinates {
        x,
        y: second.y().unwrap_or_else(|| r::Field::from(1_u64)),
        repeatedX: x,
    }
}
pub fn failure(name: &str) -> r::CompactError {
    r::CompactError::InvalidLedgerCell(format!("{name} point witness refused"))
}
#[derive(Clone)]
pub struct Witness {
    pub first: EmbeddedGroupAffine,
    pub second: EmbeddedGroupAffine,
    pub fail: Option<&'static str>,
    pub events: Rc<RefCell<Vec<Event>>>,
}
impl Witness {
    pub fn new(first: u64, second: u64, fail: Option<&'static str>) -> Self {
        Self {
            first: point(first),
            second: point(second),
            fail,
            events: Rc::default(),
        }
    }
    fn take(
        &self,
        name: &'static str,
        private: u64,
        count: u128,
    ) -> Result<(u64, r::JubjubPoint), r::CompactError> {
        self.events.borrow_mut().push((name, private, count));
        if self.fail == Some(name) {
            return Err(failure(name));
        }
        Ok((
            private + 1,
            if name == "first" {
                self.first
            } else {
                self.second
            }
            .into(),
        ))
    }
}
impl c::TryWitnesses<u64> for Witness {
    fn first(
        &self,
        ctx: WitnessContext<'_, u64, c::LedgerView<'_>>,
    ) -> Result<(u64, r::JubjubPoint), r::CompactError> {
        self.take("first", *ctx.private_state, ctx.ledger.calls()?.value())
    }
    fn second(
        &self,
        ctx: WitnessContext<'_, u64, c::LedgerView<'_>>,
    ) -> Result<(u64, r::JubjubPoint), r::CompactError> {
        self.take("second", *ctx.private_state, ctx.ledger.calls()?.value())
    }
}
pub fn manual_project(
    ctx: CircuitContext<u64>,
    value: EmbeddedGroupAffine,
) -> Result<RecordedCircuitResult<u64, Coordinates>, r::CompactError> {
    Ok(slots::calls
        .record_increment(RecordingFrame::new(ctx), 1)?
        .finish(coordinates(value, value)))
}
pub fn manual_observe(
    ctx: CircuitContext<u64>,
    w: &Witness,
    selected: bool,
) -> Result<RecordedCircuitResult<u64, Coordinates>, r::CompactError> {
    let frame = slots::calls.record_increment(RecordingFrame::new(ctx), 1)?;
    if selected {
        let (frame, _first) = frame.try_witness_metered(|ctx, meter| {
            w.take(
                "first",
                ctx.private_state,
                slots::calls.witness_read(meter)? as u128,
            )
        })?;
        let (frame, _second) = frame.try_witness_metered(|ctx, meter| {
            w.take(
                "second",
                ctx.private_state,
                slots::calls.witness_read(meter)? as u128,
            )
        })?;
        Ok(frame.finish(coordinates(w.first, w.second)))
    } else {
        Ok(frame.finish(coordinates(
            EmbeddedGroupAffine::identity(),
            EmbeddedGroupAffine::identity(),
        )))
    }
}
