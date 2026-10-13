// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Frozen signed807bed34 carrier policy. Test only; no production fallback.

use crate::{RenderError, ir::Type};
use proc_macro2::Span;

#[derive(Clone, Copy)]
#[allow(dead_code)] // Preserve the old enum; this oracle uses only its carrier branch.
pub(crate) enum UnsignedMaximum {
    Small(u128),
    Wide { high: u128, low: u128 },
}

pub(crate) fn ident(name: &str) -> Result<syn::Ident, RenderError> {
    let rust_name = name.replace('$', "_");
    syn::parse_str::<syn::Ident>(&rust_name)
        .or_else(|_| syn::parse_str::<syn::Ident>(&format!("r#{rust_name}")))
        .map_err(|_| RenderError::InvalidIdentifier(name.to_owned()))
}

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
