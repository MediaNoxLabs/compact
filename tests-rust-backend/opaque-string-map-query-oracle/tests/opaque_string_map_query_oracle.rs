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

use compact_rust_opaque_string_map_query_oracle_fixture::ledger_contract::{
    ensure, initial_state, put, recorded,
};
use midnight_compact_runtime::context::{CircuitResult, ConstructorContext};
use midnight_compact_runtime::ledger::{ContractAddress, DefaultDB, StateValue};
use midnight_compact_runtime::recording::RecordedCircuitResult;
use midnight_compact_runtime::{Field, OpaqueString};
use midnight_onchain_state::state::{
    ContractMaintenanceAuthority, ContractOperation, ContractState, EntryPointBuf,
};
use midnight_serialize::tagged_serialize;
use midnight_storage::storage::HashMap;

fn state_hex(state: StateValue<DefaultDB>) -> String {
    let mut operations: HashMap<EntryPointBuf, ContractOperation, DefaultDB> = HashMap::new();
    for name in ["ensure", "put"] {
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
fn nested_map_queries_match_typescript_with_string_key() {
    let reference: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/opaque-string-map-query-oracle.json"
    ))
    .unwrap();
    let initial = initial_state(ConstructorContext::new(7_u64)).unwrap();
    assert_eq!(
        state_hex(initial.ledger_state.get_ref().clone()),
        reference["initialHex"]
    );
    let context = initial.into_circuit_context(ContractAddress::default());
    let key = OpaqueString::from("asset-1");
    let error = ensure(context, key.clone()).err().unwrap();
    assert_eq!(
        error.to_string(),
        reference["beforeError"].as_str().unwrap()
    );
    let context = initial_state(ConstructorContext::new(7_u64))
        .unwrap()
        .into_circuit_context(ContractAddress::default());
    let written = put(context, key.clone(), Field::from(42_u64)).unwrap();
    assert_eq!(
        state_hex(written.context.query.state.get_ref().clone()),
        reference["afterPutHex"]
    );
    let value = ensure(written.context, key).unwrap();
    assert_eq!(value.result, Field::from(42_u64));
    assert_eq!(reference["result"], "42");
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
fn closed_opaque_map_calls_match_typescript_and_replay() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/opaque-string-map-query-oracle.json"
    ))
    .unwrap();
    let key = OpaqueString::from("asset-1");
    for recorded_mode in [false, true] {
        let initial = initial_state(ConstructorContext::new(7_u64)).unwrap();
        assert_eq!(
            state_hex(initial.ledger_state.get_ref().clone()),
            oracle["initialHex"]
        );
        let context = initial.into_circuit_context(ContractAddress::default());
        let error = if recorded_mode {
            recorded::ensure(context, key.clone()).err().unwrap()
        } else {
            ensure(context, key.clone()).err().unwrap()
        };
        assert_eq!(error.to_string(), oracle["calls"]["before"]["error"]);
        assert_eq!(
            oracle["calls"]["before"]["afterStateHex"],
            oracle["initialHex"]
        );
        assert_eq!(
            oracle["calls"]["before"]["queries"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
    }

    let native_initial = initial_state(ConstructorContext::new(7_u64)).unwrap();
    let recorded_initial = initial_state(ConstructorContext::new(7_u64)).unwrap();
    let native_put = put(
        native_initial.into_circuit_context(ContractAddress::default()),
        key.clone(),
        Field::from(42_u64),
    )
    .unwrap();
    let recorded_put = recorded::put(
        recorded_initial.into_circuit_context(ContractAddress::default()),
        key.clone(),
        Field::from(42_u64),
    )
    .unwrap();
    let _: () = native_put.result;
    let _: () = recorded_put.execution.result;
    assert_eq!(oracle["calls"]["put"]["result"], serde_json::json!([]));
    check_call(&oracle["calls"]["put"], &native_put, &recorded_put);

    let native_ensure = ensure(native_put.context, key.clone()).unwrap();
    let recorded_ensure = recorded::ensure(recorded_put.execution.context, key.clone()).unwrap();
    assert_eq!(native_ensure.result, Field::from(42_u64));
    assert_eq!(native_ensure.result, recorded_ensure.execution.result);
    assert_eq!(oracle["calls"]["ensure"]["result"], "42");
    check_call(&oracle["calls"]["ensure"], &native_ensure, &recorded_ensure);

    for recorded_mode in [false, true] {
        let initial = initial_state(ConstructorContext::new(7_u64)).unwrap();
        let context = initial.into_circuit_context(ContractAddress::default());
        let written = put(context, key.clone(), Field::from(42_u64)).unwrap();
        let error = if recorded_mode {
            recorded::ensure(written.context, OpaqueString::from("other"))
                .err()
                .unwrap()
        } else {
            ensure(written.context, OpaqueString::from("other"))
                .err()
                .unwrap()
        };
        assert_eq!(error.to_string(), oracle["calls"]["other"]["error"]);
        assert_eq!(
            oracle["calls"]["other"]["afterStateHex"],
            oracle["afterPutHex"]
        );
        assert_eq!(
            oracle["calls"]["other"]["queries"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
    }
}
