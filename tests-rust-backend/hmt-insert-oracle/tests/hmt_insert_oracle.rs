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

use compact_rust_hmt_insert_oracle_fixture::ledger_contract::{
    append, append_hash, forget_history, full, initial_state, known, place, place_hash, reset_tree,
};
use compact_rust_hmt_insert_oracle_fixture::types::MerkleTreeDigest;
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
    for name in [
        "append",
        "place",
        "append_hash",
        "place_hash",
        "forget_history",
        "reset_tree",
        "full",
        "known",
    ] {
        operations = operations.insert(
            EntryPointBuf(name.as_bytes().to_vec()),
            ContractOperation::new(None),
        );
    }
    let state = ContractState::new(state, operations, ContractMaintenanceAuthority::default());
    let mut bytes = Vec::new();
    tagged_serialize(&state, &mut bytes).unwrap();
    hex::encode(bytes)
}

fn assert_native_query_gas(
    label: &str,
    actual: &runtime::context::RunningCost,
    oracle: &serde_json::Value,
) {
    let capture = &oracle["nativeQueries"][label];
    let queries = capture["queries"].as_array().unwrap();
    assert!(!queries.is_empty(), "{label}: no captured ledger query");
    let actual = serde_json::to_value(actual).unwrap();
    for key in ["readTime", "computeTime", "bytesWritten", "bytesDeleted"] {
        let total: u64 = queries
            .iter()
            .map(|query| {
                query["gasCost"][key]
                    .as_str()
                    .unwrap()
                    .parse::<u64>()
                    .unwrap()
            })
            .sum();
        assert_eq!(actual[key].as_u64().unwrap(), total, "{label}: {key}");
        assert_eq!(
            capture["reportedGas"][key],
            queries.last().unwrap()["gasCost"][key],
            "{label}: TypeScript reported {key}"
        );
    }
}

fn bounded<const MAX: u128>(value: u128) -> runtime::BoundedUint<MAX> {
    runtime::BoundedUint::new(value).unwrap()
}

fn current_root(state: &StateValue<DefaultDB>) -> MerkleTreeDigest {
    let root = runtime::ledger::historic_merkle_tree_view_at_path(state, &[0])
        .unwrap()
        .root()
        .unwrap();
    MerkleTreeDigest { field: root.0 }
}

fn assert_state(
    state: &StateValue<DefaultDB>,
    oracle: &serde_json::Value,
    step: &str,
    first_free: u128,
) {
    assert_eq!(
        state_hex(state.clone()),
        oracle[step],
        "state mismatch at {step}"
    );
    let tree = runtime::ledger::historic_merkle_tree_view_at_path(state, &[0]).unwrap();
    assert_eq!(tree.first_free().unwrap().value(), first_free);
    assert!(tree.root().is_some());
}

fn assert_history(state: &StateValue<DefaultDB>, oracle: &serde_json::Value, key: &str) {
    let tree = runtime::ledger::historic_merkle_tree_view_at_path(state, &[0]).unwrap();
    let mut actual = tree
        .history()
        .unwrap()
        .into_iter()
        .map(|root| num_bigint::BigUint::from_bytes_le(&root.0.as_le_bytes()).to_string())
        .collect::<Vec<_>>();
    let mut expected = oracle[key]
        .as_array()
        .unwrap()
        .iter()
        .map(|root| root.as_str().unwrap().to_string())
        .collect::<Vec<_>>();
    actual.sort();
    expected.sort();
    assert_eq!(actual, expected, "history mismatch at {key}");
}

#[test]
fn historic_merkle_insert_modes_match_typescript_state_bytes() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/hmt-insert-oracle.json"
    ))
    .unwrap();
    let initial = initial_state(ConstructorContext::new(())).unwrap();
    assert_state(initial.ledger_state.get_ref(), &oracle, "afterInit", 0);
    assert_history(initial.ledger_state.get_ref(), &oracle, "historyAtInit");
    let initial_root = current_root(initial.ledger_state.get_ref());
    let initial_tree =
        runtime::ledger::historic_merkle_tree_view_at_path(initial.ledger_state.get_ref(), &[0])
            .unwrap();
    assert!(initial_tree.contains_root(initial_tree.root().unwrap()));

    let context = initial.into_circuit_context(ContractAddress::default());
    let initial_full = full(context).unwrap();
    assert_eq!(initial_full.result, oracle["fullAtInit"]);
    assert_native_query_gas("fullAtInit", &initial_full.gas_cost, &oracle);
    let initial_known = known(initial_full.context, initial_root.clone()).unwrap();
    assert_eq!(initial_known.result, oracle["knownAtInit"]);
    assert_native_query_gas("knownAtInit", &initial_known.gas_cost, &oracle);
    let after_append7 = append(initial_known.context, bounded::<255>(7)).unwrap();
    assert_native_query_gas("append7", &after_append7.gas_cost, &oracle);
    assert_state(
        after_append7.context.query.state.get_ref(),
        &oracle,
        "afterAppend7",
        1,
    );
    assert_history(
        after_append7.context.query.state.get_ref(),
        &oracle,
        "historyAfterAppend7",
    );
    let tree = runtime::ledger::historic_merkle_tree_view_at_path(
        after_append7.context.query.state.get_ref(),
        &[0],
    )
    .unwrap();
    let path = tree.path_for_leaf(0, bounded::<255>(7)).unwrap();
    assert_eq!(Some(path.root()), tree.root());
    let path_oracle = &oracle["pathFor7At0"];
    assert_eq!(path.leaf.value().to_string(), path_oracle["leaf"]);
    assert_eq!(
        path.path.len(),
        path_oracle["path"].as_array().unwrap().len()
    );
    for (entry, expected) in path
        .path
        .iter()
        .zip(path_oracle["path"].as_array().unwrap())
    {
        let sibling = num_bigint::BigUint::from_bytes_le(&entry.sibling.0.as_le_bytes());
        assert_eq!(sibling.to_string(), expected["sibling"]);
        assert_eq!(entry.goes_left, expected["goesLeft"]);
    }
    let found = tree.find_path_for_leaf(bounded::<255>(7)).unwrap();
    assert_eq!(
        found.path.len(),
        oracle["foundPathFor7"]["path"].as_array().unwrap().len()
    );
    assert_eq!(found.root(), path.root());
    let wrong = tree.path_for_leaf(0, bounded::<255>(8)).unwrap();
    assert_eq!(
        wrong.leaf.value().to_string(),
        oracle["wrongPathFor8At0"]["leaf"]
    );
    assert_eq!(
        wrong.path.len(),
        oracle["wrongPathFor8At0"]["path"].as_array().unwrap().len()
    );
    assert_ne!(Some(wrong.root()), tree.root());
    assert!(tree.path_for_leaf(8, bounded::<255>(7)).is_err());
    assert!(tree.find_path_for_leaf(bounded::<255>(8)).is_none());
    assert!(oracle["missingPathFor8"].is_null());

    let known_old = known(after_append7.context, initial_root.clone()).unwrap();
    assert_eq!(known_old.result, oracle["knownInitialAfterAppend"]);
    let after_place9 = place(
        known_old.context,
        bounded::<255>(9),
        bounded::<{ u64::MAX as u128 }>(3),
    )
    .unwrap();
    assert_state(
        after_place9.context.query.state.get_ref(),
        &oracle,
        "afterPlace9At3",
        4,
    );
    let tree = runtime::ledger::historic_merkle_tree_view_at_path(
        after_place9.context.query.state.get_ref(),
        &[0],
    )
    .unwrap();
    let path = tree.path_for_leaf(3, bounded::<255>(9)).unwrap();
    assert_eq!(Some(path.root()), tree.root());
    let expected = &oracle["pathFor9At3"];
    assert_eq!(path.leaf.value().to_string(), expected["leaf"]);
    for (entry, expected) in path.path.iter().zip(expected["path"].as_array().unwrap()) {
        let sibling = num_bigint::BigUint::from_bytes_le(&entry.sibling.0.as_le_bytes());
        assert_eq!(sibling.to_string(), expected["sibling"]);
        assert_eq!(entry.goes_left, expected["goesLeft"]);
    }

    let after_append11 = append(after_place9.context, bounded::<255>(11)).unwrap();
    assert_state(
        after_append11.context.query.state.get_ref(),
        &oracle,
        "afterAppend11",
        5,
    );

    let after_place13 = place(
        after_append11.context,
        bounded::<255>(13),
        bounded::<{ u64::MAX as u128 }>(1),
    )
    .unwrap();
    assert_state(
        after_place13.context.query.state.get_ref(),
        &oracle,
        "afterPlace13At1",
        5,
    );

    let root_before_reset = current_root(after_place13.context.query.state.get_ref());
    let after_forget = forget_history(after_place13.context).unwrap();
    assert_native_query_gas("forgetHistory", &after_forget.gas_cost, &oracle);
    assert_state(
        after_forget.context.query.state.get_ref(),
        &oracle,
        "afterForgetHistory",
        5,
    );
    assert_history(
        after_forget.context.query.state.get_ref(),
        &oracle,
        "historyAfterForget",
    );
    let reset_view = runtime::ledger::historic_merkle_tree_view_at_path(
        after_forget.context.query.state.get_ref(),
        &[0],
    )
    .unwrap();
    assert!(!reset_view.contains_root(runtime::ledger::MerkleTreeDigest(initial_root.field)));
    assert!(reset_view.contains_root(runtime::ledger::MerkleTreeDigest(root_before_reset.field)));
    let forgotten_old = known(after_forget.context, initial_root.clone()).unwrap();
    assert_eq!(forgotten_old.result, oracle["knownInitialAfterReset"]);
    let retained_current = known(forgotten_old.context, root_before_reset).unwrap();
    assert_eq!(retained_current.result, oracle["knownCurrentAfterReset"]);
    let before_capacity = full(retained_current.context).unwrap();
    assert_eq!(before_capacity.result, oracle["fullBeforeCapacity"]);
    let after_hash =
        append_hash(before_capacity.context, runtime::FixedBytes::new([1; 32])).unwrap();
    assert_state(
        after_hash.context.query.state.get_ref(),
        &oracle,
        "afterAppendHash",
        6,
    );
    let at7 = place_hash(
        after_hash.context,
        runtime::FixedBytes::new([2; 32]),
        bounded::<{ u64::MAX as u128 }>(7),
    )
    .unwrap();
    assert_state(
        at7.context.query.state.get_ref(),
        &oracle,
        "afterPlaceHashAt7",
        8,
    );
    let at_capacity = full(at7.context).unwrap();
    assert_eq!(at_capacity.result, oracle["fullAtCapacity"]);
    let at1 = place_hash(
        at_capacity.context,
        runtime::FixedBytes::new([3; 32]),
        bounded::<{ u64::MAX as u128 }>(1),
    )
    .unwrap();
    assert_state(
        at1.context.query.state.get_ref(),
        &oracle,
        "afterReplaceHashAt1",
        8,
    );
    let after_replacement = full(at1.context).unwrap();
    assert_eq!(after_replacement.result, oracle["fullAfterReplacement"]);
    let root_before_tree_reset = current_root(after_replacement.context.query.state.get_ref());
    let reset = reset_tree(after_replacement.context).unwrap();
    assert_state(
        reset.context.query.state.get_ref(),
        &oracle,
        "afterResetTree",
        0,
    );
    assert_eq!(
        state_hex(reset.context.query.state.get_ref().clone()),
        oracle["afterInit"]
    );
    let after_reset_full = full(reset.context).unwrap();
    assert_eq!(after_reset_full.result, oracle["fullAfterTreeReset"]);
    let old_root = known(after_reset_full.context, root_before_tree_reset).unwrap();
    assert_eq!(old_root.result, oracle["knownOldAfterTreeReset"]);
    let blank_root = known(old_root.context, initial_root).unwrap();
    assert_eq!(blank_root.result, oracle["knownBlankAfterTreeReset"]);
}
