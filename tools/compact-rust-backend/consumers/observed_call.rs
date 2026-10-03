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

use compact_contract_counter::ledger_contract::Contract;
use compact_contract_counter::runtime::ledger::ContractAddress;
use compact_contract_counter::runtime::transaction::{
    Observation, ObservedContractState, decode_verifier_key,
};
use compact_contract_counter::runtime::{Field, fab::AlignedValue};

#[test]
fn generated_crate_prepares_observed_call_without_extra_dependencies() {
    let state = std::fs::read(std::env::var("COMPACT_RUST_OBSERVED_STATE").unwrap()).unwrap();
    let key = std::fs::read(std::env::var("COMPACT_RUST_OBSERVED_VERIFIER").unwrap()).unwrap();
    let observed = ObservedContractState::decode(
        ContractAddress::default(),
        &state,
        Observation {
            transaction_hash: [1; 32],
            block_hash: [2; 32],
            block_height: 1,
        },
    )
    .unwrap();
    let verifier = decode_verifier_key(&key).unwrap();
    let call = Contract::default()
        .recording
        .increment_call(&observed, ())
        .unwrap()
        .prepare(verifier, Field::from(0u64))
        .unwrap();
    assert_eq!(call.entry_point.0, b"increment".to_vec());
    assert_eq!(call.input, AlignedValue::from(()));

    let mut trailing = key;
    trailing.push(0);
    assert!(decode_verifier_key(&trailing).is_err());
}
