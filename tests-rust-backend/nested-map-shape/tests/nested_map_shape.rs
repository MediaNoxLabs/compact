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

use compact_rust_nested_map_shape_fixture::ledger_contract::{Contract, initial_state};
use compact_rust_nested_map_shape_fixture::ledger_slots::users_by_org;
use midnight_compact_runtime::context::ConstructorContext;
use midnight_compact_runtime::ledger::{ContractAddress, DefaultDB, StateValue};
use midnight_onchain_state::state::{
    ContractMaintenanceAuthority, ContractOperation, ContractState, EntryPointBuf,
};
use midnight_serialize::tagged_serialize;
use midnight_storage::storage::HashMap;

fn state_hex(state: StateValue<DefaultDB>) -> String {
    let operations = HashMap::new().insert(
        EntryPointBuf(b"check_nested_empty".to_vec()),
        ContractOperation::new(None),
    );
    let state = ContractState::new(state, operations, ContractMaintenanceAuthority::default());
    let mut bytes = Vec::new();
    tagged_serialize(&state, &mut bytes).unwrap();
    hex::encode(bytes)
}

#[test]
fn nested_map_shape_query_is_typed_and_replayable() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/nested-map-shape.json"
    ))
    .unwrap();
    assert_eq!(users_by_org.path(), &[0]);
    let initial = initial_state(ConstructorContext::new(())).unwrap();
    assert_eq!(
        state_hex(initial.ledger_state.get_ref().clone()),
        oracle["afterInit"]
    );
    let native = Contract::default()
        .check_nested_empty(initial.into_circuit_context(ContractAddress::default()))
        .unwrap();
    assert_eq!(native.result, oracle["result"]);
    assert_eq!(
        state_hex(native.context.query.state.get_ref().clone()),
        oracle["afterCall"]
    );
    let native_gas = serde_json::to_value(native.gas_cost).unwrap();
    for key in ["readTime", "computeTime", "bytesWritten", "bytesDeleted"] {
        assert_eq!(
            native_gas[key].as_u64().unwrap().to_string(),
            oracle["gas"][key],
            "{key} TypeScript gas mismatch"
        );
    }

    let initial = initial_state(ConstructorContext::new(())).unwrap();
    let recorded = Contract::default()
        .recording
        .check_nested_empty(initial.into_circuit_context(ContractAddress::default()))
        .unwrap();
    assert!(recorded.execution.result);
    assert_eq!(
        recorded.execution.private_transcript_outputs.len(),
        oracle["privateOutputCount"].as_u64().unwrap() as usize
    );
    assert_eq!(recorded.execution.gas_cost, native.gas_cost);
    assert_eq!(
        recorded.execution.context.query.state.get_ref(),
        native.context.query.state.get_ref(),
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
}
