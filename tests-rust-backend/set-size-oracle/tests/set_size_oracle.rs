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

use compact_rust_set_size_oracle_fixture::ledger_contract::{
    check_map_empty, check_set_empty, initial_state,
};
use midnight_compact_runtime as runtime;
use midnight_onchain_state::state::{
    ContractMaintenanceAuthority, ContractOperation, ContractState, EntryPointBuf,
};
use midnight_serialize::tagged_serialize;
use midnight_storage::storage::HashMap;
use runtime::context::ConstructorContext;
use runtime::ledger::{ContractAddress, DefaultDB, StateValue};

fn state_hex(state: StateValue<DefaultDB>) -> String {
    let mut operations: HashMap<EntryPointBuf, ContractOperation, DefaultDB> = HashMap::new();
    for name in ["check_set_empty", "check_map_empty"] {
        operations = operations.insert(
            EntryPointBuf(name.as_bytes().to_vec()),
            ContractOperation::new(None),
        );
    }
    let contract_state =
        ContractState::new(state, operations, ContractMaintenanceAuthority::default());
    let mut bytes = Vec::new();
    tagged_serialize(&contract_state, &mut bytes).unwrap();
    hex::encode(bytes)
}

#[test]
fn exact_set_and_map_is_empty_oracle_matches_typescript_state() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/set-size-oracle.json"
    ))
    .unwrap();
    let initial = initial_state(ConstructorContext::new(())).unwrap();
    assert_eq!(
        state_hex(initial.ledger_state.get_ref().clone()),
        oracle["afterInit"]
    );
    let set = check_set_empty(initial.into_circuit_context(ContractAddress::default())).unwrap();
    assert_eq!(
        state_hex(set.context.query.state.get_ref().clone()),
        oracle["afterSetCheck"]
    );
    let set_flag =
        runtime::ledger::read_root_cell::<bool, _>(set.context.query.state.get_ref(), 0).unwrap();
    assert_eq!(set_flag, oracle["setFlag"]);
    let map = check_map_empty(set.context).unwrap();
    assert_eq!(
        state_hex(map.context.query.state.get_ref().clone()),
        oracle["afterMapCheck"]
    );
    let map_flag =
        runtime::ledger::read_root_cell::<bool, _>(map.context.query.state.get_ref(), 1).unwrap();
    assert_eq!(map_flag, oracle["mapFlag"]);
}
