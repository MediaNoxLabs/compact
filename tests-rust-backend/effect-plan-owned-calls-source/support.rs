// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use crate::{ledger_contract as c, ledger_slots as slots, runtime as r};
use r::context::{CircuitContext, ConstructorContext, WitnessContext};
use r::recording::{RecordedCircuitResult, RecordingFrame};
use r::{CompactError, Field, OpaqueBytes};
use std::cell::RefCell;
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
    pub refuse_call: Option<usize>,
}
impl Witness {
    fn observe(
        &self,
        tag: &'static str,
        private: u64,
        marker: Field,
        blob: OpaqueBytes,
    ) -> Result<u64, CompactError> {
        let index = self.calls.borrow().len();
        self.calls.borrow_mut().push(Event {
            tag,
            private,
            marker,
            blob,
        });
        if self.refuse_call == Some(index) {
            return Err(CompactError::AssertionFailed(format!(
                "callback {index} refused"
            )));
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
impl c::TryWitnesses<u64> for Witness {
    fn bytes(
        &self,
        ctx: WitnessContext<'_, u64, c::LedgerView<'_>>,
    ) -> Result<(u64, OpaqueBytes), CompactError> {
        Ok((self.call("bytes", ctx)?, self.payload.clone()))
    }
    fn trace(
        &self,
        ctx: WitnessContext<'_, u64, c::LedgerView<'_>>,
    ) -> Result<(u64, Field), CompactError> {
        let next = self.call("trace", ctx)?;
        Ok((next, Field::from(next)))
    }
}
pub fn context() -> CircuitContext<u64> {
    c::initial_state(ConstructorContext::new(7))
        .unwrap()
        .into_circuit_context(Default::default())
}
fn trace(
    frame: RecordingFrame<u64>,
    witness: &Witness,
    marker: u64,
) -> Result<RecordingFrame<u64>, CompactError> {
    let frame = slots::marker.record_write(frame, Field::from(marker))?;
    let (frame, _value) = frame.try_witness_metered(|ctx, meter| {
        let next = witness.observe(
            "trace",
            ctx.private_state,
            slots::marker.witness_read(meter)?,
            slots::blob.witness_read(meter)?,
        )?;
        Ok((next, Field::from(next)))
    })?;
    Ok(frame)
}
pub fn reference(
    ctx: CircuitContext<u64>,
    selected: bool,
    permit: bool,
    witness: &Witness,
) -> Result<RecordedCircuitResult<u64, Field>, CompactError> {
    let (mut frame, payload) = RecordingFrame::new(ctx).try_witness_metered(|ctx, meter| {
        let next = witness.observe(
            "bytes",
            ctx.private_state,
            slots::marker.witness_read(meter)?,
            slots::blob.witness_read(meter)?,
        )?;
        Ok((next, witness.payload.clone()))
    })?;
    // Source argument order, assemble, two nested identities, then one sequential identity.
    for marker in [1, 3, 7, 6, 6, 6] {
        frame = trace(frame, witness, marker)?;
    }
    frame = slots::blob.record_write(frame, payload)?;
    if selected {
        for marker in [2, 4, 5, 8] {
            frame = trace(frame, witness, marker)?;
        }
        if !permit {
            return Err(CompactError::AssertionFailed(
                "owned stateful helper refused".into(),
            ));
        }
    }
    Ok(frame.finish(Field::from(1_u64)))
}
pub fn private_outputs(witness: &Witness, selected: bool) -> Vec<r::fab::AlignedValue> {
    let mut values = vec![r::fab::AlignedValue::from(witness.payload.clone())];
    values.extend(
        (9_u64..=if selected { 18 } else { 14 })
            .map(|value| r::fab::AlignedValue::from(Field::from(value))),
    );
    values
}
pub fn expected_markers(selected: bool) -> Vec<u64> {
    let mut markers = vec![0, 1, 3, 7, 6, 6, 6];
    if selected {
        markers.extend([2, 4, 5, 8]);
    }
    markers
}
