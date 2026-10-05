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

use compact_rust_hmt_insert_oracle_fixture::ledger_contract::recorded;
use compact_rust_hmt_insert_oracle_fixture::ledger_contract::{
    PublicStateView, append, append_hash, forget_history, full, initial_state, known, place,
    place_hash, reset_tree,
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
    let generated = PublicStateView::from(state);
    let typed = generated.t().unwrap();
    assert_eq!(typed.root(), tree.root(), "generated root at {step}");
    assert_eq!(
        typed.first_free().unwrap(),
        tree.first_free().unwrap(),
        "generated first_free at {step}"
    );
}

fn assert_history(state: &StateValue<DefaultDB>, oracle: &serde_json::Value, key: &str) {
    let tree = runtime::ledger::historic_merkle_tree_view_at_path(state, &[0]).unwrap();
    let generated = PublicStateView::from(state);
    let typed = generated.t().unwrap();
    assert_eq!(typed.history().unwrap(), tree.history().unwrap());
    for root in tree.history().unwrap() {
        assert_eq!(typed.contains_root(root), tree.contains_root(root));
    }
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
fn recorded_historic_append_preserves_root_history_and_replays() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/hmt-insert-oracle.json"
    ))
    .unwrap();
    let initial = initial_state(ConstructorContext::new(())).unwrap();
    let context = initial.into_circuit_context(ContractAddress::default());
    let recorded = recorded::append(context, bounded::<255>(7)).unwrap();
    assert_eq!(recorded.execution.result, ());
    assert_state(
        recorded.execution.context.query.state.get_ref(),
        &oracle,
        "afterAppend7",
        1,
    );
    assert_history(
        recorded.execution.context.query.state.get_ref(),
        &oracle,
        "historyAfterAppend7",
    );
    assert_native_query_gas("append7", &recorded.execution.gas_cost, &oracle);

    let queries = oracle["nativeQueries"]["append7"]["queries"]
        .as_array()
        .unwrap();
    assert_eq!(queries.len(), 1);
    let actual_program = serde_json::to_value(recorded.public.verify_ops()).unwrap();
    assert_eq!(actual_program, queries[0]["program"]);
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
        state_hex(replay.context.state.get_ref().clone()),
        state_hex(recorded.execution.context.query.state.get_ref().clone())
    );
    assert_history(
        replay.context.state.get_ref(),
        &oracle,
        "historyAfterAppend7",
    );
}

#[test]
fn recorded_historic_reset_history_matches_typescript_and_replays() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/hmt-insert-oracle.json"
    ))
    .unwrap();
    let before_reset = || {
        let context = initial_state(ConstructorContext::new(()))
            .unwrap()
            .into_circuit_context(ContractAddress::default());
        let context = append(context, bounded::<255>(7)).unwrap().context;
        let context = place(
            context,
            bounded::<255>(9),
            bounded::<{ u64::MAX as u128 }>(3),
        )
        .unwrap()
        .context;
        let context = append(context, bounded::<255>(11)).unwrap().context;
        place(
            context,
            bounded::<255>(13),
            bounded::<{ u64::MAX as u128 }>(1),
        )
        .unwrap()
        .context
    };
    let initial_root = current_root(
        initial_state(ConstructorContext::new(()))
            .unwrap()
            .ledger_state
            .get_ref(),
    );
    let current = current_root(before_reset().query.state.get_ref());
    let native = forget_history(before_reset()).unwrap();
    let recorded = recorded::forget_history(before_reset()).unwrap();
    let tree = runtime::ledger::historic_merkle_tree_view_at_path(
        recorded.execution.context.query.state.get_ref(),
        &[0],
    )
    .unwrap();
    assert!(!tree.contains_root(runtime::ledger::MerkleTreeDigest(initial_root.field)));
    assert!(tree.contains_root(runtime::ledger::MerkleTreeDigest(current.field)));
    assert_historic_hash_recording(
        "forgetHistory",
        "afterForgetHistory",
        "historyAfterForget",
        5,
        native,
        recorded,
        &oracle,
    );
}

#[test]
fn recorded_historic_tree_reset_seeds_blank_history_and_replays() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/hmt-insert-oracle.json"
    ))
    .unwrap();
    let before_reset = || {
        let context = initial_state(ConstructorContext::new(()))
            .unwrap()
            .into_circuit_context(ContractAddress::default());
        let context = append(context, bounded::<255>(7)).unwrap().context;
        let context = place(
            context,
            bounded::<255>(9),
            bounded::<{ u64::MAX as u128 }>(3),
        )
        .unwrap()
        .context;
        let context = append(context, bounded::<255>(11)).unwrap().context;
        let context = place(
            context,
            bounded::<255>(13),
            bounded::<{ u64::MAX as u128 }>(1),
        )
        .unwrap()
        .context;
        let context = forget_history(context).unwrap().context;
        let context = full(context).unwrap().context;
        let context = append_hash(context, runtime::FixedBytes::new([1; 32]))
            .unwrap()
            .context;
        let context = place_hash(
            context,
            runtime::FixedBytes::new([2; 32]),
            bounded::<{ u64::MAX as u128 }>(7),
        )
        .unwrap()
        .context;
        let context = full(context).unwrap().context;
        let context = place_hash(
            context,
            runtime::FixedBytes::new([3; 32]),
            bounded::<{ u64::MAX as u128 }>(1),
        )
        .unwrap()
        .context;
        full(context).unwrap().context
    };
    let blank_state = initial_state(ConstructorContext::new(())).unwrap();
    let blank_root = current_root(blank_state.ledger_state.get_ref());
    let old_root = current_root(before_reset().query.state.get_ref());
    let native = reset_tree(before_reset()).unwrap();
    let recorded = recorded::reset_tree(before_reset()).unwrap();
    let state = recorded.execution.context.query.state.get_ref();
    assert_eq!(
        state_hex(state.clone()),
        state_hex(blank_state.ledger_state.get_ref().clone())
    );
    let tree = runtime::ledger::historic_merkle_tree_view_at_path(state, &[0]).unwrap();
    assert!(!tree.contains_root(runtime::ledger::MerkleTreeDigest(old_root.field)));
    assert!(tree.contains_root(runtime::ledger::MerkleTreeDigest(blank_root.field)));
    assert_historic_hash_recording(
        "resetTree",
        "afterResetTree",
        "historyAfterResetTree",
        0,
        native,
        recorded,
        &oracle,
    );
}

fn assert_historic_known_case(
    label: &str,
    native: runtime::context::CircuitResult<(), bool, DefaultDB>,
    recorded: runtime::recording::RecordedCircuitResult<(), bool, DefaultDB>,
    oracle: &serde_json::Value,
) -> (
    runtime::context::CircuitContext<(), DefaultDB>,
    runtime::context::CircuitContext<(), DefaultDB>,
) {
    assert_eq!(native.result, oracle[label], "{label}: native result");
    assert_eq!(
        recorded.execution.result, native.result,
        "{label}: recorded result"
    );
    assert_eq!(recorded.execution.gas_cost, native.gas_cost);
    assert_eq!(
        recorded.execution.context.query.effects,
        native.context.query.effects
    );
    assert!(recorded.execution.private_transcript_outputs.is_empty());
    assert_eq!(oracle["nativeQueries"][label]["privateOutputs"], 0);
    assert_native_query_gas(label, &recorded.execution.gas_cost, oracle);
    let original_state = state_hex(recorded.public.initial().state.get_ref().clone());
    let resulting_state = state_hex(recorded.execution.context.query.state.get_ref().clone());
    assert_eq!(resulting_state, original_state, "{label}: state changed");
    assert_eq!(
        state_hex(native.context.query.state.get_ref().clone()),
        original_state
    );
    let queries = oracle["nativeQueries"][label]["queries"]
        .as_array()
        .unwrap();
    assert_eq!(queries.len(), 1);
    let verify = serde_json::to_value(recorded.public.verify_ops()).unwrap();
    let verify = verify.as_array().unwrap();
    let gathered = queries[0]["program"].as_array().unwrap();
    assert_eq!(verify.len(), gathered.len());
    assert_eq!(&verify[..verify.len() - 1], &gathered[..gathered.len() - 1]);
    assert_eq!(
        gathered.last().unwrap()["popeq"]["result"],
        serde_json::Value::Null
    );
    assert_ne!(
        verify.last().unwrap()["popeq"]["result"],
        serde_json::Value::Null
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
    assert_eq!(replay.gas_cost, recorded.execution.gas_cost);
    assert_eq!(
        replay.context.effects,
        recorded.execution.context.query.effects
    );
    assert_eq!(
        state_hex(replay.context.state.get_ref().clone()),
        original_state
    );
    (native.context, recorded.execution.context)
}

#[test]
fn recorded_historic_known_matches_history_membership_true_and_false() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/hmt-insert-oracle.json"
    ))
    .unwrap();
    let initial = initial_state(ConstructorContext::new(())).unwrap();
    let blank_root = current_root(initial.ledger_state.get_ref());
    let native = initial.into_circuit_context(ContractAddress::default());
    let recorded = initial_state(ConstructorContext::new(()))
        .unwrap()
        .into_circuit_context(ContractAddress::default());
    let (native, recorded) = assert_historic_known_case(
        "knownAtInit",
        known(native, blank_root.clone()).unwrap(),
        recorded::known(recorded, blank_root.clone()).unwrap(),
        &oracle,
    );
    let native = append(native, bounded::<255>(7)).unwrap().context;
    let recorded = append(recorded, bounded::<255>(7)).unwrap().context;
    let (native, recorded) = assert_historic_known_case(
        "knownInitialAfterAppend",
        known(native, blank_root.clone()).unwrap(),
        recorded::known(recorded, blank_root.clone()).unwrap(),
        &oracle,
    );
    let native = place(
        native,
        bounded::<255>(9),
        bounded::<{ u64::MAX as u128 }>(3),
    )
    .unwrap()
    .context;
    let recorded = place(
        recorded,
        bounded::<255>(9),
        bounded::<{ u64::MAX as u128 }>(3),
    )
    .unwrap()
    .context;
    let native = append(native, bounded::<255>(11)).unwrap().context;
    let recorded = append(recorded, bounded::<255>(11)).unwrap().context;
    let native = place(
        native,
        bounded::<255>(13),
        bounded::<{ u64::MAX as u128 }>(1),
    )
    .unwrap()
    .context;
    let recorded = place(
        recorded,
        bounded::<255>(13),
        bounded::<{ u64::MAX as u128 }>(1),
    )
    .unwrap()
    .context;
    let current_after_place = current_root(native.query.state.get_ref());
    let native = forget_history(native).unwrap().context;
    let recorded = forget_history(recorded).unwrap().context;
    let (native, recorded) = assert_historic_known_case(
        "knownInitialAfterReset",
        known(native, blank_root.clone()).unwrap(),
        recorded::known(recorded, blank_root.clone()).unwrap(),
        &oracle,
    );
    let (native, recorded) = assert_historic_known_case(
        "knownCurrentAfterReset",
        known(native, current_after_place.clone()).unwrap(),
        recorded::known(recorded, current_after_place).unwrap(),
        &oracle,
    );
    let native = full(native).unwrap().context;
    let recorded = full(recorded).unwrap().context;
    let native = append_hash(native, runtime::FixedBytes::new([1; 32]))
        .unwrap()
        .context;
    let recorded = append_hash(recorded, runtime::FixedBytes::new([1; 32]))
        .unwrap()
        .context;
    let native = place_hash(
        native,
        runtime::FixedBytes::new([2; 32]),
        bounded::<{ u64::MAX as u128 }>(7),
    )
    .unwrap()
    .context;
    let recorded = place_hash(
        recorded,
        runtime::FixedBytes::new([2; 32]),
        bounded::<{ u64::MAX as u128 }>(7),
    )
    .unwrap()
    .context;
    let native = full(native).unwrap().context;
    let recorded = full(recorded).unwrap().context;
    let native = place_hash(
        native,
        runtime::FixedBytes::new([3; 32]),
        bounded::<{ u64::MAX as u128 }>(1),
    )
    .unwrap()
    .context;
    let recorded = place_hash(
        recorded,
        runtime::FixedBytes::new([3; 32]),
        bounded::<{ u64::MAX as u128 }>(1),
    )
    .unwrap()
    .context;
    let native = full(native).unwrap().context;
    let recorded = full(recorded).unwrap().context;
    let populated_root = current_root(native.query.state.get_ref());
    let native = reset_tree(native).unwrap().context;
    let recorded = reset_tree(recorded).unwrap().context;
    let (native, recorded) = assert_historic_known_case(
        "knownOldAfterTreeReset",
        known(native, populated_root.clone()).unwrap(),
        recorded::known(recorded, populated_root).unwrap(),
        &oracle,
    );
    let _ = assert_historic_known_case(
        "knownBlankAfterTreeReset",
        known(native, blank_root.clone()).unwrap(),
        recorded::known(recorded, blank_root).unwrap(),
        &oracle,
    );
}

#[test]
fn recorded_historic_hash_append_preserves_typescript_history_and_replays() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/hmt-insert-oracle.json"
    ))
    .unwrap();
    let at_hash_append = || {
        let context = initial_state(ConstructorContext::new(()))
            .unwrap()
            .into_circuit_context(ContractAddress::default());
        let context = append(context, bounded::<255>(7)).unwrap().context;
        let context = place(
            context,
            bounded::<255>(9),
            bounded::<{ u64::MAX as u128 }>(3),
        )
        .unwrap()
        .context;
        let context = append(context, bounded::<255>(11)).unwrap().context;
        let context = place(
            context,
            bounded::<255>(13),
            bounded::<{ u64::MAX as u128 }>(1),
        )
        .unwrap()
        .context;
        let context = forget_history(context).unwrap().context;
        full(context).unwrap().context
    };
    let hash = runtime::FixedBytes::new([1; 32]);
    let native = append_hash(at_hash_append(), hash).unwrap();
    let recorded = recorded::append_hash(at_hash_append(), hash).unwrap();
    assert_eq!(recorded.execution.result, ());
    assert_eq!(recorded.execution.gas_cost, native.gas_cost);
    assert_eq!(
        recorded.execution.context.query.effects,
        native.context.query.effects
    );
    assert!(recorded.execution.private_transcript_outputs.is_empty());
    assert_state(
        recorded.execution.context.query.state.get_ref(),
        &oracle,
        "afterAppendHash",
        6,
    );
    assert_history(
        recorded.execution.context.query.state.get_ref(),
        &oracle,
        "historyAfterAppendHash",
    );
    assert_eq!(
        state_hex(native.context.query.state.get_ref().clone()),
        state_hex(recorded.execution.context.query.state.get_ref().clone())
    );
    assert_native_query_gas("appendHash", &recorded.execution.gas_cost, &oracle);
    assert_eq!(
        oracle["nativeQueries"]["appendHash"]["result"],
        serde_json::json!([])
    );
    assert_eq!(oracle["nativeQueries"]["appendHash"]["privateOutputs"], 0);
    let queries = oracle["nativeQueries"]["appendHash"]["queries"]
        .as_array()
        .unwrap();
    assert_eq!(queries.len(), 1);
    assert_eq!(
        serde_json::to_value(recorded.public.verify_ops()).unwrap(),
        queries[0]["program"]
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
    assert_eq!(replay.gas_cost, recorded.execution.gas_cost);
    assert_eq!(
        replay.context.effects,
        recorded.execution.context.query.effects
    );
    assert_eq!(
        state_hex(replay.context.state.get_ref().clone()),
        state_hex(recorded.execution.context.query.state.get_ref().clone())
    );
    assert_history(
        replay.context.state.get_ref(),
        &oracle,
        "historyAfterAppendHash",
    );
}

#[test]
fn recorded_historic_indexed_hash_placement_matches_typescript_history_and_replays() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/hmt-insert-oracle.json"
    ))
    .unwrap();
    let before_place = || {
        let context = initial_state(ConstructorContext::new(()))
            .unwrap()
            .into_circuit_context(ContractAddress::default());
        let context = append(context, bounded::<255>(7)).unwrap().context;
        let context = place(
            context,
            bounded::<255>(9),
            bounded::<{ u64::MAX as u128 }>(3),
        )
        .unwrap()
        .context;
        let context = append(context, bounded::<255>(11)).unwrap().context;
        let context = place(
            context,
            bounded::<255>(13),
            bounded::<{ u64::MAX as u128 }>(1),
        )
        .unwrap()
        .context;
        let context = forget_history(context).unwrap().context;
        let context = full(context).unwrap().context;
        append_hash(context, runtime::FixedBytes::new([1; 32]))
            .unwrap()
            .context
    };
    let hash_at_7 = runtime::FixedBytes::new([2; 32]);
    let index_7 = bounded::<{ u64::MAX as u128 }>(7);
    let native = place_hash(before_place(), hash_at_7, index_7).unwrap();
    let recorded = recorded::place_hash(before_place(), hash_at_7, index_7).unwrap();
    assert_historic_hash_recording(
        "placeHashAt7",
        "afterPlaceHashAt7",
        "historyAfterPlaceHashAt7",
        8,
        native,
        recorded,
        &oracle,
    );

    let before_replace = || {
        let context = place_hash(before_place(), hash_at_7, index_7)
            .unwrap()
            .context;
        full(context).unwrap().context
    };
    let hash_at_1 = runtime::FixedBytes::new([3; 32]);
    let index_1 = bounded::<{ u64::MAX as u128 }>(1);
    let native = place_hash(before_replace(), hash_at_1, index_1).unwrap();
    let recorded = recorded::place_hash(before_replace(), hash_at_1, index_1).unwrap();
    assert_historic_hash_recording(
        "placeHashAt1",
        "afterReplaceHashAt1",
        "historyAfterReplaceHashAt1",
        8,
        native,
        recorded,
        &oracle,
    );
}

fn assert_historic_hash_recording(
    query: &str,
    state: &str,
    history: &str,
    first_free: u128,
    native: runtime::context::CircuitResult<(), (), DefaultDB>,
    recorded: runtime::recording::RecordedCircuitResult<(), (), DefaultDB>,
    oracle: &serde_json::Value,
) {
    assert_eq!(native.gas_cost, recorded.execution.gas_cost);
    assert_eq!(
        native.context.query.effects,
        recorded.execution.context.query.effects
    );
    assert!(recorded.execution.private_transcript_outputs.is_empty());
    assert_state(
        recorded.execution.context.query.state.get_ref(),
        oracle,
        state,
        first_free,
    );
    assert_history(
        recorded.execution.context.query.state.get_ref(),
        oracle,
        history,
    );
    assert_eq!(
        state_hex(native.context.query.state.get_ref().clone()),
        state_hex(recorded.execution.context.query.state.get_ref().clone())
    );
    assert_native_query_gas(query, &recorded.execution.gas_cost, oracle);
    assert_eq!(
        oracle["nativeQueries"][query]["result"],
        serde_json::json!([])
    );
    assert_eq!(oracle["nativeQueries"][query]["privateOutputs"], 0);
    let queries = oracle["nativeQueries"][query]["queries"]
        .as_array()
        .unwrap();
    assert_eq!(queries.len(), 1);
    assert_eq!(
        serde_json::to_value(recorded.public.verify_ops()).unwrap(),
        queries[0]["program"]
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
    assert_eq!(replay.gas_cost, recorded.execution.gas_cost);
    assert_eq!(
        replay.context.effects,
        recorded.execution.context.query.effects
    );
    assert_eq!(
        state_hex(replay.context.state.get_ref().clone()),
        state_hex(recorded.execution.context.query.state.get_ref().clone())
    );
    assert_history(replay.context.state.get_ref(), oracle, history);
}

#[test]
fn recorded_historic_indexed_insertion_matches_typescript_and_replays() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/hmt-insert-oracle.json"
    ))
    .unwrap();
    let start = || {
        let context = initial_state(ConstructorContext::new(()))
            .unwrap()
            .into_circuit_context(ContractAddress::default());
        append(context, bounded::<255>(7)).unwrap().context
    };
    let value = bounded::<255>(9);
    let index = bounded::<{ u64::MAX as u128 }>(3);
    let native = place(start(), value, index).unwrap();
    let recorded = recorded::place(start(), value, index).unwrap();
    assert_eq!(native.gas_cost, recorded.execution.gas_cost);
    assert_eq!(
        native.context.query.effects,
        recorded.execution.context.query.effects
    );
    assert!(recorded.execution.private_transcript_outputs.is_empty());
    assert_state(
        recorded.execution.context.query.state.get_ref(),
        &oracle,
        "afterPlace9At3",
        4,
    );
    assert_eq!(
        state_hex(native.context.query.state.get_ref().clone()),
        state_hex(recorded.execution.context.query.state.get_ref().clone())
    );
    assert_native_query_gas("place9At3", &recorded.execution.gas_cost, &oracle);
    assert_eq!(
        oracle["nativeQueries"]["place9At3"]["result"],
        serde_json::json!([])
    );
    assert_eq!(oracle["nativeQueries"]["place9At3"]["privateOutputs"], 0);
    let queries = oracle["nativeQueries"]["place9At3"]["queries"]
        .as_array()
        .unwrap();
    assert_eq!(queries.len(), 1);
    assert_eq!(
        serde_json::to_value(recorded.public.verify_ops()).unwrap(),
        queries[0]["program"]
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
    assert_eq!(replay.gas_cost, recorded.execution.gas_cost);
    assert_eq!(
        replay.context.effects,
        recorded.execution.context.query.effects
    );
    assert_eq!(
        state_hex(replay.context.state.get_ref().clone()),
        state_hex(recorded.execution.context.query.state.get_ref().clone())
    );
}

#[test]
fn recorded_historic_fullness_matches_native_vm_and_preserves_history() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/hmt-insert-oracle.json"
    ))
    .unwrap();
    let expected_program = &oracle["nativeQueries"]["fullAtInit"]["queries"][0]["program"];
    for (count, expected) in [(0_u128, false), (1, false), (8, true)] {
        let context = || {
            let mut context = initial_state(ConstructorContext::new(()))
                .unwrap()
                .into_circuit_context(ContractAddress::default());
            for value in 1..=count {
                context = append(context, bounded::<255>(value)).unwrap().context;
            }
            context
        };
        let native = full(context()).unwrap();
        let recorded = recorded::full(context()).unwrap();
        assert_eq!(recorded.execution.result, expected);
        assert_eq!(recorded.execution.result, native.result);
        assert_eq!(recorded.execution.gas_cost, native.gas_cost);
        assert_eq!(
            recorded.execution.context.query.effects,
            native.context.query.effects
        );
        assert_eq!(
            state_hex(recorded.public.initial().state.get_ref().clone()),
            state_hex(recorded.execution.context.query.state.get_ref().clone())
        );
        assert_eq!(
            state_hex(native.context.query.state.get_ref().clone()),
            state_hex(recorded.execution.context.query.state.get_ref().clone())
        );
        let mut verify = serde_json::to_value(recorded.public.verify_ops()).unwrap();
        let read = &mut verify.as_array_mut().unwrap().last_mut().unwrap()["popeq"]["result"];
        assert!(!read.is_null());
        *read = serde_json::Value::Null;
        assert_eq!(&verify, expected_program);
        let replay = recorded
            .public
            .initial()
            .query(
                recorded.public.verify_ops(),
                None,
                &recorded.execution.context.cost_model,
            )
            .unwrap();
        assert_eq!(replay.gas_cost, recorded.execution.gas_cost);
        assert_eq!(
            replay.context.effects,
            recorded.execution.context.query.effects
        );
        assert_eq!(
            state_hex(replay.context.state.get_ref().clone()),
            state_hex(recorded.execution.context.query.state.get_ref().clone())
        );
        assert_eq!(
            runtime::ledger::historic_merkle_tree_view_at_path(
                replay.context.state.get_ref(),
                &[0],
            )
            .unwrap()
            .history()
            .unwrap(),
            runtime::ledger::historic_merkle_tree_view_at_path(
                recorded.public.initial().state.get_ref(),
                &[0],
            )
            .unwrap()
            .history()
            .unwrap()
        );
    }
    assert_eq!(oracle["fullAtInit"], false);
    assert_eq!(oracle["fullAtCapacity"], true);
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
