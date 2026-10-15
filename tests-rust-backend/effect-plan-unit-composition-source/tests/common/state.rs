// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use compact_rust_effect_plan_unit_composition_source_fixture::{
    ledger_contract as c, runtime as r,
};
use midnight_onchain_state::state::{ContractOperation, EntryPointBuf};
use r::context::ConstructorContext;
use r::transaction::{Observation, ObservedContractState, VerifierKey};
use rand::{Rng, SeedableRng, rngs::StdRng};
pub fn setup() -> (ObservedContractState, VerifierKey) {
    let key: VerifierKey = StdRng::seed_from_u64(580).r#gen();
    let operations = midnight_storage::storage::HashMap::new().insert(
        EntryPointBuf(b"run".to_vec()),
        ContractOperation::new(Some(key.clone())),
    );
    let context = c::initial_state(ConstructorContext::new(5_u64), "seed".into())
        .unwrap()
        .into_circuit_context(Default::default());
    (
        ObservedContractState::new(
            Default::default(),
            r::ledger::ContractState::new(
                context.query.state.get_ref().clone(),
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
