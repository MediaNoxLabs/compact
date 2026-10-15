// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! A complete frozen helper independently authenticates output-only factoring.
use super::*;
fn tokens(result: Result<syn::Expr, RenderError>) -> Result<String, RenderError> {
    result.map(|expr| expr.to_token_stream().to_string())
}
#[test]
fn checked_strategy_rendering_and_legacy_entrypoint_match_exact_old_helper() {
    for kind in [
        ArithmeticKind::Add,
        ArithmeticKind::Subtract,
        ArithmeticKind::Multiply,
    ] {
        let operation = unsigned(kind, "255", Expr::Unit, Expr::Unit);
        for (left, right, result) in [
            ("15", "255", "65535"),
            ("0", "0", "0"),
            ("255", WIDE, WIDE),
            (WIDE, "255", WIDE),
            (WIDE, WIDE, MAX_248),
            (WIDE, "255", "255"),
            ("255", WIDE, "255"),
            ("bad", "01", "bad"),
            (WIDE, "01", "bad"),
            (WIDE, "255", "bad"),
            ("255", "255", TOO_WIDE),
        ] {
            let l: syn::Expr = syn::parse_quote!(probe_left()?);
            let r: syn::Expr = syn::parse_quote!(probe_right()?);
            let old = tokens(old_unsigned::unsigned_arithmetic_syntax(
                &operation,
                l.clone(),
                r.clone(),
                left,
                right,
                result,
            ));
            assert_eq!(
                tokens(crate::value_lowering::unsigned_arithmetic_syntax(
                    &operation,
                    l.clone(),
                    r.clone(),
                    left,
                    right,
                    result
                )),
                old
            );
            let checked = crate::pure_semantics::numeric::checked_unsigned_arithmetic(
                kind, left, right, result,
            )
            .map(|strategy| crate::value_lowering::render_unsigned_arithmetic(strategy, l, r));
            assert_eq!(tokens(checked), old);
        }
    }
}
