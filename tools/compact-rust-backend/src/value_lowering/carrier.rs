// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Native formatting of carrier facts already validated by pure_semantics.

use crate::{
    naming::checked_ident,
    pure_semantics::{
        carrier::{CarrierSink, Leaf},
        numeric::UnsignedMaximum,
    },
};
use proc_macro2::Span;

pub(crate) struct NativeCarrierSink;

impl CarrierSink for NativeCarrierSink {
    type Value = syn::Type;
    type Tuple = syn::TypeTuple;

    fn leaf(&mut self, leaf: Leaf<'_>) -> syn::Type {
        match leaf {
            Leaf::Unit => syn::parse_quote!(()),
            Leaf::Boolean => syn::parse_quote!(bool),
            Leaf::Field => syn::parse_quote!(runtime::Field),
            Leaf::JubjubPoint => syn::parse_quote!(runtime::JubjubPoint),
            Leaf::OpaqueString => syn::parse_quote!(runtime::OpaqueString),
            Leaf::OpaqueBytes => syn::parse_quote!(runtime::OpaqueBytes),
            Leaf::Bytes(length) => {
                let length = syn::LitInt::new(&length.to_string(), Span::call_site());
                syn::parse_quote!(runtime::FixedBytes<#length>)
            }
            Leaf::Named(name) => {
                let name = checked_ident(&name);
                syn::parse_quote!(crate::types::#name)
            }
            Leaf::Unsigned { spelling, maximum } => match maximum {
                UnsignedMaximum::Small(_) => {
                    let max = syn::LitInt::new(spelling, Span::call_site());
                    syn::parse_quote!(runtime::BoundedUint<#max>)
                }
                UnsignedMaximum::Wide { high, low } => {
                    let high = syn::LitInt::new(&format!("{high}u128"), Span::call_site());
                    let low = syn::LitInt::new(&format!("{low}u128"), Span::call_site());
                    syn::parse_quote!(runtime::WideUint<#high, #low>)
                }
            },
        }
    }

    fn tuple(&mut self) -> syn::TypeTuple {
        syn::TypeTuple {
            paren_token: syn::token::Paren::default(),
            elems: syn::punctuated::Punctuated::new(),
        }
    }

    fn element(&mut self, tuple: &mut syn::TypeTuple, element: syn::Type) {
        tuple.elems.push_value(element);
        tuple.elems.push_punct(syn::token::Comma::default());
    }

    fn finish_tuple(&mut self, mut tuple: syn::TypeTuple) -> syn::Type {
        if tuple.elems.len() > 1 {
            tuple.elems.pop_punct();
        }
        syn::Type::Tuple(tuple)
    }

    fn vector(&mut self, element: syn::Type, length: usize) -> syn::Type {
        let length = syn::LitInt::new(&length.to_string(), Span::call_site());
        syn::parse_quote!(runtime::FixedVector<#element, #length>)
    }
}
