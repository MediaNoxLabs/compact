// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Shared assertions for witness-only recorded calls. No proof is attempted.
use midnight_compact_runtime as r;
use midnight_onchain_state::state::{ContractOperation, EntryPointBuf};
use midnight_serialize::{Deserializable, Serializable};
use r::transaction::{
    Observation, ObservedCallError, ObservedContractState, PrepareCallError, RecordedCall,
    VerifierKey,
};

pub fn observation(
    state: r::ledger::StateValue<r::ledger::DefaultDB>,
    names: &[&str],
) -> (ObservedContractState, VerifierKey) {
    // An empty serialized verifier carrier reaches preparation policy only.
    let mut bytes = Vec::new();
    Vec::<u8>::new().serialize(&mut bytes).unwrap();
    let key = VerifierKey::deserialize(&mut bytes.as_slice(), 0).unwrap();
    let mut operations = r::ledger::LedgerHashMap::new();
    for name in names {
        operations = operations.insert(
            EntryPointBuf(name.as_bytes().to_vec()),
            ContractOperation::new(Some(key.clone())),
        );
    }
    (
        ObservedContractState::new(
            Default::default(),
            r::ledger::ContractState::new(state, operations, Default::default()),
            Observation {
                transaction_hash: [0; 32],
                block_hash: [0; 32],
                block_height: 0,
            },
        ),
        key,
    )
}

pub fn parity<P: std::fmt::Debug + PartialEq, T: std::fmt::Debug + PartialEq>(
    actual: &r::recording::RecordedCircuitResult<P, T>,
    native: &r::context::CircuitResult<P, T>,
) {
    assert_eq!(actual.execution.result, native.result);
    assert_eq!(
        actual.execution.context.private_state,
        native.context.private_state
    );
    assert_eq!(
        actual.execution.private_transcript_outputs,
        native.private_transcript_outputs
    );
    assert_eq!(
        actual.execution.context.query.state,
        native.context.query.state
    );
    assert_eq!(
        actual.execution.context.query.effects,
        native.context.query.effects
    );
    assert_eq!(actual.execution.gas_cost, native.gas_cost);
    assert!(actual.public.verify_ops().is_empty());
    assert_eq!(actual.public.initial().state, native.context.query.state);
    assert_eq!(native.gas_cost, r::context::RunningCost::default());
    let replay = actual
        .public
        .initial()
        .query(actual.public.verify_ops(), None, &native.context.cost_model)
        .unwrap();
    assert_eq!(replay.context.state, native.context.query.state);
    assert_eq!(replay.context.effects, native.context.query.effects);
}

pub fn private_oracle<P, T>(actual: &r::context::CircuitResult<P, T>, row: &serde_json::Value) {
    let outputs: Vec<_> = actual
        .private_transcript_outputs
        .iter()
        .map(|v| {
            let atoms: Vec<_> = v.value.0.iter().map(|a| &a.0).collect();
            serde_json::json!({"valueAtoms": atoms, "alignment": v.alignment})
        })
        .collect();
    assert_eq!(serde_json::json!(outputs), row["privateTranscriptOutputs"]);
}

pub fn refuses_empty<P, T: Into<r::fab::AlignedValue>>(
    call: RecordedCall<'_, P, T>,
    key: VerifierKey,
) {
    assert!(matches!(
        call.prepare(key, r::Field::from(0_u64)),
        Err(ObservedCallError::Prepare(
            PrepareCallError::EmptyTranscript
        ))
    ));
}
