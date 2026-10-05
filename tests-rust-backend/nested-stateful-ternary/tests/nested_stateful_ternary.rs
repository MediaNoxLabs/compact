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

use compact_rust_nested_stateful_ternary_fixture::ledger_contract::{initial_state, recorded, run};
use midnight_compact_runtime::context::ConstructorContext;
use midnight_compact_runtime::ledger::{ContractAddress, DefaultDB, StateValue};
use midnight_onchain_state::state::{
    ContractMaintenanceAuthority, ContractOperation, ContractState, EntryPointBuf,
};
use midnight_serialize::tagged_serialize;
use midnight_storage::storage::HashMap;

fn state_hex(state: StateValue<DefaultDB>) -> String {
    let mut operations: HashMap<EntryPointBuf, ContractOperation, DefaultDB> = HashMap::new();
    operations = operations.insert(EntryPointBuf(b"run".to_vec()), ContractOperation::new(None));
    let contract_state =
        ContractState::new(state, operations, ContractMaintenanceAuthority::default());
    let mut bytes = Vec::new();
    tagged_serialize(&contract_state, &mut bytes).unwrap();
    hex::encode(bytes)
}

fn ordered_vm_shape(operations: serde_json::Value) -> serde_json::Value {
    serde_json::Value::Array(
        operations
            .as_array()
            .unwrap()
            .iter()
            .map(|operation| {
                if let Some(push) = operation.get("push") {
                    serde_json::json!({"kind":"push", "storage":push["storage"]})
                } else if let Some(ins) = operation.get("ins") {
                    serde_json::json!({"kind":"ins", "cached":ins["cached"], "n":ins["n"]})
                } else if let Some(idx) = operation.get("idx") {
                    serde_json::json!({
                        "kind":"idx", "cached":idx["cached"], "pushPath":idx["pushPath"],
                        "pathLength":idx["path"].as_array().unwrap().len(),
                    })
                } else if let Some(addi) = operation.get("addi") {
                    serde_json::json!({"kind":"addi", "immediate":addi["immediate"]})
                } else {
                    panic!("unexpected VM operation: {operation}")
                }
            })
            .collect(),
    )
}

#[test]
fn nested_stateful_recording_matches_typescript_and_replays() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/nested-stateful-recorded.json"
    ))
    .unwrap();
    for (choice, key) in [(true, "true"), (false, "false")] {
        let expected = &oracle[key];
        let native_initial = initial_state(ConstructorContext::new(())).unwrap();
        let recorded_initial = initial_state(ConstructorContext::new(())).unwrap();
        assert_eq!(
            state_hex(native_initial.ledger_state.get_ref().clone()),
            expected["initialStateHex"],
            "{key}: initial state",
        );
        let native = run(
            native_initial.into_circuit_context(ContractAddress::default()),
            choice,
        )
        .unwrap();
        let recorded = recorded::run(
            recorded_initial.into_circuit_context(ContractAddress::default()),
            choice,
        )
        .unwrap();
        let _: () = native.result;
        let _: () = recorded.execution.result;
        assert_eq!(expected["result"], "", "{key}: TypeScript result");
        assert_eq!(native.gas_cost, recorded.execution.gas_cost, "{key}: gas");
        assert_eq!(
            native.context.query.effects, recorded.execution.context.query.effects,
            "{key}: effects"
        );
        assert_eq!(
            native.context.query.state.get_ref(),
            recorded.execution.context.query.state.get_ref(),
            "{key}: state"
        );
        assert_eq!(
            state_hex(recorded.execution.context.query.state.get_ref().clone()),
            expected["afterStateHex"],
            "{key}: TypeScript state"
        );
        assert!(native.private_transcript_outputs.is_empty());
        assert!(recorded.execution.private_transcript_outputs.is_empty());
        assert_eq!(expected["privateTranscriptCount"], 0);
        assert_eq!(
            ordered_vm_shape(serde_json::to_value(recorded.public.verify_ops()).unwrap()),
            expected["publicTranscriptShape"],
            "{key}: ordered VM",
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
            native.context.query.state.get_ref()
        );
        assert_eq!(replay.context.effects, native.context.query.effects);
        let actual = serde_json::to_value(native.gas_cost).unwrap();
        for dimension in ["readTime", "computeTime", "bytesWritten", "bytesDeleted"] {
            let expected_gas: u64 = expected["queries"]
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
            assert_eq!(actual[dimension], expected_gas, "{key}: {dimension}");
            assert_eq!(
                expected["queries"].as_array().unwrap().last().unwrap()["gasCost"][dimension],
                expected["reportedGas"][dimension],
                "{key}: final TypeScript query {dimension}",
            );
        }
    }
}

#[test]
fn nested_stateful_ternary_matches_typescript_state_bytes() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/nested-stateful-ternary.json"
    ))
    .unwrap();
    for (choice, key) in [(true, "true"), (false, "false")] {
        let context = initial_state(ConstructorContext::new(()))
            .unwrap()
            .into_circuit_context(ContractAddress::default());
        let result = run(context, choice).unwrap();
        assert_eq!(
            state_hex(result.context.query.state.get_ref().clone()),
            oracle[key]
        );
    }
}
