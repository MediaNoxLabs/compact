// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//   http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use compact_contract_observed_composite_keys::ledger_contract::{Contract, initial_state};
use compact_contract_observed_composite_keys::runtime::context::ConstructorContext;
use compact_contract_observed_composite_keys::runtime::ledger::{
    ContractAddress, ContractState, DefaultDB,
};
use compact_contract_observed_composite_keys::runtime::transaction::{
    Observation, ObservedCallError, ObservedContractState, decode_verifier_key,
};
use compact_contract_observed_composite_keys::runtime::{Field, FixedVector};
use compact_contract_observed_composite_keys::types::CompositeKey;

#[test]
fn vector_tuple_and_struct_calls_need_only_the_generated_crate() {
    let initial = initial_state(ConstructorContext::new(())).unwrap();
    let mut state = ContractState::<DefaultDB>::default();
    state.data = initial.ledger_state;
    let observed = ObservedContractState::new(
        ContractAddress::default(),
        state,
        Observation {
            transaction_hash: [1; 32],
            block_hash: [2; 32],
            block_height: 3,
        },
    );
    let contract = Contract::default();
    let vector = FixedVector::new([Field::from(3_u64), Field::from(5_u64)]);
    let pair = (Field::from(42_u64), true);
    let struct_key = CompositeKey {
        vector: vector.clone(),
        pair,
    };
    let calls = [
        (
            "insert_vector",
            contract
                .recording
                .insert_vector_call(&observed, (), vector.clone())
                .unwrap(),
        ),
        (
            "insert_tuple",
            contract
                .recording
                .insert_tuple_call(&observed, (), pair)
                .unwrap(),
        ),
        (
            "insert_struct",
            contract
                .recording
                .insert_struct_call(&observed, (), struct_key.clone())
                .unwrap(),
        ),
        (
            "record_twelve",
            contract
                .recording
                .record_twelve_call(
                    &observed,
                    (),
                    1_u64.into(),
                    2_u64.into(),
                    3_u64.into(),
                    4_u64.into(),
                    5_u64.into(),
                    6_u64.into(),
                    7_u64.into(),
                    8_u64.into(),
                    9_u64.into(),
                    10_u64.into(),
                    11_u64.into(),
                    12_u64.into(),
                )
                .unwrap(),
        ),
    ];
    let root = std::env::var("COMPACT_RUST_COMPOSITE_KEY_PROOF").unwrap();
    for (name, call) in calls {
        let verifier =
            decode_verifier_key(&std::fs::read(format!("{root}/keys/{name}.verifier")).unwrap())
                .unwrap();
        assert!(matches!(
            call.prepare(verifier, Field::from(0_u64)),
            Err(ObservedCallError::MissingOperation(actual)) if actual == name
        ));
    }
    let roundtrips = [
        (
            "roundtrip_tuple",
            contract
                .recording
                .roundtrip_tuple_call(&observed, (), pair)
                .unwrap(),
        ),
        (
            "roundtrip_struct",
            contract
                .recording
                .roundtrip_struct_call(&observed, (), struct_key)
                .unwrap(),
        ),
    ];
    for (name, call) in roundtrips {
        let verifier =
            decode_verifier_key(&std::fs::read(format!("{root}/keys/{name}.verifier")).unwrap())
                .unwrap();
        assert!(matches!(
            call.prepare(verifier, Field::from(0_u64)),
            Err(ObservedCallError::MissingOperation(actual)) if actual == name
        ));
    }
}
