// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Distinct expression and Unit-statement forms from one checked child set.
//! Statement projection is an output view, never another source/type traversal.

pub(crate) enum NativeFragment {
    Value(syn::Expr),
    Unit {
        expression: syn::Expr,
        statements: Vec<syn::Stmt>,
    },
}

impl NativeFragment {
    pub(crate) fn unit(expression: syn::Expr, statements: Vec<syn::Stmt>) -> Self {
        Self::Unit {
            expression,
            statements,
        }
    }

    pub(crate) fn into_expression(self) -> syn::Expr {
        match self {
            Self::Value(expression) | Self::Unit { expression, .. } => expression,
        }
    }

    pub(crate) fn into_views(self) -> (syn::Expr, Vec<syn::Stmt>) {
        match self {
            Self::Value(expression) => (expression, Vec::new()),
            Self::Unit {
                expression,
                statements,
            } => (expression, statements),
        }
    }
}

pub(crate) fn sequence(steps: Vec<syn::Stmt>, tail: NativeFragment) -> NativeFragment {
    match tail {
        NativeFragment::Value(value) => {
            NativeFragment::Value(syn::parse_quote!({ #(#steps)* #value }))
        }
        NativeFragment::Unit {
            expression: value,
            statements: tail,
        } => {
            let expression = syn::parse_quote!({ #(#steps)* #value });
            let mut statements = steps;
            statements.extend(tail);
            NativeFragment::unit(expression, statements)
        }
    }
}

pub(crate) fn lexical(bindings: Vec<syn::Stmt>, body: NativeFragment) -> NativeFragment {
    match body {
        NativeFragment::Value(body) => {
            NativeFragment::Value(syn::parse_quote!({ #(#bindings)* #body }))
        }
        NativeFragment::Unit {
            expression: body,
            statements: body_statements,
        } => {
            let expression = syn::parse_quote!({ #(#bindings)* #body });
            let statements = if bindings.is_empty() {
                body_statements
            } else {
                let mut statements = bindings;
                statements.extend(body_statements);
                vec![syn::parse_quote!({ #(#statements)* })]
            };
            NativeFragment::unit(expression, statements)
        }
    }
}
