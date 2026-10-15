// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Let-only symbol correction; corrected API cases never invoke panic oracle.
use super::*;
mod corrected;
mod errors;
mod ordinary;
fn field(value: &str) -> Expr {
    Expr::FieldLiteral {
        value: value.into(),
    }
}
fn binding(name: &str, ty: Type, value: Expr) -> LocalBinding {
    LocalBinding {
        name: name.into(),
        ty,
        value,
    }
}
fn lexical(bindings: Vec<LocalBinding>, body: Expr) -> Expr {
    Expr::Let {
        bindings,
        body: Box::new(body),
    }
}
fn symbol(local: &syn::Local) -> String {
    let syn::Pat::Type(typed) = &local.pat else {
        panic!("typed local");
    };
    let syn::Pat::Ident(name) = typed.pat.as_ref() else {
        panic!("identifier local");
    };
    name.ident.to_string()
}
fn local_symbols(source: &Expr, parameters: &HashMap<&str, (&Type, syn::Ident)>) -> Vec<String> {
    let (syntax, _) = crate::expression_with_calls(source, parameters, &HashMap::new()).unwrap();
    let syn::Expr::Block(block) = syntax else {
        panic!("lexical block");
    };
    block
        .block
        .stmts
        .iter()
        .filter_map(|statement| match statement {
            syn::Stmt::Local(local) => Some(symbol(local)),
            _ => None,
        })
        .collect()
}
