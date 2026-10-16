// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use compact_rust_effect_plan_maps_source_fixture::{
    ledger_contract as c, ledger_slots as slots, runtime as r, types::Present,
};
use r::context::{CircuitContext, WitnessContext};
use r::recording::{RecordedCircuitResult, RecordingFrame};
use std::{cell::RefCell, rc::Rc};
pub type Event = (&'static str, u64, u128);
pub fn failure(name: &str) -> r::CompactError {
    r::CompactError::InvalidLedgerCell(format!("{name} witness refused"))
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
            return Err(failure(name));
        }
        Ok((private + 1, self.choices[usize::from(name == "pick_value")]))
    }
}
impl c::TryWitnesses<u64> for Witness {
    fn pick_key(
        &self,
        ctx: WitnessContext<'_, u64, c::LedgerView<'_>>,
    ) -> Result<(u64, bool), r::CompactError> {
        self.take("pick_key", *ctx.private_state, ctx.ledger.calls()?.value())
    }
    fn pick_value(
        &self,
        ctx: WitnessContext<'_, u64, c::LedgerView<'_>>,
    ) -> Result<(u64, bool), r::CompactError> {
        self.take(
            "pick_value",
            *ctx.private_state,
            ctx.ledger.calls()?.value(),
        )
    }
}
#[allow(dead_code)] // Shared witness support; scalar test executable does not use lazy recording.
pub fn manual(
    ctx: CircuitContext<u64>,
    w: &Witness,
    selected: bool,
) -> Result<RecordedCircuitResult<u64, Present>, r::CompactError> {
    let mut frame = RecordingFrame::new(ctx);
    if selected {
        let (next, key) = frame.try_witness_metered(|ctx, meter| {
            w.take(
                "pick_key",
                ctx.private_state,
                slots::calls.witness_read(meter)? as u128,
            )
        })?;
        let (next, value) = next.try_witness_metered(|ctx, meter| {
            w.take(
                "pick_value",
                ctx.private_state,
                slots::calls.witness_read(meter)? as u128,
            )
        })?;
        frame = slots::table.record_insert(
            next,
            if key { "λ" } else { "other" }.into(),
            r::Field::from(if value { 20_u64 } else { 30_u64 }),
        )?;
        frame = slots::calls.record_increment(frame, 1)?;
    }
    let (frame, value) = slots::table.record_member(frame, "λ".into())?;
    Ok(frame.finish(Present { value }))
}
