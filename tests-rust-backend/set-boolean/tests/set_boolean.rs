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

use compact_rust_set_boolean_fixture::ledger_contract::{Contract, PublicStateView};
#[path = "../../boolean_observation_assertions.rs"]
mod boolean_observation_assertions;
use compact_rust_set_boolean_fixture::ledger_contract::{
    add, add_field, choose, contains, contains_field, initial_state, remove, reset_fields,
    seen_is_empty, seen_size,
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
        "add",
        "contains",
        "add_field",
        "contains_field",
        "remove",
        "seen_size",
        "seen_is_empty",
        "reset_fields",
        "choose",
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
fn public_set_view_rejects_missing_and_wrong_shaped_paths() {
    use midnight_compact_runtime::ledger::{constructor_cell, contract_state, set_view_at_path};

    let empty = contract_state::<DefaultDB>(vec![]);
    assert_eq!(
        PublicStateView::from(empty.get_ref()).seen().err().unwrap(),
        set_view_at_path::<bool, _>(empty.get_ref(), &[0])
            .err()
            .unwrap(),
    );
    let wrong_shape = contract_state::<DefaultDB>(vec![constructor_cell(false)]);
    assert_eq!(
        PublicStateView::from(wrong_shape.get_ref())
            .seen()
            .err()
            .unwrap(),
        set_view_at_path::<bool, _>(wrong_shape.get_ref(), &[0])
            .err()
            .unwrap(),
    );
}

#[test]
fn generated_set_contract_inserts_and_checks_membership() {
    let constructor = initial_state(ConstructorContext::new(())).unwrap();
    let initial = PublicStateView::from(&constructor);
    assert!(initial.seen().unwrap().is_empty());
    assert_eq!(initial.seen().unwrap().size().unwrap().value(), 0);
    assert!(initial.fields().unwrap().is_empty());
    let StateValue::Array(fields) = constructor.ledger_state.get_ref() else {
        panic!("expected ledger field array")
    };
    let StateValue::Map(set) = fields.get(0).unwrap() else {
        panic!("expected Set map")
    };
    assert_eq!(set.size(), 0);
    let StateValue::Map(set) = fields.get(1).unwrap() else {
        panic!("expected Field Set map")
    };
    assert_eq!(set.size(), 0);

    let context = constructor.into_circuit_context(ContractAddress::default());
    let result = seen_is_empty(context).unwrap();
    assert!(result.result);
    let result = seen_size(result.context).unwrap();
    assert_eq!(result.result.value(), 0);
    let result = contains(result.context, true).unwrap();
    assert!(!result.result);
    let result = add(result.context, true).unwrap();
    let result = contains(result.context, true).unwrap();
    assert!(result.result);
    let result = contains(result.context, false).unwrap();
    assert!(!result.result);
    let result = seen_size(result.context).unwrap();
    assert_eq!(result.result.value(), 1);
    let result = seen_is_empty(result.context).unwrap();
    assert!(!result.result);
    let result = add(result.context, true).unwrap();
    let result = contains_field(result.context, Field::from(42_u64)).unwrap();
    assert!(!result.result);
    let result = add_field(result.context, Field::from(42_u64)).unwrap();
    let result = contains_field(result.context, Field::from(42_u64)).unwrap();
    assert!(result.result);
    let result = contains_field(result.context, Field::from(43_u64)).unwrap();
    assert!(!result.result);
    let view = PublicStateView::from(&result);
    assert!(view.seen().unwrap().member(true));
    assert!(!view.seen().unwrap().member(false));
    assert_eq!(view.seen().unwrap().size().unwrap().value(), 1);
    assert!(view.fields().unwrap().member(Field::from(42_u64)));
    assert!(!view.fields().unwrap().member(Field::from(43_u64)));
    let raw_seen = midnight_compact_runtime::ledger::set_view_at_path::<bool, _>(
        result.context.query.state.get_ref(),
        &[0],
    )
    .unwrap();
    assert_eq!(view.seen().unwrap().member(true), raw_seen.member(true));
    assert_eq!(
        view.seen().unwrap().size().unwrap(),
        raw_seen.size().unwrap()
    );
    let raw_fields = midnight_compact_runtime::ledger::set_view_at_path::<Field, _>(
        result.context.query.state.get_ref(),
        &[1],
    )
    .unwrap();
    assert_eq!(
        view.fields().unwrap().member(Field::from(42_u64)),
        raw_fields.member(Field::from(42_u64)),
    );
    let StateValue::Array(fields) = result.context.query.state.get_ref() else {
        panic!("expected ledger field array")
    };
    let StateValue::Map(set) = fields.get(0).unwrap() else {
        panic!("expected Set map")
    };
    assert_eq!(set.size(), 1);
    let StateValue::Map(set) = fields.get(1).unwrap() else {
        panic!("expected Field Set map")
    };
    assert_eq!(set.size(), 1);

    let result = remove(result.context, true).unwrap();
    let result = seen_is_empty(result.context).unwrap();
    assert!(result.result);
    let result = seen_size(result.context).unwrap();
    assert_eq!(result.result.value(), 0);
    assert!(PublicStateView::from(&result).seen().unwrap().is_empty());
    let StateValue::Array(fields) = result.context.query.state.get_ref() else {
        panic!("expected ledger field array")
    };
    let StateValue::Map(set) = fields.get(1).unwrap() else {
        panic!("expected Field Set map")
    };
    assert_eq!(set.size(), 1);

    let result = reset_fields(result.context).unwrap();
    assert!(PublicStateView::from(&result).fields().unwrap().is_empty());
    let StateValue::Array(fields) = result.context.query.state.get_ref() else {
        panic!("expected ledger field array")
    };
    let StateValue::Map(set) = fields.get(1).unwrap() else {
        panic!("expected Field Set map")
    };
    assert_eq!(set.size(), 0);
}

#[test]
fn conditional_set_actions_keep_the_selected_branch_and_following_query() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/conditional-set-actions.json"
    ))
    .unwrap();
    let context = initial_state(ConstructorContext::new(()))
        .unwrap()
        .into_circuit_context(ContractAddress::default());
    let inserted = choose(context, true, true).unwrap();
    assert!(inserted.result);
    assert_eq!(
        state_hex(inserted.context.query.state.get_ref().clone()),
        oracle[0]["stateHex"]
    );
    let removed = choose(inserted.context, true, false).unwrap();
    assert!(!removed.result);
    assert_eq!(
        state_hex(removed.context.query.state.get_ref().clone()),
        oracle[1]["stateHex"]
    );
    let other = choose(removed.context, false, true).unwrap();
    assert!(other.result);
    assert_eq!(
        state_hex(other.context.query.state.get_ref().clone()),
        oracle[2]["stateHex"]
    );
    let present = contains(other.context, false).unwrap();
    assert!(present.result);
}

#[test]
fn conditional_set_recording_matches_typescript_and_replays_all_branches() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/conditional-set-recording-oracle.json"
    ))
    .unwrap();
    let contract = Contract::default();
    let mut native_context = initial_state(ConstructorContext::new(()))
        .unwrap()
        .into_circuit_context(ContractAddress::default());
    let mut recorded_context = initial_state(ConstructorContext::new(()))
        .unwrap()
        .into_circuit_context(ContractAddress::default());

    for (name, value, insert) in [
        ("insertTrue", true, true),
        ("removeTrue", true, false),
        ("insertFalse", false, true),
    ] {
        let native = choose(native_context, value, insert).unwrap();
        let recorded = contract
            .recording
            .choose(recorded_context, value, insert)
            .unwrap();
        let expected = &oracle[name];
        assert_eq!(native.result, expected["result"].as_bool().unwrap());
        assert_eq!(recorded.execution.result, native.result);
        assert_eq!(
            state_hex(native.context.query.state.get_ref().clone()),
            expected["stateHex"],
            "{name}: native state"
        );
        assert_eq!(
            state_hex(recorded.execution.context.query.state.get_ref().clone()),
            expected["stateHex"],
            "{name}: recorded state"
        );
        assert!(expected["privateStateNull"].as_bool().unwrap());
        assert_eq!(recorded.execution.context.private_state, ());
        boolean_observation_assertions::assert_ts_trace(name, &native, &recorded, expected);
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
        assert_eq!(
            replay.context.state.get_ref(),
            recorded.execution.context.query.state.get_ref(),
            "{name}: replay state"
        );
        native_context = native.context;
        recorded_context = recorded.execution.context;
    }
}

#[test]
fn recorded_set_mutation_and_queries_match_native_state_and_replay() {
    let context = initial_state(ConstructorContext::new(()))
        .unwrap()
        .into_circuit_context(ContractAddress::default());
    let contract = Contract::default();
    let added = contract.recording.add(context, true).unwrap();
    let replay = added
        .public
        .initial()
        .query(
            added.public.verify_ops(),
            None,
            &added.execution.context.cost_model,
        )
        .unwrap();
    assert_eq!(
        replay.context.effects,
        added.execution.context.query.effects
    );
    assert_eq!(
        replay.context.state.get_ref(),
        added.execution.context.query.state.get_ref()
    );
    assert!(PublicStateView::from(&added).seen().unwrap().member(true));
    assert!(PublicStateView::from(&replay).seen().unwrap().member(true));

    let contains = contract
        .recording
        .contains(added.execution.context, true)
        .unwrap();
    assert!(contains.execution.result);
    let replay = contains
        .public
        .initial()
        .query(
            contains.public.verify_ops(),
            None,
            &contains.execution.context.cost_model,
        )
        .unwrap();
    assert_eq!(
        replay.context.effects,
        contains.execution.context.query.effects
    );

    let size = contract
        .recording
        .seen_size(contains.execution.context)
        .unwrap();
    assert_eq!(size.execution.result.value(), 1);
    let empty = contract
        .recording
        .seen_is_empty(size.execution.context)
        .unwrap();
    assert!(!empty.execution.result);
    let removed = contract
        .recording
        .remove(empty.execution.context, true)
        .unwrap();
    let empty = contract
        .recording
        .seen_is_empty(removed.execution.context)
        .unwrap();
    assert!(empty.execution.result);
}
