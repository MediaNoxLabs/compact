// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Regression for the legacy nested-comparison interpolation panic.
use super::*;

#[test]
fn nested_boolean_equality_preserves_both_ast_children_without_interpolation_panics() {
    for outer_negated in [false, true] {
        for inner_negated in [false, true] {
            for on_left in [false, true] {
                let inner = equality(inner_negated, boolean(true), boolean(false));
                let source = if on_left {
                    equality(outer_negated, inner, boolean(true))
                } else {
                    equality(outer_negated, boolean(true), inner)
                };
                let (syntax, actual) =
                    crate::pure_arithmetic_expression(&source, &HashMap::new(), &HashMap::new())
                        .unwrap();
                assert_eq!(actual, Type::Boolean);
                let syn::Expr::Binary(binary) = syntax else {
                    panic!("equality keeps a Binary AST");
                };
                assert_eq!(matches!(binary.op, syn::BinOp::Ne(_)), outer_negated);
                let child = if on_left { binary.left } else { binary.right };
                let syn::Expr::Binary(inner) = *child else {
                    panic!("nested comparison must retain its child AST");
                };
                assert_eq!(matches!(inner.op, syn::BinOp::Ne(_)), inner_negated);
                let (result, visits) =
                    visitation::observe(|| observe(&source, &HashMap::new(), &HashMap::new()));
                result.unwrap();
                assert_eq!(visits.len(), 5);
                assert!(visits.values().all(|count| *count == 1));
            }
        }
    }
}
