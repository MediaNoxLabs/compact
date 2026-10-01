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

use compact_rust_constructor_set_actions_fixture::ledger_contract::{
    contains_history, contains_seen, initial_state,
};
use midnight_compact_runtime::context::ConstructorContext;
use midnight_compact_runtime::ledger::{ContractAddress, DefaultDB, StateValue};
use midnight_onchain_state::state::{
    ContractMaintenanceAuthority, ContractOperation, ContractState, EntryPointBuf,
};
use midnight_serialize::tagged_serialize;
use midnight_storage::storage::HashMap;

fn state_hex(state: StateValue<DefaultDB>) -> String {
    let mut operations: HashMap<EntryPointBuf, ContractOperation, DefaultDB> = HashMap::new();
    for name in ["contains_seen", "contains_history"] {
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
fn constructor_set_actions_match_typescript_state() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/constructor-set-actions.json"
    ))
    .unwrap();
    let constructor = initial_state(ConstructorContext::new(())).unwrap();
    let StateValue::Array(fields) = constructor.ledger_state.get_ref() else {
        panic!("expected ledger field array")
    };
    for index in [0, 1] {
        let StateValue::Map(set) = fields.get(index).unwrap() else {
            panic!("expected Set map")
        };
        assert_eq!(set.size(), 1);
    }
    assert_eq!(
        state_hex(constructor.ledger_state.get_ref().clone()),
        oracle["initialHex"]
    );
    let seen_true = contains_seen(
        constructor.into_circuit_context(ContractAddress::default()),
        true,
    )
    .unwrap();
    assert_eq!(seen_true.result, oracle["seenTrue"]);
    let seen_false = contains_seen(seen_true.context, false).unwrap();
    assert_eq!(seen_false.result, oracle["seenFalse"]);
    let history_true = contains_history(seen_false.context, true).unwrap();
    assert_eq!(history_true.result, oracle["historyTrue"]);
    let history_false = contains_history(history_true.context, false).unwrap();
    assert_eq!(history_false.result, oracle["historyFalse"]);
}
