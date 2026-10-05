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

use compact_rust_opaque_string_set_oracle_fixture::ledger_contract::{
    addName, hasName, initial_state, recorded,
};
use midnight_compact_runtime::OpaqueString;
use midnight_compact_runtime::context::CircuitResult;
use midnight_compact_runtime::context::ConstructorContext;
use midnight_compact_runtime::ledger::{ContractAddress, DefaultDB, StateValue};
use midnight_compact_runtime::recording::RecordedCircuitResult;
use midnight_onchain_state::state::{
    ContractMaintenanceAuthority, ContractOperation, ContractState, EntryPointBuf,
};
use midnight_serialize::tagged_serialize;
use midnight_storage::storage::HashMap;

fn state_hex(state: StateValue<DefaultDB>) -> String {
    let mut operations: HashMap<EntryPointBuf, ContractOperation, DefaultDB> = HashMap::new();
    for name in ["addName", "hasName"] {
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

#[test]
fn variable_length_set_key_matches_typescript_state() {
    let reference: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/opaque-string-set-oracle.json"
    ))
    .unwrap();
    let initial = initial_state(ConstructorContext::new(7_u64)).unwrap();
    assert_eq!(
        state_hex(initial.ledger_state.get_ref().clone()),
        reference["initialHex"]
    );
    let context = initial.into_circuit_context(ContractAddress::default());
    let key = OpaqueString::from("registry-key");
    let before = hasName(context, key.clone()).unwrap();
    assert_eq!(before.result, reference["before"].as_bool().unwrap());
    let added = addName(before.context, key.clone()).unwrap();
    assert_eq!(
        state_hex(added.context.query.state.get_ref().clone()),
        reference["afterAddHex"]
    );
    let after = hasName(added.context, key).unwrap();
    assert_eq!(after.result, reference["after"].as_bool().unwrap());
    let other = hasName(after.context, OpaqueString::from("other")).unwrap();
    assert_eq!(other.result, reference["other"].as_bool().unwrap());
}

fn vm_shape(actual: serde_json::Value) -> serde_json::Value {
    serde_json::Value::Array(
        actual
            .as_array()
            .unwrap()
            .iter()
            .map(|operation| {
                if let Some(kind) = operation.as_str() {
                    return serde_json::json!({"kind": kind});
                }
                if let Some(idx) = operation.get("idx") {
                    serde_json::json!({"kind":"idx", "cached":idx["cached"], "pushPath":idx["pushPath"], "pathLength":idx["path"].as_array().unwrap().len()})
                } else if let Some(push) = operation.get("push") {
                    serde_json::json!({"kind":"push", "storage":push["storage"]})
                } else if let Some(ins) = operation.get("ins") {
                    serde_json::json!({"kind":"ins", "cached":ins["cached"], "n":ins["n"]})
                } else if let Some(dup) = operation.get("dup") {
                    serde_json::json!({"kind":"dup", "n":dup["n"]})
                } else if let Some(popeq) = operation.get("popeq") {
                    serde_json::json!({"kind":"popeq", "cached":popeq["cached"], "resultAtoms":popeq["result"]["value"]})
                } else {
                    panic!("unexpected VM operation: {operation}")
                }
            })
            .collect(),
    )
}

fn check_call<Output>(
    oracle: &serde_json::Value,
    native: &CircuitResult<u64, Output>,
    recorded: &RecordedCircuitResult<u64, Output>,
) {
    for execution in [native, &recorded.execution] {
        assert_eq!(
            state_hex(execution.context.query.state.get_ref().clone()),
            oracle["afterStateHex"]
        );
        assert_eq!(execution.context.private_state, 7);
        assert!(execution.private_transcript_outputs.is_empty());
        let cost = serde_json::to_value(execution.gas_cost).unwrap();
        for dimension in ["readTime", "computeTime", "bytesWritten", "bytesDeleted"] {
            let oracle_total: u64 = oracle["queries"]
                .as_array()
                .unwrap()
                .iter()
                .map(|query| {
                    query["gasCost"][dimension]
                        .as_str()
                        .unwrap()
                        .parse::<u64>()
                        .unwrap()
                })
                .sum();
            assert_eq!(
                cost[dimension].as_u64().unwrap(),
                oracle_total,
                "{dimension}"
            );
        }
    }
    assert_eq!(native.gas_cost, recorded.execution.gas_cost);
    assert_eq!(
        native.context.query.effects,
        recorded.execution.context.query.effects
    );
    assert_eq!(
        recorded.public.verify_ops().len(),
        oracle["publicTranscriptShape"].as_array().unwrap().len()
    );
    assert_eq!(
        vm_shape(serde_json::to_value(recorded.public.verify_ops()).unwrap()),
        oracle["publicTranscriptShape"]
    );
    assert_eq!(oracle["privateTranscriptCount"], 0);
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
        replay.context.state.get_ref(),
        recorded.execution.context.query.state.get_ref()
    );
    assert_eq!(
        replay.context.effects,
        recorded.execution.context.query.effects
    );
}

#[test]
fn closed_opaque_set_calls_match_typescript_and_replay() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/opaque-string-set-oracle.json"
    ))
    .unwrap();
    let native_initial = initial_state(ConstructorContext::new(7_u64)).unwrap();
    let recorded_initial = initial_state(ConstructorContext::new(7_u64)).unwrap();
    assert_eq!(
        state_hex(native_initial.ledger_state.get_ref().clone()),
        oracle["initialHex"]
    );
    assert_eq!(
        state_hex(recorded_initial.ledger_state.get_ref().clone()),
        oracle["initialHex"]
    );
    let key = OpaqueString::from("registry-key");
    let native_before = hasName(
        native_initial.into_circuit_context(ContractAddress::default()),
        key.clone(),
    )
    .unwrap();
    let recorded_before = recorded::hasName(
        recorded_initial.into_circuit_context(ContractAddress::default()),
        key.clone(),
    )
    .unwrap();
    assert!(!native_before.result);
    assert_eq!(native_before.result, recorded_before.execution.result);
    check_call(&oracle["calls"]["before"], &native_before, &recorded_before);

    let native_add = addName(native_before.context, key.clone()).unwrap();
    let recorded_add = recorded::addName(recorded_before.execution.context, key.clone()).unwrap();
    let _: () = native_add.result;
    let _: () = recorded_add.execution.result;
    assert_eq!(oracle["calls"]["add"]["result"], serde_json::json!([]));
    check_call(&oracle["calls"]["add"], &native_add, &recorded_add);

    let native_after = hasName(native_add.context, key.clone()).unwrap();
    let recorded_after = recorded::hasName(recorded_add.execution.context, key).unwrap();
    assert!(native_after.result);
    assert_eq!(native_after.result, recorded_after.execution.result);
    check_call(&oracle["calls"]["after"], &native_after, &recorded_after);

    let native_other = hasName(native_after.context, OpaqueString::from("other")).unwrap();
    let recorded_other = recorded::hasName(
        recorded_after.execution.context,
        OpaqueString::from("other"),
    )
    .unwrap();
    assert!(!native_other.result);
    assert_eq!(native_other.result, recorded_other.execution.result);
    check_call(&oracle["calls"]["other"], &native_other, &recorded_other);
}
