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
use crate::naming::ident;
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

pub(crate) fn rust_type(ty: &Type) -> Result<syn::Type, RenderError> {
    Ok(match ty {
        Type::Unit => syn::parse_quote!(()),
        Type::Boolean => syn::parse_quote!(bool),
        Type::Field => syn::parse_quote!(runtime::Field),
        Type::JubjubPoint => syn::parse_quote!(runtime::JubjubPoint),
        Type::OpaqueString => syn::parse_quote!(runtime::OpaqueString),
        Type::OpaqueBytes => syn::parse_quote!(runtime::OpaqueBytes),
        Type::Bytes { length } => {
            let length = syn::LitInt::new(&length.to_string(), Span::call_site());
            syn::parse_quote!(runtime::FixedBytes<#length>)
        }
        Type::Struct { name, .. } | Type::Enum { name, .. } => {
            let name = ident(name)?;
            syn::parse_quote!(crate::types::#name)
        }
        Type::Unsigned { max } => match unsigned_maximum(max)? {
            UnsignedMaximum::Small(_) => {
                let max = syn::LitInt::new(max, Span::call_site());
                syn::parse_quote!(runtime::BoundedUint<#max>)
            }
            UnsignedMaximum::Wide { high, low } => wide_uint_type(high, low),
        },
        Type::Tuple { elements } => {
            let elements: Vec<syn::Type> =
                elements.iter().map(rust_type).collect::<Result<_, _>>()?;
            let mut tuple = syn::TypeTuple {
                paren_token: syn::token::Paren::default(),
                elems: syn::punctuated::Punctuated::new(),
            };
            for element in elements {
                tuple.elems.push_value(element);
                tuple.elems.push_punct(syn::token::Comma::default());
            }
            if tuple.elems.len() > 1 {
                tuple.elems.pop_punct();
            }
            syn::Type::Tuple(tuple)
        }
        Type::Vector { element, length } => {
            let element = rust_type(element)?;
            let length = syn::LitInt::new(&length.to_string(), Span::call_site());
            syn::parse_quote!(runtime::FixedVector<#element, #length>)
        }
        Type::LedgerMap { .. } => return Err(RenderError::UnsupportedLedgerValueType(ty.clone())),
    })
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

/// Whether conversion syntax needs an enclosing Result owner, including checked
/// operations whose valid input bounds guarantee success. The source expression
/// is evaluated outside element closures and is not included in this property.
struct Coercion {
    expression: syn::Expr,
    may_fail: bool,
}

impl Coercion {
    fn infallible(expression: syn::Expr) -> Self {
        Self {
            expression,
            may_fail: false,
        }
    }
}

struct AggregateCoercion {
    items: Vec<syn::Ident>,
    expressions: Vec<syn::Expr>,
    may_fail: bool,
}

fn coerce_aggregate_fields(
    sources: &[Type],
    targets: &[Type],
    depth: usize,
) -> Result<AggregateCoercion, RenderError> {
    debug_assert_eq!(sources.len(), targets.len());
    let items = (0..sources.len())
        .map(|index| {
            syn::Ident::new(
                &format!("__compact_cast_item_{depth}_{index}"),
                Span::call_site(),
            )
        })
        .collect::<Vec<_>>();
    let mapped = items
        .iter()
        .zip(sources.iter().zip(targets))
        .map(|(item, (source_ty, target_ty))| {
            coercion(syn::parse_quote!(#item), source_ty, target_ty, depth + 1)
        })
        .collect::<Result<Vec<_>, _>>()?;
    let may_fail = mapped.iter().any(|child| child.may_fail);
    Ok(AggregateCoercion {
        items,
        expressions: mapped.into_iter().map(|child| child.expression).collect(),
        may_fail,
    })
}

pub(crate) fn coerce_expression(
    value: syn::Expr,
    actual: &Type,
    target: &Type,
    depth: usize,
) -> Result<syn::Expr, RenderError> {
    Ok(coercion(value, actual, target, depth)?.expression)
}

fn coercion(
    value: syn::Expr,
    actual: &Type,
    target: &Type,
    depth: usize,
) -> Result<Coercion, RenderError> {
    if actual == target {
        return Ok(Coercion::infallible(value));
    }
    match (actual, target) {
        (Type::Unsigned { max }, Type::Field) => match unsigned_maximum(max)? {
            UnsignedMaximum::Small(_) => Ok(Coercion::infallible(
                syn::parse_quote!(runtime::Field::from((#value).value())),
            )),
            UnsignedMaximum::Wide { .. } => {
                Ok(Coercion::infallible(syn::parse_quote!((#value).as_field())))
            }
        },
        (Type::Unsigned { max: source_max }, Type::Unsigned { max: target_max }) => {
            let bounds = |value: UnsignedMaximum| match value {
                UnsignedMaximum::Small(low) => (0, low),
                UnsignedMaximum::Wide { high, low } => (high, low),
            };
            if bounds(unsigned_maximum(source_max)?) > bounds(unsigned_maximum(target_max)?) {
                return Err(RenderError::TypeMismatch {
                    expected: target.clone(),
                    actual: actual.clone(),
                });
            }
            Ok(Coercion {
                expression: unsigned_cast_syntax(value, source_max, target_max)?,
                may_fail: true,
            })
        }
        (
            Type::Vector {
                element: source_element,
                length: source_length,
            },
            Type::Vector {
                element: target_element,
                length: target_length,
            },
        ) if source_length == target_length => {
            let source =
                syn::Ident::new(&format!("__compact_cast_source_{depth}"), Span::call_site());
            let item = syn::Ident::new(&format!("__compact_cast_item_{depth}"), Span::call_site());
            let Coercion {
                expression: mapped,
                may_fail,
            } = coercion(
                syn::parse_quote!(#item),
                source_element,
                target_element,
                depth + 1,
            )?;
            let expression = if may_fail {
                // A Result-returning conversion must run in the enclosing owner,
                // not an infallible array.map closure. Keep one loop per vector
                // level: unrolling would multiply output by nested dimensions.
                let converted =
                    syn::Ident::new(&format!("__compact_cast_values_{depth}"), Span::call_site());
                let length = syn::LitInt::new(&source_length.to_string(), Span::call_site());
                syn::parse_quote!({
                    let #source = #value;
                    let mut #converted = ::std::vec::Vec::with_capacity(#length);
                    for #item in #source.into_array() {
                        #converted.push(#mapped);
                    }
                    runtime::FixedVector::new(<[_; #length]>::try_from(#converted)
                        .expect("Vector coercion preserves its length"))
                })
            } else {
                syn::parse_quote!({
                    let #source = #value;
                    runtime::FixedVector::new(#source.into_array().map(|#item| #mapped))
                })
            };
            Ok(Coercion {
                expression,
                may_fail,
            })
        }
        (Type::Tuple { elements: sources }, Type::Tuple { elements: targets })
            if sources.len() == targets.len() && !sources.is_empty() =>
        {
            let source =
                syn::Ident::new(&format!("__compact_cast_source_{depth}"), Span::call_site());
            let AggregateCoercion {
                items,
                expressions: mapped,
                may_fail,
            } = coerce_aggregate_fields(sources, targets, depth)?;
            Ok(Coercion {
                may_fail,
                expression: syn::parse_quote!({
                    let #source = #value;
                    let (#(#items),*,) = #source;
                    (#(#mapped),*,)
                }),
            })
        }
        (
            Type::Tuple { elements: sources },
            Type::Vector {
                element: target_element,
                length,
            },
        ) if sources.len() == *length && !sources.is_empty() => {
            let source =
                syn::Ident::new(&format!("__compact_cast_source_{depth}"), Span::call_site());
            let targets = vec![*target_element.clone(); sources.len()];
            let AggregateCoercion {
                items,
                expressions: mapped,
                may_fail,
            } = coerce_aggregate_fields(sources, &targets, depth)?;
            Ok(Coercion {
                may_fail,
                expression: syn::parse_quote!({
                    let #source = #value;
                    let (#(#items),*,) = #source;
                    runtime::FixedVector::new([#(#mapped),*])
                }),
            })
        }
        (
            Type::Vector {
                element: source_element,
                length,
            },
            Type::Tuple { elements: targets },
        ) if *length == targets.len() && !targets.is_empty() => {
            let source =
                syn::Ident::new(&format!("__compact_cast_source_{depth}"), Span::call_site());
            let sources = vec![*source_element.clone(); targets.len()];
            let AggregateCoercion {
                items,
                expressions: mapped,
                may_fail,
            } = coerce_aggregate_fields(&sources, targets, depth)?;
            Ok(Coercion {
                may_fail,
                expression: syn::parse_quote!({
                    let #source = #value;
                    let [#(#items),*] = #source.into_array();
                    (#(#mapped),*,)
                }),
            })
        }
        _ => Err(RenderError::TypeMismatch {
            expected: target.clone(),
            actual: actual.clone(),
        }),
    }
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
