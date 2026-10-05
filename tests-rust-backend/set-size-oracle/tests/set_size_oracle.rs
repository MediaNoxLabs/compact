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
use compact_rust_set_size_oracle_fixture::ledger_slots;
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

#[test]
fn nonempty_set_and_map_checks_write_false_with_exact_recorded_trace() {
    let capture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/adr221-set-size-nonempty.json"
    ))
    .unwrap();
    assert_eq!(capture["mode"], "set-size");
    let rows = capture["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 2);
    for row in rows {
        let name = row["name"].as_str().unwrap();
        let seed = || {
            let context = initial_state(ConstructorContext::new(()))
                .unwrap()
                .into_circuit_context(ContractAddress::default());
            let context = ledger_slots::s
                .insert(context, runtime::Field::from(42_u64))
                .unwrap()
                .context;
            ledger_slots::m
                .insert(
                    context,
                    runtime::Field::from(7_u64),
                    runtime::Field::from(9_u64),
                )
                .unwrap()
                .context
        };
        let native = seed();
        let recording = seed();
        assert_eq!(
            state_hex(native.query.state.get_ref().clone()),
            row["before"],
            "{name}: seeded state"
        );
        let (native, recorded) = match name {
            "check_set_empty" => (
                check_set_empty(native).unwrap(),
                recorded::check_set_empty(recording).unwrap(),
            ),
            "check_map_empty" => (
                check_map_empty(native).unwrap(),
                recorded::check_map_empty(recording).unwrap(),
            ),
            _ => panic!("unexpected ADR221 export {name}"),
        };
        assert_eq!(row["result"], "");
        let _: () = native.result;
        let _: () = recorded.execution.result;
        boolean_observation_assertions::assert_ts_trace(name, &native, &recorded, row);
        assert_eq!(
            serde_json::to_value(recorded.public.verify_ops()).unwrap(),
            row["publicTranscript"],
            "{name}: complete ordered TypeScript public program"
        );
        assert_eq!(
            native.context.query.state,
            recorded.execution.context.query.state
        );
        assert_eq!(
            native.context.query.effects,
            recorded.execution.context.query.effects
        );
        assert_eq!(
            serde_json::to_value(&native.context.query.effects).unwrap(),
            row["effects"]
        );
        assert!(native.private_transcript_outputs.is_empty());
        assert!(recorded.execution.private_transcript_outputs.is_empty());
        assert_eq!(
            state_hex(native.context.query.state.get_ref().clone()),
            row["after"],
            "{name}: TS state"
        );
        assert_replay(&recorded);
        assert_eq!(row["flags"]["set"], false);
        assert_eq!(row["flags"]["map"], false);
        assert_eq!(row["flags"]["setSize"], "1");
        assert_eq!(row["flags"]["mapSize"], "1");
        assert!(
            !ledger_slots::flag_set
                .inspect(native.context.query.state.get_ref())
                .unwrap()
        );
        assert!(
            !ledger_slots::flag_map
                .inspect(native.context.query.state.get_ref())
                .unwrap()
        );
        let actual = serde_json::to_value(native.gas_cost).unwrap();
        for dimension in ["readTime", "computeTime", "bytesWritten", "bytesDeleted"] {
            let expected = row["queryCostSum"][dimension]
                .as_str()
                .unwrap()
                .parse::<u64>()
                .unwrap();
            assert_eq!(actual[dimension], expected, "{name}: {dimension}");
            assert_eq!(
                row["queries"].as_array().unwrap().last().unwrap()["gasCost"][dimension],
                row["reportedGas"][dimension]
            );
        }
    }
}
