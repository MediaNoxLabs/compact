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
    Aligned, AlignedValue, Alignment, AlignmentAtom, Value, ValueAtom,
};
use midnight_compact_runtime::{
    BinaryHashRepr, BoundedUint, CompactError, Field, FieldRepr, FromFieldRepr, add_unsigned,
    cast_unsigned, multiply_unsigned, subtract_unsigned,
};

#[test]
fn unsigned_cast_and_add_preserve_bounds() {
    let left = BoundedUint::<255>::new(255).unwrap();
    let right = BoundedUint::<255>::new(255).unwrap();
    assert_eq!(
        add_unsigned::<255, 255, 511>(left, right).unwrap().value(),
        510
    );
    assert_eq!(
        cast_unsigned::<255, 8>(left),
        Err(CompactError::UnsignedOutOfRange { value: 255, max: 8 })
    );
    let largest = BoundedUint::<{ u128::MAX }>::new(u128::MAX).unwrap();
    assert_eq!(
        add_unsigned::<{ u128::MAX }, 1, { u128::MAX }>(largest, BoundedUint::<1>::new(1).unwrap()),
        Err(CompactError::UnsignedOverflow)
    );
}

#[test]
fn unsigned_subtraction_and_multiplication_check_arithmetic_and_result_bounds() {
    let small = BoundedUint::<255>::new(3).unwrap();
    let large = BoundedUint::<255>::new(5).unwrap();
    assert_eq!(
        subtract_unsigned::<255, 255, 255>(large, small)
            .unwrap()
            .value(),
        2
    );
    assert_eq!(
        subtract_unsigned::<255, 255, 255>(small, large),
        Err(CompactError::UnsignedUnderflow)
    );
    assert_eq!(
        multiply_unsigned::<255, 255, 255>(small, large)
            .unwrap()
            .value(),
        15
    );
    let sixteen = BoundedUint::<255>::new(16).unwrap();
    assert_eq!(
        multiply_unsigned::<255, 255, 255>(sixteen, sixteen),
        Err(CompactError::UnsignedOutOfRange {
            value: 256,
            max: 255,
        })
    );
    let largest = BoundedUint::<{ u128::MAX }>::new(u128::MAX).unwrap();
    assert_eq!(
        multiply_unsigned::<{ u128::MAX }, 2, { u128::MAX }>(
            largest,
            BoundedUint::<2>::new(2).unwrap(),
        ),
        Err(CompactError::UnsignedOverflow)
    );
}

#[test]
fn compact_uint_maximum_is_enforced_at_input_and_decode() {
    type Small = BoundedUint<8>;
    assert_eq!(Small::new(8).unwrap().value(), 8);
    assert_eq!(
        Small::new(9),
        Err(CompactError::UnsignedOutOfRange { value: 9, max: 8 })
    );
    assert_eq!(Small::from_field_repr(&[Field::from(9_u64)]), None);
}

#[test]
fn compact_uint_round_trips_through_the_ledger_field_representation() {
    type Large = BoundedUint<{ u128::MAX }>;
    let value = Large::new(u128::MAX).unwrap();
    assert_eq!(Large::from_field_repr(&value.field_vec()), Some(value));
}

#[test]
fn compact_uint_uses_the_declared_byte_alignment_and_ledger_value() {
    type Byte = BoundedUint<255>;
    assert_eq!(Byte::BYTE_LENGTH, 1);
    assert_eq!(
        Byte::alignment(),
        Alignment::singleton(AlignmentAtom::Bytes { length: 1 })
    );
    let value = Byte::new(254).unwrap();
    let aligned = AlignedValue::from(value);
    assert_eq!(aligned.value, Value(vec![ValueAtom(vec![254])]));
    assert_eq!(Byte::try_from(&*aligned.as_slice()), Ok(value));
    assert_eq!(
        Byte::try_from(&*Value(vec![ValueAtom(vec![0, 1])])),
        Err(CompactError::UnsignedOutOfRange {
            value: 256,
            max: 255
        })
    );
    assert_eq!(BoundedUint::<0>::BYTE_LENGTH, 0);
    assert_eq!(value.binary_vec(), vec![254]);
}

fn assert_ledger_encoding<const MAX: u128>(values: &[u128]) {
    let width = BoundedUint::<MAX>::BYTE_LENGTH as usize;
    for &integer in values {
        let value = BoundedUint::<MAX>::new(integer).unwrap();
        let bytes = integer.to_le_bytes()[..width].to_vec();
        let aligned = AlignedValue::from(value);
        let mut normalized = bytes.clone();
        while normalized.last() == Some(&0) {
            normalized.pop();
        }
        assert_eq!(aligned.value, Value(vec![ValueAtom(normalized)]));
        assert_eq!(
            BoundedUint::<MAX>::try_from(&*aligned.as_slice()),
            Ok(value)
        );
        assert_eq!(value.binary_vec(), bytes);
        assert_eq!(
            BoundedUint::<MAX>::from_field_repr(&value.field_vec()),
            Some(value)
        );
    }
}

#[test]
fn compact_uint_exhaustive_byte_domain_matches_ledger_encoding() {
    assert_ledger_encoding::<255>(&(0..=255).collect::<Vec<_>>());
    assert_ledger_encoding::<99>(&(0..=99).collect::<Vec<_>>());
    for integer in 100..=255 {
        assert!(BoundedUint::<99>::try_from(&*Value(vec![ValueAtom(vec![integer])])).is_err());
    }
}

#[test]
fn compact_uint_wider_boundaries_match_ledger_encoding() {
    assert_ledger_encoding::<69_999>(&[0, 1, 255, 256, 65_535, 65_536, 69_999]);
    assert_ledger_encoding::<4_999_999_999>(&[
        0,
        1,
        255,
        256,
        u32::MAX as u128,
        u32::MAX as u128 + 1,
        4_999_999_999,
    ]);
}
