// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//  	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Differential numeric boundary and native-output contracts.

use super::*;
use crate::ir::{Expr, Type};
use crate::value_lowering::{coerce_expression, unsigned_arithmetic_syntax, unsigned_cast_syntax};
use quote::ToTokens;

#[path = "old_reference.rs"]
mod old;

const TWO_128: &str = "340282366920938463463374607431768211456";
const MAX_248: &str = "452312848583266388373324160190187140051835877600158453279131187530910662655";
const TWO_248: &str = "452312848583266388373324160190187140051835877600158453279131187530910662656";
const FIELD_MODULUS: &str =
    "52435875175126190479447740508185965837690552500527637822603658699938581184513";

fn operation(kind: ArithmeticKind) -> Expr {
    let left = Box::new(Expr::Boolean { value: false });
    let right = Box::new(Expr::Boolean { value: true });
    match kind {
        ArithmeticKind::Add => Expr::UnsignedAdd {
            max: "255".into(),
            left,
            right,
        },
        ArithmeticKind::Subtract => Expr::UnsignedSubtract {
            max: "255".into(),
            left,
            right,
        },
        ArithmeticKind::Multiply => Expr::UnsignedMultiply {
            max: "255".into(),
            left,
            right,
        },
    }
}

fn output(result: Result<syn::Expr, RenderError>) -> Result<String, RenderError> {
    result.map(|value| value.to_token_stream().to_string())
}

#[test]
fn canonical_parsers_match_the_authenticated_old_policy() {
    for text in [
        "0",
        "1",
        "255",
        "256",
        "340282366920938463463374607431768211455",
        TWO_128,
        MAX_248,
        TWO_248,
        FIELD_MODULUS,
        "52435875175126190479447740508185965837690552500527637822603658699938581184512",
        "115792089237316195423570985008687907853269984665640564039457584007913129639936",
        "",
        "00",
        "01",
        "+1",
        "-1",
        " 1",
        "1 ",
        "1\n",
        "1.0",
        "0x10",
        "１",
        "١",
    ] {
        assert_eq!(
            field_literal_bytes(text),
            old::field_literal_bytes(text),
            "{text:?}"
        );
        let prior = old::unsigned_maximum(text).map(|bound| match bound {
            old::UnsignedMaximum::Small(value) => UnsignedMaximum::Small(value),
            old::UnsignedMaximum::Wide { high, low } => UnsignedMaximum::Wide { high, low },
        });
        assert_eq!(unsigned_maximum(text), prior, "{text:?}");
    }
    let overflow = "9".repeat(200);
    assert_eq!(
        field_literal_bytes(&overflow),
        old::field_literal_bytes(&overflow)
    );
}

#[test]
fn explicit_cast_strategy_covers_each_carrier_pair_and_allows_narrowing() {
    assert_eq!(
        checked_unsigned_cast("255", "15"),
        Ok(UnsignedCastStrategy::SmallToSmall {
            source: 255,
            target: 15
        })
    );
    assert_eq!(
        checked_unsigned_cast("255", TWO_128),
        Ok(UnsignedCastStrategy::SmallToWide { high: 1, low: 0 })
    );
    assert_eq!(
        checked_unsigned_cast(TWO_128, "15"),
        Ok(UnsignedCastStrategy::WideToSmall {
            high: 1,
            low: 0,
            target: 15
        })
    );
    assert_eq!(
        checked_unsigned_cast(MAX_248, TWO_128),
        Ok(UnsignedCastStrategy::WideToWide { high: 1, low: 0 })
    );
    for (source, target) in [("255", "15"), (TWO_128, "15"), (MAX_248, TWO_128)] {
        let source = Type::Unsigned { max: source.into() };
        let target = Type::Unsigned { max: target.into() };
        assert_eq!(
            coerce_expression(syn::parse_quote!(input), &source, &target, 0).err(),
            Some(RenderError::TypeMismatch {
                expected: target,
                actual: source
            })
        );
    }
}

#[test]
fn cast_adapters_preserve_old_output_and_invalid_bound_priority() {
    let bounds = [
        "0",
        "15",
        "255",
        "340282366920938463463374607431768211455",
        TWO_128,
        MAX_248,
        TWO_248,
        "01",
        "bad",
    ];
    for source in bounds {
        for target in bounds {
            assert_eq!(
                output(unsigned_cast_syntax(
                    syn::parse_quote!(input()),
                    source,
                    target
                )),
                output(old::unsigned_cast_syntax(
                    syn::parse_quote!(input()),
                    source,
                    target
                )),
                "{source} -> {target}",
            );
        }
    }
    assert_eq!(
        checked_unsigned_cast("01", "bad"),
        Err(RenderError::InvalidUnsignedMaximum("01".into()))
    );
}

#[test]
fn wide_result_addition_admits_both_operand_carriers() {
    for left in ["255", TWO_128, MAX_248] {
        for right in ["255", TWO_128, MAX_248] {
            assert_eq!(
                checked_unsigned_arithmetic(ArithmeticKind::Add, left, right, TWO_128),
                Ok(UnsignedArithmeticStrategy::WideAdd { high: 1, low: 0 })
            );
        }
    }
}

#[test]
fn unsupported_wide_operations_report_the_first_wide_bound() {
    for kind in [
        ArithmeticKind::Add,
        ArithmeticKind::Subtract,
        ArithmeticKind::Multiply,
    ] {
        for (left, right, result, refused) in [
            (TWO_128, "255", "255", TWO_128),
            ("255", TWO_128, "255", TWO_128),
            (MAX_248, TWO_128, "255", MAX_248),
        ] {
            assert_eq!(
                checked_unsigned_arithmetic(kind, left, right, result),
                Err(RenderError::UnsupportedWideUnsignedOperation {
                    operation: kind.name(),
                    max: refused.into()
                })
            );
        }
        if kind != ArithmeticKind::Add {
            assert_eq!(
                checked_unsigned_arithmetic(kind, "255", "255", MAX_248),
                Err(RenderError::UnsupportedWideUnsignedOperation {
                    operation: kind.name(),
                    max: MAX_248.into()
                })
            );
        }
    }
}

#[test]
fn arithmetic_checks_malformed_bounds_before_wide_applicability() {
    for kind in [
        ArithmeticKind::Add,
        ArithmeticKind::Subtract,
        ArithmeticKind::Multiply,
    ] {
        for (left, right, result, refused) in [
            ("01", "bad", TWO_248, "01"),
            (TWO_128, "bad", "01", "bad"),
            (TWO_128, MAX_248, "01", "01"),
            ("255", "255", TWO_248, TWO_248),
        ] {
            assert_eq!(
                checked_unsigned_arithmetic(kind, left, right, result),
                Err(RenderError::InvalidUnsignedMaximum(refused.into()))
            );
        }
    }
}

#[test]
fn arithmetic_strategy_small_bounds_are_checked_for_each_kind() {
    for kind in [
        ArithmeticKind::Add,
        ArithmeticKind::Subtract,
        ArithmeticKind::Multiply,
    ] {
        assert_eq!(
            checked_unsigned_arithmetic(kind, "15", "255", "65535"),
            Ok(UnsignedArithmeticStrategy::Small {
                kind,
                left: 15,
                right: 255,
                result: 65535
            })
        );
    }
}

#[test]
fn arithmetic_adapters_match_old_native_output_and_errors_across_bound_partitions() {
    let bounds = [
        "0",
        "15",
        "255",
        "340282366920938463463374607431768211455",
        TWO_128,
        MAX_248,
        "01",
        "bad",
    ];
    for kind in [
        ArithmeticKind::Add,
        ArithmeticKind::Subtract,
        ArithmeticKind::Multiply,
    ] {
        let operation = operation(kind);
        for left in bounds {
            for right in bounds {
                for result in bounds {
                    assert_eq!(
                        output(unsigned_arithmetic_syntax(
                            &operation,
                            syn::parse_quote!(left()),
                            syn::parse_quote!(right()),
                            left,
                            right,
                            result
                        )),
                        output(old::unsigned_arithmetic_syntax(
                            &operation,
                            syn::parse_quote!(left()),
                            syn::parse_quote!(right()),
                            left,
                            right,
                            result
                        )),
                        "{kind:?} ({left}, {right}) -> {result}",
                    );
                }
            }
        }
    }
}
