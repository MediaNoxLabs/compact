// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
#![cfg(feature = "ledger-transaction")]
mod common;
use common::*;
use compact_rust_effect_plan_own_public_key_source_fixture::{
    ledger_contract as c, ledger_slots as slots, runtime as r, types,
};
use r::context::{CircuitContext, WitnessContext};
use r::recording::{RecordedCircuitResult, RecordingFrame};
use r::transaction::{ObservedCallError, RecordedCall};
use std::{cell::RefCell, rc::Rc};
#[derive(Clone, Default)]
struct Witness {
    fail: Option<u8>,
    events: Rc<RefCell<Vec<(u8, u64)>>>,
}
fn failure() -> r::CompactError {
    r::CompactError::InvalidLedgerCell("mark refused".into())
}
impl Witness {
    fn observe(&self, step: u8, private: u64) -> Result<(u64, r::Field), r::CompactError> {
        self.events.borrow_mut().push((step, private));
        if self.fail == Some(step) {
            return Err(failure());
        }
        Ok((private + 1, r::Field::from(u64::from(step))))
    }
}
impl c::TryWitnesses<u64> for Witness {
    fn mark(
        &self,
        ctx: WitnessContext<'_, u64, c::LedgerView<'_>>,
        step: r::BoundedUint<255>,
    ) -> Result<(u64, r::Field), r::CompactError> {
        self.observe(step.value() as u8, *ctx.private_state)
    }
}
fn manual(
    ctx: CircuitContext<u64>,
    w: &Witness,
    selected: bool,
) -> Result<RecordedCircuitResult<u64, types::Result>, r::CompactError> {
    let frame = slots::calls.record_increment(RecordingFrame::new(ctx), 1)?;
    let (frame, before) = frame.try_witness_metered(|ctx, _| w.observe(1, ctx.private_state))?;
    let (frame, first) = if selected {
        frame.own_coin_public_key()?
    } else {
        (frame, [0; 32])
    };
    let (frame, middle) = frame.try_witness_metered(|ctx, _| w.observe(2, ctx.private_state))?;
    let (frame, second) = if selected {
        frame.own_coin_public_key()?
    } else {
        (frame, [0; 32])
    };
    let (frame, after) = frame.try_witness_metered(|ctx, _| w.observe(3, ctx.private_state))?;
    Ok(frame.finish(types::Result {
        first: types::ZswapCoinPublicKey {
            bytes: r::FixedBytes::new(first),
        },
        second: types::ZswapCoinPublicKey {
            bytes: r::FixedBytes::new(second),
        },
        before,
        middle,
        after,
    }))
}
#[test]
fn repeated_native_keys_preserve_interleaved_witnesses_lazy_branches_and_prepared_atoms() {
    for identity in [None, Some([0; 32]), Some([19; 32])] {
        for selected in [false, true] {
            if selected && identity.is_none() {
                continue;
            }
            let (state, verifier) = setup(identity);
            let (a, b, m, o) = (
                Witness::default(),
                Witness::default(),
                Witness::default(),
                Witness::default(),
            );
            let native = c::run(state.circuit_context(5), &a, selected).unwrap();
            let recorded = c::recorded::run(state.circuit_context(5), &b, selected).unwrap();
            let manual = manual(state.circuit_context(5), &m, selected).unwrap();
            let contract = c::Contract::from(o.clone());
            let call = contract.recording().run_call(&state, 5, selected).unwrap();
            compare(&native, &recorded, &manual);
            compare(&native, call.recorded(), &manual);
            for w in [&a, &b, &m, &o] {
                assert_eq!(*w.events.borrow(), vec![(1, 5), (2, 6), (3, 7)]);
            }
            assert_eq!(native.context.private_state, 8);
            assert_eq!(
                slots::calls
                    .inspect(native.context.query.state.get_ref())
                    .unwrap(),
                1
            );
            let key = if selected { identity.unwrap() } else { [0; 32] };
            assert_eq!(native.result.first.bytes.0, key);
            assert_eq!(native.result.second.bytes.0, key);
            let mut private = vec![r::fab::AlignedValue::from(r::Field::from(1_u64))];
            if selected {
                private.push(r::fab::AlignedValue::from(key));
            }
            private.push(r::fab::AlignedValue::from(r::Field::from(2_u64)));
            if selected {
                private.push(r::fab::AlignedValue::from(key));
            }
            private.push(r::fab::AlignedValue::from(r::Field::from(3_u64)));
            assert_eq!(native.private_transcript_outputs, private);
            assert_eq!(recorded.public.initial_coin_public_key(), identity);
            assert_eq!(recorded.public.final_coin_public_key(), identity);
            let prepared = call.prepare(verifier, r::Field::from(0_u64)).unwrap();
            assert_eq!(
                atoms(&prepared.input),
                vec![if selected { vec![1] } else { vec![] }]
            );
            let key_atom = if key == [0; 32] { vec![] } else { key.to_vec() };
            assert_eq!(
                atoms(&prepared.output),
                vec![key_atom.clone(), key_atom, vec![1], vec![2], vec![3]]
            );
        }
    }
}
#[test]
fn missing_key_and_user_refusals_stop_at_the_exact_source_callback_prefix() {
    for (identity, selected, fail, prefix, expected) in [
        (None, true, None, 1, r::CompactError::MissingCoinPublicKey),
        (None, true, Some(1), 1, failure()),
        (Some([19; 32]), true, Some(2), 2, failure()),
        (Some([19; 32]), true, Some(3), 3, failure()),
        (None, false, Some(3), 3, failure()),
    ] {
        let (state, _) = setup(identity);
        let make = || Witness {
            fail,
            ..Witness::default()
        };
        let (a, b, m, o) = (make(), make(), make(), make());
        assert_eq!(
            c::run(state.circuit_context(5), &a, selected)
                .err()
                .unwrap(),
            expected
        );
        assert_eq!(
            c::recorded::run(state.circuit_context(5), &b, selected)
                .err()
                .unwrap(),
            expected
        );
        assert_eq!(
            manual(state.circuit_context(5), &m, selected)
                .err()
                .unwrap(),
            expected
        );
        let contract = c::Contract::from(o.clone());
        assert_eq!(
            contract
                .recording()
                .run_call(&state, 5, selected)
                .err()
                .unwrap(),
            expected
        );
        let events = [(1, 5), (2, 6), (3, 7)];
        for w in [&a, &b, &m, &o] {
            assert_eq!(w.events.borrow().as_slice(), &events[..prefix]);
        }
    }
}
#[test]
fn generated_recording_cannot_be_rebound_to_a_different_observed_identity() {
    let (source, verifier) = setup(Some([19; 32]));
    for target_key in [Some([19; 32]), Some([23; 32]), None] {
        let (target, _) = setup(target_key);
        let recorded =
            c::recorded::run(source.circuit_context(5), &Witness::default(), true).unwrap();
        // Exercise the public adapter with an actual generated recording, not a fabricated frame.
        let call = RecordedCall::new(&target, recorded, "run", true);
        let prepared = call.prepare(verifier.clone(), r::Field::from(0_u64));
        if target_key == Some([19; 32]) {
            assert!(prepared.is_ok());
        } else {
            assert!(matches!(
                prepared,
                Err(ObservedCallError::CoinPublicKeyMismatch)
            ));
        }
    }
}
