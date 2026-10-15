// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
#![cfg(feature = "ledger-transaction")]
mod common;
use common::*;
use compact_rust_effect_plan_wide_casts_source_fixture::{
    ledger_contract as c, ledger_slots as slots, runtime as r, types,
};
use r::context::{CircuitContext, WitnessContext};
use r::recording::{RecordedCircuitResult, RecordingFrame};
use std::{cell::RefCell, rc::Rc};
type WitnessEvent = (&'static str, u64, u128);
#[derive(Clone)]
struct Witness {
    value: Largest,
    events: Rc<RefCell<Vec<WitnessEvent>>>,
}
impl Witness {
    fn new(value: Largest) -> Self {
        Self {
            value,
            events: Rc::default(),
        }
    }
    fn event(&self, name: &'static str, private: u64, counter: u128) {
        self.events.borrow_mut().push((name, private, counter));
    }
}
impl c::TryWitnesses<u64> for Witness {
    fn nextWide(
        &self,
        context: WitnessContext<'_, u64, c::LedgerView<'_>>,
    ) -> Result<(u64, Largest), r::CompactError> {
        self.event(
            "nextWide",
            *context.private_state,
            context.ledger.calls()?.value(),
        );
        Ok((*context.private_state + 1, self.value))
    }
    fn marker(
        &self,
        context: WitnessContext<'_, u64, c::LedgerView<'_>>,
    ) -> Result<(u64, r::Field), r::CompactError> {
        self.event(
            "marker",
            *context.private_state,
            context.ledger.calls()?.value(),
        );
        Ok((*context.private_state + 1, r::Field::from(9_u64)))
    }
}
fn manual(
    context: CircuitContext<u64>,
    witness: &Witness,
    selected: bool,
) -> Result<RecordedCircuitResult<u64, types::Checked>, r::CompactError> {
    let mut frame = slots::calls.record_increment(RecordingFrame::new(context), 1)?;
    let input;
    if selected {
        let (next, value) = frame.try_witness_metered(|ctx, meter| {
            witness.event(
                "nextWide",
                ctx.private_state,
                slots::calls.witness_read(meter)? as u128,
            );
            Ok((ctx.private_state + 1, witness.value))
        })?;
        frame = next;
        input = value;
    } else {
        input = Largest::from_le_bytes(&[7]).unwrap();
    }
    // Independent range decision; do not call the emitter's conversion helper.
    let raw = input.as_le_bytes();
    if raw[16..].iter().any(|byte| *byte != 0) {
        return Err(r::CompactError::InvalidUnsignedValue);
    }
    let low = u128::from_le_bytes(raw[..16].try_into().unwrap());
    let value = r::BoundedUint::<255>::new(low)?;
    let (frame, marker) = frame.try_witness_metered(|ctx, meter| {
        witness.event(
            "marker",
            ctx.private_state,
            slots::calls.witness_read(meter)? as u128,
        );
        Ok((ctx.private_state + 1, r::Field::from(9_u64)))
    })?;
    Ok(frame.finish(types::Checked { value, marker }))
}
#[test]
fn witness_values_are_evaluated_once_and_failure_prevents_the_following_callback() {
    for (high, low) in [
        (0, 0),
        (0, 255),
        (0, 256),
        (1, 0),
        ((1_u128 << 120) - 1, u128::MAX),
    ] {
        for selected in [false, true] {
            let value = Largest::from_le_bytes(&bytes(high, low)).unwrap();
            let (a, b, m, w) = (
                Witness::new(value),
                Witness::new(value),
                Witness::new(value),
                Witness::new(value),
            );
            let (state, key) = setup();
            let native = c::witnessed(state.circuit_context(5), &a, selected);
            let recorded = c::recorded::witnessed(state.circuit_context(5), &b, selected);
            let manual = manual(state.circuit_context(5), &m, selected);
            let contract = c::Contract::from(w.clone());
            let observed = contract.recording().witnessed_call(&state, 5, selected);
            let failed = selected && (high != 0 || low > 255);
            let expected = if failed {
                vec![("nextWide", 5, 1)]
            } else if selected {
                vec![("nextWide", 5, 1), ("marker", 6, 1)]
            } else {
                vec![("marker", 5, 1)]
            };
            for witness in [&a, &b, &m, &w] {
                assert_eq!(*witness.events.borrow(), expected);
            }
            if failed {
                let expected = if high == 0 {
                    r::CompactError::UnsignedOutOfRange {
                        value: low,
                        max: 255,
                    }
                } else {
                    r::CompactError::InvalidUnsignedValue
                };
                assert_eq!(native.err().unwrap(), expected);
                assert_eq!(recorded.err().unwrap(), expected);
                assert_eq!(manual.err().unwrap(), expected);
                assert_eq!(observed.err().unwrap(), expected);
            } else {
                let (native, recorded, manual, observed) = (
                    native.unwrap(),
                    recorded.unwrap(),
                    manual.unwrap(),
                    observed.unwrap(),
                );
                assert_eq!(
                    native.result,
                    types::Checked {
                        value: r::BoundedUint::new(if selected { low } else { 7 }).unwrap(),
                        marker: r::Field::from(9_u64)
                    }
                );
                compare(&native, &recorded, &manual);
                compare(&native, observed.recorded(), &manual);
                assert_eq!(native.context.private_state, if selected { 7 } else { 6 });
                assert_eq!(
                    native
                        .private_transcript_outputs
                        .iter()
                        .map(atoms)
                        .collect::<Vec<_>>(),
                    if selected {
                        vec![vec![bytes(high, low)], vec![vec![9]]]
                    } else {
                        vec![vec![vec![9]]]
                    }
                );
                let prepared = observed.prepare(key, r::Field::from(0_u64)).unwrap();
                assert_ts("witnessed", &prepared.input, &prepared.output, &native);
                assert_eq!(
                    atoms(&prepared.input),
                    vec![if selected { vec![1] } else { vec![] }]
                );
                assert_eq!(
                    atoms(&prepared.output),
                    vec![bytes(0, if selected { low } else { 7 }), vec![9]]
                );
            }
        }
    }
}
