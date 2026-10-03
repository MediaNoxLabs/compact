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

use compact_rust_chunked_set_observed_fixture::ledger_contract::{self, Contract, initial_state};
use compact_rust_chunked_set_observed_fixture::ledger_slots::keySet;
use midnight_base_crypto::fab::ValueAtom;
use midnight_compact_runtime as runtime;
use midnight_onchain_state::state::{
    ContractMaintenanceAuthority, ContractOperation, ContractState, EntryPointBuf,
};
use midnight_serialize::tagged_serialize;
use midnight_storage::storage::HashMap;
use runtime::Field;
use runtime::context::{CircuitResult, ConstructorContext};
use runtime::fab::{AlignedValue, Value};
use runtime::ledger::{ContractAddress, DefaultDB, StateValue, set_view_at_path};
use runtime::recording::RecordedCircuitResult;

fn oracle() -> serde_json::Value {
    serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/chunked-set-observed.json"
    ))
    .unwrap()
}

fn state_hex(state: StateValue<DefaultDB>) -> String {
    let mut operations = HashMap::new();
    for name in ["insert_key", "roundtrip_key", "key_count", "empty"] {
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
fn chunked_set_calls_match_typescript_and_replay() {
    let reference = oracle();
    assert_eq!(keySet.path(), &[1, 14]);
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
    let key = (Field::from(42_u64), true);
    let encoded = AlignedValue::from(key);
    let captured_atoms: Vec<Vec<u8>> =
        serde_json::from_value(reference["insert"]["valueAtoms"].clone()).unwrap();
    assert_eq!(
        encoded.value,
        Value(captured_atoms.into_iter().map(ValueAtom).collect())
    );
    assert_eq!(
        serde_json::to_value(encoded.alignment).unwrap(),
        reference["insert"]["alignment"]
    );
    check(
        "insert",
        ledger_contract::insert_key(context(), key).unwrap(),
        contract.recording.insert_key(context(), key).unwrap(),
        &reference["insert"],
    );
    let native = ledger_contract::roundtrip_key(context(), key).unwrap();
    assert!(!native.result);
    let recorded = contract.recording.roundtrip_key(context(), key).unwrap();
    check("roundtrip", native, recorded, &reference["roundtrip"]);
    let native = ledger_contract::key_count(context()).unwrap();
    assert_eq!(native.result.value(), 0);
    check(
        "count",
        native,
        contract.recording.key_count(context()).unwrap(),
        &reference["count"],
    );
    let native = ledger_contract::empty(context()).unwrap();
    assert!(native.result);
    check(
        "empty",
        native,
        contract.recording.empty(context()).unwrap(),
        &reference["empty"],
    );
    let inserted = ledger_contract::insert_key(context(), key).unwrap();
    assert!(
        set_view_at_path::<(Field, bool), _>(inserted.context.query.state.get_ref(), &[1, 14])
            .unwrap()
            .member(key)
    );
}
