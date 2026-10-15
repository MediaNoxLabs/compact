// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Full-IR compatibility variant, not a fresh source or new algebra-domain claim.
#![cfg(feature = "ledger-transaction")]
#![allow(
    clippy::type_complexity,
    reason = "This generated compatibility fixture deliberately exercises tuple9 and vector-of-tuple9 public types."
)]
include!(concat!(env!("OUT_DIR"), "/tuple.rs"));
use ledger_contract as c;
use midnight_onchain_state::state::ContractMaintenanceAuthority;
use r::context::ConstructorContext;
use r::ledger::ContractState;
use r::transaction::{Observation, ObservedContractState};
use runtime as r;

type Nine = (
    r::OpaqueBytes,
    r::Field,
    r::Field,
    r::Field,
    r::Field,
    r::Field,
    r::Field,
    r::Field,
    r::Field,
);
fn tuple(bytes: &[u8]) -> Nine {
    (
        bytes.to_vec().into(),
        r::Field::from(1_u64),
        r::Field::from(2_u64),
        r::Field::from(3_u64),
        r::Field::from(4_u64),
        r::Field::from(5_u64),
        r::Field::from(6_u64),
        r::Field::from(7_u64),
        r::Field::from(8_u64),
    )
}
fn observed() -> ObservedContractState {
    let state = c::initial_state(ConstructorContext::new(()))
        .unwrap()
        .into_circuit_context(Default::default())
        .query
        .state
        .get_ref()
        .clone();
    ObservedContractState::new(
        Default::default(),
        ContractState::new(
            state,
            midnight_storage::storage::HashMap::new(),
            ContractMaintenanceAuthority::default(),
        ),
        Observation {
            transaction_hash: [0; 32],
            block_hash: [0; 32],
            block_height: 0,
        },
    )
}
fn coin() -> types::QualifiedShieldedCoinInfo {
    types::QualifiedShieldedCoinInfo {
        nonce: r::FixedBytes::new([3; 32]),
        color: r::FixedBytes::new([5; 32]),
        value: r::BoundedUint::new(42).unwrap(),
        mt_index: r::BoundedUint::new(0).unwrap(),
    }
}
fn recipient() -> types::ZswapCoinPublicKey {
    types::ZswapCoinPublicKey {
        bytes: r::FixedBytes::new([7; 32]),
    }
}
fn call_error(first: Nine, vector: [Nine; 2]) -> r::CompactError {
    c::recorded::Contract
        .send_to_user_call(
            &observed(),
            (),
            coin(),
            recipient(),
            r::BoundedUint::new(1).unwrap(),
            first,
            r::FixedVector::new(vector),
        )
        .err()
        .unwrap()
}
#[test]
fn tuple9_and_vector_of_tuple9_validate_leaves_without_cellvalue_tuple_bound() {
    let invalid =
        r::CompactError::InvalidLedgerCell("opaque ledger input has trailing zero bytes".into());
    assert_eq!(
        call_error(tuple(&[1, 0]), [tuple(&[1]), tuple(&[2])]),
        invalid
    );
    assert_eq!(
        call_error(tuple(&[1]), [tuple(&[1, 0]), tuple(&[2])]),
        invalid
    );
    assert_eq!(
        call_error(tuple(&[1]), [tuple(&[1]), tuple(&[2, 0])]),
        invalid
    );
    // Observed contexts lock provisional output allocation. Canonical values
    // reach that existing body refusal; raw recorded execution with the same
    // context proves the error is independent of the new public-input guards.
    let direct = c::recorded::send_to_user(
        observed().circuit_context(()),
        coin(),
        recipient(),
        r::BoundedUint::new(1).unwrap(),
        tuple(&[0, 1]),
        r::FixedVector::new([tuple(&[]), tuple(&[2])]),
    )
    .err()
    .unwrap();
    assert_eq!(direct, r::CompactError::ZswapAllocationLocked);
    assert_eq!(
        call_error(tuple(&[0, 1]), [tuple(&[]), tuple(&[2])]),
        direct
    );
}
