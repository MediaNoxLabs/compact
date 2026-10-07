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

use compact_rust_vector_widen_fixture::pure_circuits::{
    hash_nested, hash_widened, widen, widen_elements,
};
use midnight_compact_runtime::{BoundedUint, Field, FixedVector};

#[test]
fn vector_uint_to_field_coercion_matches_typescript() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/vector-widen.json"
    ))
    .unwrap();
    let values = FixedVector::new([
        BoundedUint::<255>::new(7).unwrap(),
        BoundedUint::<255>::new(255).unwrap(),
    ]);
    let widened = widen(values.clone()).unwrap();
    assert_eq!(
        widened,
        FixedVector::new([Field::from(7_u64), Field::from(255_u64)])
    );
    assert_eq!(oracle["widened"], serde_json::json!(["7", "255"]));

    let elements = widen_elements(BoundedUint::<4294967295>::new(4294967295).unwrap()).unwrap();
    assert_eq!(elements, FixedVector::new([Field::from(4294967295_u64); 2]));
    assert_eq!(
        oracle["elements"],
        serde_json::json!(["4294967295", "4294967295"])
    );
    assert_eq!(
        hex::encode(hash_widened(values.clone()).unwrap().into_array()),
        oracle["hashHex"]
    );
    assert_eq!(
        hex::encode(hash_nested(values).unwrap().into_array()),
        oracle["nestedHashHex"]
    );
}

// These controls assert Compact source semantics, independently of the retained
// TypeScript capture above; they do not claim an additional cross-backend oracle.
#[test]
fn unsigned_vector_widening_preserves_values_at_small_carrier_bounds() {
    use compact_rust_vector_widen_fixture::pure_circuits::{
        widen_empty_unsigned, widen_unsigned, widen_unsigned_nested, widen_unsigned_tuple,
    };
    let pair = FixedVector::new([
        BoundedUint::<255>::new(0).unwrap(),
        BoundedUint::<255>::new(255).unwrap(),
    ]);
    assert_eq!(
        widen_unsigned(pair.clone())
            .unwrap()
            .into_array()
            .map(|n| n.value()),
        [0, 255]
    );
    let nested = FixedVector::new([
        pair.clone(),
        FixedVector::new([
            BoundedUint::<255>::new(1).unwrap(),
            BoundedUint::<255>::new(254).unwrap(),
        ]),
    ]);
    assert_eq!(
        widen_unsigned_nested(nested)
            .unwrap()
            .into_array()
            .map(|row| row.into_array().map(|n| n.value())),
        [[0, 255], [1, 254]]
    );
    let tuples = FixedVector::new([
        (
            BoundedUint::<255>::new(0).unwrap(),
            BoundedUint::<65535>::new(65535).unwrap(),
        ),
        (
            BoundedUint::<255>::new(255).unwrap(),
            BoundedUint::<65535>::new(0).unwrap(),
        ),
    ]);
    assert_eq!(
        widen_unsigned_tuple(tuples)
            .unwrap()
            .into_array()
            .map(|(a, b)| (a.value(), b.value())),
        [(0, 65535), (255, 0)]
    );
    assert_eq!(
        widen_empty_unsigned(FixedVector::new([])).unwrap(),
        FixedVector::new([])
    );
}

#[test]
fn unsigned_vector_widening_crosses_and_retains_the_wide_carrier() {
    use compact_rust_vector_widen_fixture::pure_circuits::{
        widen_unsigned_to_wide, widen_wide_unsigned,
    };
    use midnight_compact_runtime::WideUint;
    let small = FixedVector::new([
        BoundedUint::<255>::new(0).unwrap(),
        BoundedUint::<255>::new(255).unwrap(),
    ]);
    let wide = widen_unsigned_to_wide(small).unwrap().into_array();
    assert_eq!(wide[0].as_le_bytes(), &[0; 17]);
    let mut expected = [0; 17];
    expected[0] = 255;
    assert_eq!(wide[1].as_le_bytes(), &expected);

    type Uint129 = WideUint<1, { u128::MAX }>;
    let mut boundary = [0; 31];
    boundary[16] = 1; // 2^128, above the small carrier's maximum.
    let mut maximum = [0; 31];
    maximum[..16].fill(255);
    maximum[16] = 1; // 2^129 - 1.
    let inputs = FixedVector::new([
        Uint129::from_le_bytes(&boundary).unwrap(),
        Uint129::from_le_bytes(&maximum).unwrap(),
    ]);
    let outputs = widen_wide_unsigned(inputs).unwrap().into_array();
    assert_eq!(outputs[0].as_le_bytes(), &boundary);
    assert_eq!(outputs[1].as_le_bytes(), &maximum);
}

#[test]
fn map_and_fold_inputs_keep_their_length_while_widening_unsigned_elements() {
    use compact_rust_vector_widen_fixture::pure_circuits::{
        fold_mapped_unsigned, fold_unsigned, map_unsigned,
    };
    for values in [[0, 255], [255, 0]] {
        let input = FixedVector::new(values.map(|value| BoundedUint::<255>::new(value).unwrap()));
        assert_eq!(
            map_unsigned(input.clone())
                .unwrap()
                .into_array()
                .map(|n| n.value()),
            values
        );
        // The named fold callback returns the last element; reversing the input
        // distinguishes source-order traversal from a reversed reduction.
        assert_eq!(fold_unsigned(input.clone()).unwrap().value(), values[1]);
        assert_eq!(fold_mapped_unsigned(input).unwrap().value(), values[1]);
    }
}
