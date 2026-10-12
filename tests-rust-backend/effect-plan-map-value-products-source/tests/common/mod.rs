// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use compact_rust_effect_plan_map_value_products_source_fixture::{
    ledger_contract as c, runtime as r, types,
};
use midnight_onchain_state::state::{
    ContractMaintenanceAuthority, ContractOperation, EntryPointBuf,
};
use r::context::{CircuitResult, ConstructorContext};
use r::recording::RecordedCircuitResult;
use r::transaction::{Observation, ObservedContractState, VerifierKey};
use rand::{Rng, SeedableRng, rngs::StdRng};
pub fn setup(value: types::Stored) -> (ObservedContractState, VerifierKey) {
    let initial = c::initial_state(ConstructorContext::new(7_u64), "map-λ".into(), value).unwrap();
    let verifier: VerifierKey = StdRng::seed_from_u64(595).r#gen();
    let operations = midnight_storage::storage::HashMap::new().insert(
        EntryPointBuf(b"readFresh".to_vec()),
        ContractOperation::new(Some(verifier.clone())),
    );
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
        verifier,
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
    let replay = manual
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
        let actual_replay = actual
            .public
            .initial()
            .query(
                actual.public.verify_ops(),
                None,
                &actual.execution.context.cost_model,
            )
            .unwrap();
        assert_eq!(actual_replay.context.state, native.context.query.state);
        assert_eq!(actual_replay.context.effects, native.context.query.effects);
        assert_eq!(actual_replay.gas_cost, replay.gas_cost);
    }
}
