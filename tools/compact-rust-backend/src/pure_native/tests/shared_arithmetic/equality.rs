// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Frozen native views and neutral outcomes for focused arithmetic controls.
use super::*;

#[test]
fn equality_preserves_identity_then_scoped_recursive_tuple_capability() {
    for negated in [false, true] {
        for arity in [0, 12, 13] {
            for ty in [
                Type::Tuple {
                    elements: vec![Type::Field; arity],
                },
                Type::Vector {
                    element: Box::new(Type::Tuple {
                        elements: vec![Type::Field; arity],
                    }),
                    length: 3,
                },
            ] {
                let parameters = HashMap::from([
                    ("left", (&ty, syn::parse_quote!(l))),
                    ("right", (&ty, syn::parse_quote!(r))),
                ]);
                let result = compare(
                    &equality(negated, reference("left"), reference("right")),
                    &parameters,
                );
                if arity <= 12 {
                    assert_eq!(result, Ok(Type::Boolean));
                } else {
                    assert_eq!(
                        result,
                        Err(RenderError::UnsupportedTupleOperation {
                            operation: if negated { "inequality" } else { "equality" },
                            arity: 13,
                            limit: 12,
                            required_trait: "PartialEq"
                        })
                    );
                }
            }
        }
        let tuple = Type::Tuple {
            elements: vec![Type::Field; 13],
        };
        let boolean = Type::Boolean;
        let parameters = HashMap::from([
            ("left", (&tuple, syn::parse_quote!(l))),
            ("right", (&boolean, syn::parse_quote!(r))),
        ]);
        assert_eq!(
            compare(
                &equality(negated, reference("left"), reference("right")),
                &parameters
            ),
            Err(RenderError::TypeMismatch {
                expected: tuple,
                actual: boolean
            })
        );
    }
    // Private malformed bound remains within selected-Type scan's string cap;
    // derived Type equality must not acquire coercion's 131072-work relation.
    let malformed = uint(&"x".repeat(131073));
    let parameters = HashMap::from([
        ("left", (&malformed, syn::parse_quote!(l))),
        ("right", (&malformed, syn::parse_quote!(r))),
    ]);
    for negated in [false, true] {
        assert_eq!(
            compare(
                &equality(negated, reference("left"), reference("right")),
                &parameters
            ),
            Ok(Type::Boolean)
        );
    }
}
