// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
#![allow(dead_code)]
use compact_rust_opaque_cell_inputs_source_fixture::{ledger_contract as c, runtime as r, types};
use midnight_onchain_state::state::{
    ContractMaintenanceAuthority, ContractOperation, EntryPointBuf,
};
use r::context::{CircuitResult, ConstructorContext};
use r::recording::RecordedCircuitResult;
use r::transaction::{Observation, ObservedContractState, VerifierKey};
use rand::{Rng, SeedableRng, rngs::StdRng};

pub fn envelope(text: &str, bytes: &[u8]) -> types::Envelope {
    types::Envelope {
        payload: types::Payload {
            text: text.into(),
            bytes: bytes.to_vec().into(),
        },
        marker: r::Field::from(23_u64),
    }
}
pub fn setup() -> (ObservedContractState, VerifierKey) {
    let initial = c::initial_state(
        ConstructorContext::new(7_u64),
        "initial".into(),
        vec![1].into(),
        envelope("nested", &[2]),
    )
    .unwrap();
    let key: VerifierKey = StdRng::seed_from_u64(593).r#gen();
    let mut operations = midnight_storage::storage::HashMap::new();
    for name in ["setText", "setBytes", "setEnvelope"] {
        operations = operations.insert(
            EntryPointBuf(name.as_bytes().to_vec()),
            ContractOperation::new(Some(key.clone())),
        );
    }
    (
        ObservedContractState::new(
            Default::default(),
            r::ledger::ContractState::new(
                initial.ledger_state.get_ref().clone(),
                operations,
                ContractMaintenanceAuthority::default(),
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
pub fn compare(
    native: &CircuitResult<u64, ()>,
    recorded: &RecordedCircuitResult<u64, ()>,
    observed: &RecordedCircuitResult<u64, ()>,
    manual: &RecordedCircuitResult<u64, ()>,
) {
    assert_eq!(native.context.private_state, 7);
    assert!(native.private_transcript_outputs.is_empty());
    let expected_replay = manual
        .public
        .initial()
        .query(
            manual.public.verify_ops(),
            None,
            &manual.execution.context.cost_model,
        )
        .unwrap();
    for actual in [recorded, observed, manual] {
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
        assert_eq!(actual.public.verify_ops(), manual.public.verify_ops());
        let replay = actual
            .public
            .initial()
            .query(
                actual.public.verify_ops(),
                None,
                &actual.execution.context.cost_model,
            )
            .unwrap();
        assert_eq!(replay.context.state, native.context.query.state);
        assert_eq!(replay.context.effects, native.context.query.effects);
        assert_eq!(replay.gas_cost, expected_replay.gas_cost);
    }
}
pub fn atoms(value: &r::fab::AlignedValue) -> Vec<Vec<u8>> {
    value.value.0.iter().map(|a| a.0.clone()).collect()
}
pub fn invalid() -> r::CompactError {
    r::CompactError::InvalidLedgerCell("opaque ledger input has trailing zero bytes".into())
}
