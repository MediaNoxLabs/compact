// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use super::*;
fn unsigned(max: &str) -> Type {
    Type::Unsigned { max: max.into() }
}
#[test]
fn checked_aggregate_conversion_relation_matches_native_supported_cases() {
    let narrow = unsigned("255");
    let wide = unsigned("65535");
    let large = unsigned("680564733841876926926749214863536422911");
    let cases = vec![
        (narrow.clone(), wide.clone()),
        (wide.clone(), large.clone()),
        (large, Type::Field),
        (
            tuple(vec![narrow.clone(), narrow.clone()]),
            vector(wide.clone(), 2),
        ),
        (
            vector(narrow.clone(), 2),
            tuple(vec![wide.clone(), Type::Field]),
        ),
        (
            tuple(vec![narrow.clone(), narrow.clone()]),
            tuple(vec![wide.clone(), Type::Field]),
        ),
        (
            vector(vector(narrow.clone(), 2), 3),
            vector(vector(wide.clone(), 2), 3),
        ),
        (vector(narrow, 0), vector(wide, 0)),
        (vector(Type::OpaqueString, 0), vector(Type::OpaqueString, 0)),
        (tuple(vec![]), tuple(vec![])),
    ];
    for (source, target) in cases {
        assert!(
            validate::coercible(&source, &target),
            "{source:?} -> {target:?}"
        );
        crate::coerce_expression(syn::parse_quote!(operand), &source, &target, 0).unwrap();
    }
}
#[test]
fn checked_conversion_rejects_native_narrowing_shapes_and_empty_nonidentity() {
    let cases = vec![
        (unsigned("65535"), unsigned("255")),
        (Type::Field, unsigned("255")),
        (Type::OpaqueBytes, Type::Bytes { length: 32 }),
        (vector(Type::Field, 1), vector(Type::Field, 2)),
        (tuple(vec![]), vector(Type::Field, 0)),
        (vector(Type::Field, 0), tuple(vec![])),
        (
            tuple(vec![Type::Field]),
            tuple(vec![Type::Field, Type::Field]),
        ),
        (vector(unsigned("65535"), 2), vector(unsigned("255"), 2)),
        (
            carrier(),
            Type::Struct {
                name: "Other".into(),
                fields: match carrier() {
                    Type::Struct { fields, .. } => fields,
                    _ => unreachable!(),
                },
            },
        ),
    ];
    for (source, target) in cases {
        assert!(
            !validate::coercible(&source, &target),
            "{source:?} -> {target:?}"
        );
        assert!(crate::coerce_expression(syn::parse_quote!(operand), &source, &target, 0).is_err());
    }
}
#[test]
fn source_aggregate_conversions_retain_operand_once_then_allow_reuse() {
    let narrow = unsigned("255");
    let wide = unsigned("65535");
    let small = || Expr::UnsignedLiteral {
        value: "7".into(),
        max: "255".into(),
    };
    let source = construct_vector(vec![small(), small()], narrow.clone());
    for target in [
        vector(wide.clone(), 2),
        tuple(vec![wide.clone(), Type::Field]),
    ] {
        let conversion = Expr::Coerce {
            value: Box::new(param("source")),
            ty: target.clone(),
        };
        accept(
            local(
                "source",
                vector(narrow.clone(), 2),
                source.clone(),
                local(
                    "converted",
                    target,
                    conversion,
                    eq(param("source"), param("source")),
                ),
            ),
            vec![],
        );
    }
    let t = tuple(vec![narrow.clone(), narrow]);
    accept(
        local(
            "source",
            t,
            Expr::Tuple {
                elements: vec![small(), small()],
            },
            local(
                "converted",
                tuple(vec![wide.clone(), Type::Field]),
                Expr::Coerce {
                    value: Box::new(param("source")),
                    ty: tuple(vec![wide, Type::Field]),
                },
                eq(param("source"), param("source")),
            ),
        ),
        vec![],
    );
}
