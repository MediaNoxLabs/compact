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

use compact_rust_multi_pl_call_oracle_fixture::ledger_contract::{
    initial_state, record_update, recorded,
};
use midnight_compact_runtime as runtime;
use midnight_onchain_state::state::{
    ContractMaintenanceAuthority, ContractOperation, ContractState, EntryPointBuf,
};
use midnight_serialize::tagged_serialize;
use midnight_storage::storage::HashMap;
use runtime::context::ConstructorContext;
use runtime::ledger::{ContractAddress, DefaultDB, StateValue};
use runtime::recording::RecordedCircuitResult;

fn state_hex(state: StateValue<DefaultDB>) -> String {
    let operations: HashMap<EntryPointBuf, ContractOperation, DefaultDB> = HashMap::new().insert(
        EntryPointBuf(b"record_update".to_vec()),
        ContractOperation::new(None),
    );
    let contract_state =
        ContractState::new(state, operations, ContractMaintenanceAuthority::default());
    let mut bytes = Vec::new();
    tagged_serialize(&contract_state, &mut bytes).unwrap();
    hex::encode(bytes)
}

fn values(state: &StateValue<DefaultDB>) -> (String, String, String) {
    let StateValue::Array(fields) = state else {
        panic!("expected ledger field array")
    };
    let ops = runtime::ledger::read_counter(fields.get(0).unwrap()).unwrap();
    let ver = runtime::ledger::read_counter(fields.get(1).unwrap()).unwrap();
    let updated =
        runtime::ledger::read_root_cell::<runtime::BoundedUint<{ u64::MAX as u128 }>, _>(state, 2)
            .unwrap();
    (
        ops.to_string(),
        ver.to_string(),
        updated.value().to_string(),
    )
}

fn normalized_verify_ops(recorded: &RecordedCircuitResult<(), ()>) -> serde_json::Value {
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
    let mut ops = serde_json::to_value(recorded.public.verify_ops()).unwrap();
    normalize(&mut ops);
    ops
}

fn assert_three_query_capture(
    recorded: &RecordedCircuitResult<(), ()>,
    expected: &serde_json::Value,
) {
    let queries = expected["queries"].as_array().unwrap();
    assert_eq!(queries.len(), 3);
    // The TypeScript wrapper reports the final query, while Rust reports all three.
    assert_eq!(expected["reportedGas"], queries[2]["gasCost"]);
    let actual = &recorded.execution.gas_cost;
    for (key, observed) in [
        ("readTime", actual.read_time.into_picoseconds()),
        ("computeTime", actual.compute_time.into_picoseconds()),
        ("bytesWritten", actual.bytes_written),
        ("bytesDeleted", actual.bytes_deleted),
    ] {
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
        assert_eq!(observed, total, "{key}");
    }
    let ops = normalized_verify_ops(recorded);
    assert_eq!(ops, expected["publicTranscript"]);
    let actual_tags = ops
        .as_array()
        .unwrap()
        .iter()
        .map(|op| op.as_object().unwrap().keys().next().unwrap().as_str())
        .collect::<Vec<_>>();
    let expected_tags = queries
        .iter()
        .flat_map(|query| query["opTags"].as_array().unwrap().iter())
        .map(|tag| tag.as_str().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(actual_tags, expected_tags);
    assert_eq!(expected["privateOutputCount"], 0);
    assert!(recorded.execution.private_transcript_outputs.is_empty());
}

#[test]
fn exact_multi_ledger_call_oracle_preserves_action_order() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/multi-pl-call-oracle.json"
    ))
    .unwrap();
    let initial = initial_state(ConstructorContext::new(())).unwrap();
    assert_eq!(
        state_hex(initial.ledger_state.get_ref().clone()),
        oracle["afterInit"]
    );
    let update7 = record_update(
        initial.into_circuit_context(ContractAddress::default()),
        runtime::BoundedUint::new(7).unwrap(),
    )
    .unwrap();
    assert_eq!(
        state_hex(update7.context.query.state.get_ref().clone()),
        oracle["afterUpdate7"]
    );
    let (ops, ver, updated) = values(update7.context.query.state.get_ref());
    assert_eq!(ops, oracle["fieldsAfterUpdate7"]["ops"]);
    assert_eq!(ver, oracle["fieldsAfterUpdate7"]["ver"]);
    assert_eq!(updated, oracle["fieldsAfterUpdate7"]["updated"]);
    let update13 = record_update(update7.context, runtime::BoundedUint::new(13).unwrap()).unwrap();
    assert_eq!(
        state_hex(update13.context.query.state.get_ref().clone()),
        oracle["afterUpdate13"]
    );
    let (ops, ver, updated) = values(update13.context.query.state.get_ref());
    assert_eq!(ops, oracle["fieldsAfterUpdate13"]["ops"]);
    assert_eq!(ver, oracle["fieldsAfterUpdate13"]["ver"]);
    assert_eq!(updated, oracle["fieldsAfterUpdate13"]["updated"]);
}

#[test]
fn three_public_ledger_actions_record_in_order_with_typescript_gas_and_replay() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/multi-pl-call-oracle.json"
    ))
    .unwrap();
    let initial = || {
        initial_state(ConstructorContext::new(()))
            .unwrap()
            .into_circuit_context(ContractAddress::default())
    };
    let mut native_context = initial();
    let mut recorded_context = initial();
    for (value, call, state, fields) in [
        (7_u128, "update7", "afterUpdate7", "fieldsAfterUpdate7"),
        (13_u128, "update13", "afterUpdate13", "fieldsAfterUpdate13"),
    ] {
        let input = runtime::BoundedUint::<{ u64::MAX as u128 }>::new(value).unwrap();
        let native = record_update(native_context, input).unwrap();
        let recorded = recorded::record_update(recorded_context, input).unwrap();
        let replay = recorded
            .public
            .initial()
            .query(
                recorded.public.verify_ops(),
                None,
                &recorded.execution.context.cost_model,
            )
            .unwrap();
        assert_three_query_capture(&recorded, &oracle["calls"][call]);
        assert_eq!(native.gas_cost, recorded.execution.gas_cost);
        // Replay runs the concatenated Verify program in one VM query. Its
        // query overhead differs from the three source queries captured above.
        assert_eq!(
            native.context.query.effects,
            recorded.execution.context.query.effects
        );
        assert_eq!(native.context.query.effects, replay.context.effects);
        for state_value in [
            native.context.query.state.get_ref(),
            recorded.execution.context.query.state.get_ref(),
            replay.context.state.get_ref(),
        ] {
            assert_eq!(state_hex(state_value.clone()), oracle[state]);
            let (ops, ver, updated) = values(state_value);
            assert_eq!(ops, oracle[fields]["ops"]);
            assert_eq!(ver, oracle[fields]["ver"]);
            assert_eq!(updated, oracle[fields]["updated"]);
        }
        native_context = native.context;
        recorded_context = recorded.execution.context;
    }
    assert_eq!(
        native_context.query.state.get_ref(),
        recorded_context.query.state.get_ref()
    );
    assert!(runtime::BoundedUint::<{ u64::MAX as u128 }>::new(u64::MAX as u128 + 1).is_err());
}
