// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Frozen native views and neutral outcomes for focused arithmetic controls.
use super::*;

#[test]
fn nested_field_binary_ast_and_conversion_children_preserve_output_and_one_visit() {
    for kind in [
        ArithmeticKind::Add,
        ArithmeticKind::Subtract,
        ArithmeticKind::Multiply,
    ] {
        let source = field(
            kind,
            literal("10"),
            field(ArithmeticKind::Subtract, literal("7"), literal("2")),
        );
        assert_eq!(compare(&source, &HashMap::new()), Ok(Type::Field));
        let (syntax, _) =
            crate::pure_arithmetic_expression(&source, &HashMap::new(), &HashMap::new()).unwrap();
        let syn::Expr::Binary(binary) = syntax else {
            panic!("Field arithmetic must retain Binary AST");
        };
        assert!(
            matches!(binary.right.as_ref(), syn::Expr::Binary(inner) if matches!(inner.op, syn::BinOp::Sub(_)))
        );
        let (result, visits) =
            visitation::observe(|| observe(&source, &HashMap::new(), &HashMap::new()));
        result.unwrap();
        assert_eq!(visits.len(), 5);
        assert!(visits.values().all(|count| *count == 1));
    }
    let source = field(
        ArithmeticKind::Add,
        Expr::FieldCast {
            value: Box::new(unsigned(
                ArithmeticKind::Add,
                "255",
                Expr::UnsignedLiteral {
                    value: "1".into(),
                    max: "15".into(),
                },
                Expr::UnsignedCast {
                    max: "15".into(),
                    value: Box::new(Expr::UnsignedLiteral {
                        value: "2".into(),
                        max: "255".into(),
                    }),
                },
            )),
        },
        literal("3"),
    );
    assert_eq!(compare(&source, &HashMap::new()), Ok(Type::Field));
    let (result, visits) =
        visitation::observe(|| observe(&source, &HashMap::new(), &HashMap::new()));
    result.unwrap();
    assert_eq!(visits.len(), 7);
    assert!(visits.values().all(|count| *count == 1));
}
