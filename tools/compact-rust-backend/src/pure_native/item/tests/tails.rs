// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use super::*;

#[test]
fn value_tails_preserve_outer_try_peeling_nested_fallibility_and_binary_grouping() {
    let expressions: Vec<syn::Expr> = vec![
        syn::parse_quote!(read_once()),
        syn::parse_quote!(fallible()?),
        syn::parse_quote!({
            let value = fallible()?;
            value
        }),
        syn::parse_quote!(a - (b - c)),
        syn::parse_quote!(a * (b + c)),
        syn::parse_quote!(if flag { left()? } else { right()? }),
    ];
    for internal in [false, true] {
        for expression in &expressions {
            compare(
                syn::parse_quote!(probe),
                internal,
                vec![],
                syn::parse_quote!(runtime::Field),
                NativeFragment::Value(expression.clone()),
                Type::Field,
            );
        }
    }
    let mut call: syn::ExprCall = syn::parse_quote!(Ok(marker));
    let body: syn::Expr = syn::parse_quote!({
        let value = call_once()?;
        value
    });
    *call.args.first_mut().unwrap() = body.clone();
    assert_eq!(
        value_tail(body).to_token_stream().to_string(),
        call.to_token_stream().to_string()
    );
    let tail = value_tail(syn::parse_quote!(fallible()?));
    assert!(matches!(tail, syn::Expr::Call(_)));
}

#[test]
fn unit_statements_remain_in_order_and_finish_with_ok_unit_for_both_visibilities() {
    for internal in [false, true] {
        for statements in [
            vec![],
            vec![syn::parse_quote!(check_once()?;)],
            vec![
                syn::parse_quote!(let value = acquire_once()?;),
                syn::parse_quote!(if !value {
                    return Err(runtime::CompactError::AssertionFailed("guard".into()));
                }),
                syn::parse_quote!(release_once()?;),
            ],
        ] {
            compare(
                syn::parse_quote!(unit_probe),
                internal,
                vec![],
                syn::parse_quote!(()),
                NativeFragment::unit(
                    syn::parse_quote!({
                        ignored_expression()?;
                        ()
                    }),
                    statements,
                ),
                Type::Unit,
            );
        }
    }
}
