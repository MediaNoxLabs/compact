// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use crate::{ledger_contract as c, ledger_slots as slots, runtime as r, types};
use r::context::{CircuitContext, ConstructorContext, WitnessContext};
use r::recording::{RecordedCircuitResult, RecordingFrame};
use std::{cell::RefCell, rc::Rc};
/// A Rust-only local interface: never encoded as a public or witness FAB value.
pub type LocalTuple = (
    r::OpaqueBytes,
    r::Field,
    r::Field,
    r::Field,
    r::Field,
    r::Field,
    r::Field,
    r::Field,
    r::Field,
    r::Field,
    r::Field,
    r::Field,
);
pub fn tuple(bytes: &[u8]) -> LocalTuple {
    (
        bytes.to_vec().into(),
        r::Field::from(1_u64),
        r::Field::from(2_u64),
        r::Field::from(3_u64),
        r::Field::from(4_u64),
        r::Field::from(5_u64),
        r::Field::from(6_u64),
        r::Field::from(7_u64),
        r::Field::from(8_u64),
        r::Field::from(9_u64),
        r::Field::from(10_u64),
        r::Field::from(11_u64),
    )
}
pub fn context() -> CircuitContext<u64> {
    c::initial_state(ConstructorContext::new(5_u64))
        .unwrap()
        .into_circuit_context(Default::default())
}
#[derive(Clone, Default)]
pub struct Witness {
    pub fail_at: Option<u128>,
    pub events: Rc<RefCell<Vec<(u64, u128)>>>,
}
impl Witness {
    pub fn new(fail_at: Option<u128>) -> Self {
        Self {
            fail_at,
            events: Rc::default(),
        }
    }
    pub fn observe(&self, private: u64, counter: u128) -> Result<(u64, r::Field), r::CompactError> {
        self.events.borrow_mut().push((private, counter));
        if self.fail_at == Some(counter) {
            return Err(failure());
        }
        Ok((private + 1, r::Field::from(counter as u64)))
    }
}
pub fn failure() -> r::CompactError {
    r::CompactError::InvalidLedgerCell("internal trace refused".into())
}
impl c::TryWitnesses<u64> for Witness {
    fn trace(
        &self,
        context: WitnessContext<'_, u64, c::LedgerView<'_>>,
    ) -> Result<(u64, r::Field), r::CompactError> {
        self.observe(*context.private_state, context.ledger.calls()?.value())
    }
}
pub fn manual_step(
    frame: RecordingFrame<u64>,
    witness: &Witness,
) -> Result<RecordingFrame<u64>, r::CompactError> {
    let frame = slots::calls.record_increment(frame, 1)?;
    let (frame, _value) = frame.try_witness_metered(|ctx, meter| {
        witness.observe(ctx.private_state, slots::calls.witness_read(meter)? as u128)
    })?;
    Ok(frame)
}
pub fn manual(
    context: CircuitContext<u64>,
    witness: &Witness,
    input: r::OpaqueBytes,
    selected: bool,
) -> Result<RecordedCircuitResult<u64, types::Result>, r::CompactError> {
    let mut frame = manual_step(RecordingFrame::new(context), witness)?;
    if selected {
        frame = manual_step(frame, witness)?;
    }
    frame = manual_step(frame, witness)?;
    Ok(frame.finish(types::Result {
        chosen: input.clone(),
        original: input,
        value: r::Field::from(11_u64),
    }))
}
