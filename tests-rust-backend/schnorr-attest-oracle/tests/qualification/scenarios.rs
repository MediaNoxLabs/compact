// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use super::*;

#[test]
fn observed_schnorr_calls_match_manual_ts_fab_and_helper_before_counter_order() {
    let (oracle, digest, signature) = inputs();
    // Reconstruct the independently captured full challenge from TS's q/remainder FAB.
    let atoms = oracle["verifyTranscript"][0]["valueAtoms"]
        .as_array()
        .unwrap();
    let mut bytes = atoms[1]
        .as_array()
        .unwrap()
        .iter()
        .map(|b| b.as_u64().unwrap() as u8)
        .collect::<Vec<_>>();
    bytes.resize(31, 0);
    bytes.push(atoms[0][0].as_u64().unwrap() as u8);
    let expected_challenge = Field::from_le_bytes(&bytes).unwrap();
    for root in [Root::Verify, Root::Accept] {
        for inspect_counter in [false, true] {
            let (state, key) = setup(false, false);
            let a = Provider {
                inspect_counter,
                ..Default::default()
            };
            let b = Provider {
                inspect_counter,
                ..Default::default()
            };
            let w = Provider {
                inspect_counter,
                ..Default::default()
            };
            let m = Provider {
                inspect_counter,
                ..Default::default()
            };
            let native = root
                .native(&state, &a, digest.clone(), signature.clone())
                .unwrap();
            let recorded = root
                .recorded(&state, &b, digest.clone(), signature.clone())
                .unwrap();
            let call = root
                .observed(&state, &w, digest.clone(), signature.clone())
                .unwrap();
            let manual =
                manual::execute(root, &state, &m, digest.clone(), signature.clone()).unwrap();
            let reference_replay = manual
                .public
                .initial()
                .query(manual.public.verify_ops(), None, &native.context.cost_model)
                .unwrap();
            for actual in [&recorded, call.recorded(), &manual] {
                assert_eq!(
                    actual.execution.context.query.state,
                    native.context.query.state
                );
                assert_eq!(
                    actual.execution.context.query.effects,
                    native.context.query.effects
                );
                assert_eq!(actual.execution.gas_cost, native.gas_cost);
                assert_eq!(actual.execution.context.private_state, root.private() + 1);
                assert_eq!(
                    actual.execution.private_transcript_outputs,
                    native.private_transcript_outputs
                );
                assert_transcript(
                    &actual.execution.private_transcript_outputs,
                    &oracle[root.transcript()],
                );
                assert_eq!(actual.public.verify_ops(), manual.public.verify_ops());
                let replay = actual
                    .public
                    .initial()
                    .query(actual.public.verify_ops(), None, &native.context.cost_model)
                    .unwrap();
                assert_eq!(replay.context.state, native.context.query.state);
                assert_eq!(replay.context.effects, native.context.query.effects);
                assert_eq!(replay.gas_cost, reference_replay.gas_cost);
                if !inspect_counter {
                    check_recorded(actual, &oracle[root.trace()]);
                }
            }
            assert_eq!(native.context.private_state, root.private() + 1);
            assert_eq!(
                slots::acceptedCount
                    .inspect(native.context.query.state.get_ref())
                    .unwrap(),
                u64::from(root.accepts())
            );
            for provider in [&a, &b, &w, &m] {
                assert_eq!(
                    *provider.events.borrow(),
                    vec![Event {
                        private: root.private(),
                        challenge: expected_challenge,
                        count: inspect_counter.then_some(0)
                    }]
                );
            }
            // Extra metered witness reads are a separate Rust ordering control,
            // not falsely attributed to the unchanged TS callback implementation.
            if !inspect_counter {
                assert_eq!(
                    state_hex(native.context.query.state.get_ref().clone()),
                    oracle[if root.accepts() {
                        "afterAcceptHex"
                    } else {
                        "initialHex"
                    }]
                );
            }
            let prepared = call.prepare(key, Field::from(0_u64)).unwrap();
            assert_eq!(
                prepared.input,
                r::fab::AlignedValue::from((digest.clone(), signature.clone()))
            );
            assert_eq!(prepared.output, r::fab::AlignedValue::from(()));
            // The empty verifier carrier qualifies preparation only, not a proof.
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum Refusal {
    Closed,
    Provider,
    Quotient,
    Reconstruction,
    IdentityAnnouncement,
    IdentityKey,
    Signature,
}

#[test]
fn guard_reduction_and_signature_refusals_preserve_the_exact_callback_prefix() {
    // These are representable curve points and values: identity and a wrong
    // signature are not malformed/off-curve carrier inputs.
    for root in [Root::Verify, Root::Accept] {
        for case in [
            Refusal::Closed,
            Refusal::Provider,
            Refusal::Quotient,
            Refusal::Reconstruction,
            Refusal::IdentityAnnouncement,
            Refusal::IdentityKey,
            Refusal::Signature,
        ] {
            let (_, digest, mut signature) = inputs();
            let closed = matches!(case, Refusal::Closed);
            let identity_key = matches!(case, Refusal::IdentityKey);
            let (state, _) = setup(closed, identity_key);
            let behavior = match case {
                Refusal::Provider => Behavior::Refuse,
                Refusal::Quotient => Behavior::QuotientBound,
                Refusal::Reconstruction => Behavior::BadReduction,
                _ => Behavior::Honest,
            };
            if matches!(case, Refusal::IdentityAnnouncement) {
                signature.announcement = JubjubPoint::default();
            }
            if matches!(case, Refusal::Signature) {
                signature.response = Field::from(0_u64);
            }
            let expected = match case {
                Refusal::Closed => r::CompactError::AssertionFailed("attestor is closed".into()),
                Refusal::Provider => {
                    r::CompactError::InvalidLedgerCell("reduction provider refused".into())
                }
                Refusal::Quotient => {
                    r::CompactError::AssertionFailed("Schnorr quotient out of range".into())
                }
                Refusal::Reconstruction => {
                    r::CompactError::AssertionFailed("Invalid challenge reduction".into())
                }
                Refusal::IdentityAnnouncement | Refusal::IdentityKey => {
                    r::CompactError::AssertionFailed(
                        "Schnorr verification requires a non-identity key and announcement".into(),
                    )
                }
                _ => r::CompactError::AssertionFailed("Invalid Jubjub Schnorr signature".into()),
            };
            let mut reference_events = None;
            for lane in 0..4 {
                let provider = Provider {
                    behavior,
                    inspect_counter: true,
                    ..Default::default()
                };
                let actual = match lane {
                    0 => root
                        .native(&state, &provider, digest.clone(), signature.clone())
                        .err()
                        .unwrap(),
                    1 => root
                        .recorded(&state, &provider, digest.clone(), signature.clone())
                        .err()
                        .unwrap(),
                    2 => root
                        .observed(&state, &provider, digest.clone(), signature.clone())
                        .err()
                        .unwrap(),
                    _ => {
                        manual::execute(root, &state, &provider, digest.clone(), signature.clone())
                            .err()
                            .unwrap()
                    }
                };
                assert_eq!(actual, expected, "case {case:?}, lane {lane}");
                let events = provider.events.borrow();
                assert_eq!(events.len(), usize::from(!closed));
                if let Some(event) = events.first() {
                    assert_eq!(event.private, root.private());
                    assert_eq!(
                        event.count,
                        Some(0),
                        "counter must follow successful helper"
                    );
                }
                if let Some(reference) = &reference_events {
                    assert_eq!(&*events, reference);
                } else {
                    reference_events = Some(events.clone());
                }
            }
            assert_eq!(
                slots::acceptedCount
                    .inspect(state.contract().data.get_ref())
                    .unwrap(),
                0
            );
        }
    }
}
