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

use midnight_compact_runtime::Field;
use midnight_compact_runtime::ledger::{
    ChargedState, ContractAddress, DefaultDB, QueryContext, StateValue, constructor_list,
    constructor_map, constructor_set, insert_map, insert_set, pop_front_list, push_front_list,
    remove_map, remove_set, reset_list, reset_map, reset_set,
};
use midnight_onchain_state::state::{
    ContractMaintenanceAuthority, ContractOperation, ContractState, EntryPointBuf,
};
use midnight_onchain_vm::cost_model::INITIAL_COST_MODEL;
use midnight_serialize::tagged_serialize;
use midnight_storage::storage::HashMap;

fn expected_bytes(json: &str) -> Vec<u8> {
    let reference: serde_json::Value = serde_json::from_str(json).unwrap();
    hex::decode(reference["stateHex"].as_str().unwrap()).unwrap()
}

fn state_bytes(state: StateValue<DefaultDB>, entry_points: &[&str]) -> Vec<u8> {
    let mut operations: HashMap<EntryPointBuf, ContractOperation, DefaultDB> = HashMap::new();
    for name in entry_points {
        operations = operations.insert(
            EntryPointBuf(name.as_bytes().to_vec()),
            ContractOperation::new(None),
        );
    }
    let contract_state =
        ContractState::new(state, operations, ContractMaintenanceAuthority::default());
    let mut actual = Vec::new();
    tagged_serialize(&contract_state, &mut actual).unwrap();
    actual
}

fn context(fields: Vec<StateValue<DefaultDB>>) -> QueryContext<DefaultDB> {
    QueryContext::new(
        ChargedState::new(StateValue::Array(fields.into())),
        ContractAddress::default(),
    )
}

fn assert_sequence_step(
    reference: &serde_json::Value,
    index: usize,
    operation: &str,
    state: StateValue<DefaultDB>,
    entry_points: &[&str],
) {
    let step = &reference["steps"][index];
    assert_eq!(step["operation"], operation);
    let expected = hex::decode(step["stateHex"].as_str().unwrap()).unwrap();
    assert_eq!(
        state_bytes(state, entry_points),
        expected,
        "{operation} at step {index}"
    );
}

#[test]
fn set_insert_state_matches_typescript_transition_bytes() {
    let expected = expected_bytes(include_str!("fixtures/set-ts-after-add.json"));
    let result = insert_set(
        &context(vec![constructor_set(), constructor_set()]),
        0,
        true,
        None,
        &INITIAL_COST_MODEL,
    )
    .unwrap();
    let actual = state_bytes(
        result.context.state.get_ref().clone(),
        &[
            "add",
            "contains",
            "add_field",
            "contains_field",
            "remove",
            "seen_size",
            "seen_is_empty",
            "reset_fields",
        ],
    );
    assert_eq!(actual, expected);
}

#[test]
fn map_insert_state_matches_typescript_transition_bytes() {
    let expected = expected_bytes(include_str!("fixtures/map-ts-after-put.json"));
    let result = insert_map(
        &context(vec![constructor_map()]),
        0,
        true,
        Field::from(42_u64),
        None,
        &INITIAL_COST_MODEL,
    )
    .unwrap();
    let actual = state_bytes(
        result.context.state.get_ref().clone(),
        &[
            "put",
            "put_default",
            "has",
            "get",
            "remove_key",
            "table_size",
            "table_is_empty",
            "reset_table",
        ],
    );
    assert_eq!(actual, expected);
}

#[test]
fn list_prepend_state_matches_typescript_transition_bytes() {
    let expected = expected_bytes(include_str!("fixtures/list-ts-after-prepend.json"));
    let result = push_front_list(
        &context(vec![constructor_list()]),
        0,
        Field::from(42_u64),
        None,
        &INITIAL_COST_MODEL,
    )
    .unwrap();
    let actual = state_bytes(
        result.context.state.get_ref().clone(),
        &[
            "item_count",
            "items_empty",
            "first_item",
            "prepend",
            "drop_first",
            "clear_items",
        ],
    );
    assert_eq!(actual, expected);
}

#[test]
fn set_mutation_sequence_matches_typescript_state_bytes() {
    let reference: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/set-ts-sequence.json")).unwrap();
    let names = [
        "add",
        "contains",
        "add_field",
        "contains_field",
        "remove",
        "seen_size",
        "seen_is_empty",
        "reset_fields",
    ];
    let result = insert_set(
        &context(vec![constructor_set(), constructor_set()]),
        0,
        true,
        None,
        &INITIAL_COST_MODEL,
    )
    .unwrap();
    assert_sequence_step(
        &reference,
        0,
        "add",
        result.context.state.get_ref().clone(),
        &names,
    );
    let result = remove_set(&result.context, 0, true, None, &INITIAL_COST_MODEL).unwrap();
    assert_sequence_step(
        &reference,
        1,
        "remove",
        result.context.state.get_ref().clone(),
        &names,
    );
    let result = insert_set(
        &result.context,
        1,
        Field::from(42_u64),
        None,
        &INITIAL_COST_MODEL,
    )
    .unwrap();
    assert_sequence_step(
        &reference,
        2,
        "add_field",
        result.context.state.get_ref().clone(),
        &names,
    );
    let result = reset_set(&result.context, 1, None, &INITIAL_COST_MODEL).unwrap();
    assert_sequence_step(
        &reference,
        3,
        "reset_fields",
        result.context.state.get_ref().clone(),
        &names,
    );
}

#[test]
fn map_mutation_sequence_matches_typescript_state_bytes() {
    let reference: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/map-ts-sequence.json")).unwrap();
    let names = [
        "put",
        "put_default",
        "has",
        "get",
        "remove_key",
        "table_size",
        "table_is_empty",
        "reset_table",
    ];
    let result = insert_map(
        &context(vec![constructor_map()]),
        0,
        true,
        Field::from(42_u64),
        None,
        &INITIAL_COST_MODEL,
    )
    .unwrap();
    assert_sequence_step(
        &reference,
        0,
        "put",
        result.context.state.get_ref().clone(),
        &names,
    );
    let result = remove_map(&result.context, 0, true, None, &INITIAL_COST_MODEL).unwrap();
    assert_sequence_step(
        &reference,
        1,
        "remove_key",
        result.context.state.get_ref().clone(),
        &names,
    );
    let result = insert_map(
        &result.context,
        0,
        true,
        Field::from(0_u64),
        None,
        &INITIAL_COST_MODEL,
    )
    .unwrap();
    assert_sequence_step(
        &reference,
        2,
        "put_default",
        result.context.state.get_ref().clone(),
        &names,
    );
    let result = reset_map(&result.context, 0, None, &INITIAL_COST_MODEL).unwrap();
    assert_sequence_step(
        &reference,
        3,
        "reset_table",
        result.context.state.get_ref().clone(),
        &names,
    );
}

#[test]
fn list_mutation_sequence_matches_typescript_state_bytes() {
    let reference: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/list-ts-sequence.json")).unwrap();
    let names = [
        "item_count",
        "items_empty",
        "first_item",
        "prepend",
        "drop_first",
        "clear_items",
    ];
    let result = push_front_list(
        &context(vec![constructor_list()]),
        0,
        Field::from(42_u64),
        None,
        &INITIAL_COST_MODEL,
    )
    .unwrap();
    assert_sequence_step(
        &reference,
        0,
        "prepend",
        result.context.state.get_ref().clone(),
        &names,
    );
    let result = push_front_list(
        &result.context,
        0,
        Field::from(7_u64),
        None,
        &INITIAL_COST_MODEL,
    )
    .unwrap();
    assert_sequence_step(
        &reference,
        1,
        "prepend",
        result.context.state.get_ref().clone(),
        &names,
    );
    let result = pop_front_list(&result.context, 0, None, &INITIAL_COST_MODEL).unwrap();
    assert_sequence_step(
        &reference,
        2,
        "drop_first",
        result.context.state.get_ref().clone(),
        &names,
    );
    let result = reset_list(&result.context, 0, None, &INITIAL_COST_MODEL).unwrap();
    assert_sequence_step(
        &reference,
        3,
        "clear_items",
        result.context.state.get_ref().clone(),
        &names,
    );
}
