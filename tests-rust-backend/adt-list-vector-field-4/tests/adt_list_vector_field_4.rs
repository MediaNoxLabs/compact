// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//   http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use compact_rust_adt_list_vector_field_4_fixture::ledger_contract::{
    self, Contract, initial_state,
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
    let operations = HashMap::new().insert(
        EntryPointBuf(b"test".to_vec()),
        ContractOperation::new(None),
    );
    let state = ContractState::new(state, operations, ContractMaintenanceAuthority::default());
    let mut bytes = Vec::new();
    tagged_serialize(&state, &mut bytes).unwrap();
    hex::encode(bytes)
}

fn shape(operations: &impl serde::Serialize) -> serde_json::Value {
    let actual = serde_json::to_value(operations).unwrap();
    serde_json::Value::Array(
        actual
            .as_array()
            .unwrap()
            .iter()
            .map(|operation| {
                if let Some(kind) = operation.as_str() {
                    return serde_json::json!({"kind":kind});
                }
                if let Some(idx) = operation.get("idx") {
                    serde_json::json!({
                        "kind":"idx", "cached":idx["cached"], "pushPath":idx["pushPath"],
                        "pathLength":idx["path"].as_array().unwrap().len(),
                    })
                } else if let Some(push) = operation.get("push") {
                    serde_json::json!({"kind":"push", "storage":push["storage"]})
                } else if let Some(ins) = operation.get("ins") {
                    serde_json::json!({"kind":"ins", "cached":ins["cached"], "n":ins["n"]})
                } else if let Some(rem) = operation.get("rem") {
                    serde_json::json!({"kind":"rem", "cached":rem["cached"]})
                } else if let Some(dup) = operation.get("dup") {
                    serde_json::json!({"kind":"dup", "n":dup["n"]})
                } else if let Some(branch) = operation.get("branch") {
                    serde_json::json!({"kind":"branch", "skip":branch["skip"]})
                } else if let Some(swap) = operation.get("swap") {
                    serde_json::json!({"kind":"swap", "n":swap["n"]})
                } else if let Some(concat) = operation.get("concat") {
                    serde_json::json!({"kind":"concat", "cached":concat["cached"], "n":concat["n"]})
                } else if let Some(jmp) = operation.get("jmp") {
                    serde_json::json!({"kind":"jmp", "skip":jmp["skip"]})
                } else if let Some(addi) = operation.get("addi") {
                    serde_json::json!({"kind":"addi", "immediate":addi["immediate"]})
                } else if let Some(popeq) = operation.get("popeq") {
                    serde_json::json!({
                        "kind":"popeq", "cached":popeq["cached"],
                        "resultAtoms":popeq["result"]["value"],
                    })
                } else {
                    panic!("unexpected VM operation: {operation}");
                }
            })
            .collect(),
    )
}

#[test]
fn list_enum_recording_matches_typescript_and_replays() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/adt-list-vector-field-4.json"
    ))
    .unwrap();
    let initial = initial_state(ConstructorContext::new(())).unwrap();
    assert_eq!(
        state_hex(initial.ledger_state.get_ref().clone()),
        oracle["initialStateHex"]
    );
    let native = ledger_contract::test(
        initial_state(ConstructorContext::new(()))
            .unwrap()
            .into_circuit_context(ContractAddress::default()),
    )
    .unwrap();
    let recorded = Contract::default()
        .recording
        .test(initial.into_circuit_context(ContractAddress::default()))
        .unwrap();
    let _: () = native.result;
    let _: () = recorded.execution.result;
    assert_eq!(oracle["result"], "");
    assert_eq!(native.gas_cost, recorded.execution.gas_cost);
    assert_eq!(
        native.context.query.effects,
        recorded.execution.context.query.effects
    );
    assert_eq!(
        native.context.query.state.get_ref(),
        recorded.execution.context.query.state.get_ref(),
    );
    assert_eq!(
        state_hex(recorded.execution.context.query.state.get_ref().clone()),
        oracle["afterStateHex"],
    );
    assert_eq!(native.private_transcript_outputs.len(), 0);
    assert_eq!(recorded.execution.private_transcript_outputs.len(), 0);
    assert_eq!(oracle["privateTranscriptCount"], 0);
    assert_eq!(
        shape(&recorded.public.verify_ops()),
        oracle["publicTranscriptShape"]
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
    assert_eq!(
        replay.context.state.get_ref(),
        recorded.execution.context.query.state.get_ref(),
    );
    let actual = serde_json::to_value(native.gas_cost).unwrap();
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
        assert_eq!(
            actual[dimension], expected,
            "{dimension} differs from TypeScript"
        );
    }
}
