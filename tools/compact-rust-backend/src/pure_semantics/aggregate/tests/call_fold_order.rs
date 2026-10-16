// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use super::*;
fn folding(name: &str, initial: Expr, source: Expr) -> Expr {
    Expr::VectorFoldCall {
        name: name.into(),
        initial: Box::new(initial),
        source: Box::new(source),
        accumulator: Type::Field,
        element: Type::Field,
        length: 0,
    }
}

#[test]
fn raw_lookup_and_arity_precede_children_while_argument_mismatch_is_immediate() {
    let target = circuit("bad-name", vec![Type::Field, Type::Field], Type::Field);
    let declarations = HashMap::from([("bad-name", &target)]);
    assert_eq!(
        check(
            &Expr::Call {
                name: "missing".into(),
                arguments: vec![parameter("missing")]
            },
            HashMap::new(),
            &declarations
        ),
        Err(RenderError::UnknownCircuit("missing".into()))
    );
    assert_eq!(
        check(
            &Expr::Call {
                name: "bad-name".into(),
                arguments: vec![parameter("missing")]
            },
            HashMap::new(),
            &declarations
        ),
        Err(RenderError::ArgumentCount {
            circuit: "bad-name".into(),
            expected: 2,
            actual: 1
        })
    );
    assert_eq!(
        check(
            &Expr::Call {
                name: "bad-name".into(),
                arguments: vec![Expr::Boolean { value: false }, parameter("missing")]
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
            &Expr::Call {
                name: "bad-name".into(),
                arguments: vec![field("1"), field("2")]
            },
            HashMap::new(),
            &declarations
        ),
        Err(RenderError::InvalidIdentifier("bad-name".into()))
    );
}

#[test]
fn fold_signature_payload_initial_coercion_source_and_name_keep_original_order() {
    let wrong = circuit("bad-name", vec![Type::Boolean], Type::Boolean);
    let declarations = HashMap::from([("bad-name", &wrong)]);
    assert_eq!(
        check(
            &folding(
                "bad-name",
                parameter("missing_initial"),
                parameter("missing_source")
            ),
            HashMap::new(),
            &declarations
        ),
        Err(RenderError::TypeMismatch {
            expected: Type::Field,
            actual: Type::Boolean
        })
    );
    let target = circuit("bad-name", vec![Type::Field, Type::Field], Type::Field);
    let declarations = HashMap::from([("bad-name", &target)]);
    assert_eq!(
        check(
            &folding(
                "bad-name",
                Expr::Boolean { value: false },
                parameter("missing_source")
            ),
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
            &folding("bad-name", field("0"), parameter("missing_source")),
            HashMap::new(),
            &declarations
        ),
        Err(RenderError::UnknownParameter("missing_source".into()))
    );
    assert_eq!(
        check(
            &folding("bad-name", field("0"), Expr::Boolean { value: false }),
            HashMap::new(),
            &declarations
        ),
        Err(RenderError::TypeMismatch {
            expected: vector_type(Type::Field, 0),
            actual: Type::Boolean
        })
    );
    assert_eq!(
        check(
            &folding(
                "bad-name",
                field("0"),
                Expr::Vector {
                    element: Type::Field,
                    elements: vec![]
                }
            ),
            HashMap::new(),
            &declarations
        ),
        Err(RenderError::InvalidIdentifier("bad-name".into()))
    );
}
