// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Independent official-slot program; no generated hash/call implementation is reused.
use crate::{ledger_contract as c, ledger_slots, runtime as r, types};
use r::context::{CircuitContext, WitnessContext};
use r::recording::{RecordedCircuitResult, RecordingFrame};
use r::{BoundedUint, CompactError, Field, FixedBytes, FixedVector};
use std::cell::RefCell;
pub type Amount = BoundedUint<65535>;
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WitnessInput {
    pub private: u64,
    pub seed: Field,
    pub round: u64,
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
                "counter hash witness refused".into(),
            ));
        }
        Ok((input.private + 1, input.seed + Field::from(input.private)))
    }
}
impl c::TryWitnesses<u64> for Witness {
    fn opening(
        &self,
        ctx: WitnessContext<'_, u64, c::LedgerView<'_>>,
        seed: Field,
    ) -> Result<(u64, Field), CompactError> {
        self.evaluate(WitnessInput {
            private: *ctx.private_state,
            seed,
            round: ctx.ledger.round()?.value() as u64,
        })
    }
}
pub fn encoded(value: Field) -> FixedBytes<32> {
    FixedBytes::new(
        value
            .as_le_bytes()
            .try_into()
            .expect("canonical Field has 32 bytes"),
    )
}
pub fn reference(
    ctx: CircuitContext<u64>,
    seed: Field,
    amount: Amount,
    selected: bool,
    witness: &Witness,
) -> Result<RecordedCircuitResult<u64, types::Receipt>, CompactError> {
    let frame =
        ledger_slots::round.record_increment(RecordingFrame::new(ctx), amount.value() as u16)?;
    let (frame, value) = if selected {
        frame.try_witness_metered(|ctx, meter| {
            witness.evaluate(WitnessInput {
                private: ctx.private_state,
                seed,
                round: ledger_slots::round.witness_read(meter)?,
            })
        })?
    } else {
        (frame, seed)
    };
    let (frame, round) = ledger_slots::round.record_read(frame)?;
    let encoded = encoded(seed);
    let digest = r::persistent_hash((
        self::encoded(Field::from(round)),
        encoded,
        self::encoded(value),
    ));
    let frame = ledger_slots::stored.record_write(frame, digest)?;
    let (frame, digest) = ledger_slots::stored.record_read(frame)?;
    Ok(frame.finish(types::Receipt {
        digest,
        pair: r::persistent_hash((encoded, encoded)),
        encoded,
    }))
}
pub fn products_reference(
    ctx: CircuitContext<u64>,
    value: FixedBytes<3>,
) -> Result<RecordedCircuitResult<u64, types::ProductReceipt>, CompactError> {
    let bytes = r::persistent_hash(value);
    let frame = ledger_slots::stored.record_write(RecordingFrame::new(ctx), bytes)?;
    let (frame, bytes) = ledger_slots::stored.record_read(frame)?;
    Ok(frame.finish(types::ProductReceipt {
        bytes,
        vector: r::persistent_hash(FixedVector::new([value, value])),
        // Same official FAB sequence, independently assembled without generated Product.
        nested: r::persistent_hash((value, FixedVector::new([value, value]))),
        empty: r::persistent_hash(FixedBytes::<0>::new([])),
        emptyVector: r::persistent_hash(FixedVector::<FixedBytes<3>, 0>::new([])),
        emptyStruct: r::persistent_hash(()),
    }))
}
