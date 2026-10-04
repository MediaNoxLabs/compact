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

use compact_rust_chunked_list_fixture::ledger_contract::{self, Contract, initial_state};
use compact_rust_chunked_list_fixture::ledger_slots::items;
use midnight_compact_runtime as runtime;
use midnight_onchain_state::state::{
    ContractMaintenanceAuthority, ContractOperation, ContractState, EntryPointBuf,
};
use midnight_serialize::tagged_serialize;
use midnight_storage::storage::HashMap;
use runtime::Field;
use runtime::context::{CircuitResult, ConstructorContext, WitnessReadMeter};
use runtime::ledger::{
    ContractAddress, DefaultDB, StateValue, list_view_at_path, metered_list_view_at_path,
};
use runtime::recording::RecordedCircuitResult;

fn oracle() -> serde_json::Value {
    serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/chunked-list.json"
    ))
    .unwrap()
}

fn state_hex(state: StateValue<DefaultDB>) -> String {
    let mut operations = HashMap::new();
    for name in [
        "item_count",
        "items_empty",
        "first_item",
        "prepend",
        "drop_first",
        "clear_items",
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
    serde_json::Value::Array(actual.as_array().unwrap().iter().map(|operation| {
        if let Some(kind) = operation.as_str() {
            return serde_json::json!({"kind": kind});
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
        } else if let Some(dup) = operation.get("dup") {
            serde_json::json!({"kind":"dup", "n":dup["n"]})
        } else if let Some(popeq) = operation.get("popeq") {
            serde_json::json!({
                "kind":"popeq", "cached":popeq["cached"],
                "resultAtoms":popeq["result"]["value"],
            })
        } else {
            serde_json::json!({"kind":operation.as_object().unwrap().keys().next().unwrap()})
        }
    }).collect())
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

#[test]
fn chunked_list_calls_match_typescript_and_replay() {
    let reference = oracle();
    assert_eq!(items.path(), &[1, 14]);
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
    check(
        "count",
        ledger_contract::item_count(context()).unwrap(),
        contract.recording.item_count(context()).unwrap(),
        &reference["count"],
    );
    check(
        "empty",
        ledger_contract::items_empty(context()).unwrap(),
        contract.recording.items_empty(context()).unwrap(),
        &reference["empty"],
    );
    check(
        "head",
        ledger_contract::first_item(context()).unwrap(),
        contract.recording.first_item(context()).unwrap(),
        &reference["head"],
    );
    check(
        "prepend",
        ledger_contract::prepend(context(), Field::from(7_u64)).unwrap(),
        contract
            .recording
            .prepend(context(), Field::from(7_u64))
            .unwrap(),
        &reference["prepend"],
    );
    check(
        "drop",
        ledger_contract::drop_first(context()).unwrap(),
        contract.recording.drop_first(context()).unwrap(),
        &reference["drop"],
    );
    check(
        "reset",
        ledger_contract::clear_items(context()).unwrap(),
        contract.recording.clear_items(context()).unwrap(),
        &reference["reset"],
    );
    let after = ledger_contract::prepend(context(), Field::from(7_u64)).unwrap();
    let generated = ledger_contract::PublicStateView::from(&after);
    let view = generated.items().unwrap();
    let raw = list_view_at_path::<Field, _>(after.context.query.state.get_ref(), &[1, 14]).unwrap();
    assert_eq!(view.length().unwrap().value(), 2);
    assert_eq!(view.head().unwrap(), Some(Field::from(7_u64)));
    assert_eq!(view.head().unwrap(), raw.head().unwrap());
    assert_eq!(view.length().unwrap(), raw.length().unwrap());
    assert_eq!(view.is_empty(), raw.is_empty());

    let initial_view = ledger_contract::PublicStateView::from(&initial);
    assert_eq!(
        initial_view.items().unwrap().head().unwrap(),
        Some(Field::from(1_u64))
    );
    assert_eq!(initial_view.items().unwrap().length().unwrap().value(), 1);

    let witness_context = context();
    let meter = WitnessReadMeter::new(&witness_context);
    let direct = WitnessReadMeter::new(&witness_context);
    let slot_view = items.witness_view(&meter).unwrap();
    let direct_view = metered_list_view_at_path::<Field, _>(&direct, &[1, 14]).unwrap();
    assert_eq!(slot_view.head().unwrap(), direct_view.head().unwrap());
    assert_eq!(slot_view.length().unwrap(), direct_view.length().unwrap());
    assert_eq!(
        slot_view.is_empty().unwrap(),
        direct_view.is_empty().unwrap()
    );
    assert_eq!(meter.gas_cost(), direct.gas_cost());
    assert!(
        list_view_at_path::<Field, _>(witness_context.query.state.get_ref(), &[1, 13]).is_err()
    );
    assert_eq!(
        items
            .inspect(witness_context.query.state.get_ref())
            .unwrap()
            .head()
            .unwrap(),
        Some(Field::from(1_u64))
    );
}
