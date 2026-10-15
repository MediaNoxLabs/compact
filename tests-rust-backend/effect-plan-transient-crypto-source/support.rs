// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use crate::{ledger_contract as c, ledger_slots as slots, runtime as r, types};
use r::context::{CircuitContext, ConstructorContext, WitnessContext};
use r::recording::{RecordedCircuitResult, RecordingFrame};
use r::{CompactError, Field, FixedBytes};
use std::cell::RefCell;
#[derive(Default)]
pub struct Witness {
    pub calls: RefCell<Vec<(u64, Field, FixedBytes<32>)>>,
    pub refuse: bool,
}
impl Witness {
    fn evaluate(
        &self,
        private: u64,
        value: Field,
        stored: FixedBytes<32>,
    ) -> Result<(u64, Field), CompactError> {
        self.calls.borrow_mut().push((private, value, stored));
        if self.refuse {
            return Err(CompactError::AssertionFailed("opening refused".into()));
        }
        Ok((private + 1, value + Field::from(1_u64)))
    }
}
impl c::TryWitnesses<u64> for Witness {
    fn opening(
        &self,
        context: WitnessContext<'_, u64, c::LedgerView<'_>>,
        value: Field,
    ) -> Result<(u64, Field), CompactError> {
        self.evaluate(*context.private_state, value, context.ledger.stored()?)
    }
}
pub fn context() -> CircuitContext<u64> {
    c::initial_state(ConstructorContext::new(7))
        .unwrap()
        .into_circuit_context(Default::default())
}
pub fn reference(
    ctx: CircuitContext<u64>,
    input: FixedBytes<32>,
    seed: Field,
    selected: bool,
    witness: &Witness,
) -> Result<RecordedCircuitResult<u64, types::Receipt>, CompactError> {
    let frame = slots::stored.record_write(RecordingFrame::new(ctx), input)?;
    let (frame, stored) = slots::stored.record_read(frame)?;
    let hashed = r::transient_hash((r::degrade_to_transient(stored), seed));
    let (frame, committed) = if selected {
        let (frame, opening) = frame.try_witness_metered(|ctx, meter| {
            witness.evaluate(ctx.private_state, seed, slots::stored.witness_read(meter)?)
        })?;
        (frame, r::transient_commit(hashed, opening))
    } else {
        (frame, hashed)
    };
    let frame = slots::result.record_write(frame, committed)?;
    let (frame, result) = slots::result.record_read(frame)?;
    let digest = r::upgrade_from_transient(result);
    let (frame, commitment) = slots::result.record_read(frame)?;
    Ok(frame.finish(types::Receipt {
        digest,
        hash: hashed,
        commitment,
    }))
}
