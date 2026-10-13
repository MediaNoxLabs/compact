// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Select exact opaque hash encoding without reconstructing source aggregates.
use crate::ir::Type;

fn contains_opaque(ty: &Type) -> bool {
    match ty {
        Type::OpaqueString | Type::OpaqueBytes => true,
        Type::Struct { fields, .. } => fields.iter().any(|field| contains_opaque(&field.ty)),
        Type::Tuple { elements } => elements.iter().any(contains_opaque),
        Type::Vector { element, .. } => contains_opaque(element),
        _ => false,
    }
}

/// Receives an already-lowered operand once. Only qualified dynamic hash call
/// sites use this helper; ordinary Value conversion and fixed inputs stay intact.
pub(crate) fn syntax(value: syn::Expr, ty: &Type) -> syn::Expr {
    if contains_opaque(ty) {
        syn::parse_quote!(runtime::exact_hash_input(#value))
    } else {
        value
    }
}

/// Private transcript encoding has its own boundary; preserving an opaque
/// preimage here does not relax its later preparation/serialization policy.
pub(crate) fn private_output(value: syn::Expr, ty: &Type) -> syn::Expr {
    if contains_opaque(ty) {
        syn::parse_quote!(runtime::exact_private_output(#value))
    } else {
        syn::parse_quote!(runtime::fab::AlignedValue::from(#value))
    }
}

pub(crate) fn witness_method(ty: &Type) -> syn::Ident {
    if contains_opaque(ty) {
        syn::parse_quote!(try_witness_metered_exact)
    } else {
        syn::parse_quote!(try_witness_metered)
    }
}
