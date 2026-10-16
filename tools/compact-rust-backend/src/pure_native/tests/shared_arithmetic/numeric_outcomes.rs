// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Frozen native views and neutral outcomes for focused arithmetic controls.
use super::*;

#[test]
fn all_unsigned_operations_preserve_width_and_bound_outcomes() {
    // Distinct maxima exercise left/right/result parsing rather than relying on
    // literal typing to catch malformed or unsupported annotations earlier.
    for left in ["0", "255", WIDE, MAX_248, "01", "bad"] {
        for right in ["15", WIDE, "01"] {
            for result in ["65535", WIDE, MAX_248, TOO_WIDE, "bad"] {
                let lt = uint(left);
                let rt = uint(right);
                let parameters = HashMap::from([
                    ("left$key", (&lt, syn::parse_quote!(left_arg))),
                    ("right$key", (&rt, syn::parse_quote!(right_arg))),
                ]);
                for kind in [
                    ArithmeticKind::Add,
                    ArithmeticKind::Subtract,
                    ArithmeticKind::Multiply,
                ] {
                    let _ = compare(
                        &unsigned(kind, result, reference("left$key"), reference("right$key")),
                        &parameters,
                    );
                }
            }
        }
    }
}

#[test]
fn every_ordered_operator_keeps_left_checks_before_right_checks_after_visiting() {
    for operator in [
        ComparisonOperator::Less,
        ComparisonOperator::LessEqual,
        ComparisonOperator::Greater,
        ComparisonOperator::GreaterEqual,
    ] {
        for left in [Type::Boolean, uint("bad"), uint(WIDE), uint("15")] {
            for right in [Type::Boolean, uint("01"), uint(WIDE), uint("255")] {
                let parameters = HashMap::from([
                    ("left", (&left, syn::parse_quote!(l))),
                    ("right", (&right, syn::parse_quote!(r))),
                ]);
                let _ = compare(
                    &ordered(operator, reference("left"), reference("right")),
                    &parameters,
                );
            }
        }
    }
}
