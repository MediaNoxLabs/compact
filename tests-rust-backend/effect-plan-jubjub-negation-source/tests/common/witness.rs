// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use super::values::point;
use compact_rust_effect_plan_jubjub_negation_source_fixture::{
    ledger_contract as c, ledger_slots as slots, runtime as r, types,
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
    pub payload: r::JubjubPoint,
    pub permit: bool,
    pub fail: Option<&'static str>,
    pub events: Rc<RefCell<Vec<Event>>>,
}
impl Witness {
    pub fn new(row: &serde_json::Value) -> Self {
        Self {
            payload: point(&row["payload"]),
            permit: row["permit"].as_bool().unwrap(),
            fail: match row["fail"].as_str() {
                None => None,
                Some("point") => Some("point"),
                Some("permit") => Some("permit"),
                Some(n) => panic!("unknown failure {n}"),
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
    fn point(
        &self,
        ctx: WitnessContext<'_, u64, c::LedgerView<'_>>,
    ) -> Result<(u64, r::JubjubPoint), r::CompactError> {
        Ok((
            self.take("point", *ctx.private_state, ctx.ledger.calls()?.value())?,
            self.payload,
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
/// Independent state/witness recorder. Negated coordinates are frozen TS answers;
/// this reference never calls the Rust ec_neg implementation under test.
pub fn manual<T>(
    ctx: CircuitContext<u64>,
    w: &Witness,
    selected: bool,
    guard: Option<(types::Coordinates, r::JubjubPoint)>,
    output: T,
) -> Result<RecordedCircuitResult<u64, T>, r::CompactError> {
    let mut frame = slots::calls.record_increment(RecordingFrame::new(ctx), 1)?;
    if selected {
        let (next, _) = frame.try_witness_metered(|ctx, meter| {
            Ok((
                w.take(
                    "point",
                    ctx.private_state,
                    slots::calls.witness_read(meter)? as u128,
                )?,
                w.payload,
            ))
        })?;
        frame = next;
    }
    if let Some((negated, expected)) = guard {
        if negated.x != expected.x().unwrap_or(r::Field::from(0_u64)) {
            return Err(r::CompactError::AssertionFailed("wrong x".into()));
        }
        if negated.y != expected.y().unwrap_or(r::Field::from(1_u64)) {
            return Err(r::CompactError::AssertionFailed("wrong y".into()));
        }
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
