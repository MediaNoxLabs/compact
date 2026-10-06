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

//! Conversions between Compact standard-library Merkle structs and ledger-8 paths.

use proc_macro2::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Fields, GenericArgument, PathArguments, Type};

fn named_fields(input: &DeriveInput) -> syn::Result<&syn::FieldsNamed> {
    let Data::Struct(data) = &input.data else {
        return Err(syn::Error::new_spanned(
            input,
            "Merkle conversion requires a struct",
        ));
    };
    let Fields::Named(fields) = &data.fields else {
        return Err(syn::Error::new_spanned(
            input,
            "Merkle conversion requires named fields",
        ));
    };
    if !input.generics.params.is_empty() {
        return Err(syn::Error::new_spanned(
            input,
            "Merkle conversion requires a concrete type",
        ));
    }
    Ok(fields)
}

fn field<'a>(fields: &'a syn::FieldsNamed, name: &str) -> syn::Result<&'a syn::Field> {
    fields
        .named
        .iter()
        .find(|field| field.ident.as_ref().is_some_and(|ident| ident == name))
        .ok_or_else(|| syn::Error::new_spanned(fields, format!("missing {name} field")))
}

pub(crate) fn digest(input: DeriveInput) -> syn::Result<TokenStream> {
    let fields = named_fields(&input)?;
    field(fields, "field")?;
    let name = &input.ident;
    Ok(quote! {
        impl ::core::convert::From<::midnight_compact_runtime::ledger::MerkleTreeDigest> for #name {
            fn from(digest: ::midnight_compact_runtime::ledger::MerkleTreeDigest) -> Self {
                Self { field: digest.0 }
            }
        }
        impl ::core::convert::From<#name> for ::midnight_compact_runtime::ledger::MerkleTreeDigest {
            fn from(digest: #name) -> Self {
                Self(digest.field)
            }
        }
    })
}

pub(crate) fn entry(input: DeriveInput) -> syn::Result<TokenStream> {
    let fields = named_fields(&input)?;
    field(fields, "sibling")?;
    field(fields, "goes_left")?;
    let name = &input.ident;
    Ok(quote! {
        impl ::core::convert::From<::midnight_compact_runtime::ledger::MerklePathEntry> for #name {
            fn from(entry: ::midnight_compact_runtime::ledger::MerklePathEntry) -> Self {
                Self { sibling: entry.sibling.into(), goes_left: entry.goes_left }
            }
        }
        impl ::core::convert::From<#name> for ::midnight_compact_runtime::ledger::MerklePathEntry {
            fn from(entry: #name) -> Self {
                Self { sibling: entry.sibling.into(), goes_left: entry.goes_left }
            }
        }
    })
}

pub(crate) fn path(input: DeriveInput) -> syn::Result<TokenStream> {
    let fields = named_fields(&input)?;
    let leaf_ty = &field(fields, "leaf")?.ty;
    let vector_ty = &field(fields, "path")?.ty;
    let Type::Path(vector) = vector_ty else {
        return Err(syn::Error::new_spanned(
            vector_ty,
            "path must be a FixedVector",
        ));
    };
    let Some(segment) = vector.path.segments.last() else {
        return Err(syn::Error::new_spanned(
            vector_ty,
            "path must be a FixedVector",
        ));
    };
    if segment.ident != "FixedVector" {
        return Err(syn::Error::new_spanned(
            vector_ty,
            "path must be a FixedVector",
        ));
    }
    let PathArguments::AngleBracketed(args) = &segment.arguments else {
        return Err(syn::Error::new_spanned(
            vector_ty,
            "FixedVector needs entry type and depth",
        ));
    };
    let mut args = args.args.iter();
    let (Some(GenericArgument::Type(entry_ty)), Some(GenericArgument::Const(depth)), None) =
        (args.next(), args.next(), args.next())
    else {
        return Err(syn::Error::new_spanned(
            vector_ty,
            "FixedVector needs entry type and depth",
        ));
    };
    let name = &input.ident;
    Ok(quote! {
        impl #name {
            pub fn from_ledger_path(
                path: ::midnight_compact_runtime::ledger::MerklePath<#leaf_ty>,
            ) -> ::core::result::Result<Self, ::midnight_compact_runtime::CompactError> {
                let entries: [#entry_ty; #depth] = path.path
                    .into_iter()
                    .map(::core::convert::Into::into)
                    .collect::<::std::vec::Vec<#entry_ty>>()
                    .try_into()
                    .map_err(|_| ::midnight_compact_runtime::CompactError::InvalidLedgerCell(
                        "Merkle path depth differs from Compact type".into()
                    ))?;
                ::core::result::Result::Ok(Self { leaf: path.leaf, path: ::midnight_compact_runtime::FixedVector::new(entries) })
            }

            pub fn into_ledger_path(
                self,
            ) -> ::midnight_compact_runtime::ledger::MerklePath<#leaf_ty> {
                ::midnight_compact_runtime::ledger::MerklePath {
                    leaf: self.leaf,
                    path: self.path.into_array().into_iter().map(::core::convert::Into::into).collect(),
                }
            }
        }
    })
}

#[cfg(test)]
#[path = "tests/merkle.rs"]
mod validation_tests;
