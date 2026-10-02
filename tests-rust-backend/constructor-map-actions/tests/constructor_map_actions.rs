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

use compact_rust_constructor_map_actions_fixture::ledger_contract::{
    Contract, get_false_history, get_true, history_size, initial_state, table_size,
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
    let mut operations: HashMap<EntryPointBuf, ContractOperation, DefaultDB> = HashMap::new();
    for name in [
        "table_size",
        "history_size",
        "get_true",
        "get_false_history",
    ] {
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
fn constructor_map_actions_match_typescript_state() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/constructor-map-actions.json"
    ))
    .unwrap();
    let constructor = initial_state(ConstructorContext::new(())).unwrap();
    assert_eq!(
        state_hex(constructor.ledger_state.get_ref().clone()),
        oracle["initialHex"]
    );
    let table = table_size(constructor.into_circuit_context(ContractAddress::default())).unwrap();
    assert_eq!(table.result.value().to_string(), oracle["tableSize"]);
    let history = history_size(table.context).unwrap();
    assert_eq!(history.result.value().to_string(), oracle["historySize"]);
    let table_value = get_true(history.context).unwrap();
    assert_eq!(table_value.result, Field::from(1_u64));
    assert_eq!(oracle["tableValue"], "1");
    let history_value = get_false_history(table_value.context).unwrap();
    assert_eq!(history_value.result, Field::from(0_u64));
    assert_eq!(oracle["historyValue"], "0");
}

#[test]
fn constructor_map_lookup_has_a_replayable_generated_call() {
    let context = initial_state(ConstructorContext::new(()))
        .unwrap()
        .into_circuit_context(ContractAddress::default());
    let native_context = initial_state(ConstructorContext::new(()))
        .unwrap()
        .into_circuit_context(ContractAddress::default());
    let native = get_true(native_context).unwrap();
    let recorded = Contract::default().recording.get_true(context).unwrap();
    assert_eq!(recorded.execution.result, Field::from(1_u64));
    assert_eq!(recorded.execution.result, native.result);
    assert_eq!(recorded.execution.gas_cost, native.gas_cost);
    let replay = recorded
        .public
        .initial()
        .query(
            recorded.public.verify_ops(),
            None,
            &recorded.execution.context.cost_model,
        )
        .unwrap();
    assert_eq!(replay.context.effects, native.context.query.effects);
}
