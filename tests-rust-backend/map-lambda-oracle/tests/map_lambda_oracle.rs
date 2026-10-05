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

use compact_rust_map_lambda_oracle_fixture::ledger_contract::{initial_state, ping, recorded};
use midnight_compact_runtime::context::ConstructorContext;
use midnight_compact_runtime::ledger::{self, ContractAddress, DefaultDB, StateValue};
use midnight_compact_runtime::{BoundedUint, FixedVector};
use midnight_onchain_state::state::{
    ContractMaintenanceAuthority, ContractOperation, ContractState, EntryPointBuf,
};
use midnight_serialize::tagged_serialize;
use midnight_storage::storage::HashMap;

fn state_hex(state: StateValue<DefaultDB>) -> String {
    let operations = HashMap::new().insert(
        EntryPointBuf(b"ping".to_vec()),
        ContractOperation::new(None),
    );
    let state = ContractState::new(state, operations, ContractMaintenanceAuthority::default());
    let mut bytes = Vec::new();
    tagged_serialize(&state, &mut bytes).unwrap();
    hex::encode(bytes)
}

fn values(state: &StateValue<DefaultDB>) -> Vec<String> {
    let vector =
        ledger::read_root_cell::<FixedVector<BoundedUint<{ u64::MAX as u128 }>, 3>, _>(state, 0)
            .unwrap();
    vector
        .into_array()
        .into_iter()
        .map(|value| value.value().to_string())
        .collect()
}

#[test]
fn vector_map_arithmetic_matches_typescript_state() {
    let reference: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/map-lambda-oracle.json"
    ))
    .unwrap();
    let initial = initial_state(ConstructorContext::new(())).unwrap();
    let state = initial.ledger_state.get_ref().clone();
    assert_eq!(state_hex(state.clone()), reference["afterInit"]["stateHex"]);
    assert_eq!(
        serde_json::to_value(values(&state)).unwrap(),
        reference["afterInit"]["values"]
    );

    let next = ping(initial.into_circuit_context(ContractAddress::default())).unwrap();
    let state = next.context.query.state.get_ref().clone();
    assert_eq!(state_hex(state.clone()), reference["afterPing"]["stateHex"]);
    assert_eq!(
        serde_json::to_value(values(&state)).unwrap(),
        reference["afterPing"]["values"]
    );
}

#[test]
fn recorded_identity_map_matches_typescript_and_replays() {
    let reference: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/map-lambda-oracle.json"
    ))
    .unwrap();
    let context = || {
        initial_state(ConstructorContext::new(()))
            .unwrap()
            .into_circuit_context(ContractAddress::default())
    };
    let native = ping(context()).unwrap();
    let recorded = recorded::ping(context()).unwrap();
    let capture = &reference["ping"];
    assert_eq!(capture["result"], serde_json::json!([]));
    assert_eq!(capture["privateOutputs"], 0);
    assert!(recorded.execution.private_transcript_outputs.is_empty());
    assert_eq!(native.gas_cost, recorded.execution.gas_cost);
    assert_eq!(
        native.context.query.effects,
        recorded.execution.context.query.effects
    );
    assert_eq!(
        state_hex(native.context.query.state.get_ref().clone()),
        reference["afterPing"]["stateHex"]
    );
    assert_eq!(
        state_hex(recorded.execution.context.query.state.get_ref().clone()),
        reference["afterPing"]["stateHex"]
    );
    let gas = serde_json::to_value(recorded.execution.gas_cost).unwrap();
    assert_eq!(capture["queries"].as_array().unwrap().len(), 1);
    for key in ["readTime", "computeTime", "bytesWritten", "bytesDeleted"] {
        assert_eq!(
            gas[key].as_u64().unwrap().to_string(),
            capture["gasCost"][key].as_str().unwrap()
        );
        assert_eq!(
            capture["gasCost"][key],
            capture["queries"][0]["gasCost"][key]
        );
    }
    assert_eq!(
        serde_json::to_value(recorded.public.verify_ops()).unwrap(),
        capture["queries"][0]["program"]
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
        reference["afterPing"]["stateHex"]
    );
}
