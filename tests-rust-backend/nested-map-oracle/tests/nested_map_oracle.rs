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

use compact_rust_nested_map_oracle_fixture::ledger_contract::{initial_state, ping};
use midnight_compact_runtime::context::ConstructorContext;
use midnight_compact_runtime::ledger::{self, ContractAddress, DefaultDB, StateValue};
use midnight_onchain_state::state::{
    ContractMaintenanceAuthority, ContractOperation, ContractState, EntryPointBuf,
};
use midnight_serialize::tagged_serialize;
use midnight_storage::storage::HashMap;

fn state_hex(state: StateValue<DefaultDB>) -> String {
    let operations = HashMap::new().insert(
        EntryPointBuf(b"ping".to_vec()),
        ContractOperation::new(None),
    );
    let state = ContractState::new(state, operations, ContractMaintenanceAuthority::default());
    let mut bytes = Vec::new();
    tagged_serialize(&state, &mut bytes).unwrap();
    hex::encode(bytes)
}

fn assert_empty_nested_map(state: &StateValue<DefaultDB>) {
    let StateValue::Array(fields) = state else {
        panic!("expected public ledger array");
    };
    let StateValue::Map(map) = fields.get(1).unwrap() else {
        panic!("expected outer ledger Map");
    };
    assert_eq!(map.size(), 0);
}

#[test]
fn nested_map_initial_shape_and_cell_write_match_typescript() {
    let reference: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/nested-map-oracle.json"
    ))
    .unwrap();
    let initial = initial_state(ConstructorContext::new(())).unwrap();
    let state = initial.ledger_state.get_ref().clone();
    assert_empty_nested_map(&state);
    assert_eq!(state_hex(state.clone()), reference["afterInit"]["stateHex"]);
    assert_eq!(
        ledger::read_root_cell::<bool, _>(&state, 0).unwrap(),
        reference["afterInit"]["flag"]
    );

    let next = ping(initial.into_circuit_context(ContractAddress::default())).unwrap();
    let state = next.context.query.state.get_ref().clone();
    assert_empty_nested_map(&state);
    assert_eq!(state_hex(state.clone()), reference["afterPing"]["stateHex"]);
    assert_eq!(
        ledger::read_root_cell::<bool, _>(&state, 0).unwrap(),
        reference["afterPing"]["flag"]
    );
}
