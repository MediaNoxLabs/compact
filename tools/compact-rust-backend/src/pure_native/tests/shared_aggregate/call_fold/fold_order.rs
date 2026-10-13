// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

use super::*;

#[test]
fn fold_signature_premises_use_original_result_payload_before_any_child() {
    for target in [
        declaration("step", vec![], Type::Field),
        declaration("step", vec![Type::Boolean, Type::Field], Type::Field),
        declaration("step", vec![Type::Field, Type::Boolean], Type::Field),
        declaration("step", vec![Type::Field, Type::Field], Type::Boolean),
    ] {
        let circuits = HashMap::from([("step", &target)]);
        assert_eq!(
            compare(
                &folding(
                    "step",
                    reference("missing_initial"),
                    reference("missing_source"),
                    Type::Field,
                    Type::Field,
                    0
                ),
                &HashMap::new(),
                &circuits
            ),
            Err(RenderError::TypeMismatch {
                expected: Type::Field,
                actual: target.result.clone()
            })
        );
    }
}

#[test]
fn fold_initial_coercion_source_coercion_and_name_follow_signature_gate() {
    let target = declaration("bad-name", vec![Type::Field, Type::Field], Type::Field);
    let circuits = HashMap::from([("bad-name", &target)]);
    let source =
        |initial, values| folding("bad-name", initial, values, Type::Field, Type::Field, 0);
    assert_eq!(
        compare(
            &source(reference("missing_initial"), reference("missing_source")),
            &HashMap::new(),
            &circuits
        ),
        Err(RenderError::UnknownParameter("missing_initial".into()))
    );
    assert_eq!(
        compare(
            &source(boolean(true), reference("missing_source")),
            &HashMap::new(),
            &circuits
        ),
        Err(RenderError::TypeMismatch {
            expected: Type::Field,
            actual: Type::Boolean
        })
    );
    assert_eq!(
        compare(
            &source(field("0"), reference("missing_source")),
            &HashMap::new(),
            &circuits
        ),
        Err(RenderError::UnknownParameter("missing_source".into()))
    );
    assert_eq!(
        compare(
            &source(field("0"), boolean(true)),
            &HashMap::new(),
            &circuits
        ),
        Err(RenderError::TypeMismatch {
            expected: vector_type(Type::Field, 0),
            actual: Type::Boolean
        })
    );
    assert_eq!(
        compare(
            &source(field("0"), vector(vec![])),
            &HashMap::new(),
            &circuits
        ),
        Err(RenderError::InvalidIdentifier("bad-name".into()))
    );
    let target = declaration("step$raw", vec![Type::Field, Type::Field], Type::Field);
    let circuits = HashMap::from([("step$raw", &target)]);
    let source = folding(
        "step$raw",
        Expr::UnsignedLiteral {
            value: "0".into(),
            max: "15".into(),
        },
        Expr::Vector {
            element: uint("15"),
            elements: vec![Expr::UnsignedLiteral {
                value: "1".into(),
                max: "15".into(),
            }],
        },
        Type::Field,
        Type::Field,
        1,
    );
    assert_eq!(
        compare(&source, &HashMap::new(), &circuits),
        Ok(Type::Field)
    );
}
