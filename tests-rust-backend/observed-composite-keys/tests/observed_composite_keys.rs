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
    self, Contract, PublicStateView, initial_state,
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
    check_input(
        &oracle["tupleRoundtrip"],
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
        &oracle["structRoundtrip"],
        AlignedValue::from(CompositeKey {
            vector: FixedVector::new([Field::from(3_u64), Field::from(5_u64)]),
            pair,
        }),
        Alignment(
            [
                field_alignment(3),
                vec![AlignmentSegment::Atom(AlignmentAtom::Bytes { length: 1 })],
            ]
            .concat(),
        ),
    );
    check_input(
        &oracle["twelve"],
        AlignedValue::concat(
            &(1_u64..=12)
                .map(|value| AlignedValue::from(Field::from(value)))
                .collect::<Vec<_>>(),
        ),
        Alignment(field_alignment(12)),
    );
    for (name, field_count) in [
        ("vector", 2),
        ("tuple", 1),
        ("struct", 3),
        ("tupleRoundtrip", 1),
        ("structRoundtrip", 3),
    ] {
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

fn check_recorded<Output: std::fmt::Debug + PartialEq>(
    name: &str,
    native: CircuitResult<(), Output, DefaultDB>,
    recorded: RecordedCircuitResult<(), Output, DefaultDB>,
) {
    let oracle = oracle();
    assert_eq!(recorded.execution.result, native.result, "{name}: result");
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
            if let Some(kind) = operation.as_str() {
                return serde_json::json!({"kind": kind});
            }
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
            } else if let Some(rem) = operation.get("rem") {
                serde_json::json!({"kind": "rem", "cached": rem["cached"]})
            } else if let Some(dup) = operation.get("dup") {
                serde_json::json!({"kind": "dup", "n": dup["n"]})
            } else if let Some(popeq) = operation.get("popeq") {
                serde_json::json!({
                    "kind": "popeq",
                    "cached": popeq["cached"],
                    "resultAtoms": popeq["result"]["value"],
                })
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
        let initial = initial_state(ConstructorContext::new(())).unwrap();
        let view = PublicStateView::from(&initial);
        assert!(view.vectorKeys().unwrap().is_empty());
        assert!(view.tupleKeys().unwrap().is_empty());
        assert!(view.structKeys().unwrap().is_empty());
        assert!(view.sums().unwrap().is_empty());
        initial.into_circuit_context(address)
    };
    let vector_native = ledger_contract::insert_vector(context(), vector.clone()).unwrap();
    let vector_recorded = contract
        .recording
        .insert_vector(context(), vector.clone())
        .unwrap();
    for result in [&vector_native, &vector_recorded.execution] {
        let keys = PublicStateView::from(result).vectorKeys().unwrap();
        assert!(keys.member(vector.clone()));
        assert_eq!(keys.size().unwrap().value(), 1);
    }
    check_recorded("vector", vector_native, vector_recorded);

    let tuple_native = ledger_contract::insert_tuple(context(), pair).unwrap();
    let tuple_recorded = contract.recording.insert_tuple(context(), pair).unwrap();
    for result in [&tuple_native, &tuple_recorded.execution] {
        let keys = PublicStateView::from(result).tupleKeys().unwrap();
        assert!(keys.member(pair));
        assert_eq!(keys.size().unwrap().value(), 1);
    }
    check_recorded("tuple", tuple_native, tuple_recorded);

    let struct_native = ledger_contract::insert_struct(context(), key.clone()).unwrap();
    let struct_recorded = contract
        .recording
        .insert_struct(context(), key.clone())
        .unwrap();
    for result in [&struct_native, &struct_recorded.execution] {
        let keys = PublicStateView::from(result).structKeys().unwrap();
        assert!(keys.member(key.clone()));
        assert_eq!(keys.size().unwrap().value(), 1);
    }
    check_recorded("struct", struct_native, struct_recorded);
    let tuple_native = ledger_contract::roundtrip_tuple(context(), pair).unwrap();
    let tuple_recorded = contract.recording.roundtrip_tuple(context(), pair).unwrap();
    assert!(!tuple_native.result && !tuple_recorded.execution.result);
    check_recorded("tupleRoundtrip", tuple_native, tuple_recorded);
    let struct_key = CompositeKey {
        vector: FixedVector::new([Field::from(3_u64), Field::from(5_u64)]),
        pair,
    };
    let struct_native = ledger_contract::roundtrip_struct(context(), struct_key.clone()).unwrap();
    let struct_recorded = contract
        .recording
        .roundtrip_struct(context(), struct_key)
        .unwrap();
    assert!(!struct_native.result && !struct_recorded.execution.result);
    check_recorded("structRoundtrip", struct_native, struct_recorded);
    let twelve_native = ledger_contract::record_twelve(
        context(),
        1_u64.into(),
        2_u64.into(),
        3_u64.into(),
        4_u64.into(),
        5_u64.into(),
        6_u64.into(),
        7_u64.into(),
        8_u64.into(),
        9_u64.into(),
        10_u64.into(),
        11_u64.into(),
        12_u64.into(),
    )
    .unwrap();
    let twelve_recorded = contract
        .recording
        .record_twelve(
            context(),
            1_u64.into(),
            2_u64.into(),
            3_u64.into(),
            4_u64.into(),
            5_u64.into(),
            6_u64.into(),
            7_u64.into(),
            8_u64.into(),
            9_u64.into(),
            10_u64.into(),
            11_u64.into(),
            12_u64.into(),
        )
        .unwrap();
    for result in [&twelve_native, &twelve_recorded.execution] {
        let sums = PublicStateView::from(result).sums().unwrap();
        assert!(sums.member(Field::from(78_u64)));
        assert_eq!(sums.size().unwrap().value(), 1);
    }
    check_recorded("twelve", twelve_native, twelve_recorded);
}
