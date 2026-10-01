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

use compact_rust_jubjub_cell_fixture::ledger_contract::{
    initial_state, read_box, read_point, set_box, set_point,
};
use compact_rust_jubjub_cell_fixture::types::PointBox;
use midnight_compact_runtime::context::ConstructorContext;
use midnight_compact_runtime::ledger::{ContractAddress, StateValue, read_cell};
use midnight_compact_runtime::{
    BinaryHashRepr, Field, FieldRepr, FromFieldRepr, JubjubPoint, hash_to_curve,
};

fn assert_coords(point: JubjubPoint, oracle: &serde_json::Value) {
    assert_eq!(
        hex::encode(
            point
                .x()
                .unwrap_or_else(|| Field::from(0_u64))
                .as_le_bytes()
        ),
        oracle["x"].as_str().unwrap()
    );
    assert_eq!(
        hex::encode(
            point
                .y()
                .unwrap_or_else(|| Field::from(0_u64))
                .as_le_bytes()
        ),
        oracle["y"].as_str().unwrap()
    );
}

#[test]
fn point_and_struct_cells_round_trip_with_compact_default_encoding() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/jubjub-cell.json"
    ))
    .unwrap();
    let constructor = initial_state(ConstructorContext::new(())).unwrap();
    let StateValue::Array(fields) = constructor.ledger_state.get_ref() else {
        panic!("expected ledger root array");
    };
    let default_point = read_cell::<JubjubPoint, _>(&fields.get(0).unwrap()).unwrap();
    assert_coords(default_point, &oracle["before"]);
    assert_eq!(
        default_point.field_vec(),
        vec![Field::from(0_u64), Field::from(1_u64)]
    );
    assert_eq!(
        JubjubPoint::from_field_repr(&default_point.field_vec()),
        Some(default_point)
    );
    assert_eq!(default_point.binary_vec().len(), 64);

    let context = constructor.into_circuit_context(ContractAddress::default());
    let before = read_point(context).unwrap();
    assert_coords(before.result, &oracle["before"]);
    let point = hash_to_curve(Field::from(42_u64));
    let write = set_point(before.context, point).unwrap();
    let read = read_point(write.context).unwrap();
    assert_coords(read.result, &oracle["after"]);
    assert_eq!(
        JubjubPoint::from_field_repr(&point.field_vec()),
        Some(point)
    );

    let boxed = PointBox {
        point,
        count: Field::from(7_u64),
    };
    let write = set_box(read.context, boxed.clone()).unwrap();
    let read = read_box(write.context).unwrap();
    assert_coords(read.result.point, &oracle["boxed"]["point"]);
    let expected_count: u64 = oracle["boxed"]["count"].as_str().unwrap().parse().unwrap();
    assert_eq!(read.result.count, Field::from(expected_count));
    assert_eq!(PointBox::from_field_repr(&boxed.field_vec()), Some(boxed));
}
