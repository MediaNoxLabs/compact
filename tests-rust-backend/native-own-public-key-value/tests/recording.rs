// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
#![cfg(feature = "ledger-transaction")]

use compact_rust_native_own_public_key_value_fixture::{ledger_contract as c, runtime as r};
use midnight_onchain_state::state::{ContractOperation, EntryPointBuf};
use midnight_serialize::{Deserializable, Serializable};
use r::context::{CircuitResult, ConstructorContext, RunningCost};
use r::recording::RecordedCircuitResult;
use r::transaction::{
    Observation, ObservedCallError, ObservedContractState, PrepareCallError, VerifierKey,
};

fn setup(identity: Option<[u8; 32]>) -> (ObservedContractState, VerifierKey) {
    // Empty transcript refusal happens before proof-key use. This serialized
    // empty carrier is deliberately not a key suitable for producing proofs.
    let mut bytes = Vec::new();
    Vec::<u8>::new().serialize(&mut bytes).unwrap();
    let verifier = VerifierKey::deserialize(&mut bytes.as_slice(), 0).unwrap();
    let mut operations = midnight_storage::storage::HashMap::new();
    for name in ["key", "key_bytes"] {
        operations = operations.insert(
            EntryPointBuf(name.as_bytes().to_vec()),
            ContractOperation::new(Some(verifier.clone())),
        );
    }
    let initial = c::initial_state(ConstructorContext::new(())).unwrap();
    let state = ObservedContractState::new(
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
    );
    let state = match identity {
        Some(bytes) => {
            state.with_coin_public_key(r::ledger::CoinPublicKey(r::ledger::HashOutput(bytes)))
        }
        None => state,
    };
    (state, verifier)
}

fn compare<T: std::fmt::Debug + PartialEq>(
    native: &CircuitResult<(), T>,
    recorded: &RecordedCircuitResult<(), T>,
    key: [u8; 32],
) {
    let actual = &recorded.execution;
    assert_eq!(actual.result, native.result);
    assert_eq!(actual.context.private_state, ());
    assert_eq!(actual.context.query.state, native.context.query.state);
    assert_eq!(actual.context.query.effects, native.context.query.effects);
    assert_eq!(actual.gas_cost, RunningCost::ZERO);
    assert_eq!(actual.gas_cost, native.gas_cost);
    assert_eq!(
        actual.private_transcript_outputs,
        native.private_transcript_outputs
    );
    assert_eq!(actual.private_transcript_outputs.len(), 1);
    let mut key_atom = key.to_vec();
    while key_atom.last() == Some(&0) {
        key_atom.pop();
    }
    let output = &actual.private_transcript_outputs[0];
    assert_eq!(
        output
            .value
            .0
            .iter()
            .map(|atom| atom.0.clone())
            .collect::<Vec<_>>(),
        vec![key_atom]
    );
    assert_eq!(output.alignment, r::fab::AlignedValue::from(key).alignment);
    assert!(recorded.public.verify_ops().is_empty());
    assert_eq!(recorded.public.initial_coin_public_key(), Some(key));
    assert_eq!(recorded.public.final_coin_public_key(), Some(key));
    assert_eq!(recorded.public.initial().state, actual.context.query.state);
    let replay = recorded
        .public
        .initial()
        .query(
            recorded.public.verify_ops(),
            None,
            &actual.context.cost_model,
        )
        .unwrap();
    assert_eq!(replay.context.state, actual.context.query.state);
    assert_eq!(replay.context.effects, actual.context.query.effects);
    // Invoking the official VM query has overhead even for an empty program.
    // Native/recorded execution gas is checked above; the TS oracle does not
    // measure this separate replay invocation.
}

fn oracle() -> serde_json::Value {
    serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/native-own-public-key-value.json"
    ))
    .unwrap()
}

fn oracle_key() -> [u8; 32] {
    oracle()["coinPublicKey"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| u8::try_from(v.as_u64().unwrap()).unwrap())
        .collect::<Vec<_>>()
        .try_into()
        .unwrap()
}

fn check_oracle<T>(recorded: &RecordedCircuitResult<(), T>, expected: &serde_json::Value) {
    assert_eq!(expected["proofRequired"], false);
    assert_eq!(expected["publicTranscript"], serde_json::json!([]));
    assert_eq!(expected["stateBefore"], expected["stateAfter"]);
    let output = &recorded.execution.private_transcript_outputs[0];
    assert_eq!(
        serde_json::to_value(
            output
                .value
                .0
                .iter()
                .map(|atom| &atom.0)
                .collect::<Vec<_>>()
        )
        .unwrap(),
        expected["privateTranscriptOutputs"][0]["valueAtoms"]
    );
    assert_eq!(
        serde_json::to_value(&output.alignment).unwrap(),
        expected["privateTranscriptOutputs"][0]["alignment"]
    );
}

#[test]
fn key_records_once_and_observes_identity_but_cannot_prepare_an_empty_program() {
    for identity in [[0; 32], [19; 32], oracle_key()] {
        let (state, verifier) = setup(Some(identity));
        let native = c::key(state.circuit_context(())).unwrap();
        assert_eq!(native.result.bytes.0, identity);
        let recorded = c::recorded::key(state.circuit_context(())).unwrap();
        compare(&native, &recorded, identity);
        let contract = c::Contract::default();
        let observed = contract.recording().key_call(&state, ()).unwrap();
        compare(&native, observed.recorded(), identity);
        if identity == oracle_key() {
            check_oracle(&recorded, &oracle()["key"]);
            check_oracle(observed.recorded(), &oracle()["key"]);
        }
        assert!(matches!(
            observed.prepare(verifier, r::Field::from(0_u64)),
            Err(ObservedCallError::Prepare(
                PrepareCallError::EmptyTranscript
            ))
        ));
    }
}

#[test]
fn key_missing_identity_is_not_a_zero_key() {
    let (state, _) = setup(None);
    assert_eq!(
        c::recorded::key(state.circuit_context(())).err().unwrap(),
        r::CompactError::MissingCoinPublicKey
    );
    let contract = c::Contract::default();
    assert_eq!(
        contract.recording().key_call(&state, ()).err().unwrap(),
        r::CompactError::MissingCoinPublicKey
    );
}

#[test]
fn key_bytes_records_once_and_observes_identity_but_cannot_prepare_an_empty_program() {
    for identity in [[0; 32], [19; 32], oracle_key()] {
        let (state, verifier) = setup(Some(identity));
        let native = c::key_bytes(state.circuit_context(())).unwrap();
        assert_eq!(native.result.0, identity);
        let recorded = c::recorded::key_bytes(state.circuit_context(())).unwrap();
        compare(&native, &recorded, identity);
        let contract = c::Contract::default();
        let observed = contract.recording().key_bytes_call(&state, ()).unwrap();
        compare(&native, observed.recorded(), identity);
        if identity == oracle_key() {
            check_oracle(&recorded, &oracle()["keyBytes"]);
            check_oracle(observed.recorded(), &oracle()["keyBytes"]);
        }
        assert!(matches!(
            observed.prepare(verifier, r::Field::from(0_u64)),
            Err(ObservedCallError::Prepare(
                PrepareCallError::EmptyTranscript
            ))
        ));
    }
}

#[test]
fn key_bytes_missing_identity_is_not_a_zero_key() {
    let (state, _) = setup(None);
    assert_eq!(
        c::recorded::key_bytes(state.circuit_context(()))
            .err()
            .unwrap(),
        r::CompactError::MissingCoinPublicKey
    );
    let contract = c::Contract::default();
    assert_eq!(
        contract
            .recording()
            .key_bytes_call(&state, ())
            .err()
            .unwrap(),
        r::CompactError::MissingCoinPublicKey
    );
}
