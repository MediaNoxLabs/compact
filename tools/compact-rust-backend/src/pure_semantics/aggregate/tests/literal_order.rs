// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use super::*;

#[test]
fn struct_shape_count_field_type_field_name_and_type_name_keep_their_positions() {
    let declarations = HashMap::new();
    let bad = Expr::StructLiteral {
        ty: Type::Field,
        fields: vec![parameter("missing")],
    };
    assert_eq!(
        check(&bad, HashMap::new(), &declarations),
        Err(RenderError::InvalidStructField("<literal>".into()))
    );
    let ty = Type::Struct {
        name: "Bad-name".into(),
        fields: vec![
            StructField {
                name: "bad-name".into(),
                ty: Type::Field,
            },
            StructField {
                name: "later".into(),
                ty: Type::Field,
            },
        ],
    };
    assert_eq!(
        check(
            &Expr::StructLiteral {
                ty: ty.clone(),
                fields: vec![parameter("missing")]
            },
            HashMap::new(),
            &declarations
        ),
        Err(RenderError::InvalidStructField("Bad-name".into()))
    );
    assert_eq!(
        check(
            &Expr::StructLiteral {
                ty: ty.clone(),
                fields: vec![Expr::Boolean { value: true }, parameter("missing")]
            },
            HashMap::new(),
            &declarations
        ),
        Err(RenderError::TypeMismatch {
            expected: Type::Field,
            actual: Type::Boolean
        })
    );
    assert_eq!(
        check(
            &Expr::StructLiteral {
                ty: ty.clone(),
                fields: vec![field("1"), parameter("missing")]
            },
            HashMap::new(),
            &declarations
        ),
        Err(RenderError::InvalidIdentifier("bad-name".into()))
    );
    let ty = Type::Struct {
        name: "Bad-name".into(),
        fields: vec![StructField {
            name: "good".into(),
            ty: Type::Field,
        }],
    };
    assert_eq!(
        check(
            &Expr::StructLiteral {
                ty: ty.clone(),
                fields: vec![parameter("missing")]
            },
            HashMap::new(),
            &declarations
        ),
        Err(RenderError::UnknownParameter("missing".into()))
    );
    assert_eq!(
        check(
            &Expr::StructLiteral {
                ty,
                fields: vec![field("1")]
            },
            HashMap::new(),
            &declarations
        ),
        Err(RenderError::InvalidIdentifier("Bad-name".into()))
    );
}

#[test]
fn empty_vector_only_folds_carrier_and_nonempty_matching_malformed_types_stay_private_compatible() {
    let declarations = HashMap::new();
    let bad = uint("bad");
    assert_eq!(
        check(
            &Expr::Vector {
                element: bad.clone(),
                elements: vec![]
            },
            HashMap::new(),
            &declarations
        ),
        Err(RenderError::InvalidUnsignedMaximum("bad".into()))
    );
    assert_eq!(
        check(
            &Expr::Vector {
                element: bad.clone(),
                elements: vec![parameter("value")]
            },
            HashMap::from([("value", (&bad, ()))]),
            &declarations
        ),
        Ok(vector_type(bad.clone(), 1))
    );
    assert_eq!(
        check(
            &Expr::Vector {
                element: bad,
                elements: vec![Expr::Boolean { value: true }, parameter("missing")]
            },
            HashMap::new(),
            &declarations
        ),
        Err(RenderError::TypeMismatch {
            expected: uint("bad"),
            actual: Type::Boolean
        })
    );
    assert_eq!(
        check(
            &Expr::Tuple {
                elements: vec![field("0"); 13]
            },
            HashMap::new(),
            &declarations
        ),
        Ok(Type::Tuple {
            elements: vec![Type::Field; 13]
        })
    );
}
