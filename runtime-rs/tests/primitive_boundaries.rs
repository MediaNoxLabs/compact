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

//! Malformed public decoder inputs and checked arithmetic boundaries.
use midnight_base_crypto::fab::{Value, ValueAtom};
use midnight_compact_runtime::{
    BoundedUint, CompactError, Field, FieldRepr, FixedBytes, FixedVector, FromFieldRepr, WideUint,
    add_unsigned, narrow_wide_uint, subtract_unsigned,
};

type Wide = WideUint<1, 10>;

#[test]
fn wide_decoders_reject_wrong_arity_and_fab_overwidth() {
    for atoms in [
        vec![],
        vec![ValueAtom(vec![1]), ValueAtom(vec![2])],
        vec![ValueAtom(vec![0; 18])],
    ] {
        assert_eq!(
            Wide::try_from(&*Value(atoms)),
            Err(CompactError::InvalidUnsignedValue)
        );
    }
    assert_eq!(Wide::from_field_repr(&[]), None);
    assert_eq!(
        Wide::from_field_repr(&[Field::from(0), Field::from(0)]),
        None
    );
    assert_eq!(
        Wide::from_le_bytes(&[0; 33]),
        Err(CompactError::InvalidUnsignedValue)
    );
}

#[test]
fn byte_input_padding_is_distinct_from_fab_declared_width() {
    let minimal = Wide::from_le_bytes(&[42]).unwrap();
    let mut padded = [0; 32];
    padded[0] = 42;
    assert_eq!(Wide::from_le_bytes(&padded), Ok(minimal));
    assert_eq!(
        Wide::try_from(&*Value(vec![ValueAtom(padded.to_vec())])),
        Err(CompactError::InvalidUnsignedValue)
    );
    padded[31] = 1;
    assert_eq!(
        Wide::from_le_bytes(&padded),
        Err(CompactError::InvalidUnsignedValue)
    );
    assert_eq!(
        WideUint::<0, 255>::from_le_bytes(&[1]),
        Err(CompactError::InvalidUnsignedValue)
    );
    assert_eq!(
        WideUint::<{ u128::MAX }, { u128::MAX }>::from_le_bytes(&[1]),
        Err(CompactError::InvalidUnsignedValue)
    );
}

#[test]
fn wide_narrowing_distinguishes_nonzero_high_limb_and_target_bound() {
    let low = Wide::from_le_bytes(&[9]).unwrap();
    assert_eq!(
        narrow_wide_uint::<8, 1, 10>(low),
        Err(CompactError::UnsignedOutOfRange { value: 9, max: 8 })
    );
    let mut bytes = [0; 17];
    bytes[16] = 1;
    assert_eq!(
        narrow_wide_uint::<{ u128::MAX }, 1, 10>(Wide::from_le_bytes(&bytes).unwrap()),
        Err(CompactError::InvalidUnsignedValue)
    );
}

#[test]
fn fixed_bytes_enforce_exact_field_count_and_zero_width() {
    assert_eq!(
        FixedBytes::<0>::from_field_repr(&[]),
        Some(FixedBytes::new([]))
    );
    assert_eq!(FixedBytes::<0>::from_field_repr(&[Field::from(0)]), None);
    assert_eq!(FixedBytes::<31>::from_field_repr(&[]), None);
    assert_eq!(
        FixedBytes::<31>::from_field_repr(&[Field::from(0), Field::from(0)]),
        None
    );
    assert_eq!(FixedBytes::<32>::from_field_repr(&[Field::from(0)]), None);
    assert_eq!(
        FixedBytes::<32>::from_field_repr(&[Field::from(0); 3]),
        None
    );
}

#[test]
fn fixed_bytes_reject_bits_outside_full_and_partial_field_chunks() {
    let mut high = [0; 32];
    high[31] = 1; // 2^248 is a valid field value but does not fit 31 bytes.
    let too_wide = Field::from_le_bytes(&high).unwrap();
    assert_eq!(FixedBytes::<31>::from_field_repr(&[too_wide]), None);
    // The upstream byte representation puts the one-byte remainder first.
    assert_eq!(
        FixedBytes::<32>::from_field_repr(&[Field::from(256), Field::from(0)]),
        None
    );
    assert_eq!(
        FixedBytes::<32>::from_field_repr(&[Field::from(0), too_wide]),
        None
    );
    let mut expected = [0; 32];
    expected[0] = 7;
    expected[31] = 9;
    assert_eq!(
        FixedBytes::<32>::from_field_repr(&[Field::from(9), Field::from(7)]),
        Some(FixedBytes::new(expected))
    );
}

#[test]
fn fixed_vectors_reject_invalid_nested_elements_and_preserve_zero_width_cases() {
    type Pair = FixedVector<(Field, bool), 2>;
    let valid = [
        Field::from(3),
        Field::from(1),
        Field::from(5),
        Field::from(0),
    ];
    assert!(Pair::from_field_repr(&valid).is_some());
    assert_eq!(Pair::from_field_repr(&valid[..3]), None);
    assert_eq!(Pair::from_field_repr(&[Field::from(0); 5]), None);
    for index in [1, 3] {
        let mut invalid = valid;
        invalid[index] = Field::from(2);
        assert_eq!(Pair::from_field_repr(&invalid), None);
    }
    let empty = FixedVector::<Field, 0>::new([]);
    assert_eq!(empty.field_vec(), vec![]);
    assert_eq!(FixedVector::<Field, 0>::from_field_repr(&[]), Some(empty));
    assert_eq!(
        FixedVector::<Field, 0>::from_field_repr(&[Field::from(0)]),
        None
    );
    assert_eq!(
        FixedVector::<(), 2>::from_field_repr(&[]),
        Some(FixedVector::new([(), ()]))
    );
}

#[test]
fn successful_host_arithmetic_still_enforces_the_declared_result_bound() {
    let five = BoundedUint::<10>::new(5).unwrap();
    let four = BoundedUint::<10>::new(4).unwrap();
    assert_eq!(
        add_unsigned::<10, 10, 8>(five, four),
        Err(CompactError::UnsignedOutOfRange { value: 9, max: 8 })
    );
    assert_eq!(add_unsigned::<10, 10, 9>(five, four).unwrap().value(), 9);
    assert_eq!(
        subtract_unsigned::<10, 10, 0>(five, four),
        Err(CompactError::UnsignedOutOfRange { value: 1, max: 0 })
    );
    assert_eq!(
        subtract_unsigned::<10, 10, 1>(five, four).unwrap().value(),
        1
    );
    assert_eq!(
        subtract_unsigned::<10, 10, 0>(four, five),
        Err(CompactError::UnsignedUnderflow)
    );
    assert_eq!(
        subtract_unsigned::<10, 10, 0>(five, five).unwrap().value(),
        0
    );
}
