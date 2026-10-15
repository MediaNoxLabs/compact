// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Default's Rust tuple12 capability is independent of FAB11 and named derives8.
use super::*;
fn tuple(length: usize) -> Type {
    Type::Tuple {
        elements: vec![Type::Field; length],
    }
}
fn vector(element: Type, length: usize) -> Type {
    Type::Vector {
        element: Box::new(element),
        length,
    }
}
fn named(member: Type) -> Type {
    Type::Struct {
        name: "DefaultCarrier".into(),
        fields: vec![crate::ir::StructField {
            name: "value".into(),
            ty: member,
        }],
    }
}
fn default(ty: &Type) -> Expr {
    Expr::Default { ty: ty.clone() }
}

#[test]
fn default_capability_covers_existing_carriers_and_independent_tuple_boundaries() {
    for ty in [
        Type::Unit,
        Type::Boolean,
        Type::Field,
        Type::JubjubPoint,
        Type::Bytes { length: 0 },
        Type::Bytes { length: 32 },
        Type::OpaqueString,
        Type::OpaqueBytes,
        validate::uint("255"),
        validate::uint("340282366920938463463374607431768211456"),
        Type::Enum {
            name: "Choice".into(),
            variants: vec!["First".into(), "Second".into()],
        },
        Type::Struct {
            name: "Empty".into(),
            fields: vec![],
        },
        tuple(0),
        tuple(11),
        tuple(12),
        vector(Type::Field, 0),
        vector(tuple(12), 2),
        named(tuple(8)),
        named(vector(Type::OpaqueString, 2)),
    ] {
        assert!(validate::default_value_type(&ty), "{ty:?}");
        let checked = checked(&source(default(&ty), ty.clone()));
        assert!(matches!(
            checked.plan().body.bindings[0].operation,
            Operation::Default
        ));
        render(&checked).unwrap();
    }
    assert!(validate::value_type(&tuple(13)));
    assert!(validate::default_value_type(&tuple(12)));
    assert!(!validate::witness_result_type(&tuple(12)));
    assert!(!validate::public_interface_type(&tuple(12)));
    for ty in [
        tuple(13),
        vector(tuple(13), 0),
        vector(tuple(13), 2),
        named(tuple(9)),
        named(vector(tuple(9), 0)),
    ] {
        assert!(!validate::default_value_type(&ty), "{ty:?}");
        gap(&source(default(&ty), ty));
    }
    // Constructed local tuple13 remains legal: Default alone needs its trait bound.
    checked(&source(
        Expr::Tuple {
            elements: vec![field("0"); 13],
        },
        tuple(13),
    ));
}

#[test]
fn default_checker_independently_refuses_unimplemented_trait_and_noncanonical_types() {
    let checked = checked(&source(
        Expr::Tuple {
            elements: vec![field("0"); 13],
        },
        tuple(13),
    ));
    let mut forged = checked.plan().clone();
    forged.body.bindings.last_mut().unwrap().operation = Operation::Default;
    let error = validate::validate(&forged, checked.declarations()).unwrap_err();
    assert_eq!(error.path, "return_value.value");
    for ty in [
        validate::uint("00"),
        Type::Enum {
            name: "EmptyChoice".into(),
            variants: vec![],
        },
    ] {
        assert!(!validate::default_value_type(&ty));
        assert!(check(&source(default(&ty), ty), &HashMap::new(), &HashMap::new()).is_err());
    }
    let canonical = named(Type::Field);
    let checked = super::checked(&source(default(&canonical), canonical));
    let mut forged = checked.plan().clone();
    forged.body.bindings[0].value.ty = named(Type::Boolean);
    assert!(validate::validate(&forged, checked.declarations()).is_err());
}
