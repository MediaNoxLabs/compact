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

use compact_rust_constructor_list_actions_fixture::ledger_contract::{
    Contract, drop_first, history_count, initial_state, item_count,
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
    for name in ["item_count", "history_count"] {
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
fn constructor_list_actions_match_typescript_state() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/constructor-list-actions.json"
    ))
    .unwrap();
    let constructor = initial_state(ConstructorContext::new(())).unwrap();
    assert_eq!(
        state_hex(constructor.ledger_state.get_ref().clone()),
        oracle["initialHex"]
    );
    let items = item_count(constructor.into_circuit_context(ContractAddress::default())).unwrap();
    assert_eq!(items.result.value().to_string(), oracle["itemCount"]);
    let history = history_count(items.context).unwrap();
    assert_eq!(history.result.value().to_string(), oracle["historyCount"]);
    assert_eq!(history.result.value(), 1);
}

#[test]
fn constructor_list_head_and_drop_recording_replay() {
    let initial = initial_state(ConstructorContext::new(())).unwrap();
    let context = initial.into_circuit_context(ContractAddress::default());
    let head = Contract::default().recording.first_item(context).unwrap();
    assert!(head.execution.result.is_some);
    assert_eq!(head.execution.result.value, Field::from(1_u64));
    let replay = head
        .public
        .initial()
        .query(
            head.public.verify_ops(),
            None,
            &head.execution.context.cost_model,
        )
        .unwrap();
    assert_eq!(replay.context.effects, head.execution.context.query.effects);

    let native = drop_first(
        initial_state(ConstructorContext::new(()))
            .unwrap()
            .into_circuit_context(ContractAddress::default()),
    )
    .unwrap();
    let recorded = Contract::default()
        .recording
        .drop_first(head.execution.context)
        .unwrap();
    assert_eq!(
        native.context.query.state.get_ref(),
        recorded.execution.context.query.state.get_ref()
    );
    let replay = recorded
        .public
        .initial()
        .query(
            recorded.public.verify_ops(),
            None,
            &recorded.execution.context.cost_model,
        )
        .unwrap();
    assert_eq!(
        replay.context.effects,
        recorded.execution.context.query.effects
    );
}
