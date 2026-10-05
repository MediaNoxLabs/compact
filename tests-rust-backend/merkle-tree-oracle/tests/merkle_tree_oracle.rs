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

use compact_rust_merkle_tree_oracle_fixture::ledger_contract::recorded;
use compact_rust_merkle_tree_oracle_fixture::ledger_contract::{
    PublicStateView, append, append_hash, full, initial_state, known, place, place_default,
    place_hash, reset_tree,
};
use compact_rust_merkle_tree_oracle_fixture::types::MerkleTreeDigest;
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
        "place_default",
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

fn bounded<const MAX: u128>(value: u128) -> runtime::BoundedUint<MAX> {
    runtime::BoundedUint::new(value).unwrap()
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

fn current_root(state: &StateValue<DefaultDB>) -> MerkleTreeDigest {
    let root = runtime::ledger::merkle_tree_view_at_path(state, &[0])
        .unwrap()
        .root()
        .unwrap();
    MerkleTreeDigest { field: root.0 }
}

#[test]
fn recorded_direct_root_checks_match_typescript_and_replay_both_outcomes() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/merkle-tree-oracle.json"
    ))
    .unwrap();
    for (label, expected) in [
        ("knownCurrentAfterAppend", true),
        ("knownInitialAfterAppend", false),
    ] {
        let context = || {
            append(
                initial_state(ConstructorContext::new(()))
                    .unwrap()
                    .into_circuit_context(ContractAddress::default()),
                bounded::<255>(7),
            )
            .unwrap()
            .context
        };
        let root = if expected {
            current_root(context().query.state.get_ref())
        } else {
            current_root(
                initial_state(ConstructorContext::new(()))
                    .unwrap()
                    .ledger_state
                    .get_ref(),
            )
        };
        let native = known(context(), root.clone()).unwrap();
        let recorded = recorded::known(context(), root).unwrap();
        let capture = &oracle["nativeQueries"][label];
        assert_eq!(native.result, expected);
        assert_eq!(recorded.execution.result, expected);
        assert_eq!(capture["result"], expected);
        assert_eq!(native.gas_cost, recorded.execution.gas_cost);
        assert_eq!(
            native.context.query.effects,
            recorded.execution.context.query.effects
        );
        assert_native_query_gas(label, &recorded.execution.gas_cost, &oracle);
        assert!(recorded.execution.private_transcript_outputs.is_empty());
        assert_eq!(capture["privateOutputs"], 0);
        assert_eq!(
            state_hex(native.context.query.state.get_ref().clone()),
            oracle["afterAppend7"]
        );
        assert_eq!(
            state_hex(recorded.execution.context.query.state.get_ref().clone()),
            oracle["afterAppend7"]
        );
        assert_eq!(capture["queries"].as_array().unwrap().len(), 1);
        let mut program = serde_json::to_value(recorded.public.verify_ops()).unwrap();
        assert_eq!(program.as_array().unwrap().len(), 7);
        assert!(!program[6]["popeq"]["result"].is_null());
        program[6]["popeq"]["result"] = serde_json::Value::Null;
        assert_eq!(program, capture["queries"][0]["program"]);
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
            oracle["afterAppend7"]
        );
    }
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
    let tree = runtime::ledger::merkle_tree_view_at_path(state, &[0]).unwrap();
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

#[test]
fn recorded_plain_append_matches_ledger8_program_and_replays() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/merkle-tree-oracle.json"
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
}

#[test]
fn recorded_indexed_insertions_match_typescript_and_replay() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/merkle-tree-oracle.json"
    ))
    .unwrap();
    let place_start = || {
        let context = initial_state(ConstructorContext::new(()))
            .unwrap()
            .into_circuit_context(ContractAddress::default());
        append(context, bounded::<255>(7)).unwrap().context
    };
    let native = place(
        place_start(),
        bounded::<255>(9),
        bounded::<{ u64::MAX as u128 }>(3),
    )
    .unwrap();
    let recorded = recorded::place(
        place_start(),
        bounded::<255>(9),
        bounded::<{ u64::MAX as u128 }>(3),
    )
    .unwrap();
    assert_indexed_recording("place9At3", "afterPlace9At3", 4, native, recorded, &oracle);

    let default_start = || {
        let context = place(
            place_start(),
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
    let native = place_default(default_start(), bounded::<{ u64::MAX as u128 }>(6)).unwrap();
    let recorded =
        recorded::place_default(default_start(), bounded::<{ u64::MAX as u128 }>(6)).unwrap();
    assert_indexed_recording(
        "placeDefaultAt6",
        "afterDefaultAt6",
        7,
        native,
        recorded,
        &oracle,
    );
}

#[test]
fn recorded_plain_hash_append_matches_typescript_and_replays() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/merkle-tree-oracle.json"
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
        let context = place_default(context, bounded::<{ u64::MAX as u128 }>(6))
            .unwrap()
            .context;
        full(context).unwrap().context
    };
    let hash = runtime::FixedBytes::new([1; 32]);
    let native = append_hash(at_hash_append(), hash).unwrap();
    let recorded = recorded::append_hash(at_hash_append(), hash).unwrap();
    assert_indexed_recording(
        "appendHash",
        "afterAppendHash",
        8,
        native,
        recorded,
        &oracle,
    );
}

#[test]
fn recorded_plain_indexed_hash_placement_matches_typescript_and_replays() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/merkle-tree-oracle.json"
    ))
    .unwrap();
    let at_hash_placement = || {
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
        let context = place_default(context, bounded::<{ u64::MAX as u128 }>(6))
            .unwrap()
            .context;
        let context = full(context).unwrap().context;
        let context = append_hash(context, runtime::FixedBytes::new([1; 32]))
            .unwrap()
            .context;
        full(context).unwrap().context
    };
    let hash = runtime::FixedBytes::new([2; 32]);
    let index = bounded::<{ u64::MAX as u128 }>(1);
    let native = place_hash(at_hash_placement(), hash, index).unwrap();
    let recorded = recorded::place_hash(at_hash_placement(), hash, index).unwrap();
    assert_indexed_recording(
        "placeHashAt1",
        "afterReplaceHashAt1",
        8,
        native,
        recorded,
        &oracle,
    );
}

fn assert_indexed_recording(
    query: &str,
    state: &str,
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
}

#[test]
fn recorded_plain_fullness_matches_native_vm_and_replays_without_a_write() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/merkle-tree-oracle.json"
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
    }
    assert_eq!(oracle["fullAtInit"], false);
    assert_eq!(oracle["fullAtCapacity"], true);

    let context = initial_state(ConstructorContext::new(()))
        .unwrap()
        .into_circuit_context(ContractAddress::default());
    let wrong_path = runtime::slots::MerkleSlot::<runtime::BoundedUint<255>, 3, false>::new(&[9]);
    assert!(
        wrong_path
            .record_is_full(runtime::recording::RecordingFrame::new(context))
            .is_err()
    );
    let context = initial_state(ConstructorContext::new(()))
        .unwrap()
        .into_circuit_context(ContractAddress::default());
    let wrong_depth = runtime::slots::MerkleSlot::<runtime::BoundedUint<255>, 64, false>::new(&[0]);
    assert!(
        wrong_depth
            .record_is_full(runtime::recording::RecordingFrame::new(context))
            .is_err()
    );
    let mut context = initial_state(ConstructorContext::new(()))
        .unwrap()
        .into_circuit_context(ContractAddress::default());
    context.gas_limit = Some(runtime::context::RunningCost::ZERO);
    assert!(recorded::full(context).is_err());
}

#[test]
fn merkle_tree_operations_match_typescript_state_bytes() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/merkle-tree-oracle.json"
    ))
    .unwrap();
    let initial = initial_state(ConstructorContext::new(())).unwrap();
    assert_state(initial.ledger_state.get_ref(), &oracle, "afterInit", 0);
    let initial_root = current_root(initial.ledger_state.get_ref());

    let context = initial.into_circuit_context(ContractAddress::default());
    let at_init = full(context).unwrap();
    assert_eq!(at_init.result, oracle["fullAtInit"]);
    assert_native_query_gas("fullAtInit", &at_init.gas_cost, &oracle);
    let known_init = known(at_init.context, initial_root.clone()).unwrap();
    assert_eq!(known_init.result, oracle["knownAtInit"]);
    assert_native_query_gas("knownAtInit", &known_init.gas_cost, &oracle);
    let after_append7 = append(known_init.context, bounded::<255>(7)).unwrap();
    assert_native_query_gas("append7", &after_append7.gas_cost, &oracle);
    assert_state(
        after_append7.context.query.state.get_ref(),
        &oracle,
        "afterAppend7",
        1,
    );
    let tree = runtime::ledger::merkle_tree_view_at_path(
        after_append7.context.query.state.get_ref(),
        &[0],
    )
    .unwrap();
    let path = tree.path_for_leaf(0, bounded::<255>(7)).unwrap();
    let typed = PublicStateView::from(&after_append7).t().unwrap();
    assert_eq!(
        typed.path_for_leaf(0, bounded::<255>(7)).unwrap().root(),
        path.root()
    );
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
    let tree =
        runtime::ledger::merkle_tree_view_at_path(after_place9.context.query.state.get_ref(), &[0])
            .unwrap();
    let path = tree.path_for_leaf(3, bounded::<255>(9)).unwrap();
    let typed = PublicStateView::from(&after_place9).t().unwrap();
    assert_eq!(
        typed.path_for_leaf(3, bounded::<255>(9)).unwrap().root(),
        path.root()
    );
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
    let after_default =
        place_default(after_place13.context, bounded::<{ u64::MAX as u128 }>(6)).unwrap();
    assert_state(
        after_default.context.query.state.get_ref(),
        &oracle,
        "afterDefaultAt6",
        7,
    );
    let before_capacity = full(after_default.context).unwrap();
    assert_eq!(before_capacity.result, oracle["fullBeforeCapacity"]);
    let after_hash =
        append_hash(before_capacity.context, runtime::FixedBytes::new([1; 32])).unwrap();
    assert_state(
        after_hash.context.query.state.get_ref(),
        &oracle,
        "afterAppendHash",
        8,
    );
    let at_capacity = full(after_hash.context).unwrap();
    assert_eq!(at_capacity.result, oracle["fullAtCapacity"]);
    let after_replace = place_hash(
        at_capacity.context,
        runtime::FixedBytes::new([2; 32]),
        bounded::<{ u64::MAX as u128 }>(1),
    )
    .unwrap();
    assert_state(
        after_replace.context.query.state.get_ref(),
        &oracle,
        "afterReplaceHashAt1",
        8,
    );
    let full_after = full(after_replace.context).unwrap();
    assert_eq!(full_after.result, oracle["fullAfterReplacement"]);
    let pre_reset_root = current_root(full_after.context.query.state.get_ref());
    let known_current = known(full_after.context, pre_reset_root.clone()).unwrap();
    assert_eq!(known_current.result, oracle["knownCurrent"]);
    let known_initial = known(known_current.context, initial_root.clone()).unwrap();
    assert_eq!(known_initial.result, oracle["knownInitialBeforeReset"]);
    let reset = reset_tree(known_initial.context).unwrap();
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
    let full_after_reset = full(reset.context).unwrap();
    assert_eq!(full_after_reset.result, oracle["fullAfterTreeReset"]);
    let old_root = known(full_after_reset.context, pre_reset_root).unwrap();
    assert_eq!(old_root.result, oracle["knownOldAfterTreeReset"]);
    let blank_root = known(old_root.context, initial_root).unwrap();
    assert_eq!(blank_root.result, oracle["knownBlankAfterTreeReset"]);
}
