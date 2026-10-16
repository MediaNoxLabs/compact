// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Test-only exact complete helper from signed 5c995cff; do not update in place.
use crate::pure_semantics::numeric::{
    ArithmeticKind, UnsignedArithmeticStrategy, checked_unsigned_arithmetic,
};
use crate::{RenderError, ir::Expr};
use proc_macro2::Span;

pub(crate) fn unsigned_arithmetic_syntax(
    operation: &Expr,
    left: syn::Expr,
    right: syn::Expr,
    left_max: &str,
    right_max: &str,
    result_max: &str,
) -> Result<syn::Expr, RenderError> {
    let kind = match operation {
        Expr::UnsignedAdd { .. } => ArithmeticKind::Add,
        Expr::UnsignedSubtract { .. } => ArithmeticKind::Subtract,
        Expr::UnsignedMultiply { .. } => ArithmeticKind::Multiply,
        _ => unreachable!("unsigned arithmetic caller"),
    };
    Ok(
        match checked_unsigned_arithmetic(kind, left_max, right_max, result_max)? {
            UnsignedArithmeticStrategy::WideAdd { high, low } => {
                let high = syn::LitInt::new(&format!("{high}u128"), Span::call_site());
                let low = syn::LitInt::new(&format!("{low}u128"), Span::call_site());
                syn::parse_quote!(runtime::add_wide_unsigned::<#high,#low,_,_>(#left,#right)?)
            }
            UnsignedArithmeticStrategy::Small {
                kind,
                left: left_bound,
                right: right_bound,
                result,
            } => {
                let left_max = syn::LitInt::new(&left_bound.to_string(), Span::call_site());
                let right_max = syn::LitInt::new(&right_bound.to_string(), Span::call_site());
                let result_max = syn::LitInt::new(&result.to_string(), Span::call_site());
                let helper: syn::Path = match kind {
                    ArithmeticKind::Add => syn::parse_quote!(runtime::add_unsigned),
                    ArithmeticKind::Subtract => syn::parse_quote!(runtime::subtract_unsigned),
                    ArithmeticKind::Multiply => syn::parse_quote!(runtime::multiply_unsigned),
                };
                syn::parse_quote!(#helper::<#left_max,#right_max,#result_max>(#left,#right)?)
            }
        },
    )
}
