// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Tagged witnesses and an independent official-slot arithmetic program.
use crate::{ledger_contract as c, ledger_slots as slots, runtime as r, types};
use r::context::{CircuitContext, ConstructorContext, WitnessContext};
use r::recording::{RecordedCircuitResult, RecordingFrame};
use r::{BoundedUint, CompactError, Field};
use std::cell::RefCell;
pub type U8 = BoundedUint<255>;
pub type U16 = BoundedUint<65535>;
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WitnessInput {
    pub tag: Field,
    pub value: U8,
    pub private: u64,
    pub difference: U8,
    pub narrowed: U8,
}
#[derive(Default)]
pub struct Witness {
    pub calls: RefCell<Vec<WitnessInput>>,
    pub refuse_tag: Option<Field>,
}
impl Witness {
    pub fn evaluate(&self, input: WitnessInput) -> Result<(u64, U8), CompactError> {
        self.calls.borrow_mut().push(input.clone());
        if self.refuse_tag == Some(input.tag) {
            return Err(CompactError::AssertionFailed(
                "operand witness refused".into(),
            ));
        }
        Ok((input.private + 1, input.value))
    }
}
impl c::TryWitnesses<u64> for Witness {
    fn operand(
        &self,
        ctx: WitnessContext<'_, u64, c::LedgerView<'_>>,
        tag: Field,
        value: U8,
    ) -> Result<(u64, U8), CompactError> {
        self.evaluate(WitnessInput {
            tag,
            value,
            private: *ctx.private_state,
            difference: ctx.ledger.difference()?,
            narrowed: ctx.ledger.narrowed()?,
        })
    }
}
pub fn context() -> CircuitContext<u64> {
    c::initial_state(ConstructorContext::new(7))
        .unwrap()
        .into_circuit_context(Default::default())
}
fn operand(
    frame: RecordingFrame<u64>,
    tag: u64,
    value: U8,
    witness: &Witness,
) -> Result<(RecordingFrame<u64>, U8), CompactError> {
    frame.try_witness_metered(|ctx, meter| {
        witness.evaluate(WitnessInput {
            tag: Field::from(tag),
            value,
            private: ctx.private_state,
            difference: slots::difference.witness_read(meter)?,
            narrowed: slots::narrowed.witness_read(meter)?,
        })
    })
}
fn multiply(
    frame: RecordingFrame<u64>,
    left: U8,
    right: U8,
    tag: u64,
    witness: &Witness,
) -> Result<(RecordingFrame<u64>, U16), CompactError> {
    let (frame, left) = operand(frame, tag, left, witness)?;
    let (frame, right) = operand(frame, tag + 1, right, witness)?;
    let value = r::multiply_unsigned::<255, 255, 65535>(left, right)?;
    Ok((frame, value))
}
pub fn reference(
    ctx: CircuitContext<u64>,
    left: U8,
    right: U8,
    selected: bool,
    witness: &Witness,
) -> Result<RecordedCircuitResult<u64, types::Receipt>, CompactError> {
    let (frame, left_value) = operand(RecordingFrame::new(ctx), 1, left, witness)?;
    let (frame, right_value) = operand(frame, 2, right, witness)?;
    let difference = r::subtract_unsigned::<255, 255, 255>(left_value, right_value)?;
    let frame = slots::difference.record_write(frame, difference)?;
    let (frame, product) = if selected {
        multiply(frame, left, right, 3, witness)?
    } else {
        (frame, U16::new(0)?)
    };
    let frame = slots::product.record_write(frame, product)?;
    let (frame, difference) = slots::difference.record_read(frame)?;
    let (frame, product) = slots::product.record_read(frame)?;
    Ok(frame.finish(types::Receipt {
        difference,
        product,
    }))
}
pub fn narrow_reference(
    ctx: CircuitContext<u64>,
    left: U8,
    right: U8,
    selected: bool,
    witness: &Witness,
) -> Result<RecordedCircuitResult<u64, types::NarrowReceipt>, CompactError> {
    let (frame, value) = if selected {
        let (frame, product) = multiply(RecordingFrame::new(ctx), left, right, 5, witness)?;
        (frame, r::cast_unsigned::<65535, 255>(product)?)
    } else {
        (RecordingFrame::new(ctx), U8::new(0)?)
    };
    let frame = slots::narrowed.record_write(frame, value)?;
    let (frame, value) = slots::narrowed.record_read(frame)?;
    Ok(frame.finish(types::NarrowReceipt { value }))
}
