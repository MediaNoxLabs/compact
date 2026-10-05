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

use compact_rust_widening_arith_oracle_fixture::ledger_contract::{
    initial_state, recordArea, recorded,
};
use compact_rust_widening_arith_oracle_fixture::pure_circuits::{
    ageThresholdDays, areaOf, productBytes, sumBytes,
};
use midnight_compact_runtime as runtime;
use midnight_onchain_state::state::{
    ContractMaintenanceAuthority, ContractOperation, ContractState, EntryPointBuf,
};
use midnight_serialize::tagged_serialize;
use midnight_storage::storage::HashMap;
use runtime::context::ConstructorContext;
use runtime::ledger::{ContractAddress, DefaultDB, StateValue};

fn state_hex(state: StateValue<DefaultDB>) -> String {
    let operations: HashMap<EntryPointBuf, ContractOperation, DefaultDB> = HashMap::new().insert(
        EntryPointBuf(b"recordArea".to_vec()),
        ContractOperation::new(None),
    );
    let contract_state =
        ContractState::new(state, operations, ContractMaintenanceAuthority::default());
    let mut bytes = Vec::new();
    tagged_serialize(&contract_state, &mut bytes).unwrap();
    hex::encode(bytes)
}

fn count(state: &StateValue<DefaultDB>) -> String {
    let StateValue::Array(fields) = state else {
        panic!("expected ledger field array")
    };
    runtime::ledger::read_counter(fields.get(0).unwrap())
        .unwrap()
        .to_string()
}

fn normalized_ops(mut value: serde_json::Value) -> serde_json::Value {
    fn normalize(value: &mut serde_json::Value) {
        match value {
            serde_json::Value::Object(object) => {
                if object.contains_key("alignment")
                    && let Some(serde_json::Value::Array(chunks)) = object.get_mut("value")
                {
                    for chunk in chunks {
                        if let serde_json::Value::Array(bytes) = chunk {
                            let bytes = bytes
                                .iter()
                                .map(|byte| byte.as_u64().unwrap() as u8)
                                .collect::<Vec<_>>();
                            *chunk = serde_json::json!({ "bytesHex": hex::encode(bytes) });
                        }
                    }
                }
                for child in object.values_mut() {
                    normalize(child);
                }
            }
            serde_json::Value::Array(values) => {
                for child in values {
                    normalize(child);
                }
            }
            _ => {}
        }
    }
    normalize(&mut value);
    value
}

#[test]
fn exact_widening_arithmetic_oracle_matches_typescript_boundaries() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/widening-arith-oracle.json"
    ))
    .unwrap();
    let byte = runtime::BoundedUint::<255>::new(255).unwrap();
    let word = runtime::BoundedUint::<65535>::new(65535).unwrap();
    assert_eq!(
        sumBytes(byte, byte).unwrap().value().to_string(),
        oracle["sumBytes"]
    );
    assert_eq!(
        ageThresholdDays(byte).unwrap().value().to_string(),
        oracle["ageThresholdDays"]
    );
    assert_eq!(
        productBytes(byte, byte).unwrap().value().to_string(),
        oracle["productBytes"]
    );
    assert_eq!(
        areaOf(word, word).unwrap().value().to_string(),
        oracle["areaOf"]
    );

    let initial = initial_state(ConstructorContext::new(())).unwrap();
    assert_eq!(
        state_hex(initial.ledger_state.get_ref().clone()),
        oracle["afterInit"]
    );
    let max = recordArea(
        initial.into_circuit_context(ContractAddress::default()),
        word,
        word,
    )
    .unwrap();
    assert_eq!(
        state_hex(max.context.query.state.get_ref().clone()),
        oracle["afterRecordMax"]
    );
    assert_eq!(
        count(max.context.query.state.get_ref()),
        oracle["countAfterRecordMax"]
    );
    let small = recordArea(
        max.context,
        runtime::BoundedUint::new(5).unwrap(),
        runtime::BoundedUint::new(7).unwrap(),
    )
    .unwrap();
    assert_eq!(
        state_hex(small.context.query.state.get_ref().clone()),
        oracle["afterRecordSmall"]
    );
    assert_eq!(
        count(small.context.query.state.get_ref()),
        oracle["countAfterRecordSmall"]
    );
}

#[test]
fn recorded_unsigned_pure_call_preserves_counter_trace_and_gas() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/widening-arith-oracle.json"
    ))
    .unwrap();
    let mut native_context = initial_state(ConstructorContext::new(()))
        .unwrap()
        .into_circuit_context(ContractAddress::default());
    let mut recorded_context = initial_state(ConstructorContext::new(()))
        .unwrap()
        .into_circuit_context(ContractAddress::default());
    for (width, height, state_key, queries_key, transcript_key, private_key) in [
        (
            65535,
            65535,
            "afterRecordMax",
            "maxQueries",
            "maxPublicTranscript",
            "maxPrivateOutputCount",
        ),
        (
            5,
            7,
            "afterRecordSmall",
            "smallQueries",
            "smallPublicTranscript",
            "smallPrivateOutputCount",
        ),
    ] {
        let width = runtime::BoundedUint::<65535>::new(width).unwrap();
        let height = runtime::BoundedUint::<65535>::new(height).unwrap();
        let native = recordArea(native_context, width, height).unwrap();
        let call = recorded::recordArea(recorded_context, width, height).unwrap();
        let replay = call
            .public
            .initial()
            .query(
                call.public.verify_ops(),
                None,
                &call.execution.context.cost_model,
            )
            .unwrap();
        assert_eq!(native.gas_cost, call.execution.gas_cost);
        assert_eq!(native.gas_cost, replay.gas_cost);
        assert_eq!(
            native.context.query.effects,
            call.execution.context.query.effects
        );
        assert_eq!(native.context.query.effects, replay.context.effects);
        assert_eq!(oracle[private_key], 0);
        assert!(call.execution.private_transcript_outputs.is_empty());
        let rust_cost = serde_json::to_value(native.gas_cost).unwrap();
        let queries = oracle[queries_key].as_array().unwrap();
        assert_eq!(queries.len(), 1);
        for dimension in ["readTime", "computeTime", "bytesWritten", "bytesDeleted"] {
            assert_eq!(
                rust_cost[dimension].to_string(),
                queries[0]["gasCost"][dimension].as_str().unwrap()
            );
        }
        let actual = serde_json::to_value(call.public.verify_ops()).unwrap();
        let tags = actual
            .as_array()
            .unwrap()
            .iter()
            .map(|operation| {
                operation
                    .as_object()
                    .unwrap()
                    .keys()
                    .next()
                    .unwrap()
                    .as_str()
            })
            .collect::<Vec<_>>();
        assert_eq!(serde_json::json!(tags), queries[0]["opTags"]);
        assert_eq!(tags, ["idx", "addi", "ins"]);
        assert_eq!(
            actual.as_array().unwrap().len(),
            oracle[transcript_key].as_array().unwrap().len()
        );
        assert_eq!(normalized_ops(actual), oracle[transcript_key]);
        for state in [
            native.context.query.state.get_ref(),
            call.execution.context.query.state.get_ref(),
            replay.context.state.get_ref(),
        ] {
            assert_eq!(state_hex(state.clone()), oracle[state_key]);
        }
        native_context = native.context;
        recorded_context = call.execution.context;
    }
}
