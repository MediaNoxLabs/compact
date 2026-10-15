// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use super::Largest;
use compact_rust_effect_plan_wide_field_cast_source_fixture::{
    ledger_contract as c, ledger_slots as slots, runtime as r, types::Checked,
};
use r::context::{CircuitContext, CircuitResult, WitnessContext};
use r::recording::{RecordedCircuitResult, RecordingFrame};
use r::transaction::{ObservedContractState, RecordedCall};
use std::{cell::RefCell, rc::Rc};
pub type Wide = r::WideUint<1, { u128::MAX }>;
pub type Event = (&'static str, u64, u128);
pub fn field(raw: &[u8]) -> r::Field {
    let mut padded = [0; 32];
    padded[..raw.len()].copy_from_slice(raw);
    // Independent official field decoding of known canonical integer bytes.
    // Do not reuse WideUint::as_field, the operation under test.
    r::Field::from_le_bytes(&padded).unwrap()
}
pub fn failure(name: &str) -> r::CompactError {
    r::CompactError::InvalidLedgerCell(format!("{name} witness refused"))
}
#[derive(Clone)]
pub struct Witness {
    pub raw: Vec<u8>,
    fail: Option<&'static str>,
    pub events: Rc<RefCell<Vec<Event>>>,
}
impl Witness {
    pub fn new(raw: Vec<u8>, fail: Option<&'static str>) -> Self {
        Self {
            raw,
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
    fn next129(
        &self,
        ctx: WitnessContext<'_, u64, c::LedgerView<'_>>,
    ) -> Result<(u64, Wide), r::CompactError> {
        Ok((
            self.take("next129", *ctx.private_state, ctx.ledger.calls()?.value())?,
            Wide::from_le_bytes(&self.raw)?,
        ))
    }
    fn next248(
        &self,
        ctx: WitnessContext<'_, u64, c::LedgerView<'_>>,
    ) -> Result<(u64, Largest), r::CompactError> {
        Ok((
            self.take("next248", *ctx.private_state, ctx.ledger.calls()?.value())?,
            Largest::from_le_bytes(&self.raw)?,
        ))
    }
    fn marker(
        &self,
        ctx: WitnessContext<'_, u64, c::LedgerView<'_>>,
    ) -> Result<(u64, r::Field), r::CompactError> {
        Ok((
            self.take("marker", *ctx.private_state, ctx.ledger.calls()?.value())?,
            r::Field::from(9_u64),
        ))
    }
}
#[derive(Clone, Copy)]
pub enum Width {
    Bits129,
    Bits248,
}
impl Width {
    pub fn name(self) -> &'static str {
        match self {
            Self::Bits129 => "witness129",
            Self::Bits248 => "witness248",
        }
    }
    pub fn witness(self) -> &'static str {
        match self {
            Self::Bits129 => "next129",
            Self::Bits248 => "next248",
        }
    }
    pub fn maximum(self) -> (u128, u128) {
        match self {
            Self::Bits129 => (1, u128::MAX),
            Self::Bits248 => ((1_u128 << 120) - 1, u128::MAX),
        }
    }
    pub fn native(
        self,
        ctx: CircuitContext<u64>,
        w: &Witness,
        selected: bool,
    ) -> Result<CircuitResult<u64, Checked>, r::CompactError> {
        match self {
            Self::Bits129 => c::witness129(ctx, w, selected),
            Self::Bits248 => c::witness248(ctx, w, selected),
        }
    }
    pub fn recorded(
        self,
        ctx: CircuitContext<u64>,
        w: &Witness,
        selected: bool,
    ) -> Result<RecordedCircuitResult<u64, Checked>, r::CompactError> {
        match self {
            Self::Bits129 => c::recorded::witness129(ctx, w, selected),
            Self::Bits248 => c::recorded::witness248(ctx, w, selected),
        }
    }
    pub fn observed<'a>(
        self,
        state: &'a ObservedContractState,
        w: &Witness,
        selected: bool,
    ) -> Result<RecordedCall<'a, u64, Checked>, r::CompactError> {
        let contract = c::Contract::from(w.clone());
        match self {
            Self::Bits129 => contract.recording().witness129_call(state, 5, selected),
            Self::Bits248 => contract.recording().witness248_call(state, 5, selected),
        }
    }
}
pub fn manual(
    ctx: CircuitContext<u64>,
    w: &Witness,
    width: Width,
    selected: bool,
) -> Result<RecordedCircuitResult<u64, Checked>, r::CompactError> {
    let frame = slots::calls.record_increment(RecordingFrame::new(ctx), 1)?;
    let frame = if selected {
        match width {
            Width::Bits129 => {
                frame
                    .try_witness_metered(|ctx, meter| {
                        Ok((
                            w.take(
                                "next129",
                                ctx.private_state,
                                slots::calls.witness_read(meter)? as u128,
                            )?,
                            Wide::from_le_bytes(&w.raw)?,
                        ))
                    })?
                    .0
            }
            Width::Bits248 => {
                frame
                    .try_witness_metered(|ctx, meter| {
                        Ok((
                            w.take(
                                "next248",
                                ctx.private_state,
                                slots::calls.witness_read(meter)? as u128,
                            )?,
                            Largest::from_le_bytes(&w.raw)?,
                        ))
                    })?
                    .0
            }
        }
    } else {
        frame
    };
    let value = if selected {
        field(&w.raw)
    } else {
        r::Field::from(7_u64)
    };
    let (frame, marker) = frame.try_witness_metered(|ctx, meter| {
        Ok((
            w.take(
                "marker",
                ctx.private_state,
                slots::calls.witness_read(meter)? as u128,
            )?,
            r::Field::from(9_u64),
        ))
    })?;
    Ok(frame.finish(Checked { value, marker }))
}

/// Explicit decimal case identities accompany the independent little-endian
/// carrier construction, keeping equal-output lazy cases distinct in the TS oracle.
pub fn boundary_cases(width: Width) -> [(u128, u128, &'static str); 5] {
    let (high, low) = width.maximum();
    let maximum = match width {
        Width::Bits129 => "680564733841876926926749214863536422911",
        Width::Bits248 => {
            "452312848583266388373324160190187140051835877600158453279131187530910662655"
        }
    };
    [
        (0, 0, "0"),
        (0, 1, "1"),
        (0, u128::MAX, "340282366920938463463374607431768211455"),
        (1, 0, "340282366920938463463374607431768211456"),
        (high, low, maximum),
    ]
}
