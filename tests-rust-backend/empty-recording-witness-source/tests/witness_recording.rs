// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
#![cfg(feature = "ledger-transaction")]
mod common;
use common::{args, setup, uint};
use compact_rust_empty_recording_witness_source_fixture::ledger_contract as c;
use midnight_compact_runtime as r;
use r::context::WitnessContext;
use r::transaction::{ObservedCallError, PrepareCallError};
use std::{cell::RefCell, rc::Rc};
#[derive(Clone)]
struct Witness {
    value: bool,
    fail: bool,
    events: Rc<RefCell<Vec<u64>>>,
}
impl Witness {
    fn new(value: bool, fail: bool) -> Self {
        Self {
            value,
            fail,
            events: Default::default(),
        }
    }
}
fn provider_error() -> r::CompactError {
    r::CompactError::AssertionFailed("permit provider failed".into())
}
impl c::TryWitnesses<u64> for Witness {
    fn permit(
        &self,
        context: WitnessContext<'_, u64, c::LedgerView<'_>>,
    ) -> Result<(u64, bool), r::CompactError> {
        self.events.borrow_mut().push(*context.private_state);
        if self.fail {
            Err(provider_error())
        } else {
            Ok((*context.private_state + 1, self.value))
        }
    }
}
#[test]
fn witness_private_effect_survives_empty_public_recording() {
    let (observed, key) = setup();
    for (enforce, now) in [(true, 110), (false, 130)] {
        let (policy, attestation) = args(enforce);
        let nw = Witness::new(true, false);
        let rw = Witness::new(true, false);
        let ow = Witness::new(true, false);
        let native = c::recordFreshEnough(
            observed.circuit_context(7),
            &nw,
            policy.clone(),
            attestation.clone(),
            uint(now),
        )
        .unwrap();
        let recorded = c::recorded::recordFreshEnough(
            observed.circuit_context(7),
            &rw,
            policy.clone(),
            attestation.clone(),
            uint(now),
        )
        .unwrap();
        let generated = c::Contract::from(ow.clone());
        let call = generated
            .recording()
            .recordFreshEnough_call(&observed, 7, policy.clone(), attestation.clone(), uint(now))
            .unwrap();
        let (frame, permit) = r::recording::RecordingFrame::new(observed.circuit_context(7))
            .try_witness_metered(|ctx, _| Ok((ctx.private_state + 1, true)))
            .unwrap();
        assert!(permit);
        let manual = frame.finish(());
        assert_eq!(
            native.context.query.state.get_ref(),
            observed.contract().data.get_ref()
        );
        assert_eq!(native.gas_cost, Default::default());
        assert_eq!(native.context.private_state, 8);
        assert_eq!(
            native.private_transcript_outputs,
            vec![r::fab::AlignedValue::from(true)]
        );
        for candidate in [&recorded, call.recorded(), &manual] {
            assert!(candidate.public.verify_ops().is_empty());
            assert_eq!(
                candidate.execution.context.query.state,
                native.context.query.state
            );
            assert_eq!(
                candidate.execution.context.query.effects,
                native.context.query.effects
            );
            assert_eq!(candidate.execution.gas_cost, native.gas_cost);
            assert_eq!(candidate.execution.context.private_state, 8);
            assert_eq!(
                candidate.execution.private_transcript_outputs,
                native.private_transcript_outputs
            );
        }
        for witness in [&nw, &rw, &ow] {
            assert_eq!(*witness.events.borrow(), [7]);
        }
        assert!(matches!(
            call.prepare(key.clone(), r::Field::from(0_u64)),
            Err(ObservedCallError::Prepare(
                PrepareCallError::EmptyTranscript
            ))
        ));
        let input = r::fab::AlignedValue::from((policy, attestation, uint(now)));
        assert!(matches!(
            r::transaction::prepare_call(
                recorded,
                r::transaction::CallSpec::new(
                    "recordFreshEnough",
                    key.clone(),
                    input,
                    r::Field::from(0_u64)
                )
            ),
            Err(PrepareCallError::EmptyTranscript)
        ));
    }
}
#[test]
fn guards_and_provider_refusals_preserve_the_exact_callback_prefix() {
    let (observed, _) = setup();
    for (now, permit, provider_fails, message, callback) in [
        (
            90,
            true,
            false,
            "attestation creation time cannot be in the future",
            false,
        ),
        (
            130,
            true,
            false,
            "attestation exceeds the max-age policy",
            false,
        ),
        (110, false, false, "witness continuation refused", true),
        (110, true, true, "permit provider failed", true),
    ] {
        let (policy, attestation) = args(true);
        let nw = Witness::new(permit, provider_fails);
        let rw = Witness::new(permit, provider_fails);
        let ow = Witness::new(permit, provider_fails);
        let generated = c::Contract::from(ow.clone());
        let errors = [
            c::recordFreshEnough(
                observed.circuit_context(7),
                &nw,
                policy.clone(),
                attestation.clone(),
                uint(now),
            )
            .err()
            .unwrap(),
            c::recorded::recordFreshEnough(
                observed.circuit_context(7),
                &rw,
                policy.clone(),
                attestation.clone(),
                uint(now),
            )
            .err()
            .unwrap(),
            generated
                .recording()
                .recordFreshEnough_call(&observed, 7, policy, attestation, uint(now))
                .err()
                .unwrap(),
        ];
        for error in errors {
            assert_eq!(error, r::CompactError::AssertionFailed(message.into()));
        }
        for witness in [&nw, &rw, &ow] {
            assert_eq!(
                *witness.events.borrow(),
                if callback { vec![7] } else { vec![] }
            );
        }
        // Consumed error contexts are not evidence of transaction rollback.
    }
}
