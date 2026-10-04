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

use compact_rust_chunked_cell_fixture::ledger_contract::{self, Contract, initial_state};
use compact_rust_chunked_cell_fixture::ledger_slots::{active, amount};
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
use runtime::ledger::{ContractAddress, DefaultDB, StateValue, read_cell_at_path};
use runtime::recording::RecordedCircuitResult;

fn oracle() -> serde_json::Value {
    serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/chunked-cell.json"
    ))
    .unwrap()
}

fn state_hex(state: StateValue<DefaultDB>) -> String {
    let mut operations = HashMap::new();
    for name in [
        "set_active",
        "get_active",
        "assert_active",
        "set_amount",
        "get_amount",
        "add_amount",
        "active_equals",
        "plus_amount",
        "subtract_amount",
        "multiply_amount",
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
    native: CircuitResult<(), Output, DefaultDB>,
    recorded: RecordedCircuitResult<(), Output, DefaultDB>,
    reference: &serde_json::Value,
) {
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
    for state in [
        native.context.query.state.get_ref(),
        replay.context.state.get_ref(),
    ] {
        let view = ledger_contract::PublicStateView::from(state);
        assert_eq!(
            view.active().unwrap(),
            read_cell_at_path::<bool, _>(state, active.path()).unwrap()
        );
        assert_eq!(
            view.amount().unwrap(),
            read_cell_at_path::<Field, _>(state, amount.path()).unwrap()
        );
    }
    assert_eq!(
        ledger_contract::PublicStateView::from(&recorded)
            .active()
            .unwrap(),
        ledger_contract::PublicStateView::from(native.context.query.state.get_ref())
            .active()
            .unwrap(),
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
fn chunked_cell_calls_match_typescript_and_replay() {
    let reference = oracle();
    assert_eq!(active.path(), &[1, 14]);
    assert_eq!(amount.path(), &[1, 13]);
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
    check_input("set_active", false, &reference["setActive"]);
    check_input("assert_active", true, &reference["assertActive"]);
    check_input("set_amount", Field::from(11_u64), &reference["setAmount"]);
    check_input("add_amount", Field::from(7_u64), &reference["addAmount"]);
    check_input("active_equals", true, &reference["activeEquals"]);
    check_input("plus_amount", Field::from(7_u64), &reference["plusAmount"]);
    check_input(
        "subtract_amount",
        Field::from(2_u64),
        &reference["subtractAmount"],
    );
    check_input(
        "multiply_amount",
        Field::from(7_u64),
        &reference["multiplyAmount"],
    );
    check(
        "set_active",
        ledger_contract::set_active(context(), false).unwrap(),
        contract.recording.set_active(context(), false).unwrap(),
        &reference["setActive"],
    );
    check(
        "get_active",
        ledger_contract::get_active(context()).unwrap(),
        contract.recording.get_active(context()).unwrap(),
        &reference["getActive"],
    );
    check(
        "assert_active",
        ledger_contract::assert_active(context(), true).unwrap(),
        contract.recording.assert_active(context(), true).unwrap(),
        &reference["assertActive"],
    );
    check(
        "set_amount",
        ledger_contract::set_amount(context(), Field::from(11_u64)).unwrap(),
        contract
            .recording
            .set_amount(context(), Field::from(11_u64))
            .unwrap(),
        &reference["setAmount"],
    );
    check(
        "get_amount",
        ledger_contract::get_amount(context()).unwrap(),
        contract.recording.get_amount(context()).unwrap(),
        &reference["getAmount"],
    );
    check(
        "add_amount",
        ledger_contract::add_amount(context(), Field::from(7_u64)).unwrap(),
        contract
            .recording
            .add_amount(context(), Field::from(7_u64))
            .unwrap(),
        &reference["addAmount"],
    );
    let equals = contract.recording.active_equals(context(), true).unwrap();
    assert!(equals.execution.result);
    assert_eq!(reference["activeEquals"]["result"], "true");
    check(
        "active_equals",
        ledger_contract::active_equals(context(), true).unwrap(),
        equals,
        &reference["activeEquals"],
    );
    let sum = contract
        .recording
        .plus_amount(context(), Field::from(7_u64))
        .unwrap();
    assert_eq!(sum.execution.result, Field::from(10_u64));
    assert_eq!(reference["plusAmount"]["result"], "10");
    check(
        "plus_amount",
        ledger_contract::plus_amount(context(), Field::from(7_u64)).unwrap(),
        sum,
        &reference["plusAmount"],
    );
    check(
        "subtract_amount",
        ledger_contract::subtract_amount(context(), Field::from(2_u64)).unwrap(),
        contract
            .recording
            .subtract_amount(context(), Field::from(2_u64))
            .unwrap(),
        &reference["subtractAmount"],
    );
    check(
        "multiply_amount",
        ledger_contract::multiply_amount(context(), Field::from(7_u64)).unwrap(),
        contract
            .recording
            .multiply_amount(context(), Field::from(7_u64))
            .unwrap(),
        &reference["multiplyAmount"],
    );

    let first = amount.read(context()).unwrap();
    let second = amount
        .write(first.context, first.result + Field::from(7_u64))
        .unwrap();
    for (index, gas) in [first.gas_cost, second.gas_cost].into_iter().enumerate() {
        let actual = serde_json::to_value(gas).unwrap();
        let expected = &reference["addAmount"]["queries"][index]["gasCost"];
        for dimension in ["readTime", "computeTime", "bytesWritten", "bytesDeleted"] {
            assert_eq!(
                actual[dimension].as_u64().unwrap(),
                expected[dimension]
                    .as_str()
                    .unwrap()
                    .parse::<u64>()
                    .unwrap(),
                "add_amount query {index}: {dimension} TypeScript gas"
            );
        }
    }
    for (name, expected_value) in [
        ("subtractAmount", Field::from(1_u64)),
        ("multiplyAmount", Field::from(21_u64)),
    ] {
        let first = amount.read(context()).unwrap();
        let second = amount.write(first.context, expected_value).unwrap();
        for (index, gas) in [first.gas_cost, second.gas_cost].into_iter().enumerate() {
            let actual = serde_json::to_value(gas).unwrap();
            let expected = &reference[name]["queries"][index]["gasCost"];
            for dimension in ["readTime", "computeTime", "bytesWritten", "bytesDeleted"] {
                assert_eq!(
                    actual[dimension].as_u64().unwrap(),
                    expected[dimension]
                        .as_str()
                        .unwrap()
                        .parse::<u64>()
                        .unwrap(),
                    "{name} query {index}: {dimension} TypeScript gas"
                );
            }
        }
    }
    assert_eq!(
        read_cell_at_path::<Field, _>(second.context.query.state.get_ref(), &[1, 13]).unwrap(),
        Field::from(10_u64)
    );
    assert!(read_cell_at_path::<bool, _>(second.context.query.state.get_ref(), &[1, 14]).unwrap());
    let witness_context = context();
    let meter = WitnessReadMeter::new(&witness_context);
    assert!(active.witness_read(&meter).unwrap());
    assert_eq!(amount.witness_read(&meter).unwrap(), Field::from(3_u64));
    assert_eq!(
        ledger_contract::assert_active(context(), false).is_err(),
        contract.recording.assert_active(context(), false).is_err()
    );
}
