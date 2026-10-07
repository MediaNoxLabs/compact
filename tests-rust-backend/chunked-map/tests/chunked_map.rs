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

use compact_rust_chunked_map_fixture::ledger_contract::{self, Contract, initial_state};
use compact_rust_chunked_map_fixture::ledger_slots::table;
use midnight_base_crypto::fab::ValueAtom;
use midnight_compact_runtime as runtime;
use midnight_onchain_state::state::{
    ContractMaintenanceAuthority, ContractOperation, ContractState, EntryPointBuf,
};
use midnight_serialize::tagged_serialize;
use midnight_storage::storage::HashMap;
use runtime::Field;
use runtime::context::{CircuitResult, ConstructorContext, WitnessReadMeter};
use runtime::fab::{AlignedValue, Value};
use runtime::ledger::{
    ContractAddress, DefaultDB, StateValue, map_view_at_path, metered_map_view_at_path,
};
use runtime::recording::RecordedCircuitResult;

fn oracle() -> serde_json::Value {
    serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/chunked-map.json"
    ))
    .unwrap()
}

fn state_hex(state: StateValue<DefaultDB>) -> String {
    let mut operations = HashMap::new();
    for name in [
        "put",
        "put_pair",
        "put_default",
        "has",
        "get",
        "remove_key",
        "table_size",
        "table_is_empty",
        "reset_table",
    ] {
        operations = operations.insert(
            EntryPointBuf(name.as_bytes().to_vec()),
            ContractOperation::new(None),
        );
    }
    let state = ContractState::new(state, operations, ContractMaintenanceAuthority::default());
    let mut bytes = Vec::new();
    tagged_serialize(&state, &mut bytes).unwrap();
    hex::encode(bytes)
}

fn shape(call: &RecordedCircuitResult<(), impl std::fmt::Debug>) -> serde_json::Value {
    let actual = serde_json::to_value(call.public.verify_ops()).unwrap();
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

fn check<Output: std::fmt::Debug + PartialEq>(
    name: &str,
    expected_result: Output,
    native: CircuitResult<(), Output, DefaultDB>,
    recorded: RecordedCircuitResult<(), Output, DefaultDB>,
    reference: &serde_json::Value,
) {
    assert_eq!(native.result, expected_result, "{name}: native result");
    assert_eq!(
        recorded.execution.result, expected_result,
        "{name}: recorded result"
    );
    assert_eq!(recorded.execution.result, native.result, "{name}: result");
    assert_eq!(recorded.execution.gas_cost, native.gas_cost, "{name}: gas");
    assert_eq!(
        recorded.execution.context.query.effects, native.context.query.effects,
        "{name}: effects"
    );
    assert_eq!(
        recorded.execution.context.query.state.get_ref(),
        native.context.query.state.get_ref(),
        "{name}: state"
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
        native.context.query.state.get_ref(),
        "{name}: replay"
    );
    assert_eq!(
        shape(&recorded),
        reference["publicTranscriptShape"],
        "{name}: TypeScript VM"
    );
    assert_eq!(
        state_hex(native.context.query.state.get_ref().clone()),
        reference["afterStateHex"],
        "{name}: TypeScript state"
    );
    let gas = serde_json::to_value(native.gas_cost).unwrap();
    for dimension in ["readTime", "computeTime", "bytesWritten", "bytesDeleted"] {
        let expected: u64 = reference["queries"]
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
            expected,
            "{name}: {dimension} TypeScript gas"
        );
    }
}

fn check_input(name: &str, input: impl Into<AlignedValue>, reference: &serde_json::Value) {
    let encoded = input.into();
    let atoms: Vec<Vec<u8>> = serde_json::from_value(reference["valueAtoms"].clone()).unwrap();
    assert_eq!(
        encoded.value,
        Value(atoms.into_iter().map(ValueAtom).collect()),
        "{name}: TypeScript input atoms"
    );
    assert_eq!(
        serde_json::to_value(encoded.alignment).unwrap(),
        reference["alignment"],
        "{name}: TypeScript input alignment"
    );
}

#[test]
fn chunked_map_calls_match_typescript_and_replay() {
    let reference = oracle();
    assert_eq!(table.path(), &[1, 14]);
    let initial = initial_state(ConstructorContext::new(())).unwrap();
    assert_eq!(
        state_hex(initial.ledger_state.get_ref().clone()),
        reference["initialStateHex"]
    );
    let context = || {
        initial_state(ConstructorContext::new(()))
            .unwrap()
            .into_circuit_context(ContractAddress::default())
    };
    let contract = Contract::default();
    check_input("put", (false, Field::from(7_u64)), &reference["put"]);
    check_input(
        "pair",
        (true, false, Field::from(42_u64)),
        &reference["pair"],
    );
    check(
        "put",
        (),
        ledger_contract::put(context(), false, Field::from(7_u64)).unwrap(),
        contract
            .recording
            .put(context(), false, Field::from(7_u64))
            .unwrap(),
        &reference["put"],
    );
    check(
        "pair",
        (),
        ledger_contract::put_pair(context(), true, false, Field::from(42_u64)).unwrap(),
        contract
            .recording
            .put_pair(context(), true, false, Field::from(42_u64))
            .unwrap(),
        &reference["pair"],
    );
    let first = table.insert(context(), true, Field::from(42_u64)).unwrap();
    let second = table
        .insert(first.context, false, Field::from(42_u64))
        .unwrap();
    for (index, gas) in [first.gas_cost, second.gas_cost].into_iter().enumerate() {
        let actual = serde_json::to_value(gas).unwrap();
        let expected = &reference["pair"]["queries"][index]["gasCost"];
        for dimension in ["readTime", "computeTime", "bytesWritten", "bytesDeleted"] {
            assert_eq!(
                actual[dimension].as_u64().unwrap(),
                expected[dimension]
                    .as_str()
                    .unwrap()
                    .parse::<u64>()
                    .unwrap(),
                "pair query {index}: {dimension} TypeScript gas"
            );
        }
    }
    check(
        "default",
        (),
        ledger_contract::put_default(context(), false).unwrap(),
        contract.recording.put_default(context(), false).unwrap(),
        &reference["default"],
    );
    check(
        "has",
        true,
        ledger_contract::has(context(), true).unwrap(),
        contract.recording.has(context(), true).unwrap(),
        &reference["has"],
    );
    check(
        "get",
        Field::from(1_u64),
        ledger_contract::get(context(), true).unwrap(),
        contract.recording.get(context(), true).unwrap(),
        &reference["get"],
    );
    check(
        "remove",
        (),
        ledger_contract::remove_key(context(), true).unwrap(),
        contract.recording.remove_key(context(), true).unwrap(),
        &reference["remove"],
    );
    check(
        "size",
        runtime::BoundedUint::<{ u64::MAX as u128 }>::new(1).unwrap(),
        ledger_contract::table_size(context()).unwrap(),
        contract.recording.table_size(context()).unwrap(),
        &reference["size"],
    );
    check(
        "empty",
        false,
        ledger_contract::table_is_empty(context()).unwrap(),
        contract.recording.table_is_empty(context()).unwrap(),
        &reference["empty"],
    );
    check(
        "reset",
        (),
        ledger_contract::reset_table(context()).unwrap(),
        contract.recording.reset_table(context()).unwrap(),
        &reference["reset"],
    );

    let after = ledger_contract::put(context(), false, Field::from(7_u64)).unwrap();
    let view =
        map_view_at_path::<bool, Field, _>(after.context.query.state.get_ref(), &[1, 14]).unwrap();
    let typed = ledger_contract::PublicStateView::from(&after)
        .table()
        .unwrap();
    assert_eq!(view.size().unwrap().value(), 2);
    assert_eq!(view.lookup(false).unwrap(), Field::from(7_u64));
    assert_eq!(typed.size().unwrap(), view.size().unwrap());
    assert_eq!(typed.lookup(false).unwrap(), view.lookup(false).unwrap());

    let witness_context = context();
    let meter = WitnessReadMeter::new(&witness_context);
    let direct = WitnessReadMeter::new(&witness_context);
    let slot_view = table.witness_view(&meter).unwrap();
    let direct_view = metered_map_view_at_path::<bool, Field, _>(&direct, &[1, 14]).unwrap();
    assert_eq!(
        slot_view.lookup(true).unwrap(),
        direct_view.lookup(true).unwrap()
    );
    assert_eq!(
        slot_view.member(true).unwrap(),
        direct_view.member(true).unwrap()
    );
    assert_eq!(slot_view.size().unwrap(), direct_view.size().unwrap());
    assert_eq!(
        slot_view.is_empty().unwrap(),
        direct_view.is_empty().unwrap()
    );
    assert_eq!(meter.gas_cost(), direct.gas_cost());
    assert!(
        map_view_at_path::<bool, Field, _>(witness_context.query.state.get_ref(), &[1, 13])
            .is_err()
    );
}
