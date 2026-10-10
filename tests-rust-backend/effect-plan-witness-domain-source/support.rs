// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use crate::{ledger_contract as c, ledger_slots as slots, runtime as r, types};
use r::context::{CircuitContext, ConstructorContext, WitnessContext};
use r::recording::{RecordedCircuitResult, RecordingFrame};
use r::{CompactError, Field, FixedVector, OpaqueBytes};
use std::cell::RefCell;
pub type Wide = r::WideUint<4294967295, { u128::MAX }>;
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Event {
    pub tag: &'static str,
    pub private: u64,
    pub marker: Field,
    pub blob: OpaqueBytes,
}
#[derive(Default)]
pub struct Witness {
    pub calls: RefCell<Vec<Event>>,
    pub payload: OpaqueBytes,
    pub refuse: Option<&'static str>,
}
impl Witness {
    fn observe(
        &self,
        tag: &'static str,
        private: u64,
        marker: Field,
        blob: OpaqueBytes,
    ) -> Result<u64, CompactError> {
        self.calls.borrow_mut().push(Event {
            tag,
            private,
            marker,
            blob,
        });
        if self.refuse == Some(tag) {
            return Err(CompactError::AssertionFailed(format!("{tag} refused")));
        }
        Ok(private + 1)
    }
    fn call(
        &self,
        tag: &'static str,
        ctx: WitnessContext<'_, u64, c::LedgerView<'_>>,
    ) -> Result<u64, CompactError> {
        self.observe(
            tag,
            *ctx.private_state,
            ctx.ledger.marker()?,
            ctx.ledger.blob()?,
        )
    }
}
pub fn vector_value() -> FixedVector<Field, 2> {
    FixedVector::new([Field::from(5_u64), Field::from(11_u64)])
}
pub fn tuple_value() -> (Field, bool) {
    (Field::from(23_u64), true)
}
pub fn wide_value() -> Wide {
    let mut bytes = [0; 20];
    bytes[16] = 1;
    bytes[0] = 255;
    Wide::from_le_bytes(&bytes).unwrap()
}
pub fn packet_value() -> types::Packet {
    types::Packet {
        first: Field::from(29_u64),
        values: vector_value(),
        pair: tuple_value(),
        large: wide_value(),
    }
}
impl c::TryWitnesses<u64> for Witness {
    fn bytes(
        &self,
        ctx: WitnessContext<'_, u64, c::LedgerView<'_>>,
    ) -> Result<(u64, OpaqueBytes), CompactError> {
        Ok((self.call("bytes", ctx)?, self.payload.clone()))
    }
    fn vector(
        &self,
        ctx: WitnessContext<'_, u64, c::LedgerView<'_>>,
    ) -> Result<(u64, FixedVector<Field, 2>), CompactError> {
        Ok((self.call("vector", ctx)?, vector_value()))
    }
    fn tuple(
        &self,
        ctx: WitnessContext<'_, u64, c::LedgerView<'_>>,
    ) -> Result<(u64, (Field, bool)), CompactError> {
        Ok((self.call("tuple", ctx)?, tuple_value()))
    }
    fn wide(
        &self,
        ctx: WitnessContext<'_, u64, c::LedgerView<'_>>,
    ) -> Result<(u64, Wide), CompactError> {
        Ok((self.call("wide", ctx)?, wide_value()))
    }
    fn packet(
        &self,
        ctx: WitnessContext<'_, u64, c::LedgerView<'_>>,
    ) -> Result<(u64, types::Packet), CompactError> {
        Ok((self.call("packet", ctx)?, packet_value()))
    }
}
pub fn context() -> CircuitContext<u64> {
    c::initial_state(ConstructorContext::new(7))
        .unwrap()
        .into_circuit_context(Default::default())
}
fn witnessed<T: Clone>(
    frame: RecordingFrame<u64>,
    witness: &Witness,
    tag: &'static str,
    value: T,
) -> Result<(RecordingFrame<u64>, T), CompactError>
where
    r::fab::AlignedValue: From<T>,
{
    frame.try_witness_metered(|context, meter| {
        let private = witness.observe(
            tag,
            context.private_state,
            slots::marker.witness_read(meter)?,
            slots::blob.witness_read(meter)?,
        )?;
        Ok((private, value))
    })
}
pub fn reference(
    ctx: CircuitContext<u64>,
    selected: bool,
    witness: &Witness,
) -> Result<RecordedCircuitResult<u64, bool>, CompactError> {
    let frame = slots::marker.record_write(RecordingFrame::new(ctx), Field::from(1_u64))?;
    let (frame, payload) = witnessed(frame, witness, "bytes", witness.payload.clone())?;
    let frame = slots::blob.record_write(frame, payload.clone())?;
    if selected {
        let (frame, values) = witnessed(frame, witness, "vector", vector_value())?;
        let (frame, _pair) = witnessed(frame, witness, "tuple", tuple_value())?;
        let (frame, _wide) = witnessed(frame, witness, "wide", wide_value())?;
        let (frame, _packet) = witnessed(frame, witness, "packet", packet_value())?;
        let frame = slots::marker.record_write(frame, r::transient_hash(values))?;
        Ok(frame.finish(true))
    } else {
        Ok(frame.finish(true))
    }
}
pub fn private_outputs(witness: &Witness, selected: bool) -> Vec<r::fab::AlignedValue> {
    let mut values = vec![r::fab::AlignedValue::from(witness.payload.clone())];
    if selected {
        values.extend([
            r::fab::AlignedValue::from(vector_value()),
            r::fab::AlignedValue::from(tuple_value()),
            r::fab::AlignedValue::from(wide_value()),
            r::fab::AlignedValue::from(packet_value()),
        ]);
    }
    values
}
