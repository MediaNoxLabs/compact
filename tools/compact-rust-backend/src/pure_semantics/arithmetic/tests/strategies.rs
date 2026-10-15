// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Parse-before-width diagnostics and the wide-result Add exception.
use super::*;

#[test]
fn all_bounds_are_checked_before_wide_applicability_and_mixed_width_add_is_allowed() {
    let left = parameter("left");
    let right = parameter("right");
    for (l, r, result, expected) in [
        ("bad", "01", "wrong", "bad"),
        (WIDE, "01", "wrong", "01"),
        (WIDE, "255", "wrong", "wrong"),
    ] {
        let lt = uint(l);
        let rt = uint(r);
        let mut children = LeafChildren {
            bindings: HashMap::from([("left", (&lt, ())), ("right", (&rt, ()))]),
            visits: vec![],
        };
        let mut sink = Consumer::default();
        assert_eq!(
            fold_arithmetic(
                ArithmeticSource::Unsigned {
                    kind: ArithmeticKind::Subtract,
                    max: result,
                    left: &left,
                    right: &right
                },
                &mut children,
                &mut sink
            )
            .err(),
            Some(RenderError::InvalidUnsignedMaximum(expected.into()))
        );
        assert!(sink.facts.is_empty());
    }
    for (l, r) in [("255", WIDE), (WIDE, "255"), (WIDE, WIDE), ("255", "255")] {
        let lt = uint(l);
        let rt = uint(r);
        let mut children = LeafChildren {
            bindings: HashMap::from([("left", (&lt, ())), ("right", (&rt, ()))]),
            visits: vec![],
        };
        let mut sink = Consumer::default();
        assert_eq!(
            fold_arithmetic(
                ArithmeticSource::Unsigned {
                    kind: ArithmeticKind::Add,
                    max: WIDE,
                    left: &left,
                    right: &right
                },
                &mut children,
                &mut sink
            )
            .unwrap()
            .ty,
            uint(WIDE)
        );
        assert!(matches!(
            sink.facts.as_slice(),
            [ArithmeticFacts::Unsigned(
                UnsignedArithmeticStrategy::WideAdd { high: 1, low: 0 }
            )]
        ));
    }
}
