// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Control formatting from checked children, source facts and occupied symbols.
//! This module accepts no source Expr/Type and makes no support decisions.

use super::fragment::NativeFragment;
use proc_macro2::Span;

pub(crate) struct IfFacts {
    pub(crate) equal_arms: bool,
    pub(crate) evaluate_condition: bool,
    pub(crate) unit_result: bool,
}

fn lift_block_condition<'a>(
    condition: syn::Expr,
    names: impl Iterator<Item = &'a syn::Ident> + Clone,
) -> (Vec<syn::Stmt>, syn::Expr) {
    fn contains_local(expression: &syn::Expr) -> bool {
        match expression {
            syn::Expr::Block(block) => block
                .block
                .stmts
                .iter()
                .any(|statement| matches!(statement, syn::Stmt::Local(_))),
            syn::Expr::Paren(paren) => contains_local(&paren.expr),
            syn::Expr::Group(group) => contains_local(&group.expr),
            _ => false,
        }
    }
    if !contains_local(&condition) {
        return (Vec::new(), condition);
    }
    let mut suffix = 0;
    let name = loop {
        let candidate = if suffix == 0 {
            "__compact_condition".to_owned()
        } else {
            format!("__compact_condition_{suffix}")
        };
        if names.clone().all(|name| *name != candidate) {
            break syn::Ident::new(&candidate, Span::call_site());
        }
        suffix += 1;
    };
    (
        vec![syn::parse_quote!(let #name: bool = #condition;)],
        syn::parse_quote!(#name),
    )
}

pub(crate) fn assertion<'a>(
    condition: syn::Expr,
    message: &str,
    names: impl Iterator<Item = &'a syn::Ident> + Clone,
) -> NativeFragment {
    let (mut statements, condition) = lift_block_condition(condition, names);
    let assertion: syn::Stmt = syn::parse_quote!(if !(#condition) {
        return Err(runtime::CompactError::AssertionFailed(#message.to_owned()));
    });
    let expression = syn::parse_quote!({ #(#statements)* #assertion });
    statements.push(assertion);
    NativeFragment::unit(expression, statements)
}

pub(crate) fn conditional<'a>(
    facts: IfFacts,
    condition: syn::Expr,
    then: NativeFragment,
    otherwise: NativeFragment,
    names: impl Iterator<Item = &'a syn::Ident> + Clone,
) -> NativeFragment {
    let (prefix, condition) = lift_block_condition(condition, names);
    let (then_expression, then_statements) = then.into_views();
    let (otherwise_expression, otherwise_statements) = otherwise.into_views();
    let expression = conditional_expression(
        &facts,
        &prefix,
        &condition,
        then_expression,
        otherwise_expression,
    );
    if facts.unit_result {
        let statements = conditional_statements(
            &facts,
            prefix,
            condition,
            then_statements,
            otherwise_statements,
        );
        NativeFragment::unit(expression, statements)
    } else {
        NativeFragment::Value(expression)
    }
}

fn conditional_expression(
    facts: &IfFacts,
    prefix: &[syn::Stmt],
    condition: &syn::Expr,
    then: syn::Expr,
    otherwise: syn::Expr,
) -> syn::Expr {
    if facts.equal_arms {
        if prefix.is_empty() {
            if facts.evaluate_condition {
                syn::parse_quote!({ let _ = #condition; #then })
            } else {
                then
            }
        } else {
            syn::parse_quote!({ #(#prefix)* #then })
        }
    } else {
        let conditional: syn::Expr = syn::parse_quote!(if #condition { #then } else { #otherwise });
        if prefix.is_empty() {
            conditional
        } else {
            syn::parse_quote!({ #(#prefix)* #conditional })
        }
    }
}

fn conditional_statements(
    facts: &IfFacts,
    mut prefix: Vec<syn::Stmt>,
    condition: syn::Expr,
    then: Vec<syn::Stmt>,
    otherwise: Vec<syn::Stmt>,
) -> Vec<syn::Stmt> {
    if facts.equal_arms {
        if facts.evaluate_condition && prefix.is_empty() {
            prefix.push(syn::parse_quote!(let _ = #condition;));
        }
        prefix.extend(then);
        return prefix;
    }
    if otherwise.is_empty()
        && let [syn::Stmt::Expr(syn::Expr::If(inner), _)] = then.as_slice()
        && inner.else_branch.is_none()
    {
        let inner_condition = &inner.cond;
        let inner_body = &inner.then_branch;
        prefix.push(syn::parse_quote!(if (#condition) && (#inner_condition) #inner_body));
        return prefix;
    }
    if otherwise.is_empty() && then.is_empty() {
        if prefix.is_empty() && facts.evaluate_condition {
            prefix.push(syn::parse_quote!(let _ = #condition;));
        }
    } else if otherwise.is_empty() {
        prefix.push(syn::parse_quote!(if #condition { #(#then)* }));
    } else if then.is_empty() {
        prefix.push(syn::parse_quote!(if !(#condition) { #(#otherwise)* }));
    } else {
        prefix.push(syn::parse_quote!(if #condition { #(#then)* } else { #(#otherwise)* }));
    }
    prefix
}
