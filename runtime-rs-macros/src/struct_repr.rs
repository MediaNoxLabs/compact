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

//! Ordered struct composition of the upstream field and binary representation traits.

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Data, DeriveInput, Fields};

pub(crate) fn expand(input: DeriveInput) -> syn::Result<TokenStream> {
    if !input.generics.params.is_empty() || input.generics.where_clause.is_some() {
        return Err(syn::Error::new_spanned(
            &input.generics,
            "CompactStructRepr requires a concrete struct without generic bounds",
        ));
    }
    let Data::Struct(data) = &input.data else {
        return Err(syn::Error::new_spanned(
            &input,
            "CompactStructRepr requires a named struct",
        ));
    };
    let Fields::Named(fields) = &data.fields else {
        return Err(syn::Error::new_spanned(
            &input,
            "CompactStructRepr requires named fields",
        ));
    };
    let name = &input.ident;
    let names: Vec<_> = fields
        .named
        .iter()
        .map(|field| field.ident.as_ref().unwrap())
        .collect();
    let types: Vec<_> = fields.named.iter().map(|field| &field.ty).collect();
    // Field names never become local bindings: they may legally equal our temporaries.
    let locals: Vec<_> = (0..names.len())
        .map(|i| format_ident!("__compact_field_{i}"))
        .collect();
    let decode = if names.is_empty() {
        quote! {
            if __compact_repr.is_empty() {
                ::core::option::Option::Some(Self {})
            } else {
                ::core::option::Option::None
            }
        }
    } else {
        quote! {
            let mut __compact_input = __compact_repr;
            #(
                let __compact_size = <#types as ::midnight_compact_runtime::FromFieldRepr>::FIELD_SIZE;
                if __compact_size > __compact_input.len() {
                    return ::core::option::Option::None;
                }
                let #locals = <#types as ::midnight_compact_runtime::FromFieldRepr>::from_field_repr(
                    &__compact_input[..__compact_size]
                )?;
                __compact_input = &__compact_input[__compact_size..];
            )*
            if __compact_input.is_empty() {
                ::core::option::Option::Some(Self { #(#names: #locals),* })
            } else {
                ::core::option::Option::None
            }
        }
    };
    Ok(quote! {
        impl ::midnight_compact_runtime::FieldRepr for #name {
            fn field_repr<__CompactWriter: ::midnight_compact_runtime::MemWrite<::midnight_compact_runtime::Fr>>(
                &self, __compact_writer: &mut __CompactWriter,
            ) {
                #(::midnight_compact_runtime::FieldRepr::field_repr(&self.#names, __compact_writer);)*
            }
            fn field_size(&self) -> ::core::primitive::usize {
                0 #(+ ::midnight_compact_runtime::FieldRepr::field_size(&self.#names))*
            }
        }
        impl ::midnight_compact_runtime::BinaryHashRepr for #name {
            fn binary_repr<__CompactWriter: ::midnight_compact_runtime::MemWrite<::core::primitive::u8>>(
                &self, __compact_writer: &mut __CompactWriter,
            ) {
                #(::midnight_compact_runtime::BinaryHashRepr::binary_repr(&self.#names, __compact_writer);)*
            }
            fn binary_len(&self) -> ::core::primitive::usize {
                0 #(+ ::midnight_compact_runtime::BinaryHashRepr::binary_len(&self.#names))*
            }
        }
        impl ::midnight_compact_runtime::FromFieldRepr for #name {
            const FIELD_SIZE: ::core::primitive::usize = 0 #(+ <#types as ::midnight_compact_runtime::FromFieldRepr>::FIELD_SIZE)*;
            fn from_field_repr(__compact_repr: &[::midnight_compact_runtime::Fr]) -> ::core::option::Option<Self> {
                #decode
            }
        }
    })
}

#[cfg(test)]
#[path = "tests/struct_repr.rs"]
mod tests;
