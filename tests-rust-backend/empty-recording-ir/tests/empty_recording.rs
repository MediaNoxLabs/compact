// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
#![cfg(feature = "ledger-transaction")]
use compact_rust_empty_recording_ir_fixture::{ledger_contract as c, types};
use midnight_compact_runtime as r;
use midnight_onchain_state::state::{ContractOperation, EntryPointBuf};
use midnight_serialize::{Deserializable, Serializable};
use r::context::ConstructorContext;
use r::transaction::{
    Observation, ObservedCallError, ObservedContractState, PrepareCallError, VerifierKey,
};
fn uint(n: u128) -> r::BoundedUint<{ u64::MAX as u128 }> {
    r::BoundedUint::new(n).unwrap()
}
fn args(enforce: bool) -> (types::VerifierPolicy, types::Attestation) {
    (
        types::VerifierPolicy {
            enforceMaxAge: enforce,
            maxAge: uint(20),
        },
        types::Attestation {
            proof: types::StatusProof {
                createdAt: uint(100),
                issuer: uint(1),
            },
            hasExpiration: false,
            expiresAt: uint(0),
        },
    )
}
fn setup() -> (ObservedContractState, VerifierKey) {
    // Empty-program preparation rejects before inspecting proof material. This
    // serialized empty carrier is installed for observation identity, not a key.
    let mut bytes = Vec::new();
    Vec::<u8>::new().serialize(&mut bytes).unwrap();
    let key = VerifierKey::deserialize(&mut bytes.as_slice(), 0).unwrap();
    let initial = c::initial_state(ConstructorContext::new(7_u64)).unwrap();
    let operations = r::ledger::LedgerHashMap::new().insert(
        EntryPointBuf(b"recordFreshEnough".to_vec()),
        ContractOperation::new(Some(key.clone())),
    );
    (
        ObservedContractState::new(
            Default::default(),
            r::ledger::ContractState::new(
                initial.ledger_state.get_ref().clone(),
                operations,
                Default::default(),
            ),
            Observation {
                transaction_hash: [0; 32],
                block_hash: [0; 32],
                block_height: 0,
            },
        ),
        key,
    )
}
#[test]
fn empty_public_recording_preserves_execution_but_cannot_prepare() {
    let (observed, key) = setup();
    for (enforce, now) in [(true, 110), (false, 130)] {
        let (policy, attestation) = args(enforce);
        let native = c::recordFreshEnough(
            observed.circuit_context(7),
            policy.clone(),
            attestation.clone(),
            uint(now),
        )
        .unwrap();
        let recorded = c::recorded::recordFreshEnough(
            observed.circuit_context(7),
            policy.clone(),
            attestation.clone(),
            uint(now),
        )
        .unwrap();
        let call = c::recorded::Contract
            .recordFreshEnough_call(&observed, 7, policy.clone(), attestation.clone(), uint(now))
            .unwrap();
        let manual = r::recording::RecordingFrame::new(observed.circuit_context(7)).finish(());
        assert_eq!(
            native.context.query.state.get_ref(),
            observed.contract().data.get_ref()
        );
        assert_eq!(native.gas_cost, Default::default());
        assert_eq!(native.context.private_state, 7);
        assert!(native.private_transcript_outputs.is_empty());
        for actual in [&recorded, call.recorded(), &manual] {
            assert!(actual.public.verify_ops().is_empty());
            assert_eq!(
                actual.execution.context.query.state,
                native.context.query.state
            );
            assert_eq!(
                actual.execution.context.query.effects,
                native.context.query.effects
            );
            assert_eq!(actual.execution.gas_cost, native.gas_cost);
            assert_eq!(actual.execution.context.private_state, 7);
            assert!(actual.execution.private_transcript_outputs.is_empty());
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
fn empty_trace_does_not_erase_guard_failures() {
    let (observed, _) = setup();
    for (now, message) in [
        (90, "attestation creation time cannot be in the future"),
        (130, "attestation exceeds the max-age policy"),
    ] {
        let (policy, attestation) = args(true);
        let errors = [
            c::recordFreshEnough(
                observed.circuit_context(7),
                policy.clone(),
                attestation.clone(),
                uint(now),
            )
            .err()
            .unwrap(),
            c::recorded::recordFreshEnough(
                observed.circuit_context(7),
                policy.clone(),
                attestation.clone(),
                uint(now),
            )
            .err()
            .unwrap(),
            c::recorded::Contract
                .recordFreshEnough_call(&observed, 7, policy, attestation, uint(now))
                .err()
                .unwrap(),
        ];
        for error in errors {
            assert_eq!(error, r::CompactError::AssertionFailed(message.into()));
        }
    }
}
