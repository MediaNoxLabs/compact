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

//! Representation equivalence against upstream derives and explicit field delegation.

use midnight_compact_runtime::{
    BinaryHashRepr, BoundedUint, CompactStructRepr, Field, FieldRepr, FixedVector, Fr,
    FromFieldRepr, MemWrite,
};

#[derive(Debug, Clone, PartialEq, Eq, FieldRepr, BinaryHashRepr, FromFieldRepr)]
struct Upstream {
    enabled: bool,
    amount: BoundedUint<255>,
    fields: FixedVector<Field, 2>,
}

#[derive(Debug, Clone, PartialEq, Eq, CompactStructRepr)]
struct Owned {
    enabled: bool,
    amount: BoundedUint<255>,
    fields: FixedVector<Field, 2>,
}

#[derive(Debug, PartialEq, Eq, FieldRepr, BinaryHashRepr, FromFieldRepr)]
struct UpstreamNested {
    first: Upstream,
    last: Field,
}

#[derive(Debug, PartialEq, Eq, CompactStructRepr)]
struct OwnedNested {
    first: Owned,
    last: Field,
}

#[derive(Debug, PartialEq, Eq, CompactStructRepr)]
struct Empty {}

// Upstream supports a unit empty struct; its named-empty decoder emits an invalid
// empty FIELD_SIZE expression. Compare the supported zero-field representation.
#[derive(Debug, PartialEq, Eq, FieldRepr, BinaryHashRepr, FromFieldRepr)]
struct UpstreamEmpty;

fn samples() -> (Owned, Upstream) {
    let fields = FixedVector::new([Field::from(17_u64), Field::from(31_u64)]);
    (
        Owned {
            enabled: true,
            amount: BoundedUint::new(42).unwrap(),
            fields: fields.clone(),
        },
        Upstream {
            enabled: true,
            amount: BoundedUint::new(42).unwrap(),
            fields,
        },
    )
}

fn equivalent<A, B>(a: &A, b: &B)
where
    A: FieldRepr + BinaryHashRepr + FromFieldRepr + PartialEq + std::fmt::Debug,
    B: FieldRepr + BinaryHashRepr + FromFieldRepr + PartialEq + std::fmt::Debug,
{
    assert_eq!(a.field_vec(), b.field_vec());
    assert_eq!(a.binary_vec(), b.binary_vec());
    assert_eq!(a.field_size(), b.field_size());
    assert_eq!(a.binary_len(), b.binary_len());
    assert_eq!(A::FIELD_SIZE, B::FIELD_SIZE);
    let fields = a.field_vec();
    assert_eq!(A::from_field_repr(&fields).as_ref(), Some(a));
    assert_eq!(B::from_field_repr(&fields).as_ref(), Some(b));
    for end in 0..fields.len() {
        assert_eq!(A::from_field_repr(&fields[..end]), None);
        assert_eq!(B::from_field_repr(&fields[..end]), None);
    }
    let mut trailing = fields;
    trailing.push(Field::from(0_u64));
    assert_eq!(A::from_field_repr(&trailing), None);
    assert_eq!(B::from_field_repr(&trailing), None);
}

#[test]
fn ordinary_vector_and_bounded_fields_match_upstream_and_manual_order() {
    let (owned, upstream) = samples();
    equivalent(&owned, &upstream);
    let mut manual_fields = Vec::new();
    owned.enabled.field_repr(&mut manual_fields);
    owned.amount.field_repr(&mut manual_fields);
    owned.fields.field_repr(&mut manual_fields);
    assert_eq!(owned.field_vec(), manual_fields);
    let mut manual_bytes = Vec::new();
    owned.enabled.binary_repr(&mut manual_bytes);
    owned.amount.binary_repr(&mut manual_bytes);
    owned.fields.binary_repr(&mut manual_bytes);
    assert_eq!(owned.binary_vec(), manual_bytes);
}

#[test]
fn nested_structs_preserve_upstream_order_sizes_and_decode_boundaries() {
    let (owned, upstream) = samples();
    equivalent(
        &OwnedNested {
            first: owned,
            last: Field::from(99_u64),
        },
        &UpstreamNested {
            first: upstream,
            last: Field::from(99_u64),
        },
    );
}

#[test]
fn empty_struct_has_zero_representation_and_rejects_trailing_field() {
    equivalent(&Empty {}, &UpstreamEmpty);
    assert!(Empty {}.field_vec().is_empty());
    assert!(Empty {}.binary_vec().is_empty());
}

#[test]
fn member_decoder_rejection_is_preserved() {
    let (owned, _) = samples();
    let mut fields = owned.field_vec();
    fields[1] = Field::from(256_u64);
    assert_eq!(Owned::from_field_repr(&fields), None);
    assert_eq!(Upstream::from_field_repr(&fields), None);
}

#[allow(dead_code, non_camel_case_types)]
mod hostile {
    use midnight_compact_runtime as rt;
    pub struct Option;
    pub struct Some;
    pub struct None;
    pub struct Fr;
    pub struct MemWrite;
    pub struct FieldRepr;
    pub struct BinaryHashRepr;
    pub struct FromFieldRepr;
    pub struct usize;
    pub struct u8;
    pub struct runtime;

    #[derive(Debug, PartialEq, Eq, rt::CompactStructRepr)]
    pub struct __CompactWriter {
        pub __compact_input: bool,
        pub __compact_size: rt::BoundedUint<255>,
        pub __compact_field_0: rt::Field,
        pub __compact_repr: rt::Field,
        pub __compact_writer: rt::Field,
        pub r#type: rt::FixedVector<rt::Field, 0>,
    }

    #[derive(Debug, PartialEq, Eq, rt::CompactStructRepr)]
    pub struct Nested {
        pub value: __CompactWriter,
    }
}

#[test]
fn hostile_names_and_zero_width_member_decode_without_local_shadowing() {
    let value = hostile::Nested {
        value: hostile::__CompactWriter {
            __compact_input: true,
            __compact_size: BoundedUint::new(42).unwrap(),
            __compact_field_0: Field::from(17_u64),
            __compact_repr: Field::from(31_u64),
            __compact_writer: Field::from(99_u64),
            r#type: FixedVector::new([]),
        },
    };
    assert_eq!(
        value.field_vec(),
        vec![
            Field::from(1_u64),
            Field::from(42_u64),
            Field::from(17_u64),
            Field::from(31_u64),
            Field::from(99_u64)
        ]
    );
    assert_eq!(
        hostile::Nested::from_field_repr(&value.field_vec()),
        Some(value)
    );
}
