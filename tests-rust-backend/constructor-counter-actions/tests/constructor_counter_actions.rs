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

use compact_rust_constructor_counter_actions_fixture::ledger_contract::{
    initial_state, read_count, read_value,
};
use midnight_compact_runtime::context::ConstructorContext;
use midnight_compact_runtime::ledger::{ContractAddress, DefaultDB, StateValue, read_counter};
use midnight_compact_runtime::{BoundedUint, Field};
use midnight_onchain_state::state::{
    ContractMaintenanceAuthority, ContractOperation, ContractState, EntryPointBuf,
};
use midnight_serialize::tagged_serialize;
use midnight_storage::storage::HashMap;

fn state_hex(state: StateValue<DefaultDB>) -> String {
    let mut operations: HashMap<EntryPointBuf, ContractOperation, DefaultDB> = HashMap::new();
    for name in ["read_count", "read_value"] {
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
fn constructor_counter_steps_match_typescript_state() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/constructor-counter-actions.json"
    ))
    .unwrap();
    let constructor = initial_state(
        ConstructorContext::new(()),
        BoundedUint::<65535>::new(1).unwrap(),
    )
    .unwrap();
    let state = constructor.ledger_state.get_ref();
    let StateValue::Array(fields) = state else {
        panic!("expected ledger field array")
    };
    assert_eq!(read_counter(&fields.get(1).unwrap()).unwrap(), 4);
    assert_eq!(state_hex(state.clone()), oracle["initialHex"]);
    let read = read_count(constructor.into_circuit_context(ContractAddress::default())).unwrap();
    assert_eq!(read.result.value().to_string(), oracle["count"]);
    let value = read_value(read.context).unwrap();
    assert_eq!(value.result, Field::from(2_u64));
    assert_eq!(oracle["value"], "2");
}
