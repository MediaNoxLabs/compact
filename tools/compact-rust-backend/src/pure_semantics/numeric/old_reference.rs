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

//! Exact numeric policy and syntax reference from signed 807bed34. Test only.

use crate::RenderError;
use crate::ir::Expr;
use crate::value_lowering::wide_uint_type;
use proc_macro2::Span;

pub(crate) fn field_literal_bytes(value: &str) -> Result<[u8; 32], RenderError> {
    let invalid = || RenderError::InvalidFieldLiteral(value.to_owned());
    if value.is_empty()
        || (value.len() > 1 && value.starts_with('0'))
        || !value.bytes().all(|byte| byte.is_ascii_digit())
    {
        return Err(invalid());
    }
    let mut bytes = [0_u8; 32];
    for digit in value.bytes() {
        let mut carry = u16::from(digit - b'0');
        for byte in &mut bytes {
            let expanded = u16::from(*byte) * 10 + carry;
            *byte = expanded as u8;
            carry = expanded >> 8;
        }
        if carry != 0 {
            return Err(invalid());
        }
    }
    if midnight_transient_crypto::curve::Fr::from_le_bytes(&bytes).is_none() {
        return Err(invalid());
    }
    Ok(bytes)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum UnsignedMaximum {
    Small(u128),
    Wide { high: u128, low: u128 },
}

pub(crate) fn unsigned_maximum(value: &str) -> Result<UnsignedMaximum, RenderError> {
    let bytes = field_literal_bytes(value)
        .map_err(|_| RenderError::InvalidUnsignedMaximum(value.to_owned()))?;
    if bytes[31] != 0 {
        return Err(RenderError::InvalidUnsignedMaximum(value.to_owned()));
    }
    let low = u128::from_le_bytes(bytes[..16].try_into().expect("low limb"));
    let high = u128::from_le_bytes(bytes[16..].try_into().expect("high limb"));
    if high == 0 {
        Ok(UnsignedMaximum::Small(low))
    } else {
        Ok(UnsignedMaximum::Wide { high, low })
    }
}

pub(crate) fn unsigned_cast_syntax(
    value: syn::Expr,
    source_max: &str,
    target_max: &str,
) -> Result<syn::Expr, RenderError> {
    let source = unsigned_maximum(source_max)?;
    let target = unsigned_maximum(target_max)?;
    Ok(match (source, target) {
        (UnsignedMaximum::Small(source), UnsignedMaximum::Small(target)) => {
            let source = syn::LitInt::new(&source.to_string(), Span::call_site());
            let target = syn::LitInt::new(&target.to_string(), Span::call_site());
            syn::parse_quote!(runtime::cast_unsigned::<#source, #target>(#value)?)
        }
        (UnsignedMaximum::Small(_), UnsignedMaximum::Wide { high, low }) => {
            let target = wide_uint_type(high, low);
            syn::parse_quote!(<#target>::from_le_bytes(&(#value).value().to_le_bytes())?)
        }
        (UnsignedMaximum::Wide { high, low }, UnsignedMaximum::Small(target)) => {
            let target = syn::LitInt::new(&target.to_string(), Span::call_site());
            let high = syn::LitInt::new(&high.to_string(), Span::call_site());
            let low = syn::LitInt::new(&low.to_string(), Span::call_site());
            syn::parse_quote!(runtime::narrow_wide_uint::<#target, #high, #low>(#value)?)
        }
        (UnsignedMaximum::Wide { .. }, UnsignedMaximum::Wide { high, low }) => {
            let target = wide_uint_type(high, low);
            syn::parse_quote!(<#target>::from_le_bytes((#value).as_le_bytes())?)
        }
    })
}

/// Both expression emitters share admission and exact runtime arithmetic selection.
pub(crate) fn unsigned_arithmetic_syntax(
    operation: &Expr,
    left: syn::Expr,
    right: syn::Expr,
    left_max: &str,
    right_max: &str,
    result_max: &str,
) -> Result<syn::Expr, RenderError> {
    let left_bound = unsigned_maximum(left_max)?;
    let right_bound = unsigned_maximum(right_max)?;
    let result_bound = unsigned_maximum(result_max)?;
    if matches!(operation, Expr::UnsignedAdd { .. })
        && let UnsignedMaximum::Wide { high, low } = result_bound
    {
        let high = syn::LitInt::new(&format!("{high}u128"), Span::call_site());
        let low = syn::LitInt::new(&format!("{low}u128"), Span::call_site());
        return Ok(syn::parse_quote!(runtime::add_wide_unsigned::<#high,#low,_,_>(#left,#right)?));
    }
    for (bound, text) in [
        (left_bound, left_max),
        (right_bound, right_max),
        (result_bound, result_max),
    ] {
        if matches!(bound, UnsignedMaximum::Wide { .. }) {
            let operation = match operation {
                Expr::UnsignedAdd { .. } => "addition",
                Expr::UnsignedSubtract { .. } => "subtraction",
                Expr::UnsignedMultiply { .. } => "multiplication",
                _ => unreachable!("unsigned arithmetic caller"),
            };
            return Err(RenderError::UnsupportedWideUnsignedOperation {
                operation,
                max: text.to_owned(),
            });
        }
    }
    let left_max = syn::LitInt::new(left_max, Span::call_site());
    let right_max = syn::LitInt::new(right_max, Span::call_site());
    let result_max = syn::LitInt::new(result_max, Span::call_site());
    let helper: syn::Path = match operation {
        Expr::UnsignedAdd { .. } => syn::parse_quote!(runtime::add_unsigned),
        Expr::UnsignedSubtract { .. } => syn::parse_quote!(runtime::subtract_unsigned),
        Expr::UnsignedMultiply { .. } => syn::parse_quote!(runtime::multiply_unsigned),
        _ => unreachable!("unsigned arithmetic caller"),
    };
    Ok(syn::parse_quote!(#helper::<#left_max,#right_max,#result_max>(#left,#right)?))
}
