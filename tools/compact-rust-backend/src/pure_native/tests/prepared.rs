// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use super::*;

#[test]
fn prepared_unit_views_are_distinct_without_any_source_or_type_checker() {
    let assertion = control::assertion(syn::parse_quote!(flag), "guard", std::iter::empty());
    let fragment = fragment::lexical(
        vec![syn::parse_quote!(let saved: bool = flag;)],
        fragment::sequence(
            assertion.into_views().1,
            NativeFragment::unit(syn::parse_quote!(()), Vec::new()),
        ),
    );
    let (expression, statements) = fragment.into_views();
    let expected_expression: syn::Expr = syn::parse_quote!({
        let saved: bool = flag;
        {
            if !(flag) {
                return Err(runtime::CompactError::AssertionFailed("guard".to_owned()));
            }
            ()
        }
    });
    let expected_statement: syn::Stmt = syn::parse_quote!({
        let saved: bool = flag;
        if !(flag) {
            return Err(runtime::CompactError::AssertionFailed("guard".to_owned()));
        }
    });
    assert_eq!(
        expression.to_token_stream().to_string(),
        expected_expression.to_token_stream().to_string()
    );
    assert_eq!(
        quote::quote!(#(#statements)*).to_string(),
        expected_statement.to_token_stream().to_string()
    );
}

#[test]
fn prepared_equal_arms_keep_one_fallible_condition_and_single_selected_call() {
    let arm = || {
        NativeFragment::unit(
            syn::parse_quote!(check()?),
            vec![syn::parse_quote!(check()?;)],
        )
    };
    let fragment = control::conditional(
        IfFacts {
            equal_arms: true,
            evaluate_condition: true,
            unit_result: true,
        },
        syn::parse_quote!(probe()?),
        arm(),
        arm(),
        std::iter::empty(),
    );
    let (expression, statements) = fragment.into_views();
    let expected_expression: syn::Expr = syn::parse_quote!({
        let _ = probe()?;
        check()?
    });
    let expected_statements: syn::Block = syn::parse_quote!({
        let _ = probe()?;
        check()?;
    });
    assert_eq!(
        expression.to_token_stream().to_string(),
        expected_expression.to_token_stream().to_string()
    );
    let expected = expected_statements.stmts;
    assert_eq!(
        quote::quote!(#(#statements)*).to_string(),
        quote::quote!(#(#expected)*).to_string()
    );
}
