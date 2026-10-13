// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Exact original complete helper from signed e2bf38a5, unchanged test authority.
use crate::{ir::Type, value_lowering::copy_type};

pub(crate) fn retained_value(value: syn::Expr, ty: &Type) -> syn::Expr {
    if copy_type(ty) {
        value
    } else {
        syn::parse_quote!((#value).clone())
    }
}
