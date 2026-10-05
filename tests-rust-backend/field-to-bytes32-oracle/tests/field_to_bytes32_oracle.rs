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

use compact_rust_field_to_bytes32_oracle_fixture::{ledger_contract, pure_circuits};
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
        "../../../runtime-rs/tests/fixtures/field-to-bytes32-oracle.json"
    ))
    .unwrap()
}

fn state_hex(state: StateValue<DefaultDB>) -> String {
    let operations = HashMap::new().insert(
        EntryPointBuf(b"snapshot".to_vec()),
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
                } else if let Some(dup) = operation.get("dup") {
                    serde_json::json!({ "kind": "dup", "n": dup["n"] })
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
fn explicit_field_cast_matches_typescript_for_boundaries_and_counter_read() {
    let expected = oracle();
    let max_bytes =
        hex::decode("00000000fffffffffe5bfeff02a4bd5305d8a10908d83933487d9d2953a7ed73").unwrap();
    let max_field = Field::from_le_bytes(&max_bytes).expect("modulus minus one");
    for (index, value) in [Field::from(0_u64), Field::from(257_u64), max_field]
        .into_iter()
        .enumerate()
    {
        let rust = pure_circuits::encode(value).unwrap();
        assert_eq!(
            hex::encode(rust.into_array()),
            expected["pure"][index]["resultHex"],
            "pure Field-to-Bytes32 case {index}",
        );
    }

    let native_initial = ledger_contract::initial_state(ConstructorContext::new(())).unwrap();
    let recorded_initial = ledger_contract::initial_state(ConstructorContext::new(())).unwrap();
    assert_eq!(
        state_hex(native_initial.ledger_state.get_ref().clone()),
        expected["initialStateHex"],
    );
    let native =
        ledger_contract::snapshot(native_initial.into_circuit_context(ContractAddress::default()))
            .unwrap();
    let observed = ledger_contract::recorded::snapshot(
        recorded_initial.into_circuit_context(ContractAddress::default()),
    )
    .unwrap();
    let snapshot = &expected["snapshot"];
    assert_eq!(native.result, observed.execution.result);
    assert_eq!(
        hex::encode(native.result.into_array()),
        snapshot["resultHex"]
    );
    assert_eq!(native.gas_cost, observed.execution.gas_cost);
    assert_eq!(
        native.context.query.effects,
        observed.execution.context.query.effects
    );
    assert_eq!(
        shape(serde_json::to_value(observed.public.verify_ops()).unwrap()),
        snapshot["publicTranscriptShape"],
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
        native.context.query.state.get_ref(),
    );
    assert_eq!(
        state_hex(native.context.query.state.get_ref().clone()),
        snapshot["afterStateHex"],
    );
    assert_eq!(native.private_transcript_outputs.len(), 0);
    assert_eq!(observed.execution.private_transcript_outputs.len(), 0);
    assert_eq!(snapshot["privateTranscriptCount"], 0);
    let gas = serde_json::to_value(native.gas_cost).unwrap();
    for dimension in ["readTime", "computeTime", "bytesWritten", "bytesDeleted"] {
        assert_eq!(
            gas[dimension].as_u64().unwrap().to_string(),
            snapshot["reportedGas"][dimension].as_str().unwrap(),
        );
    }
    assert_eq!(
        ledger_contract::PublicStateView::from(&native)
            .instance()
            .unwrap()
            .value(),
        257,
    );
}
