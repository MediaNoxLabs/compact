// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Independent source oracle using official slot operations, never generated pure helpers.
use crate::{ledger_contract as c, ledger_slots, runtime as r, types::Receipt};
use r::context::{CircuitContext, WitnessContext};
use r::recording::{RecordedCircuitResult, RecordingFrame};
use r::{CompactError, Field};
use std::cell::RefCell;
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WitnessInput {
    pub private: u64,
    pub seed: Field,
    pub stored: Field,
}
#[derive(Default)]
pub struct Witness {
    pub calls: RefCell<Vec<WitnessInput>>,
    pub refuse: bool,
}
impl Witness {
    pub fn evaluate(&self, input: WitnessInput) -> Result<(u64, Field), CompactError> {
        self.calls.borrow_mut().push(input.clone());
        if self.refuse {
            return Err(CompactError::AssertionFailed(
                "owned witness refused".into(),
            ));
        }
        Ok((input.private + 1, input.seed + Field::from(input.private)))
    }
}
impl c::TryWitnesses<u64> for Witness {
    fn nextValue(
        &self,
        ctx: WitnessContext<'_, u64, c::LedgerView<'_>>,
        seed: Field,
    ) -> Result<(u64, Field), CompactError> {
        self.evaluate(WitnessInput {
            private: *ctx.private_state,
            seed,
            stored: ctx.ledger.stored()?,
        })
    }
}
pub fn reference(
    ctx: CircuitContext<u64>,
    selected: bool,
    seed: Field,
    permit: bool,
    witness: &Witness,
) -> Result<RecordedCircuitResult<u64, Receipt>, CompactError> {
    let total = Field::from(if selected { 18_u64 } else { 0 });
    let frame = ledger_slots::stored.record_write(RecordingFrame::new(ctx), total)?;
    let (frame, value) = frame.try_witness_metered(|ctx, meter| {
        witness.evaluate(WitnessInput {
            private: ctx.private_state,
            seed,
            stored: ledger_slots::stored.witness_read(meter)?,
        })
    })?;
    if !permit {
        return Err(CompactError::AssertionFailed("owned helper refused".into()));
    }
    let frame = ledger_slots::stored.record_write(frame, value + total)?;
    let (frame, value) = ledger_slots::stored.record_read(frame)?;
    Ok(frame.finish(Receipt {
        value,
        matched: true,
        marker: if selected {
            seed + seed
        } else {
            Field::from(0_u64)
        },
    }))
}
pub fn conversion_reference(
    ctx: CircuitContext<u64>,
    seed: Field,
) -> Result<RecordedCircuitResult<u64, Receipt>, CompactError> {
    let frame =
        ledger_slots::stored.record_write(RecordingFrame::new(ctx), seed + Field::from(26_u64))?;
    let (frame, value) = ledger_slots::stored.record_read(frame)?;
    Ok(frame.finish(Receipt {
        value,
        matched: true,
        marker: Field::from(18_u64),
    }))
}
