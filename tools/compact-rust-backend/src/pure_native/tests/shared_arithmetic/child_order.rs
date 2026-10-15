// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Frozen native views and neutral outcomes for focused arithmetic controls.
use super::*;

#[test]
fn child_failure_beats_operand_type_max_and_tuple_rules() {
    for ty in [
        Type::Boolean,
        uint("bad"),
        uint(WIDE),
        Type::Tuple {
            elements: vec![Type::Field; 13],
        },
    ] {
        let parameters = HashMap::from([("left", (&ty, syn::parse_quote!(left_argument)))]);
        let sources = [
            field(ArithmeticKind::Add, reference("left"), reference("missing")),
            unsigned(
                ArithmeticKind::Multiply,
                "bad",
                reference("left"),
                reference("missing"),
            ),
            equality(false, reference("left"), reference("missing")),
            ordered(
                ComparisonOperator::Less,
                reference("left"),
                reference("missing"),
            ),
        ];
        for source in sources {
            assert_eq!(
                compare(&source, &parameters),
                Err(RenderError::UnknownParameter("missing".into()))
            );
            let (_, visits) =
                visitation::observe(|| observe(&source, &parameters, &HashMap::new()));
            assert_eq!(visits.len(), 3);
            assert!(visits.values().all(|count| *count == 1));
        }
    }
    let bad = unsigned(
        ArithmeticKind::Subtract,
        "bad",
        reference("first_missing"),
        reference("second_missing"),
    );
    assert_eq!(
        compare(&bad, &HashMap::new()),
        Err(RenderError::UnknownParameter("first_missing".into()))
    );
    let (_, visits) = visitation::observe(|| observe(&bad, &HashMap::new(), &HashMap::new()));
    assert_eq!(visits.len(), 2);
    assert!(visits.values().all(|count| *count == 1));
    // Child success still permits original left-kind-before-right-kind rules.
    let boolean = Type::Boolean;
    let unit = Type::Unit;
    let parameters = HashMap::from([
        ("left", (&boolean, syn::parse_quote!(l))),
        ("right", (&unit, syn::parse_quote!(r))),
    ]);
    assert_eq!(
        compare(
            &field(ArithmeticKind::Add, reference("left"), reference("right")),
            &parameters
        ),
        Err(RenderError::TypeMismatch {
            expected: Type::Field,
            actual: Type::Boolean
        })
    );
    assert_eq!(
        compare(
            &unsigned(
                ArithmeticKind::Add,
                "bad",
                reference("left"),
                reference("right")
            ),
            &parameters
        ),
        Err(RenderError::TypeMismatch {
            expected: uint("bad"),
            actual: Type::Boolean
        })
    );
}
