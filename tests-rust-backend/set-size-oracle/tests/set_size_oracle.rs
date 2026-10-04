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

use compact_rust_set_size_oracle_fixture::ledger_contract::{
    check_map_empty, check_set_empty, initial_state, recorded,
};
#[path = "../../boolean_observation_assertions.rs"]
mod boolean_observation_assertions;
use midnight_compact_runtime as runtime;
use midnight_onchain_state::state::{
    ContractMaintenanceAuthority, ContractOperation, ContractState, EntryPointBuf,
};
use midnight_serialize::tagged_serialize;
use midnight_storage::storage::HashMap;
use runtime::context::ConstructorContext;
use runtime::ledger::{ContractAddress, DefaultDB, StateValue};
use runtime::recording::RecordedCircuitResult;

fn assert_replay(call: &RecordedCircuitResult<(), ()>) {
    let replay = call
        .public
        .initial()
        .query(
            call.public.verify_ops(),
            None,
            &call.execution.context.cost_model,
        )
        .unwrap();
    assert_eq!(
        replay.context.state.get_ref(),
        call.execution.context.query.state.get_ref()
    );
    assert_eq!(replay.context.effects, call.execution.context.query.effects);
}

fn state_hex(state: StateValue<DefaultDB>) -> String {
    let mut operations: HashMap<EntryPointBuf, ContractOperation, DefaultDB> = HashMap::new();
    for name in ["check_set_empty", "check_map_empty"] {
        operations = operations.insert(
            EntryPointBuf(name.as_bytes().to_vec()),
            ContractOperation::new(None),
        );
    }
    let contract_state =
        ContractState::new(state, operations, ContractMaintenanceAuthority::default());
    let mut bytes = Vec::new();
    tagged_serialize(&contract_state, &mut bytes).unwrap();
    hex::encode(bytes)
}

#[test]
fn exact_set_and_map_is_empty_oracle_matches_typescript_state() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/set-size-oracle.json"
    ))
    .unwrap();
    let initial = initial_state(ConstructorContext::new(())).unwrap();
    assert_eq!(
        state_hex(initial.ledger_state.get_ref().clone()),
        oracle["afterInit"]
    );
    let set = check_set_empty(initial.into_circuit_context(ContractAddress::default())).unwrap();
    assert_eq!(
        state_hex(set.context.query.state.get_ref().clone()),
        oracle["afterSetCheck"]
    );
    let set_flag =
        runtime::ledger::read_root_cell::<bool, _>(set.context.query.state.get_ref(), 0).unwrap();
    assert_eq!(set_flag, oracle["setFlag"]);
    let map = check_map_empty(set.context).unwrap();
    assert_eq!(
        state_hex(map.context.query.state.get_ref().clone()),
        oracle["afterMapCheck"]
    );
    let map_flag =
        runtime::ledger::read_root_cell::<bool, _>(map.context.query.state.get_ref(), 1).unwrap();
    assert_eq!(map_flag, oracle["mapFlag"]);
}

#[test]
fn recorded_set_and_map_is_empty_match_native_typescript_and_replay() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/set-size-oracle.json"
    ))
    .unwrap();
    let trace: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/set-size-boolean-observation.json"
    ))
    .unwrap();
    let native = initial_state(ConstructorContext::new(())).unwrap();
    let recording = initial_state(ConstructorContext::new(())).unwrap();
    let native = native.into_circuit_context(ContractAddress::default());
    let recording = recording.into_circuit_context(ContractAddress::default());

    let native_set = check_set_empty(native).unwrap();
    let recorded_set = recorded::check_set_empty(recording).unwrap();
    boolean_observation_assertions::assert_ts_trace(
        "set-size Set",
        &native_set,
        &recorded_set,
        &trace["empty"]["set"],
    );
    assert_replay(&recorded_set);
    assert_eq!(recorded_set.execution.gas_cost, native_set.gas_cost);
    assert_eq!(
        recorded_set.execution.context.query.effects,
        native_set.context.query.effects
    );
    assert_eq!(
        state_hex(recorded_set.execution.context.query.state.get_ref().clone()),
        oracle["afterSetCheck"]
    );
    assert!(
        runtime::ledger::read_root_cell::<bool, _>(
            recorded_set.execution.context.query.state.get_ref(),
            0
        )
        .unwrap()
    );

    let native_map = check_map_empty(native_set.context).unwrap();
    let recorded_map = recorded::check_map_empty(recorded_set.execution.context).unwrap();
    boolean_observation_assertions::assert_ts_trace(
        "set-size Map",
        &native_map,
        &recorded_map,
        &trace["empty"]["map"],
    );
    assert_replay(&recorded_map);
    assert_eq!(recorded_map.execution.gas_cost, native_map.gas_cost);
    assert_eq!(
        recorded_map.execution.context.query.effects,
        native_map.context.query.effects
    );
    assert_eq!(
        state_hex(recorded_map.execution.context.query.state.get_ref().clone()),
        oracle["afterMapCheck"]
    );
    assert!(
        runtime::ledger::read_root_cell::<bool, _>(
            recorded_map.execution.context.query.state.get_ref(),
            1
        )
        .unwrap()
    );
}
