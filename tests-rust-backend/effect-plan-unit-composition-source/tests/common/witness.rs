// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use compact_rust_effect_plan_unit_composition_source_fixture::{
    ledger_contract as c, ledger_slots as slots, runtime as r, types::Entry,
};
use r::context::{CircuitContext, WitnessContext};
use r::recording::{RecordedCircuitResult, RecordingFrame};
use std::{cell::RefCell, rc::Rc};
pub type Event = (&'static str, u64, u128);
pub fn provider_failure(name: &str) -> r::CompactError {
    r::CompactError::InvalidLedgerCell(format!("{name} provider refused"))
}
pub fn denied(reason: &str) -> r::CompactError {
    r::CompactError::AssertionFailed(reason.into())
}
#[derive(Clone)]
pub struct Witness {
    pub choices: [bool; 2],
    pub fail: Option<&'static str>,
    pub events: Rc<RefCell<Vec<Event>>>,
}
impl Witness {
    pub fn new(choices: [bool; 2], fail: Option<&'static str>) -> Self {
        Self {
            choices,
            fail,
            events: Rc::default(),
        }
    }
    fn take(
        &self,
        name: &'static str,
        private: u64,
        count: u128,
    ) -> Result<(u64, bool), r::CompactError> {
        self.events.borrow_mut().push((name, private, count));
        if self.fail == Some(name) {
            return Err(provider_failure(name));
        }
        Ok((private + 1, self.choices[usize::from(name == "second")]))
    }
}
impl c::TryWitnesses<u64> for Witness {
    fn first(
        &self,
        ctx: WitnessContext<'_, u64, c::LedgerView<'_>>,
    ) -> Result<(u64, bool), r::CompactError> {
        self.take("first", *ctx.private_state, ctx.ledger.calls()?.value())
    }
    fn second(
        &self,
        ctx: WitnessContext<'_, u64, c::LedgerView<'_>>,
    ) -> Result<(u64, bool), r::CompactError> {
        self.take("second", *ctx.private_state, ctx.ledger.calls()?.value())
    }
}
pub fn manual(
    context: CircuitContext<u64>,
    w: &Witness,
    key: r::OpaqueString,
    enabled: bool,
    selected: bool,
) -> Result<RecordedCircuitResult<u64, ()>, r::CompactError> {
    if !enabled {
        return Err(denied("disabled"));
    }
    let mut frame = RecordingFrame::new(context);
    if selected {
        let (next, first) = frame.try_witness_metered(|ctx, meter| {
            w.take(
                "first",
                ctx.private_state,
                slots::calls.witness_read(meter)? as u128,
            )
        })?;
        if !first {
            return Err(denied("first denied"));
        }
        frame = slots::calls.record_increment(next, 1)?;
        frame = slots::entries.record_insert(frame, key.clone(), Entry { label: key })?;
        let (next, second) = frame.try_witness_metered(|ctx, meter| {
            w.take(
                "second",
                ctx.private_state,
                slots::calls.witness_read(meter)? as u128,
            )
        })?;
        if !second {
            return Err(denied("second denied"));
        }
        frame = slots::calls.record_increment(next, 1)?;
        frame = slots::done.record_write(frame, true)?;
    }
    Ok(frame.finish(()))
}
