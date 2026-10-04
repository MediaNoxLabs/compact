// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//  	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use compact_rust_constructor_cell_literal_fixture::ledger_contract::{
    PublicStateView, initial_state, read_value,
};
use midnight_compact_runtime::Field;
use midnight_compact_runtime::context::ConstructorContext;
use midnight_compact_runtime::ledger::{ContractAddress, DefaultDB, StateValue};
use midnight_onchain_state::state::{
    ContractMaintenanceAuthority, ContractOperation, ContractState, EntryPointBuf,
};
use midnight_serialize::tagged_serialize;
use midnight_storage::storage::HashMap;

fn state_hex(state: StateValue<DefaultDB>) -> String {
    let operations: HashMap<EntryPointBuf, ContractOperation, DefaultDB> = HashMap::new().insert(
        EntryPointBuf(b"read_value".to_vec()),
        ContractOperation::new(None),
    );
    let contract_state =
        ContractState::new(state, operations, ContractMaintenanceAuthority::default());
    let mut bytes = Vec::new();
    tagged_serialize(&contract_state, &mut bytes).unwrap();
    hex::encode(bytes)
}

#[test]
fn literal_constructor_matches_typescript_state_and_read() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/constructor-cell-literal.json"
    ))
    .unwrap();
    let constructor = initial_state(ConstructorContext::new(())).unwrap();
    let public = PublicStateView::from(constructor.ledger_state.get_ref());
    assert_eq!(public.value().unwrap(), Field::from(7_u64));
    assert_eq!(
        state_hex(constructor.ledger_state.get_ref().clone()),
        oracle["initialHex"]
    );
    let result = read_value(constructor.into_circuit_context(ContractAddress::default())).unwrap();
    assert_eq!(
        result.result,
        Field::from(oracle["read"].as_str().unwrap().parse::<u64>().unwrap())
    );
}

#[cfg(feature = "ledger-transaction")]
#[test]
fn observed_contract_exposes_typed_public_value() {
    use midnight_compact_runtime::transaction::{Observation, ObservedContractState};

    let constructor = initial_state(ConstructorContext::new(())).unwrap();
    let operations: HashMap<EntryPointBuf, ContractOperation, DefaultDB> = HashMap::new();
    let contract = ContractState::new(
        constructor.ledger_state.get_ref().clone(),
        operations,
        ContractMaintenanceAuthority::default(),
    );
    let observed = ObservedContractState::new(
        ContractAddress::default(),
        contract,
        Observation {
            transaction_hash: [0; 32],
            block_hash: [0; 32],
            block_height: 0,
        },
    );
    assert_eq!(
        PublicStateView::from(&observed).value().unwrap(),
        Field::from(7_u64)
    );
    assert_eq!(
        PublicStateView::from(observed.contract()).value().unwrap(),
        Field::from(7_u64)
    );
}
