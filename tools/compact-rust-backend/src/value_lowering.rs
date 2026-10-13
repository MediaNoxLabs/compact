// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//  	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Shared typed-value lowering for pure, native and recorded emitters.
//!
//! Maps Compact types and bounds to Rust carriers and checked conversion syntax.
//! Runtime operations remain authoritative; this module neither executes ledger
//! queries nor decides whole-circuit purity or recording admission.

use crate::RenderError;
use crate::ir::{Expr, Type};
use proc_macro2::Span;

use crate::pure_semantics::numeric::{
    ArithmeticKind, UnsignedArithmeticStrategy, checked_unsigned_arithmetic, checked_unsigned_cast,
};
pub(crate) use crate::pure_semantics::numeric::{
    UnsignedMaximum, field_literal_bytes, unsigned_maximum,
};

pub(crate) fn wide_uint_type(high: u128, low: u128) -> syn::Type {
    let high = syn::LitInt::new(&format!("{high}u128"), Span::call_site());
    let low = syn::LitInt::new(&format!("{low}u128"), Span::call_site());
    syn::parse_quote!(runtime::WideUint<#high, #low>)
}

pub(crate) fn field_to_bytes_32_syntax(value: syn::Expr) -> syn::Expr {
    // Compact's Field-to-Bytes cast is zero-padded little endian. The pinned
    // midnight-zk Field has a canonical 32-byte representation.
    syn::parse_quote!(runtime::FixedBytes::<32>::new(
        (#value).as_le_bytes().try_into().expect("canonical Field is 32 bytes")
    ))
}

pub(crate) fn unsigned_cast_syntax(
    value: syn::Expr,
    source_max: &str,
    target_max: &str,
) -> Result<syn::Expr, RenderError> {
    Ok(coercion::render_unsigned_cast(
        value,
        checked_unsigned_cast(source_max, target_max)?,
    ))
}

/// Both expression emitters consume the same checked runtime arithmetic strategy.
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

pub(crate) fn rust_type(ty: &Type) -> Result<syn::Type, RenderError> {
    crate::pure_semantics::carrier::fold_carrier(ty, &mut carrier::NativeCarrierSink)
}

fn map_slot_value_type(value: &Type) -> Result<Option<syn::Type>, RenderError> {
    if let Type::LedgerMap { key, value } = value {
        let Some((key, value)) = map_slot_types(key, value)? else {
            return Ok(None);
        };
        return Ok(Some(
            syn::parse_quote!(runtime::slots::MapNode<#key, #value>),
        ));
    }
    match rust_type(value) {
        Ok(ty) => Ok(Some(ty)),
        Err(RenderError::UnsupportedLedgerValueType(_)) => Ok(None),
        Err(error) => Err(error),
    }
}

pub(crate) fn map_slot_types(
    key: &Type,
    value: &Type,
) -> Result<Option<(syn::Type, syn::Type)>, RenderError> {
    let key = match rust_type(key) {
        Ok(ty) => ty,
        Err(RenderError::UnsupportedLedgerValueType(_)) => return Ok(None),
        Err(error) => return Err(error),
    };
    let value = match map_slot_value_type(value)? {
        Some(ty) => ty,
        None => return Ok(None),
    };
    Ok(Some((key, value)))
}

pub(crate) fn coerce_expression(
    value: syn::Expr,
    actual: &Type,
    target: &Type,
    depth: usize,
) -> Result<syn::Expr, RenderError> {
    let plan = crate::pure_semantics::coercion::plan_coercion(actual, target)?;
    Ok(coercion::render(&plan, value, depth))
}

// Existing output contracts inspect conversion fallibility independently of
// source-expression evaluation. Production support belongs to the neutral plan.
#[cfg(test)]
struct Coercion {
    expression: syn::Expr,
    may_fail: bool,
}

#[cfg(test)]
fn coercion(
    value: syn::Expr,
    actual: &Type,
    target: &Type,
    depth: usize,
) -> Result<Coercion, RenderError> {
    let plan = crate::pure_semantics::coercion::plan_coercion(actual, target)?;
    Ok(Coercion {
        expression: coercion::render(&plan, value, depth),
        may_fail: plan.may_fail(),
    })
}

pub(crate) fn copy_type(ty: &Type) -> bool {
    match ty {
        Type::Struct { .. }
        | Type::Vector { .. }
        | Type::LedgerMap { .. }
        | Type::OpaqueString
        | Type::OpaqueBytes => false,
        Type::Tuple { elements } => elements.iter().all(copy_type),
        Type::Unit
        | Type::Boolean
        | Type::Field
        | Type::JubjubPoint
        | Type::Bytes { .. }
        | Type::Enum { .. }
        | Type::Unsigned { .. } => true,
    }
}

/// Keep an owned value available at an emission boundary without cloning Copy types.
pub(crate) fn retained_value(value: syn::Expr, ty: &Type) -> syn::Expr {
    if copy_type(ty) {
        value
    } else {
        syn::parse_quote!((#value).clone())
    }
}

#[cfg(test)]
mod tests;

pub(crate) mod carrier;

mod coercion;
