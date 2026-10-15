// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use super::*;

#[test]
fn struct_literal_error_pairs_keep_field_name_and_late_type_name_order() {
    let params = HashMap::new();
    let circuits = HashMap::new();
    assert_eq!(
        compare(
            &Expr::StructLiteral {
                ty: Type::Field,
                fields: vec![reference("missing")]
            },
            &params,
            &circuits
        ),
        Err(RenderError::InvalidStructField("<literal>".into()))
    );
    let bad_field = structure(
        "bad-type",
        vec![("bad-field", Type::Field), ("next", Type::Field)],
    );
    assert_eq!(
        compare(
            &Expr::StructLiteral {
                ty: bad_field.clone(),
                fields: vec![reference("missing")]
            },
            &params,
            &circuits
        ),
        Err(RenderError::InvalidStructField("bad-type".into()))
    );
    assert_eq!(
        compare(
            &Expr::StructLiteral {
                ty: bad_field.clone(),
                fields: vec![boolean(true), reference("missing")]
            },
            &params,
            &circuits
        ),
        Err(RenderError::TypeMismatch {
            expected: Type::Field,
            actual: Type::Boolean
        })
    );
    assert_eq!(
        compare(
            &Expr::StructLiteral {
                ty: bad_field,
                fields: vec![field("1"), reference("missing")]
            },
            &params,
            &circuits
        ),
        Err(RenderError::InvalidIdentifier("bad-field".into()))
    );
    let ty = structure("bad-type", vec![("good", Type::Field)]);
    assert_eq!(
        compare(
            &Expr::StructLiteral {
                ty: ty.clone(),
                fields: vec![reference("missing")]
            },
            &params,
            &circuits
        ),
        Err(RenderError::UnknownParameter("missing".into()))
    );
    assert_eq!(
        compare(
            &Expr::StructLiteral {
                ty,
                fields: vec![field("1")]
            },
            &params,
            &circuits
        ),
        Err(RenderError::InvalidIdentifier("bad-type".into()))
    );
}

#[test]
fn exact_struct_shorthand_tuple_punctuation_and_ordinary_wide_tuples_are_preserved() {
    let ty = structure("Record$Raw", vec![("foo", Type::Field)]);
    let value_type = Type::Field;
    for symbol in [syn::parse_quote!(foo), syn::parse_quote!(other)] {
        let params = HashMap::from([("source$raw", (&value_type, symbol))]);
        assert_eq!(
            compare(
                &Expr::StructLiteral {
                    ty: ty.clone(),
                    fields: vec![reference("source$raw")]
                },
                &params,
                &HashMap::new()
            ),
            Ok(ty.clone())
        );
    }
    for count in [0, 1, 2, 13] {
        let source = Expr::Tuple {
            elements: vec![field("1"); count],
        };
        assert_eq!(
            compare(&source, &HashMap::new(), &HashMap::new()),
            Ok(Type::Tuple {
                elements: vec![Type::Field; count]
            })
        );
        let (output, _) =
            crate::pure_aggregate_expression(&source, &HashMap::new(), &HashMap::new()).unwrap();
        let syn::Expr::Tuple(tuple) = output else {
            panic!("tuple AST");
        };
        assert_eq!(tuple.elems.trailing_punct(), count == 1);
    }
}

#[test]
fn only_empty_vector_validates_annotation_carrier_and_first_mismatch_stops_later_children() {
    let bad = uint("malformed");
    let params = HashMap::from([("value", (&bad, syn::parse_quote!(value)))]);
    assert_eq!(
        compare(
            &Expr::Vector {
                element: bad.clone(),
                elements: vec![]
            },
            &params,
            &HashMap::new()
        ),
        Err(RenderError::InvalidUnsignedMaximum("malformed".into()))
    );
    assert_eq!(
        compare(
            &Expr::Vector {
                element: bad.clone(),
                elements: vec![reference("value")]
            },
            &params,
            &HashMap::new()
        ),
        Ok(vector_type(bad.clone(), 1))
    );
    assert_eq!(
        compare(
            &Expr::Vector {
                element: bad.clone(),
                elements: vec![boolean(false), reference("missing")]
            },
            &params,
            &HashMap::new()
        ),
        Err(RenderError::TypeMismatch {
            expected: bad,
            actual: Type::Boolean
        })
    );
    assert_eq!(
        compare(&vector(vec![]), &HashMap::new(), &HashMap::new()),
        Ok(vector_type(Type::Field, 0))
    );
}
