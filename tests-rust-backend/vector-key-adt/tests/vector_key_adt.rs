// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use compact_rust_vector_key_adt_fixture::ledger_contract::{
    initial_state, mapInsert, mapInsertDefault, mapLookup, mapMember, mapRemove, recorded,
    setInsert, setMember, setRemove,
};
use midnight_compact_runtime as runtime;
use midnight_onchain_state::state::{
    ContractMaintenanceAuthority, ContractOperation, ContractState, EntryPointBuf,
};
use midnight_serialize::tagged_serialize;
use midnight_storage::storage::HashMap;
use runtime::context::ConstructorContext;
use runtime::ledger::{ContractAddress, DefaultDB, StateValue};

const CIRCUITS: &[&str] = &[
    "setInsert",
    "setMember",
    "setRemove",
    "mapInsert",
    "mapMember",
    "mapLookup",
    "mapRemove",
    "mapInsertDefault",
];

fn state_hex(state: StateValue<DefaultDB>) -> String {
    let mut operations: HashMap<EntryPointBuf, ContractOperation, DefaultDB> = HashMap::new();
    for name in CIRCUITS {
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

fn assert_recorded_step(
    name: &str,
    state: &str,
    capture_name: &str,
    native: &runtime::context::CircuitResult<(), ()>,
    call: &runtime::recording::RecordedCircuitResult<(), ()>,
    oracle: &serde_json::Value,
) {
    let capture = &oracle["nativeQueries"][capture_name];
    let queries = capture["queries"].as_array().unwrap();
    let expected_program = serde_json::Value::Array(
        queries
            .iter()
            .flat_map(|query| query["program"].as_array().unwrap().iter().cloned())
            .collect(),
    );
    let actual_program = serde_json::to_value(call.public.verify_ops()).unwrap();
    let actual_ops = actual_program.as_array().unwrap();
    let expected_ops = expected_program.as_array().unwrap();
    assert_eq!(
        actual_ops.len(),
        expected_ops.len(),
        "{name}: VM program length"
    );
    for (index, (actual, expected)) in actual_ops.iter().zip(expected_ops).enumerate() {
        assert_eq!(actual, expected, "{name}: VM operation {index}");
    }
    let actual_gas = serde_json::to_value(call.execution.gas_cost).unwrap();
    for dimension in ["readTime", "computeTime", "bytesWritten", "bytesDeleted"] {
        let oracle_gas: u64 = queries
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
            actual_gas[dimension].as_u64().unwrap(),
            oracle_gas,
            "{name}: {dimension}"
        );
        assert_eq!(
            capture["reportedGas"][dimension],
            queries.last().unwrap()["gasCost"][dimension]
        );
    }
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
        call.execution.gas_cost, native.gas_cost,
        "{name}: native gas"
    );
    assert_eq!(
        call.execution.context.query.effects,
        native.context.query.effects
    );
    assert_eq!(replay.context.effects, native.context.query.effects);
    assert_eq!(
        state_hex(call.execution.context.query.state.get_ref().clone()),
        oracle[state],
        "{name}: state",
    );
    assert_eq!(
        state_hex(replay.context.state.get_ref().clone()),
        oracle[state],
        "{name}: replayed state",
    );
}

#[test]
fn vector_key_recorded_calls_match_native_and_replay() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/vector-key-adt.json"
    ))
    .unwrap();
    let mut native_context = initial_state(ConstructorContext::new(()))
        .unwrap()
        .into_circuit_context(ContractAddress::default());
    let mut recorded_context = initial_state(ConstructorContext::new(()))
        .unwrap()
        .into_circuit_context(ContractAddress::default());

    macro_rules! check_step {
        ($name:ident, $state:literal, $capture:literal) => {{
            let native = $name(native_context).unwrap();
            let call = recorded::$name(recorded_context).unwrap();
            assert_recorded_step(stringify!($name), $state, $capture, &native, &call, &oracle);
            native_context = native.context;
            recorded_context = call.execution.context;
        }};
    }

    check_step!(setInsert, "afterSetInsert", "setInsert");
    check_step!(setMember, "afterSetMember", "setMemberPresent");
    check_step!(mapInsert, "afterMapInsert", "mapInsert");
    check_step!(mapMember, "afterMapMember", "mapMemberPresent");
    check_step!(mapLookup, "afterMapLookup", "mapLookupPresent");
    check_step!(setRemove, "afterSetRemove", "setRemove");
    check_step!(setMember, "afterSetMissing", "setMemberMissing");
    check_step!(mapRemove, "afterMapRemove", "mapRemove");
    check_step!(mapMember, "afterMapMissing", "mapMemberMissing");
    check_step!(
        mapInsertDefault,
        "afterMapInsertDefault",
        "mapInsertDefault"
    );
    check_step!(mapLookup, "afterMapDefaultLookup", "mapLookupDefault");
    assert_eq!(
        runtime::ledger::read_root_cell::<runtime::Field, _>(
            recorded_context.query.state.get_ref(),
            3,
        )
        .unwrap(),
        runtime::Field::from(0u64),
    );
    assert_eq!(
        state_hex(native_context.query.state.get_ref().clone()),
        state_hex(recorded_context.query.state.get_ref().clone()),
    );
}

#[test]
fn vector_keys_match_typescript_state_bytes_and_lookup() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/vector-key-adt.json"
    ))
    .unwrap();
    let initial = initial_state(ConstructorContext::new(())).unwrap();
    assert_eq!(
        state_hex(initial.ledger_state.get_ref().clone()),
        oracle["afterInit"]
    );

    let inserted = setInsert(initial.into_circuit_context(ContractAddress::default())).unwrap();
    assert_eq!(
        state_hex(inserted.context.query.state.get_ref().clone()),
        oracle["afterSetInsert"]
    );
    let member = setMember(inserted.context).unwrap();
    assert_eq!(
        state_hex(member.context.query.state.get_ref().clone()),
        oracle["afterSetMember"]
    );
    assert_eq!(
        runtime::ledger::read_root_cell::<bool, _>(member.context.query.state.get_ref(), 2)
            .unwrap(),
        oracle["setPresent"]
    );

    let inserted = mapInsert(member.context).unwrap();
    assert_eq!(
        state_hex(inserted.context.query.state.get_ref().clone()),
        oracle["afterMapInsert"]
    );
    let member = mapMember(inserted.context).unwrap();
    assert_eq!(
        state_hex(member.context.query.state.get_ref().clone()),
        oracle["afterMapMember"]
    );
    assert_eq!(
        runtime::ledger::read_root_cell::<bool, _>(member.context.query.state.get_ref(), 2)
            .unwrap(),
        oracle["mapPresent"]
    );
    let lookup = mapLookup(member.context).unwrap();
    assert_eq!(
        state_hex(lookup.context.query.state.get_ref().clone()),
        oracle["afterMapLookup"]
    );
    assert_eq!(
        runtime::ledger::read_root_cell::<runtime::Field, _>(
            lookup.context.query.state.get_ref(),
            3,
        )
        .unwrap(),
        runtime::Field::from(oracle["stored"].as_str().unwrap().parse::<u64>().unwrap())
    );

    let removed = setRemove(lookup.context).unwrap();
    assert_eq!(
        state_hex(removed.context.query.state.get_ref().clone()),
        oracle["afterSetRemove"]
    );
    let member = setMember(removed.context).unwrap();
    assert_eq!(
        state_hex(member.context.query.state.get_ref().clone()),
        oracle["afterSetMissing"]
    );
    assert_eq!(
        runtime::ledger::read_root_cell::<bool, _>(member.context.query.state.get_ref(), 2)
            .unwrap(),
        oracle["setMissing"]
    );

    let removed = mapRemove(member.context).unwrap();
    assert_eq!(
        state_hex(removed.context.query.state.get_ref().clone()),
        oracle["afterMapRemove"]
    );
    let member = mapMember(removed.context).unwrap();
    assert_eq!(
        state_hex(member.context.query.state.get_ref().clone()),
        oracle["afterMapMissing"]
    );
    assert_eq!(
        runtime::ledger::read_root_cell::<bool, _>(member.context.query.state.get_ref(), 2)
            .unwrap(),
        oracle["mapMissing"]
    );
    let inserted = mapInsertDefault(member.context).unwrap();
    assert_eq!(
        state_hex(inserted.context.query.state.get_ref().clone()),
        oracle["afterMapInsertDefault"]
    );
    let lookup = mapLookup(inserted.context).unwrap();
    assert_eq!(
        state_hex(lookup.context.query.state.get_ref().clone()),
        oracle["afterMapDefaultLookup"]
    );
    assert_eq!(
        runtime::ledger::read_root_cell::<runtime::Field, _>(
            lookup.context.query.state.get_ref(),
            3,
        )
        .unwrap(),
        runtime::Field::from(
            oracle["defaultStored"]
                .as_str()
                .unwrap()
                .parse::<u64>()
                .unwrap()
        )
    );
}
