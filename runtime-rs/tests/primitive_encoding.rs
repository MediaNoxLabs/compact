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

use midnight_base_crypto::fab::{
    Aligned, AlignedValue, Alignment, AlignmentAtom, AlignmentSegment, Value, ValueAtom,
};
use midnight_compact_runtime::{
    BinaryHashRepr, Field, FieldRepr, FixedBytes, FixedVector, FromFieldRepr, JubjubPoint,
};

#[test]
fn jubjub_identity_uses_ledger_fab_and_normalizes_legacy_zero_sentinel() {
    let default = JubjubPoint::default();
    let native = JubjubPoint::from_field_repr(&[Field::from(0_u64), Field::from(0_u64)]).unwrap();
    assert!(default.is_identity() && native.is_identity());
    assert_eq!(
        default.field_vec(),
        vec![Field::from(0_u64), Field::from(1_u64)]
    );
    assert_eq!(native.field_vec(), default.field_vec());
    let mut repr = Vec::new();
    default.field_repr(&mut repr);
    assert_eq!(repr, default.field_vec());
    let mut expected_binary = vec![0_u8; 64];
    expected_binary[32] = 1;
    assert_eq!(default.binary_vec(), expected_binary);
    assert_eq!(
        JubjubPoint::from_field_repr(&default.field_vec()),
        Some(default)
    );
    assert_eq!(
        JubjubPoint::from_field_repr(&native.field_vec()),
        Some(native)
    );
    assert_eq!(AlignedValue::from(default).value, Value::from(default));
    assert_eq!(AlignedValue::from(default).value, Value::from(native));
    assert!(JubjubPoint::from_field_repr(&[Field::from(0_u64), Field::from(2_u64)]).is_none());
    let invalid = Value::from((Field::from(0_u64), Field::from(2_u64)));
    assert!(JubjubPoint::try_from(&*invalid).is_err());
}

#[test]
fn boolean_uses_ledger_single_byte_alignment_and_normalized_values() {
    assert_eq!(
        bool::alignment(),
        Alignment::singleton(AlignmentAtom::Bytes { length: 1 })
    );
    assert_eq!(
        AlignedValue::from(false).value,
        Value(vec![ValueAtom(vec![])])
    );
    assert_eq!(
        AlignedValue::from(true).value,
        Value(vec![ValueAtom(vec![1])])
    );
}

#[test]
fn field_uses_ledger_field_alignment_and_field_value() {
    let value = Field::from(42_u64);
    assert_eq!(
        Field::alignment(),
        Alignment::singleton(AlignmentAtom::Field)
    );
    assert_eq!(
        AlignedValue::from(value).value,
        Value(vec![ValueAtom(vec![42])])
    );
}

#[test]
fn two_argument_fab_matches_independent_typescript_put_input() {
    let oracle: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/map-boolean-put-input.json")).unwrap();
    let captured_atoms: Vec<Vec<u8>> =
        serde_json::from_value(oracle["valueAtoms"].clone()).unwrap();
    let encoded = AlignedValue::from((true, Field::from(42_u64)));
    assert_eq!(
        encoded.value,
        Value(captured_atoms.into_iter().map(ValueAtom).collect())
    );
    assert_eq!(
        oracle["alignment"],
        serde_json::json!([
            {"tag": "atom", "value": {"tag": "bytes", "length": 1}},
            {"tag": "atom", "value": {"tag": "field"}}
        ])
    );
    assert_eq!(
        encoded.alignment,
        Alignment(vec![
            AlignmentSegment::Atom(AlignmentAtom::Bytes { length: 1 }),
            AlignmentSegment::Atom(AlignmentAtom::Field),
        ])
    );
    assert_eq!(
        oracle["publicTranscriptShape"],
        serde_json::json!([
            {"kind": "idx", "cached": false, "pushPath": true, "pathLength": 1},
            {"kind": "push", "storage": false},
            {"kind": "push", "storage": true},
            {"kind": "ins", "cached": false, "n": 1},
            {"kind": "ins", "cached": true, "n": 1}
        ])
    );
}

#[test]
fn fixed_bytes_use_upstream_alignment_value_and_field_encoding() {
    let bytes = FixedBytes::<4>::new([1, 2, 0, 0]);
    assert_eq!(FixedBytes::<4>::alignment(), <[u8; 4]>::alignment());
    assert_eq!(
        AlignedValue::from(bytes).value,
        AlignedValue::from([1, 2, 0, 0]).value
    );
    assert_eq!(bytes.field_vec(), [1, 2, 0, 0].field_vec());
    assert_eq!(
        FixedBytes::<4>::from_field_repr(&bytes.field_vec()),
        Some(bytes)
    );
    assert_eq!(bytes.binary_vec(), vec![1, 2, 0, 0]);

    let wide = FixedBytes::<32>::new([7; 32]);
    assert_eq!(wide.field_vec(), [7; 32].field_vec());
    assert_eq!(
        FixedBytes::<32>::from_field_repr(&wide.field_vec()),
        Some(wide)
    );
}

#[test]
fn fixed_vector_concatenates_upstream_element_representations() {
    let vector = FixedVector::<Field, 2>::new([Field::from(3_u64), Field::from(5_u64)]);
    assert_eq!(
        vector.field_vec(),
        vec![Field::from(3_u64), Field::from(5_u64)]
    );
    assert_eq!(
        FixedVector::<Field, 2>::from_field_repr(&vector.field_vec()),
        Some(vector.clone())
    );
    assert_eq!(vector.0.len(), 2);
    assert_eq!(vector.binary_len(), 2 * Field::from(0_u64).binary_len());
    assert_eq!(AlignedValue::from(vector).alignment.0.len(), 2);
}

#[test]
fn nested_vectors_and_tuple_elements_have_recursive_field_representations() {
    let nested = FixedVector::new([
        FixedVector::new([Field::from(1_u64), Field::from(2_u64)]),
        FixedVector::new([Field::from(3_u64), Field::from(4_u64)]),
    ]);
    assert_eq!(nested.field_vec().len(), 4);
    assert_eq!(
        FixedVector::<FixedVector<Field, 2>, 2>::from_field_repr(&nested.field_vec()),
        Some(nested)
    );

    let tuples = FixedVector::new([(Field::from(5_u64), true), (Field::from(6_u64), false)]);
    assert_eq!(
        FixedVector::<(Field, bool), 2>::from_field_repr(&tuples.field_vec()),
        Some(tuples)
    );
}
