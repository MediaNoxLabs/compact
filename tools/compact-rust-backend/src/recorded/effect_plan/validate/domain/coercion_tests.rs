// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Shared conversion facts retain the stricter recorded value domain.
use super::{Type, coercible, unsigned_bound, value_type};
use crate::ir::StructField;
mod old_reference;

fn uint(max: &str) -> Type {
    Type::Unsigned { max: max.into() }
}
fn tuple(elements: Vec<Type>) -> Type {
    Type::Tuple { elements }
}
fn vector(element: Type, length: usize) -> Type {
    Type::Vector {
        element: Box::new(element),
        length,
    }
}
fn named(field: Type) -> Type {
    Type::Struct {
        name: "Record".into(),
        fields: vec![StructField {
            name: "value".into(),
            ty: field,
        }],
    }
}

#[test]
fn recorded_conversion_matches_the_frozen_relation_on_bounded_domains() {
    let mut cases = vec![
        Type::Unit,
        Type::Boolean,
        Type::Field,
        Type::JubjubPoint,
        Type::OpaqueString,
        Type::OpaqueBytes,
        Type::Bytes { length: 0 },
        Type::Bytes { length: 32 },
        Type::Bytes { length: 65537 },
        uint("0"),
        uint("15"),
        uint("255"),
        uint("340282366920938463463374607431768211456"),
        uint(""),
        uint("00"),
        uint("-1"),
        tuple(vec![]),
        tuple(vec![Type::Boolean; 13]),
        Type::Struct {
            name: "Empty".into(),
            fields: vec![],
        },
        named(Type::OpaqueString),
        named(tuple(vec![Type::Boolean; 9])),
        Type::Enum {
            name: "Tag".into(),
            variants: vec!["A".into(), "B".into()],
        },
        Type::Enum {
            name: "EmptyTag".into(),
            variants: vec![],
        },
        Type::LedgerMap {
            key: Box::new(Type::Boolean),
            value: Box::new(Type::Field),
        },
    ];
    for ty in [
        Type::Boolean,
        Type::Field,
        uint("15"),
        uint("255"),
        uint("00"),
        Type::OpaqueBytes,
    ] {
        cases.extend([
            vector(ty.clone(), 0),
            vector(ty.clone(), 2),
            tuple(vec![ty.clone(), ty.clone()]),
            vector(tuple(vec![ty.clone(), Type::Boolean]), 2),
            tuple(vec![vector(ty.clone(), 2), vector(ty, 2)]),
        ]);
    }
    for source in &cases {
        for target in &cases {
            assert_eq!(
                coercible(source, target),
                old_reference::coercible(source, target),
                "source={source:?}, target={target:?}"
            );
        }
    }
}

#[test]
fn native_identity_does_not_bypass_recorded_domain_requirements() {
    for ty in [
        uint("00"),
        Type::LedgerMap {
            key: Box::new(Type::Boolean),
            value: Box::new(Type::Boolean),
        },
        named(tuple(vec![Type::Boolean; 9])),
        Type::Enum {
            name: "Empty".into(),
            variants: vec![],
        },
    ] {
        assert!(crate::pure_semantics::coercion::plan_coercion(&ty, &ty).is_ok());
        assert!(!coercible(&ty, &ty));
    }
    let owned = tuple(vec![Type::OpaqueString, Type::OpaqueBytes]);
    assert!(coercible(&owned, &owned));
}

#[test]
fn root_preflight_refuses_oversized_private_recorded_conversions() {
    let large = Type::Enum {
        name: "TooLarge".into(),
        variants: vec!["x".repeat(262145)],
    };
    assert!(!coercible(&large, &large));
    let near = vector(uint("15"), usize::MAX);
    assert!(
        coercible(&near, &vector(uint("255"), usize::MAX)),
        "declared vector length does not unroll conversion planning"
    );
}
