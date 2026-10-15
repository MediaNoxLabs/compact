// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use compact_rust_effect_plan_unit_keccak_source_fixture::{
    ledger_contract as c, ledger_slots as slots, runtime as r, types::Digest,
};
use r::context::{CircuitContext, WitnessContext};
use r::recording::{RecordedCircuitResult, RecordingFrame};
use std::{cell::RefCell, rc::Rc};
#[derive(Clone, Copy)]
pub enum Vector {
    Zero,
    Ff,
    Ramp,
}
impl Vector {
    pub const ALL: [Self; 3] = [Self::Zero, Self::Ff, Self::Ramp];
    pub fn name(self) -> &'static str {
        match self {
            Self::Zero => "zero",
            Self::Ff => "ff",
            Self::Ramp => "ramp",
        }
    }
    pub fn payload(self) -> r::FixedBytes<32> {
        r::FixedBytes(match self {
            Self::Zero => [0; 32],
            Self::Ff => [255; 32],
            Self::Ramp => std::array::from_fn(|i| i as u8),
        })
    }
    /// Frozen independent official TS answers over canonical FAB bytes. Zero32
    /// normalizes to an empty atom, so its digest is Keccak(empty), not Keccak(32 zeros).
    pub fn digest(self) -> r::FixedBytes<32> {
        let hex = match self {
            Self::Zero => "c5d2460186f7233c927e7db2dcc703c0e500b653ca82273b7bfad8045d85a470",
            Self::Ff => "a9c584056064687e149968cbab758a3376d22aedc6a55823d1b3ecbee81b8fb9",
            Self::Ramp => "8ae1aa597fa146ebd3aa2ceddf360668dea5e526567e92b0321816a4e895bd2d",
        };
        r::FixedBytes(std::array::from_fn(|i| {
            u8::from_str_radix(&hex[2 * i..2 * i + 2], 16).unwrap()
        }))
    }
}
pub type Event = (&'static str, u64, bool);
pub fn provider_failure(name: &str) -> r::CompactError {
    r::CompactError::InvalidLedgerCell(format!("{name} provider refused"))
}
pub fn denied(message: &str) -> r::CompactError {
    r::CompactError::AssertionFailed(message.into())
}
#[derive(Clone)]
pub struct Witness {
    pub vector: Vector,
    pub mismatch: bool,
    pub authorized: bool,
    pub fail: Option<&'static str>,
    pub events: Rc<RefCell<Vec<Event>>>,
}
impl Witness {
    pub fn new(
        vector: Vector,
        mismatch: bool,
        authorized: bool,
        fail: Option<&'static str>,
    ) -> Self {
        Self {
            vector,
            mismatch,
            authorized,
            fail,
            events: Rc::default(),
        }
    }
    fn take(&self, name: &'static str, private: u64, active: bool) -> Result<u64, r::CompactError> {
        self.events.borrow_mut().push((name, private, active));
        if self.fail == Some(name) {
            return Err(provider_failure(name));
        }
        Ok(private + 1)
    }
    fn expected(&self) -> r::FixedBytes<32> {
        if self.mismatch {
            r::FixedBytes([0; 32])
        } else {
            self.vector.digest()
        }
    }
}
#[allow(non_snake_case)]
impl c::TryWitnesses<u64> for Witness {
    fn expectedHash(
        &self,
        ctx: WitnessContext<'_, u64, c::LedgerView<'_>>,
    ) -> Result<(u64, r::FixedBytes<32>), r::CompactError> {
        Ok((
            self.take("expectedHash", *ctx.private_state, ctx.ledger.active()?)?,
            self.expected(),
        ))
    }
    fn digestPayload(
        &self,
        ctx: WitnessContext<'_, u64, c::LedgerView<'_>>,
    ) -> Result<(u64, r::FixedBytes<32>), r::CompactError> {
        Ok((
            self.take("digestPayload", *ctx.private_state, ctx.ledger.active()?)?,
            self.vector.payload(),
        ))
    }
    fn authorized(
        &self,
        ctx: WitnessContext<'_, u64, c::LedgerView<'_>>,
    ) -> Result<(u64, bool), r::CompactError> {
        Ok((
            self.take("authorized", *ctx.private_state, ctx.ledger.active()?)?,
            self.authorized,
        ))
    }
}
fn authorization(
    frame: RecordingFrame<u64>,
    w: &Witness,
) -> Result<RecordingFrame<u64>, r::CompactError> {
    let (frame, answer) = frame.try_witness_metered(|ctx, meter| {
        Ok((
            w.take(
                "authorized",
                ctx.private_state,
                slots::active.witness_read(meter)?,
            )?,
            w.authorized,
        ))
    })?;
    if !answer {
        return Err(denied("not authorized"));
    }
    Ok(frame)
}
pub fn manual_unit(
    ctx: CircuitContext<u64>,
    w: &Witness,
    selected: bool,
) -> Result<RecordedCircuitResult<u64, ()>, r::CompactError> {
    let mut frame = RecordingFrame::new(ctx);
    if selected {
        let (next, expected) = frame.try_witness_metered(|ctx, meter| {
            Ok((
                w.take(
                    "expectedHash",
                    ctx.private_state,
                    slots::active.witness_read(meter)?,
                )?,
                w.expected(),
            ))
        })?;
        // Independent known answer; do not call the Rust Keccak implementation under test.
        if w.vector.digest() != expected {
            return Err(denied("hash mismatch"));
        }
        frame = authorization(next, w)?;
        frame = slots::active.record_write(frame, false)?;
    }
    Ok(frame.finish(()))
}
pub fn manual_digest(
    ctx: CircuitContext<u64>,
    w: &Witness,
    selected: bool,
) -> Result<RecordedCircuitResult<u64, Digest>, r::CompactError> {
    let mut frame = RecordingFrame::new(ctx);
    let value = if selected {
        let (next, payload) = frame.try_witness_metered(|ctx, meter| {
            Ok((
                w.take(
                    "digestPayload",
                    ctx.private_state,
                    slots::active.witness_read(meter)?,
                )?,
                w.vector.payload(),
            ))
        })?;
        assert_eq!(payload, w.vector.payload());
        frame = next;
        w.vector.digest()
    } else {
        r::FixedBytes([0; 32])
    };
    frame = authorization(frame, w)?;
    frame = slots::active.record_write(frame, false)?;
    Ok(frame.finish(Digest { value }))
}
