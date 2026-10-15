// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use super::answers;
use compact_rust_effect_plan_aligned_hashes_source_fixture::{
    ledger_contract as c, ledger_slots as slots, runtime as r, types::Hashes,
};
use r::context::{CircuitContext, WitnessContext};
use r::recording::{RecordedCircuitResult, RecordingFrame};
use std::{cell::RefCell, rc::Rc};
pub type Event = (&'static str, u64, u128);
pub fn provider_failure(name: &str) -> r::CompactError {
    r::CompactError::InvalidLedgerCell(format!("{name} provider refused"))
}
pub fn denied(message: &str) -> r::CompactError {
    r::CompactError::AssertionFailed(message.into())
}
#[derive(Clone)]
pub struct Witness {
    pub value: u64,
    pub other: u64,
    pub fail: Option<&'static str>,
    pub mismatch: Option<&'static str>,
    pub events: Rc<RefCell<Vec<Event>>>,
}
impl Witness {
    pub fn new(
        value: u64,
        other: u64,
        fail: Option<&'static str>,
        mismatch: Option<&'static str>,
    ) -> Self {
        Self {
            value,
            other,
            fail,
            mismatch,
            events: Rc::default(),
        }
    }
    fn take(&self, name: &'static str, private: u64, calls: u128) -> Result<u64, r::CompactError> {
        self.events.borrow_mut().push((name, private, calls));
        if self.fail == Some(name) {
            return Err(provider_failure(name));
        }
        Ok(private + 1)
    }
    fn expected_input(&self, name: &str) -> u64 {
        if self.mismatch == Some(name) {
            17
        } else {
            self.value
        }
    }
}
#[allow(non_snake_case)]
impl c::TryWitnesses<u64> for Witness {
    fn curveInput(
        &self,
        ctx: WitnessContext<'_, u64, c::LedgerView<'_>>,
    ) -> Result<(u64, r::Field), r::CompactError> {
        Ok((
            self.take(
                "curveInput",
                *ctx.private_state,
                ctx.ledger.calls()?.value(),
            )?,
            r::Field::from(self.value),
        ))
    }
    fn hashInput(
        &self,
        ctx: WitnessContext<'_, u64, c::LedgerView<'_>>,
    ) -> Result<(u64, r::Field), r::CompactError> {
        Ok((
            self.take("hashInput", *ctx.private_state, ctx.ledger.calls()?.value())?,
            r::Field::from(self.other),
        ))
    }
    fn expectedPoint(
        &self,
        ctx: WitnessContext<'_, u64, c::LedgerView<'_>>,
    ) -> Result<(u64, r::JubjubPoint), r::CompactError> {
        Ok((
            self.take(
                "expectedPoint",
                *ctx.private_state,
                ctx.ledger.calls()?.value(),
            )?,
            answers::point(self.expected_input("expectedPoint")),
        ))
    }
    fn expectedDigest(
        &self,
        ctx: WitnessContext<'_, u64, c::LedgerView<'_>>,
    ) -> Result<(u64, r::FixedBytes<32>), r::CompactError> {
        Ok((
            self.take(
                "expectedDigest",
                *ctx.private_state,
                ctx.ledger.calls()?.value(),
            )?,
            answers::digest(self.expected_input("expectedDigest")),
        ))
    }
}
pub fn manual_close(
    ctx: CircuitContext<u64>,
    w: &Witness,
    with_point: bool,
) -> Result<RecordedCircuitResult<u64, ()>, r::CompactError> {
    let mut frame = RecordingFrame::new(ctx);
    if with_point {
        let (next, point) = frame.try_witness_metered(|ctx, meter| {
            Ok((
                w.take(
                    "expectedPoint",
                    ctx.private_state,
                    slots::calls.witness_read(meter)? as u128,
                )?,
                answers::point(w.expected_input("expectedPoint")),
            ))
        })?;
        if point != answers::point(w.value) {
            return Err(denied("point mismatch"));
        }
        frame = next;
    }
    let (next, digest) = frame.try_witness_metered(|ctx, meter| {
        Ok((
            w.take(
                "expectedDigest",
                ctx.private_state,
                slots::calls.witness_read(meter)? as u128,
            )?,
            answers::digest(w.expected_input("expectedDigest")),
        ))
    })?;
    if digest != answers::digest(w.value) {
        return Err(denied("digest mismatch"));
    }
    frame = slots::calls.record_increment(next, 1)?;
    Ok(frame.finish(()))
}
fn manual_result(
    ctx: CircuitContext<u64>,
    w: &Witness,
    selected: bool,
    with_point: bool,
) -> Result<RecordedCircuitResult<u64, Hashes>, r::CompactError> {
    let mut frame = slots::calls.record_increment(RecordingFrame::new(ctx), 1)?;
    let mut x = r::Field::from(0_u64);
    let mut digest = r::FixedBytes([0; 32]);
    if selected {
        if with_point {
            let (next, value) = frame.try_witness_metered(|ctx, meter| {
                Ok((
                    w.take(
                        "curveInput",
                        ctx.private_state,
                        slots::calls.witness_read(meter)? as u128,
                    )?,
                    r::Field::from(w.value),
                ))
            })?;
            assert_eq!(value, r::Field::from(w.value));
            x = answers::point(w.value).x().unwrap();
            frame = next;
        }
        let (next, value) = frame.try_witness_metered(|ctx, meter| {
            Ok((
                w.take(
                    "hashInput",
                    ctx.private_state,
                    slots::calls.witness_read(meter)? as u128,
                )?,
                r::Field::from(w.other),
            ))
        })?;
        assert_eq!(value, r::Field::from(w.other));
        digest = answers::digest(w.other);
        frame = next;
    }
    Ok(frame.finish(Hashes { x, digest }))
}
pub fn manual_hashes(
    ctx: CircuitContext<u64>,
    w: &Witness,
    selected: bool,
) -> Result<RecordedCircuitResult<u64, Hashes>, r::CompactError> {
    manual_result(ctx, w, selected, true)
}
pub fn manual_hash_only(
    ctx: CircuitContext<u64>,
    w: &Witness,
    selected: bool,
) -> Result<RecordedCircuitResult<u64, Hashes>, r::CompactError> {
    manual_result(ctx, w, selected, false)
}
