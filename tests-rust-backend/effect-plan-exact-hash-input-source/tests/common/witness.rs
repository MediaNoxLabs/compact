// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use compact_rust_effect_plan_exact_hash_input_source_fixture::{
    ledger_contract as c, ledger_slots as slots, runtime as r, types::Hashes,
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
    pub payload: Vec<u8>,
    pub permit: bool,
    pub fail: Option<&'static str>,
    pub events: Rc<RefCell<Vec<Event>>>,
}
impl Witness {
    pub fn new(payload: &[u8], permit: bool, fail: Option<&'static str>) -> Self {
        Self {
            payload: payload.to_vec(),
            permit,
            fail,
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
}
impl c::TryWitnesses<u64> for Witness {
    fn bytes(
        &self,
        ctx: WitnessContext<'_, u64, c::LedgerView<'_>>,
    ) -> Result<(u64, r::OpaqueBytes), r::CompactError> {
        Ok((
            self.take("bytes", *ctx.private_state, ctx.ledger.calls()?.value())?,
            r::OpaqueBytes(self.payload.clone()),
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
/// Hash result comes from the independent TS answer; this manual recorder owns
/// only state/witness semantics and never calls hashing under test.
pub fn manual(
    ctx: CircuitContext<u64>,
    w: &Witness,
    selected: bool,
    output: Hashes,
) -> Result<RecordedCircuitResult<u64, Hashes>, r::CompactError> {
    let mut frame = slots::calls.record_increment(RecordingFrame::new(ctx), 1)?;
    if selected {
        let (next, payload) = frame.try_witness_metered_exact(|ctx, meter| {
            Ok((
                w.take(
                    "bytes",
                    ctx.private_state,
                    slots::calls.witness_read(meter)? as u128,
                )?,
                r::OpaqueBytes(w.payload.clone()),
            ))
        })?;
        assert_eq!(payload.0, w.payload);
        frame = next;
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
