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

//! Native formatting of an already checked conversion plan.
//!
//! No Type pairs, numeric parsing or support decisions enter this interpreter.
//! The runtime expression is bound exactly once at aggregate boundaries.

use super::wide_uint_type;
use crate::pure_semantics::{
    coercion::{AggregateCarrier, CoercionKind, CoercionPlan},
    numeric::UnsignedCastStrategy,
};
use proc_macro2::Span;

pub(super) fn render_unsigned_cast(value: syn::Expr, strategy: UnsignedCastStrategy) -> syn::Expr {
    match strategy {
        UnsignedCastStrategy::SmallToSmall { source, target } => {
            let source = syn::LitInt::new(&source.to_string(), Span::call_site());
            let target = syn::LitInt::new(&target.to_string(), Span::call_site());
            syn::parse_quote!(runtime::cast_unsigned::<#source, #target>(#value)?)
        }
        UnsignedCastStrategy::SmallToWide { high, low } => {
            let target = wide_uint_type(high, low);
            syn::parse_quote!(<#target>::from_le_bytes(&(#value).value().to_le_bytes())?)
        }
        UnsignedCastStrategy::WideToSmall { high, low, target } => {
            let target = syn::LitInt::new(&target.to_string(), Span::call_site());
            let high = syn::LitInt::new(&high.to_string(), Span::call_site());
            let low = syn::LitInt::new(&low.to_string(), Span::call_site());
            syn::parse_quote!(runtime::narrow_wide_uint::<#target, #high, #low>(#value)?)
        }
        UnsignedCastStrategy::WideToWide { high, low } => {
            let target = wide_uint_type(high, low);
            syn::parse_quote!(<#target>::from_le_bytes((#value).as_le_bytes())?)
        }
    }
}

pub(super) fn render(plan: &CoercionPlan, value: syn::Expr, depth: usize) -> syn::Expr {
    match plan.kind() {
        CoercionKind::Identity => value,
        CoercionKind::UnsignedToField { wide: false } => {
            syn::parse_quote!(runtime::Field::from((#value).value()))
        }
        CoercionKind::UnsignedToField { wide: true } => syn::parse_quote!((#value).as_field()),
        CoercionKind::UnsignedWiden { cast } => render_unsigned_cast(value, *cast),
        CoercionKind::Vector { length, element } => {
            let source =
                syn::Ident::new(&format!("__compact_cast_source_{depth}"), Span::call_site());
            let item = syn::Ident::new(&format!("__compact_cast_item_{depth}"), Span::call_site());
            let mapped = render(element, syn::parse_quote!(#item), depth + 1);
            if element.may_fail() {
                let converted =
                    syn::Ident::new(&format!("__compact_cast_values_{depth}"), Span::call_site());
                let length = syn::LitInt::new(&length.to_string(), Span::call_site());
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
            }
        }
        CoercionKind::Aggregate {
            source: source_carrier,
            target,
            elements,
        } => {
            let source =
                syn::Ident::new(&format!("__compact_cast_source_{depth}"), Span::call_site());
            let items = (0..elements.len())
                .map(|index| {
                    syn::Ident::new(
                        &format!("__compact_cast_item_{depth}_{index}"),
                        Span::call_site(),
                    )
                })
                .collect::<Vec<_>>();
            let mapped = items
                .iter()
                .zip(elements)
                .map(|(item, plan)| render(plan, syn::parse_quote!(#item), depth + 1))
                .collect::<Vec<_>>();
            let destructure: syn::Stmt = match source_carrier {
                AggregateCarrier::Tuple => syn::parse_quote!(let (#(#items),*,) = #source;),
                AggregateCarrier::Vector => {
                    syn::parse_quote!(let [#(#items),*] = #source.into_array();)
                }
            };
            let result: syn::Expr = match target {
                AggregateCarrier::Tuple => syn::parse_quote!((#(#mapped),*,)),
                AggregateCarrier::Vector => {
                    syn::parse_quote!(runtime::FixedVector::new([#(#mapped),*]))
                }
            };
            syn::parse_quote!({
                let #source = #value;
                #destructure
                #result
            })
        }
    }
}
