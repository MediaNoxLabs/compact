// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use crate::{
    naming::checked_ident,
    pure_semantics::{identifier::CheckedName, ownership::Ownership},
    value_lowering::retained_value_with_ownership,
};

pub(super) fn field(value: syn::Expr, field: CheckedName, ownership: Ownership) -> syn::Expr {
    let name = checked_ident(&field);
    retained_value_with_ownership(syn::parse_quote!((#value).#name), ownership)
}
pub(super) fn tuple(value: syn::Expr, index: usize) -> syn::Expr {
    let index = syn::Index::from(index);
    syn::parse_quote!((#value).#index)
}
