// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use super::*;
fn mapping(name: &str, input: Type, source: Expr, body: Expr, result: Type, length: usize) -> Expr {
    Expr::VectorMap {
        parameter: Parameter {
            name: name.into(),
            ty: input,
        },
        source: Box::new(source),
        body: Box::new(body),
        result,
        length,
    }
}

#[test]
fn map_coercion_precedes_binding_name_and_zero_length_still_checks_scoped_body() {
    let declarations = HashMap::new();
    let wrong_source = mapping(
        "bad-name",
        Type::Field,
        Expr::Boolean { value: true },
        parameter("missing"),
        Type::Field,
        0,
    );
    assert_eq!(
        check(&wrong_source, HashMap::new(), &declarations),
        Err(RenderError::TypeMismatch {
            expected: vector_type(Type::Field, 0),
            actual: Type::Boolean
        })
    );
    let source = || Expr::Vector {
        element: Type::Field,
        elements: vec![],
    };
    assert_eq!(
        check(
            &mapping(
                "bad-name",
                Type::Field,
                source(),
                parameter("missing"),
                Type::Field,
                0
            ),
            HashMap::new(),
            &declarations
        ),
        Err(RenderError::InvalidIdentifier("bad-name".into()))
    );
    assert_eq!(
        check(
            &mapping(
                "item$raw",
                Type::Field,
                source(),
                parameter("missing"),
                Type::Field,
                0
            ),
            HashMap::new(),
            &declarations
        ),
        Err(RenderError::UnknownParameter("missing".into()))
    );
    assert_eq!(
        check(
            &mapping(
                "item$raw",
                Type::Field,
                source(),
                parameter("item$raw"),
                Type::Field,
                0
            ),
            HashMap::new(),
            &declarations
        ),
        Ok(vector_type(Type::Field, 0))
    );
}

#[test]
fn lexical_raw_key_shadow_restores_outer_type_after_nested_body_visit() {
    let declarations = HashMap::new();
    let outer = Type::Boolean;
    let body = mapping(
        "item$raw",
        Type::Field,
        Expr::Vector {
            element: Type::Field,
            elements: vec![field("1")],
        },
        parameter("item$raw"),
        Type::Field,
        1,
    );
    let source = Expr::Tuple {
        elements: vec![body, parameter("item$raw")],
    };
    let mut visitor = Visitor::new(HashMap::from([("item$raw", (&outer, ()))]), &declarations);
    let mut sink = Consumer::default();
    assert_eq!(
        visitor.check(&source, &mut sink).unwrap().ty,
        Type::Tuple {
            elements: vec![vector_type(Type::Field, 1), Type::Boolean]
        }
    );
    assert_eq!(visitor.visits.len(), 6);
    assert_eq!(
        visitor.check(&parameter("item$raw"), &mut sink).unwrap().ty,
        Type::Boolean
    );
}
