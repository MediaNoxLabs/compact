// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Presentation-only shorthand and exact tuple punctuation from prepared values.
use crate::{naming::checked_ident, pure_semantics::identifier::CheckedName};

pub(super) fn structure(name: CheckedName, fields: Vec<(CheckedName, syn::Expr)>) -> syn::Expr {
    let fields: Vec<syn::FieldValue> = fields
        .into_iter()
        .map(|(field, rendered)| {
            let field_name = checked_ident(&field);
            let shorthand = matches!(
                &rendered,
                syn::Expr::Path(path)
                    if path.qself.is_none()
                        && path.path.leading_colon.is_none()
                        && path.path.segments.len() == 1
                        && path.path.segments[0].ident == field_name
                        && matches!(path.path.segments[0].arguments, syn::PathArguments::None)
            );
            if shorthand {
                syn::parse_quote!(#field_name)
            } else {
                syn::parse_quote!(#field_name: #rendered)
            }
        })
        .collect();
    let name = checked_ident(&name);
    syn::parse_quote!(crate::types::#name { #(#fields),* })
}
pub(super) fn tuple(values: Vec<syn::Expr>) -> syn::Expr {
    let mut tuple = syn::ExprTuple {
        attrs: vec![],
        paren_token: syn::token::Paren::default(),
        elems: syn::punctuated::Punctuated::new(),
    };
    for value in values {
        tuple.elems.push_value(value);
        tuple.elems.push_punct(syn::token::Comma::default());
    }
    if tuple.elems.len() > 1 {
        tuple.elems.pop_punct();
    }
    syn::Expr::Tuple(tuple)
}
