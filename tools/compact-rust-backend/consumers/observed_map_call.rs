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

use compact_contract_map_boolean_field::ledger_contract::{Contract, initial_state};
use compact_contract_map_boolean_field::runtime::context::ConstructorContext;
use compact_contract_map_boolean_field::runtime::ledger::{
    ContractAddress, ContractState, DefaultDB,
};
use compact_contract_map_boolean_field::runtime::transaction::{
    Observation, ObservedCallError, ObservedContractState, decode_verifier_key,
};
use compact_contract_map_boolean_field::runtime::Field;

#[test]
fn two_argument_observed_call_uses_only_the_generated_crate() {
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
    let verifier = decode_verifier_key(
        &std::fs::read(std::env::var("COMPACT_RUST_OBSERVED_MAP_VERIFIER").unwrap()).unwrap(),
    )
    .unwrap();
    let call = Contract::default()
        .recording
        .put_call(&observed, (), true, Field::from(42_u64))
        .unwrap();
    assert!(matches!(
        call.prepare(verifier, Field::from(0_u64)),
        Err(ObservedCallError::MissingOperation(name)) if name == "put"
    ));
}
