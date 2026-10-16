// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use compact_rust_effect_plan_maps_source_fixture::{
    ledger_contract as c, ledger_slots as slots, runtime as r,
};
use midnight_onchain_state::state::{ContractOperation, EntryPointBuf};
use r::context::ConstructorContext;
use r::transaction::{Observation, ObservedContractState, VerifierKey};
use rand::{Rng, SeedableRng, rngs::StdRng};
pub fn setup() -> (ObservedContractState, VerifierKey) {
    let key: VerifierKey = StdRng::seed_from_u64(579).r#gen();
    let mut operations = midnight_storage::storage::HashMap::new();
    for name in [
        "member",
        "lookup",
        "put",
        "remove",
        "lazy_put",
        "direct_member",
        "direct_lookup",
    ] {
        operations = operations.insert(
            EntryPointBuf(name.as_bytes().to_vec()),
            ContractOperation::new(Some(key.clone())),
        );
    }
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
pub fn inspect<T>(
    result: &r::context::CircuitResult<u64, T>,
    expected: &[(&str, u64)],
    calls: u64,
) {
    let state = result.context.query.state.get_ref();
    let table = slots::table.inspect(state).unwrap();
    assert_eq!(table.size().unwrap().value(), expected.len() as u128);
    for (key, value) in expected {
        assert_eq!(table.lookup((*key).into()).unwrap(), r::Field::from(*value));
    }
    assert_eq!(slots::calls.inspect(state).unwrap(), calls);
}
