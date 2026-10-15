// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use super::values::field;
use compact_rust_effect_plan_unit_value_operations_source_fixture::{
    ledger_contract as c, ledger_slots as slots, runtime as r,
};
use r::context::{CircuitContext, WitnessContext};
use r::recording::{RecordedCircuitResult, RecordingFrame};
use std::{cell::RefCell, rc::Rc};
pub type Event = (&'static str, u64, u128);
pub fn provider_failure(name: &str) -> r::CompactError {
    r::CompactError::InvalidLedgerCell(format!("{name} provider refused"))
}
#[derive(Clone)]
pub struct Witness {
    pub x: r::Field,
    pub y: r::Field,
    pub permit: bool,
    pub fail: Option<&'static str>,
    pub events: Rc<RefCell<Vec<Event>>>,
}
impl Witness {
    pub fn new(row: &serde_json::Value) -> Self {
        Self {
            x: field(&row["x"]),
            y: field(&row["y"]),
            permit: row["permit"].as_bool().unwrap(),
            fail: match row["fail"].as_str() {
                None => None,
                Some("coordinate_x") => Some("coordinate_x"),
                Some("coordinate_y") => Some("coordinate_y"),
                Some("permit") => Some("permit"),
                Some(n) => panic!("unexpected provider {n}"),
            },
            events: Rc::default(),
        }
    }
    fn take(&self, name: &'static str, private: u64, calls: u128) -> Result<u64, r::CompactError> {
        self.events.borrow_mut().push((name, private, calls));
        if self.fail == Some(name) {
            Err(provider_failure(name))
        } else {
            Ok(private + 1)
        }
    }
}
impl c::TryWitnesses<u64> for Witness {
    fn coordinate_x(
        &self,
        ctx: WitnessContext<'_, u64, c::LedgerView<'_>>,
    ) -> Result<(u64, r::Field), r::CompactError> {
        Ok((
            self.take(
                "coordinate_x",
                *ctx.private_state,
                ctx.ledger.calls()?.value(),
            )?,
            self.x,
        ))
    }
    fn coordinate_y(
        &self,
        ctx: WitnessContext<'_, u64, c::LedgerView<'_>>,
    ) -> Result<(u64, r::Field), r::CompactError> {
        Ok((
            self.take(
                "coordinate_y",
                *ctx.private_state,
                ctx.ledger.calls()?.value(),
            )?,
            self.y,
        ))
    }
    fn permit(
        &self,
        ctx: WitnessContext<'_, u64, c::LedgerView<'_>>,
    ) -> Result<(u64, bool), r::CompactError> {
        Ok((
            self.take("permit", *ctx.private_state, ctx.ledger.calls()?.value())?,
            self.permit,
        ))
    }
}
/// Hand-recorded Counter/witness transcript. Constructor/guard outcomes below
/// are explicit frozen-case laws, not a second call to the implementation.
pub fn manual<T>(
    ctx: CircuitContext<u64>,
    w: &Witness,
    row: &serde_json::Value,
    output: T,
) -> Result<RecordedCircuitResult<u64, T>, r::CompactError> {
    let mut frame = slots::calls.record_increment(RecordingFrame::new(ctx), 1)?;
    if row["circuit"] == "close" || row["selected"] == true {
        for (name, value) in [("coordinate_x", w.x), ("coordinate_y", w.y)] {
            let (next, _) = frame.try_witness_metered(|ctx, meter| {
                Ok((
                    w.take(
                        name,
                        ctx.private_state,
                        slots::calls.witness_read(meter)? as u128,
                    )?,
                    value,
                ))
            })?;
            frame = next;
        }
    }
    match row["id"].as_str().unwrap() {
        "evaluate-invalid-coordinates" => return Err(r::CompactError::InvalidJubjubPoint),
        "close-wrong-x" => return Err(r::CompactError::AssertionFailed("wrong x".into())),
        "close-wrong-default" => {
            return Err(r::CompactError::AssertionFailed("wrong default".into()));
        }
        _ => {}
    }
    let (frame, permit) = frame.try_witness_metered(|ctx, meter| {
        Ok((
            w.take(
                "permit",
                ctx.private_state,
                slots::calls.witness_read(meter)? as u128,
            )?,
            w.permit,
        ))
    })?;
    if !permit {
        return Err(r::CompactError::AssertionFailed("denied".into()));
    }
    Ok(frame.finish(output))
}
