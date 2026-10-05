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

use compact_rust_field_cast_uint128_fixture::ledger_contract::{initial_state, recorded, save};
use compact_rust_field_cast_uint128_fixture::pure_circuits::as_field;
use midnight_compact_runtime::context::ConstructorContext;
use midnight_compact_runtime::ledger::{ContractAddress, DefaultDB, StateValue};
use midnight_compact_runtime::{BoundedUint, Field};
use midnight_onchain_state::state::{
    ContractMaintenanceAuthority, ContractOperation, ContractState, EntryPointBuf,
};
use midnight_serialize::tagged_serialize;
use midnight_storage::storage::HashMap;

fn state_hex(state: StateValue<DefaultDB>) -> String {
    let operations: HashMap<EntryPointBuf, ContractOperation, DefaultDB> = HashMap::new().insert(
        EntryPointBuf(b"save".to_vec()),
        ContractOperation::new(None),
    );
    let contract_state =
        ContractState::new(state, operations, ContractMaintenanceAuthority::default());
    let mut bytes = Vec::new();
    tagged_serialize(&contract_state, &mut bytes).unwrap();
    hex::encode(bytes)
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
fn uint128_to_field_cast_preserves_high_bits_and_matches_typescript() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/field-cast-uint128.json"
    ))
    .unwrap();
    let value = oracle["input"].as_str().unwrap().parse::<u128>().unwrap();
    let input = BoundedUint::<{ u128::MAX }>::new(value).unwrap();
    let expected = Field::from(value);
    assert_eq!(as_field(input).unwrap(), expected);
    assert_eq!(oracle["pure"], value.to_string());
    let context = initial_state(ConstructorContext::new(()))
        .unwrap()
        .into_circuit_context(ContractAddress::default());
    let saved = save(context, input).unwrap();
    assert_eq!(saved.result, expected);
    assert_eq!(oracle["returned"], value.to_string());
    assert_eq!(
        state_hex(saved.context.query.state.get_ref().clone()),
        oracle["afterHex"]
    );
}

#[test]
fn recorded_wide_field_cast_matches_typescript_queries_and_replays() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/field-cast-uint128.json"
    ))
    .unwrap();
    let input = BoundedUint::<{ u128::MAX }>::new(
        oracle["input"].as_str().unwrap().parse::<u128>().unwrap(),
    )
    .unwrap();
    let native = save(
        initial_state(ConstructorContext::new(()))
            .unwrap()
            .into_circuit_context(ContractAddress::default()),
        input,
    )
    .unwrap();
    let recorded = recorded::save(
        initial_state(ConstructorContext::new(()))
            .unwrap()
            .into_circuit_context(ContractAddress::default()),
        input,
    )
    .unwrap();
    let replay = recorded
        .public
        .initial()
        .query(
            recorded.public.verify_ops(),
            None,
            &recorded.execution.context.cost_model,
        )
        .unwrap();

    assert_eq!(recorded.execution.result, native.result);
    assert_eq!(
        recorded.execution.result,
        Field::from(
            oracle["returned"]
                .as_str()
                .unwrap()
                .parse::<u128>()
                .unwrap()
        )
    );
    assert_eq!(native.gas_cost, recorded.execution.gas_cost);
    assert_eq!(
        native.context.query.effects,
        recorded.execution.context.query.effects
    );
    assert_eq!(native.context.query.effects, replay.context.effects);
    assert!(recorded.execution.private_transcript_outputs.is_empty());
    assert_eq!(oracle["privateOutputCount"], 0);
    for state in [
        native.context.query.state.get_ref(),
        recorded.execution.context.query.state.get_ref(),
        replay.context.state.get_ref(),
    ] {
        assert_eq!(state_hex(state.clone()), oracle["afterHex"]);
    }
    let rust_cost = serde_json::to_value(native.gas_cost).unwrap();
    for dimension in ["readTime", "computeTime", "bytesWritten", "bytesDeleted"] {
        let expected: u64 = oracle["queries"]
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
        assert_eq!(rust_cost[dimension], expected, "{dimension}");
    }
    let actual = serde_json::to_value(recorded.public.verify_ops()).unwrap();
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
    let expected_tags = oracle["queries"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|query| {
            query["opTags"]
                .as_array()
                .unwrap()
                .iter()
                .map(|tag| tag.as_str().unwrap())
        })
        .collect::<Vec<_>>();
    assert_eq!(tags, expected_tags);
    assert_eq!(tags, ["push", "push", "ins", "dup", "idx", "popeq"]);
    assert_eq!(normalized_ops(actual), oracle["publicTranscript"]);
}
