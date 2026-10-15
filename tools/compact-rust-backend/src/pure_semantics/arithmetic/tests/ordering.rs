// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Paired child and operand failures preserve observable first-refusal order.
use super::*;

#[test]
fn right_child_error_precedes_operand_rules_and_left_error_stops_traversal() {
    let left = parameter("left");
    let right = parameter("missing");
    for ty in [
        Type::Boolean,
        uint("bad"),
        uint(WIDE),
        Type::Tuple {
            elements: vec![Type::Field; 13],
        },
    ] {
        for source in [
            ArithmeticSource::Field {
                kind: ArithmeticKind::Add,
                left: &left,
                right: &right,
            },
            ArithmeticSource::Unsigned {
                kind: ArithmeticKind::Multiply,
                max: "bad",
                left: &left,
                right: &right,
            },
            ArithmeticSource::Equality {
                negated: false,
                left: &left,
                right: &right,
            },
            ArithmeticSource::Ordered {
                operator: ComparisonOperator::Less,
                left: &left,
                right: &right,
            },
        ] {
            let mut children = LeafChildren {
                bindings: HashMap::from([("left", (&ty, ()))]),
                visits: vec![],
            };
            let mut sink = Consumer::default();
            assert_eq!(
                fold_arithmetic(source, &mut children, &mut sink).err(),
                Some(RenderError::UnknownParameter("missing".into()))
            );
            assert_eq!(
                children.visits,
                [
                    std::ptr::from_ref(&left) as usize,
                    std::ptr::from_ref(&right) as usize
                ]
            );
            assert!(sink.facts.is_empty());
        }
    }
    let mut children = LeafChildren {
        bindings: HashMap::new(),
        visits: vec![],
    };
    let mut sink = Consumer::default();
    assert_eq!(
        fold_arithmetic(
            ArithmeticSource::Equality {
                negated: true,
                left: &left,
                right: &right
            },
            &mut children,
            &mut sink
        )
        .err(),
        Some(RenderError::UnknownParameter("left".into()))
    );
    assert_eq!(children.visits, [std::ptr::from_ref(&left) as usize]);
    assert!(sink.facts.is_empty());
}

#[test]
fn ordered_and_equality_rules_keep_first_refusal_after_children() {
    let left = parameter("left");
    let right = parameter("right");
    for (lt, rt, expected) in [
        (
            Type::Boolean,
            uint("bad"),
            RenderError::ExpectedUnsigned(Type::Boolean),
        ),
        (
            uint("bad"),
            Type::Boolean,
            RenderError::InvalidUnsignedMaximum("bad".into()),
        ),
        (
            uint(WIDE),
            Type::Boolean,
            RenderError::UnsupportedWideUnsignedOperation {
                operation: "ordered comparison",
                max: WIDE.into(),
            },
        ),
        (
            uint("255"),
            Type::Boolean,
            RenderError::ExpectedUnsigned(Type::Boolean),
        ),
    ] {
        let mut children = LeafChildren {
            bindings: HashMap::from([("left", (&lt, ())), ("right", (&rt, ()))]),
            visits: vec![],
        };
        let mut sink = Consumer::default();
        assert_eq!(
            fold_arithmetic(
                ArithmeticSource::Ordered {
                    operator: ComparisonOperator::Less,
                    left: &left,
                    right: &right
                },
                &mut children,
                &mut sink
            )
            .err(),
            Some(expected)
        );
        assert_eq!(children.visits.len(), 2);
        assert!(sink.facts.is_empty());
    }
    let wide_tuple = Type::Tuple {
        elements: vec![Type::Field; 13],
    };
    let boolean = Type::Boolean;
    let mut children = LeafChildren {
        bindings: HashMap::from([("left", (&wide_tuple, ())), ("right", (&boolean, ()))]),
        visits: vec![],
    };
    assert_eq!(
        fold_arithmetic(
            ArithmeticSource::Equality {
                negated: false,
                left: &left,
                right: &right
            },
            &mut children,
            &mut Consumer::default()
        )
        .err(),
        Some(RenderError::TypeMismatch {
            expected: wide_tuple,
            actual: boolean
        })
    );
}
