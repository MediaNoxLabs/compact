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

//! Derive Compact user-type codecs from Rust syntax built by the typed backend.

use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::quote;
use syn::{Data, DeriveInput, Fields, LitInt, parse_macro_input};

mod merkle;
mod witness_bridge;

/// Derive the fallible witness trait and infallible adapter from one signature list.
#[proc_macro_attribute]
pub fn compact_witness_bridge(attr: TokenStream, item: TokenStream) -> TokenStream {
    if !attr.is_empty() {
        return syn::Error::new(
            Span::call_site(),
            "compact_witness_bridge takes no arguments",
        )
        .into_compile_error()
        .into();
    }
    let input = parse_macro_input!(item as syn::ItemTrait);
    witness_bridge::expand(input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

#[proc_macro_derive(CompactMerkleTreeDigest)]
pub fn compact_merkle_tree_digest(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    merkle::digest(input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

#[proc_macro_derive(CompactMerklePathEntry)]
pub fn compact_merkle_path_entry(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    merkle::entry(input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

#[proc_macro_derive(CompactMerklePath)]
pub fn compact_merkle_path(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    merkle::path(input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

#[proc_macro_derive(CompactCellValue)]
pub fn compact_cell_value(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match expand(input) {
        Ok(tokens) => tokens.into(),
        Err(error) => error.to_compile_error().into(),
    }
}

#[proc_macro_derive(CompactEnum)]
pub fn compact_enum(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    expand_enum(input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

fn enum_byte_length(variants: usize) -> LitInt {
    let max_ordinal = variants - 1;
    let length = ((usize::BITS - max_ordinal.leading_zeros()) as usize).div_ceil(8);
    LitInt::new(&length.to_string(), Span::call_site())
}

fn expand_enum(input: DeriveInput) -> syn::Result<TokenStream2> {
    if !input.generics.params.is_empty() {
        return Err(syn::Error::new_spanned(
            input.generics,
            "CompactEnum requires a concrete generated type",
        ));
    }
    let name = input.ident;
    let Data::Enum(data) = input.data else {
        return Err(syn::Error::new_spanned(
            name,
            "CompactEnum requires a unit enum",
        ));
    };
    if data.variants.is_empty() {
        return Err(syn::Error::new_spanned(
            name,
            "CompactEnum requires enum variants",
        ));
    }
    let variants = data
        .variants
        .iter()
        .map(|variant| {
            if !matches!(variant.fields, Fields::Unit) || variant.discriminant.is_some() {
                Err(syn::Error::new_spanned(
                    variant,
                    "CompactEnum supports unit variants without Rust discriminants only",
                ))
            } else {
                Ok(&variant.ident)
            }
        })
        .collect::<syn::Result<Vec<_>>>()?;
    let to_ordinal = variants
        .iter()
        .enumerate()
        .map(|(index, variant)| {
            let index = LitInt::new(&index.to_string(), Span::call_site());
            quote!(Self::#variant => #index,)
        })
        .collect::<Vec<_>>();
    let from_ordinal = variants
        .iter()
        .enumerate()
        .map(|(index, variant)| {
            let index = LitInt::new(&index.to_string(), Span::call_site());
            quote!(#index => Some(Self::#variant),)
        })
        .collect::<Vec<_>>();
    let first = variants[0];
    let byte_length = enum_byte_length(variants.len());
    Ok(quote! {
        impl Default for #name {
            fn default() -> Self { Self::#first }
        }

        impl ::midnight_compact_runtime::FieldRepr for #name {
            fn field_repr<W: ::midnight_compact_runtime::MemWrite<::midnight_compact_runtime::Fr>>(
                &self,
                writer: &mut W,
            ) {
                let ordinal: u128 = match self { #(#to_ordinal)* };
                ::midnight_compact_runtime::FieldRepr::field_repr(&ordinal, writer);
            }
            fn field_size(&self) -> usize { 1 }
        }

        impl ::midnight_compact_runtime::BinaryHashRepr for #name {
            fn binary_repr<W: ::midnight_compact_runtime::MemWrite<u8>>(
                &self,
                writer: &mut W,
            ) {
                let ordinal: u128 = match self { #(#to_ordinal)* };
                writer.write(&ordinal.to_le_bytes()[..#byte_length]);
            }
            fn binary_len(&self) -> usize { #byte_length }
        }

        impl ::midnight_compact_runtime::FromFieldRepr for #name {
            const FIELD_SIZE: usize = 1;
            fn from_field_repr(repr: &[::midnight_compact_runtime::Fr]) -> Option<Self> {
                let ordinal = <u128 as ::midnight_compact_runtime::FromFieldRepr>::from_field_repr(repr)?;
                match ordinal { #(#from_ordinal)* _ => None }
            }
        }
    })
}

fn expand(input: DeriveInput) -> syn::Result<TokenStream2> {
    if !input.generics.params.is_empty() {
        return Err(syn::Error::new_spanned(
            input.generics,
            "CompactCellValue requires a concrete generated type",
        ));
    }
    let name = input.ident;
    match input.data {
        Data::Struct(data) => {
            let Fields::Named(fields) = data.fields else {
                return Err(syn::Error::new_spanned(
                    name,
                    "CompactCellValue requires named struct fields",
                ));
            };
            let names = fields
                .named
                .iter()
                .map(|field| field.ident.as_ref().unwrap())
                .collect::<Vec<_>>();
            let types = fields
                .named
                .iter()
                .map(|field| &field.ty)
                .collect::<Vec<_>>();
            let decode = names.iter().zip(&types).map(|(field, ty)| quote! {
                #field: {
                    let length = <#ty as ::midnight_compact_runtime::fab::Aligned>::alignment().0.len();
                    if offset + length > __compact_cell_value.0.len() {
                        return Err(::midnight_compact_runtime::CompactError::InvalidLedgerCell(
                            "struct atom count differs from declared type".into()
                        ));
                    }
                    let decoded = <#ty as ::midnight_compact_runtime::ledger::CellValue>::decode_cell_value(
                        &__compact_cell_value[offset..offset + length]
                    )?;
                    offset += length;
                    decoded
                },
            }).collect::<Vec<_>>();
            Ok(quote! {
                impl ::midnight_compact_runtime::fab::Aligned for #name {
                    fn alignment() -> ::midnight_compact_runtime::fab::Alignment {
                        let parts: Vec<::midnight_compact_runtime::fab::Alignment> = vec![
                            #(<#types as ::midnight_compact_runtime::fab::Aligned>::alignment()),*
                        ];
                        ::midnight_compact_runtime::fab::Alignment::concat(parts.iter())
                    }
                }

                impl From<#name> for ::midnight_compact_runtime::fab::Value {
                    fn from(value: #name) -> Self {
                        let parts: Vec<::midnight_compact_runtime::fab::Value> = vec![
                            #(value.#names.into()),*
                        ];
                        ::midnight_compact_runtime::fab::Value::concat(parts.iter())
                    }
                }

                impl ::midnight_compact_runtime::ledger::CellValue for #name {
                    fn decode_cell_value(
                        __compact_cell_value: &::midnight_compact_runtime::fab::ValueSlice,
                    ) -> Result<Self, ::midnight_compact_runtime::CompactError> {
                        let mut offset = 0;
                        let decoded = Self { #(#decode)* };
                        if offset != __compact_cell_value.0.len() {
                            return Err(::midnight_compact_runtime::CompactError::InvalidLedgerCell(
                                "struct has trailing atoms".into()
                            ));
                        }
                        Ok(decoded)
                    }
                }
            })
        }
        Data::Enum(data) => {
            if data.variants.is_empty() {
                return Err(syn::Error::new_spanned(
                    name,
                    "CompactCellValue requires enum variants",
                ));
            }
            let variants = data
                .variants
                .iter()
                .map(|variant| {
                    if !matches!(variant.fields, Fields::Unit) {
                        Err(syn::Error::new_spanned(
                            variant,
                            "CompactCellValue supports unit enum variants only",
                        ))
                    } else {
                        Ok(&variant.ident)
                    }
                })
                .collect::<syn::Result<Vec<_>>>()?;
            let to_ordinal = variants
                .iter()
                .enumerate()
                .map(|(index, variant)| {
                    let index = LitInt::new(&index.to_string(), Span::call_site());
                    quote!(#name::#variant => #index,)
                })
                .collect::<Vec<_>>();
            let from_ordinal = variants
                .iter()
                .enumerate()
                .map(|(index, variant)| {
                    let index = LitInt::new(&index.to_string(), Span::call_site());
                    quote!(#index => Ok(Self::#variant),)
                })
                .collect::<Vec<_>>();
            let byte_length = enum_byte_length(variants.len());
            Ok(quote! {
                impl ::midnight_compact_runtime::fab::Aligned for #name {
                    fn alignment() -> ::midnight_compact_runtime::fab::Alignment {
                        ::midnight_compact_runtime::fab::Alignment::singleton(
                            ::midnight_compact_runtime::fab::AlignmentAtom::Bytes { length: #byte_length }
                        )
                    }
                }

                impl From<#name> for ::midnight_compact_runtime::fab::Value {
                    fn from(value: #name) -> Self {
                        let ordinal: u128 = match value { #(#to_ordinal)* };
                        Self::from(ordinal)
                    }
                }

                impl ::midnight_compact_runtime::ledger::CellValue for #name {
                    fn decode_cell_value(
                        value: &::midnight_compact_runtime::fab::ValueSlice,
                    ) -> Result<Self, ::midnight_compact_runtime::CompactError> {
                        let ordinal = <u128 as TryFrom<&::midnight_compact_runtime::fab::ValueSlice>>::try_from(value)
                            .map_err(|error| ::midnight_compact_runtime::CompactError::InvalidLedgerCell(error.to_string()))?;
                        match ordinal {
                            #(#from_ordinal)*
                            _ => Err(::midnight_compact_runtime::CompactError::InvalidLedgerCell(
                                "enum ordinal is outside declared variants".into()
                            )),
                        }
                    }
                }
            })
        }
        _ => Err(syn::Error::new_spanned(
            name,
            "CompactCellValue supports structs and enums only",
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::{enum_byte_length, expand_enum};

    #[test]
    fn enum_binary_width_follows_the_declared_variant_count() {
        assert_eq!(enum_byte_length(1).to_string(), "0");
        assert_eq!(enum_byte_length(256).to_string(), "1");
        assert_eq!(enum_byte_length(257).to_string(), "2");
    }

    #[test]
    fn enum_derive_rejects_non_mechanical_shapes() {
        let generic = syn::parse_quote!(
            enum Generic<T> {
                Value(T),
            }
        );
        assert!(
            expand_enum(generic)
                .unwrap_err()
                .to_string()
                .contains("concrete generated type")
        );

        let tuple = syn::parse_quote!(
            enum Tuple {
                Value(u8),
            }
        );
        assert!(
            expand_enum(tuple)
                .unwrap_err()
                .to_string()
                .contains("unit variants")
        );

        let discriminant = syn::parse_quote!(
            enum Numbered {
                First = 7,
            }
        );
        assert!(
            expand_enum(discriminant)
                .unwrap_err()
                .to_string()
                .contains("without Rust discriminants")
        );

        let empty = syn::parse_quote!(
            enum Empty {}
        );
        assert!(
            expand_enum(empty)
                .unwrap_err()
                .to_string()
                .contains("requires enum variants")
        );
    }
}

#[cfg(test)]
#[path = "tests/cell_value.rs"]
mod cell_value_tests;
