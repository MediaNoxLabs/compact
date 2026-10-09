// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Independent manual source oracle; invokes official slot operations, never generated helpers.
use crate::{
    ledger_contract as c, ledger_slots, runtime as r,
    types::{Envelope, Inner, Phase},
};
use r::context::{CircuitContext, WitnessContext};
use r::recording::{RecordedCircuitResult, RecordingFrame};
use r::{CompactError, Field, FixedBytes};
use std::cell::RefCell;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WitnessInput {
    pub private: u64,
    pub value: Envelope,
    pub tag: FixedBytes<3>,
    pub phase: Phase,
}
#[derive(Default)]
pub struct Witness {
    pub calls: RefCell<Vec<WitnessInput>>,
    pub refuse: bool,
}
impl Witness {
    pub fn evaluate(&self, input: WitnessInput) -> Result<(u64, Envelope), CompactError> {
        self.calls.borrow_mut().push(input.clone());
        if self.refuse {
            return Err(CompactError::AssertionFailed("rich witness refused".into()));
        }
        let value = Envelope {
            inner: Inner {
                amount: input.value.inner.amount + Field::from(input.private),
                tag: FixedBytes::new([input.tag.0[2], input.tag.0[1], input.tag.0[0]]),
            },
            phase: if input.value.phase == Phase::cold {
                Phase::hot
            } else {
                Phase::cold
            },
        };
        Ok((input.private + 1, value))
    }
}
impl c::TryWitnesses<u64> for Witness {
    fn nextEnvelope(
        &self,
        ctx: WitnessContext<'_, u64, c::LedgerView<'_>>,
        value: Envelope,
        tag: FixedBytes<3>,
        phase: Phase,
    ) -> Result<(u64, Envelope), CompactError> {
        self.evaluate(WitnessInput {
            private: *ctx.private_state,
            value,
            tag,
            phase,
        })
    }
}
pub fn initial_value(selected: bool, seed: Field) -> Envelope {
    if selected {
        Envelope {
            inner: Inner {
                amount: seed,
                tag: FixedBytes::new(*b"tag"),
            },
            phase: Phase::hot,
        }
    } else {
        Envelope::default()
    }
}
pub fn reference(
    context: CircuitContext<u64>,
    selected: bool,
    seed: Field,
    witness: &Witness,
) -> Result<RecordedCircuitResult<u64, Field>, CompactError> {
    let base = initial_value(selected, seed);
    let frame = ledger_slots::stored.record_write(RecordingFrame::new(context), base.clone())?;
    let (frame, value) = frame.try_witness_metered(|ctx, _| {
        witness.evaluate(WitnessInput {
            private: ctx.private_state,
            value: base,
            tag: FixedBytes::new(*b"tag"),
            phase: Phase::hot,
        })
    })?;
    // Source pure identity and stateful persist preserve the same rich value.
    let frame = ledger_slots::stored.record_write(frame, value.clone())?;
    let (frame, saved) = ledger_slots::stored.record_read(frame)?;
    let frame = ledger_slots::tag.record_write(frame, value.inner.tag)?;
    let frame = ledger_slots::phase.record_write(frame, value.phase)?;
    let (frame, read) = ledger_slots::stored.record_read(frame)?;
    Ok(frame.finish(read.inner.amount + saved.inner.amount + value.inner.amount))
}
