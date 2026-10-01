//! Derive Compact user-type codecs from Rust syntax built by the typed backend.

use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::quote;
use syn::{Data, DeriveInput, Fields, LitInt, parse_macro_input};

mod merkle;

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
            let max_ordinal = variants.len() - 1;
            let byte_length = ((usize::BITS - max_ordinal.leading_zeros()) as usize).div_ceil(8);
            let byte_length = LitInt::new(&byte_length.to_string(), Span::call_site());
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
