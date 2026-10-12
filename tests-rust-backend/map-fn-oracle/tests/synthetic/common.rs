// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
pub use crate::ledger_contract as c;
pub use midnight_compact_runtime as r;
use std::{cell::RefCell, rc::Rc};
pub fn field(n: u64) -> r::Field {
    r::Field::from(n)
}
pub fn vector<const N: usize>(values: [u64; N]) -> r::FixedVector<r::Field, N> {
    r::FixedVector::new(values.map(field))
}
pub fn setup() -> (
    r::transaction::ObservedContractState,
    r::transaction::VerifierKey,
) {
    let initial = c::initial_state(r::context::ConstructorContext::new(7_u64)).unwrap();
    crate::support::observation(
        initial.ledger_state.get_ref().clone(),
        &["products", "nested", "empty", "fallible", "witnessed"],
    )
}
#[derive(Clone, Default)]
pub struct Offset {
    pub events: Rc<RefCell<Vec<u64>>>,
    pub fail: bool,
}
impl c::TryWitnesses<u64> for Offset {
    fn offset(
        &self,
        ctx: r::context::WitnessContext<'_, u64, c::LedgerView<'_>>,
    ) -> Result<(u64, r::Field), r::CompactError> {
        self.events.borrow_mut().push(*ctx.private_state);
        if self.fail {
            return Err(r::CompactError::InvalidLedgerCell("offset refused".into()));
        }
        Ok((*ctx.private_state + 1, field(10)))
    }
}
pub fn pure_parity<T: std::fmt::Debug + PartialEq + Clone>(
    state: &r::transaction::ObservedContractState,
    native: &r::context::CircuitResult<u64, T>,
    actual: &r::recording::RecordedCircuitResult<u64, T>,
    expected: T,
) {
    assert_eq!(native.result, expected);
    assert_eq!(native.context.private_state, 7);
    assert!(native.private_transcript_outputs.is_empty());
    crate::support::parity(actual, native);
    let manual = r::recording::RecordingFrame::new(state.circuit_context(7)).finish(expected);
    crate::support::parity(&manual, native);
    assert_eq!(actual.public.verify_ops(), manual.public.verify_ops());
}
