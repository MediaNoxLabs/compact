// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use compact_rust_empty_recording_witness_source_fixture::{ledger_contract as c, types};
use midnight_compact_runtime as r;
use midnight_onchain_state::state::{ContractOperation, EntryPointBuf};
use midnight_serialize::{Deserializable, Serializable};
use r::context::ConstructorContext;
use r::transaction::{Observation, ObservedContractState, VerifierKey};
pub fn uint(n: u128) -> r::BoundedUint<{ u64::MAX as u128 }> {
    r::BoundedUint::new(n).unwrap()
}
pub fn args(enforce: bool) -> (types::VerifierPolicy, types::Attestation) {
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
pub fn setup() -> (ObservedContractState, VerifierKey) {
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
