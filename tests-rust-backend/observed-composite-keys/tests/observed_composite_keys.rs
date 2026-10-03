// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//  http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use compact_rust_observed_composite_keys_fixture::ledger_contract::{
    self, Contract, initial_state,
};
use compact_rust_observed_composite_keys_fixture::types::CompositeKey;
use midnight_base_crypto::fab::{AlignmentSegment, ValueAtom};
use midnight_compact_runtime as runtime;
use runtime::context::{CircuitResult, ConstructorContext};
use runtime::fab::{AlignedValue, Alignment, AlignmentAtom, Value};
use runtime::ledger::{ContractAddress, DefaultDB};
use runtime::recording::RecordedCircuitResult;
use runtime::{Field, FixedVector};

fn oracle() -> serde_json::Value {
    serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/observed-composite-keys.json"
    ))
    .unwrap()
}

fn check_input(captured: &serde_json::Value, encoded: AlignedValue, expected_alignment: Alignment) {
    let captured_atoms: Vec<Vec<u8>> =
        serde_json::from_value(captured["valueAtoms"].clone()).unwrap();
    assert_eq!(
        encoded.value,
        Value(captured_atoms.into_iter().map(ValueAtom).collect())
    );
    assert_eq!(encoded.alignment, expected_alignment);
}

fn field_alignment(count: usize) -> Vec<AlignmentSegment> {
    vec![AlignmentSegment::Atom(AlignmentAtom::Field); count]
}

#[test]
fn vector_tuple_and_nested_struct_inputs_match_typescript_fab() {
    let oracle = oracle();
    let vector = FixedVector::new([Field::from(3_u64), Field::from(5_u64)]);
    let pair = (Field::from(42_u64), true);
    let key = CompositeKey {
        vector: vector.clone(),
        pair,
    };
    check_input(
        &oracle["vector"],
        AlignedValue::from(vector),
        Alignment(field_alignment(2)),
    );
    check_input(
        &oracle["tuple"],
        AlignedValue::from(pair),
        Alignment(
            [
                field_alignment(1),
                vec![AlignmentSegment::Atom(AlignmentAtom::Bytes { length: 1 })],
            ]
            .concat(),
        ),
    );
    check_input(
        &oracle["struct"],
        AlignedValue::from(key),
        Alignment(
            [
                field_alignment(3),
                vec![AlignmentSegment::Atom(AlignmentAtom::Bytes { length: 1 })],
            ]
            .concat(),
        ),
    );
    for (name, field_count) in [("vector", 2), ("tuple", 1), ("struct", 3)] {
        let alignment = oracle[name]["alignment"].as_array().unwrap();
        assert_eq!(alignment.len(), field_count + usize::from(name != "vector"));
        assert!(
            alignment[..field_count].iter().all(
                |segment| segment == &serde_json::json!({"tag":"atom","value":{"tag":"field"}})
            )
        );
        if name != "vector" {
            assert_eq!(
                alignment[field_count],
                serde_json::json!({"tag":"atom","value":{"tag":"bytes","length":1}})
            );
        }
    }
}

fn check_recorded(
    name: &str,
    native: CircuitResult<(), (), DefaultDB>,
    recorded: RecordedCircuitResult<(), (), DefaultDB>,
) {
    let oracle = oracle();
    assert_eq!(recorded.execution.gas_cost, native.gas_cost, "{name}: gas");
    assert_eq!(
        recorded.execution.context.query.effects, native.context.query.effects,
        "{name}: effects"
    );
    assert_eq!(
        recorded.execution.context.query.state.get_ref(),
        native.context.query.state.get_ref(),
        "{name}: ledger state"
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
        "{name}: replay state"
    );
    let actual = serde_json::to_value(recorded.public.verify_ops()).unwrap();
    let actual_shape: Vec<serde_json::Value> = actual
        .as_array()
        .unwrap()
        .iter()
        .map(|operation| {
            if let Some(idx) = operation.get("idx") {
                serde_json::json!({
                    "kind": "idx",
                    "cached": idx["cached"],
                    "pushPath": idx["pushPath"],
                    "pathLength": idx["path"].as_array().unwrap().len(),
                })
            } else if let Some(push) = operation.get("push") {
                serde_json::json!({"kind": "push", "storage": push["storage"]})
            } else if let Some(ins) = operation.get("ins") {
                serde_json::json!({"kind": "ins", "cached": ins["cached"], "n": ins["n"]})
            } else {
                panic!("{name}: unexpected VM operation: {operation}");
            }
        })
        .collect();
    assert_eq!(
        serde_json::Value::Array(actual_shape),
        oracle[name]["publicTranscriptShape"]
    );
}

#[test]
fn vector_tuple_and_nested_struct_recorded_set_inserts_replay() {
    let address = ContractAddress::default();
    let contract = Contract::default();
    let vector = FixedVector::new([Field::from(3_u64), Field::from(5_u64)]);
    let pair = (Field::from(42_u64), true);
    let key = CompositeKey {
        vector: vector.clone(),
        pair,
    };
    let context = || {
        initial_state(ConstructorContext::new(()))
            .unwrap()
            .into_circuit_context(address)
    };
    check_recorded(
        "vector",
        ledger_contract::insert_vector(context(), vector.clone()).unwrap(),
        contract.recording.insert_vector(context(), vector).unwrap(),
    );
    check_recorded(
        "tuple",
        ledger_contract::insert_tuple(context(), pair).unwrap(),
        contract.recording.insert_tuple(context(), pair).unwrap(),
    );
    check_recorded(
        "struct",
        ledger_contract::insert_struct(context(), key.clone()).unwrap(),
        contract.recording.insert_struct(context(), key).unwrap(),
    );
}
