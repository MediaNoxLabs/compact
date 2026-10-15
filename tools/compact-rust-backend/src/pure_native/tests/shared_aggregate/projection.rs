// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use super::*;

#[test]
fn projections_preserve_kind_index_raw_field_name_and_identifier_precedence() {
    let ty = structure(
        "Private$Record",
        vec![("bad-name", Type::Field), ("good", Type::OpaqueString)],
    );
    let bindings = HashMap::from([("record", (&ty, syn::parse_quote!(record)))]);
    for (field_name, index, expected) in [
        (
            "missing",
            0,
            RenderError::InvalidStructField("missing".into()),
        ),
        (
            "bad-name",
            1,
            RenderError::InvalidStructField("bad-name".into()),
        ),
        (
            "bad-name",
            9,
            RenderError::InvalidStructField("bad-name".into()),
        ),
        (
            "bad-name",
            0,
            RenderError::InvalidIdentifier("bad-name".into()),
        ),
    ] {
        assert_eq!(
            compare(
                &Expr::StructField {
                    value: Box::new(reference("record")),
                    field: field_name.into(),
                    index
                },
                &bindings,
                &HashMap::new()
            ),
            Err(expected)
        );
    }
    assert_eq!(
        compare(
            &Expr::StructField {
                value: Box::new(reference("missing")),
                field: "bad-name".into(),
                index: 99
            },
            &bindings,
            &HashMap::new()
        ),
        Err(RenderError::UnknownParameter("missing".into()))
    );
    assert_eq!(
        compare(
            &Expr::TupleIndex {
                value: Box::new(field("1")),
                index: 99
            },
            &HashMap::new(),
            &HashMap::new()
        ),
        Err(RenderError::ExpectedTuple(Type::Field))
    );
    assert_eq!(
        compare(
            &Expr::TupleIndex {
                value: Box::new(Expr::Tuple { elements: vec![] }),
                index: 0
            },
            &HashMap::new(),
            &HashMap::new()
        ),
        Err(RenderError::InvalidTupleIndex(0))
    );
}

#[test]
fn copy_owned_and_unit_projection_views_remain_distinct_from_tuple_projection() {
    for member in [
        Type::Unit,
        Type::Field,
        Type::OpaqueString,
        vector_type(Type::Field, 1),
        Type::Tuple {
            elements: vec![Type::Field; 13],
        },
    ] {
        let ty = structure("Envelope", vec![("value", member.clone())]);
        let bindings = HashMap::from([("record", (&ty, syn::parse_quote!(record)))]);
        assert_eq!(
            compare(
                &Expr::StructField {
                    value: Box::new(reference("record")),
                    field: "value".into(),
                    index: 0
                },
                &bindings,
                &HashMap::new()
            ),
            Ok(member.clone())
        );
        let tuple = Type::Tuple {
            elements: vec![member.clone()],
        };
        let bindings = HashMap::from([("tuple", (&tuple, syn::parse_quote!(tuple)))]);
        assert_eq!(
            compare(
                &Expr::TupleIndex {
                    value: Box::new(reference("tuple")),
                    index: 0
                },
                &bindings,
                &HashMap::new()
            ),
            Ok(member)
        );
    }
}
