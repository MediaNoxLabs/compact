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

use compact_rust_let_return_oracle_fixture::ledger_contract::{
    PublicStateView, initial_state, recorded, replace,
};
use midnight_compact_runtime as runtime;
use midnight_onchain_state::state::{
    ContractMaintenanceAuthority, ContractOperation, ContractState, EntryPointBuf,
};
use midnight_serialize::tagged_serialize;
use midnight_storage::storage::HashMap;
use runtime::Field;
use runtime::context::ConstructorContext;
use runtime::ledger::{ContractAddress, DefaultDB, StateValue};

fn oracle() -> serde_json::Value {
    serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/let-return-oracle.json"
    ))
    .unwrap()
}

fn state_hex(state: StateValue<DefaultDB>) -> String {
    let operations = HashMap::new().insert(
        EntryPointBuf(b"replace".to_vec()),
        ContractOperation::new(None),
    );
    let state = ContractState::new(state, operations, ContractMaintenanceAuthority::default());
    let mut bytes = Vec::new();
    tagged_serialize(&state, &mut bytes).unwrap();
    hex::encode(bytes)
}

fn shape(operations: serde_json::Value) -> serde_json::Value {
    serde_json::Value::Array(
        operations
            .as_array()
            .unwrap()
            .iter()
            .map(|operation| {
                if let Some(kind) = operation.as_str() {
                    return serde_json::json!({ "kind": kind });
                }
                if let Some(idx) = operation.get("idx") {
                    serde_json::json!({
                        "kind": "idx", "cached": idx["cached"],
                        "pushPath": idx["pushPath"],
                        "pathLength": idx["path"].as_array().unwrap().len(),
                    })
                } else if let Some(push) = operation.get("push") {
                    serde_json::json!({ "kind": "push", "storage": push["storage"] })
                } else if let Some(ins) = operation.get("ins") {
                    serde_json::json!({ "kind": "ins", "cached": ins["cached"], "n": ins["n"] })
                } else if let Some(dup) = operation.get("dup") {
                    serde_json::json!({ "kind": "dup", "n": dup["n"] })
                } else if let Some(rem) = operation.get("rem") {
                    serde_json::json!({ "kind": "rem", "cached": rem["cached"] })
                } else if let Some(popeq) = operation.get("popeq") {
                    serde_json::json!({
                        "kind": "popeq", "cached": popeq["cached"],
                        "resultAtoms": popeq["result"]["value"],
                    })
                } else {
                    panic!("unexpected VM operation: {operation}")
                }
            })
            .collect(),
    )
}

#[test]
fn root_let_returns_the_pre_write_value_and_replays_twice() {
    let expected = oracle();
    let native_initial = initial_state(ConstructorContext::new(())).unwrap();
    let recorded_initial = initial_state(ConstructorContext::new(())).unwrap();
    assert_eq!(
        state_hex(native_initial.ledger_state.get_ref().clone()),
        expected["initialStateHex"]
    );
    let mut native_context = native_initial.into_circuit_context(ContractAddress::default());
    let mut recorded_context = recorded_initial.into_circuit_context(ContractAddress::default());
    for (name, next, prior) in [("first", 9_u64, 0_u64), ("second", 13, 9)] {
        let reference = &expected[name];
        let native = replace(native_context, Field::from(next)).unwrap();
        let observed = recorded::replace(recorded_context, Field::from(next)).unwrap();
        assert_eq!(native.result, Field::from(prior));
        assert_eq!(observed.execution.result, native.result);
        assert_eq!(reference["result"], prior.to_string());
        assert_eq!(native.gas_cost, observed.execution.gas_cost);
        assert_eq!(
            native.context.query.effects,
            observed.execution.context.query.effects
        );
        assert_eq!(native.private_transcript_outputs.len(), 0);
        assert_eq!(observed.execution.private_transcript_outputs.len(), 0);
        assert_eq!(reference["privateTranscriptCount"], 0);
        assert_eq!(
            shape(serde_json::to_value(observed.public.verify_ops()).unwrap()),
            reference["publicTranscriptShape"],
            "{name}: ordered TypeScript VM",
        );
        let replay = observed
            .public
            .initial()
            .query(
                observed.public.verify_ops(),
                None,
                &observed.execution.context.cost_model,
            )
            .unwrap();
        assert_eq!(
            replay.context.state.get_ref(),
            native.context.query.state.get_ref()
        );
        assert_eq!(
            state_hex(native.context.query.state.get_ref().clone()),
            reference["afterStateHex"],
            "{name}: TypeScript state",
        );
        assert_eq!(
            PublicStateView::from(&native).stored().unwrap(),
            Field::from(next),
        );
        let gas = serde_json::to_value(native.gas_cost).unwrap();
        for dimension in ["readTime", "computeTime", "bytesWritten", "bytesDeleted"] {
            // The TypeScript circuit's reportedGas currently contains only
            // the final write query for this root-Let shape. Compare the
            // complete query meter instead of that incomplete aggregate.
            let oracle_total: u64 = reference["queries"]
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
                gas[dimension].as_u64().unwrap(),
                oracle_total,
                "{name}: {dimension}",
            );
        }
        native_context = native.context;
        recorded_context = observed.execution.context;
    }
}
