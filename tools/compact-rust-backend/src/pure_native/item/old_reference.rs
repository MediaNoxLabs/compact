// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Original pure-item assembly region, independently frozen before #630.
use crate::{ir::Type, pure_native::fragment::NativeFragment};
struct Circuit {
    internal: bool,
}
pub(super) fn pure(
    name: syn::Ident,
    internal: bool,
    args: Vec<syn::FnArg>,
    result: syn::Type,
    body: NativeFragment,
    actual: Type,
) -> syn::Item {
    let circuit = Circuit { internal };
    let (body, unit_body) = body.into_views();
    let value_tail: syn::Expr = if let syn::Expr::Try(fallible) = &body {
        (*fallible.expr).clone()
    } else {
        syn::parse_quote!(Ok(#body))
    };
    let item: syn::Item = if circuit.internal && actual == Type::Unit {
        syn::parse_quote! {
            pub(crate) fn #name(#(#args),*) -> Result<#result, runtime::CompactError> {
                #(#unit_body)*
                Ok(())
            }
        }
    } else if actual == Type::Unit {
        syn::parse_quote! {
            pub fn #name(#(#args),*) -> Result<#result, runtime::CompactError> {
                #(#unit_body)*
                Ok(())
            }
        }
    } else if circuit.internal {
        syn::parse_quote! {
            pub(crate) fn #name(#(#args),*) -> Result<#result, runtime::CompactError> {
                #value_tail
            }
        }
    } else {
        syn::parse_quote! {
            pub fn #name(#(#args),*) -> Result<#result, runtime::CompactError> {
                #value_tail
            }
        }
    };
    item
}
