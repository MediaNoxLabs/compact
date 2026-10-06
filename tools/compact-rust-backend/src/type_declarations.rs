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

//! Public source types and their representation derives, isolated from support names.

use crate::ir::{StructField, Type, TypeAlias};
use crate::{RenderError, ident, is_compact_struct_instantiation, located, rust_type};
use proc_macro2::Span;
use std::collections::{BTreeMap, HashSet};
use syn::visit_mut::{self, VisitMut};

pub(crate) struct TypeDeclarations {
    pub(crate) module: Option<syn::Item>,
    pub(crate) alias_exports: Option<syn::Item>,
}

// Types here share a namespace with arbitrary Compact source names. Bind only
// emitter-owned support paths; retain crate::types paths for source types.
fn declaration_type(ty: &Type) -> Result<syn::Type, RenderError> {
    struct SupportPaths;
    impl VisitMut for SupportPaths {
        fn visit_path_mut(&mut self, path: &mut syn::Path) {
            visit_mut::visit_path_mut(self, path);
            if path.leading_colon.is_none()
                && path.segments.first().is_some_and(|s| s.ident == "runtime")
            {
                path.leading_colon = Some(Default::default());
                path.segments.first_mut().unwrap().ident =
                    syn::Ident::new("midnight_compact_runtime", Span::call_site());
            } else if path.is_ident("bool") {
                *path = syn::parse_quote!(::core::primitive::bool);
            }
        }
    }
    let mut syntax = rust_type(ty)?;
    SupportPaths.visit_type_mut(&mut syntax);
    Ok(syntax)
}

pub(crate) fn render(
    struct_definitions: &BTreeMap<String, Vec<StructField>>,
    enum_definitions: &BTreeMap<String, Vec<String>>,
    aliases: &[TypeAlias],
    has_ledger_slots_module: bool,
) -> Result<TypeDeclarations, RenderError> {
    let mut struct_items = Vec::<syn::Item>::new();
    let has_merkle_path = struct_definitions
        .keys()
        .any(|name| is_compact_struct_instantiation(name, "MerkleTreePath"));
    for (name, fields) in struct_definitions {
        let conversion = if has_merkle_path {
            if is_compact_struct_instantiation(name, "MerkleTreeDigest") {
                Some("CompactMerkleTreeDigest")
            } else if is_compact_struct_instantiation(name, "MerkleTreePathEntry") {
                Some("CompactMerklePathEntry")
            } else if is_compact_struct_instantiation(name, "MerkleTreePath") {
                Some("CompactMerklePath")
            } else {
                None
            }
        } else {
            None
        };
        let name = ident(name)?;
        let fields = fields
            .iter()
            .map(|field| {
                let field_name = ident(&field.name)?;
                let field_ty = declaration_type(&field.ty)?;
                Ok(syn::parse_quote!(pub #field_name: #field_ty))
            })
            .collect::<Result<Vec<syn::Field>, RenderError>>()?;
        let mut item: syn::ItemStruct = syn::parse_quote! {
            #[derive(::core::clone::Clone, ::core::fmt::Debug, ::core::default::Default,
                ::core::cmp::PartialEq, ::core::cmp::Eq,
                ::midnight_compact_runtime::CompactCellValue,
                ::midnight_compact_runtime::CompactStructRepr)]
            pub struct #name { #(#fields),* }
        };
        if let Some(conversion) = conversion {
            let conversion = syn::Ident::new(conversion, Span::call_site());
            item.attrs
                .push(syn::parse_quote!(#[derive(::midnight_compact_runtime::#conversion)]));
        }
        struct_items.push(syn::Item::Struct(item));
    }
    for (name, variants) in enum_definitions {
        let name = ident(name)?;
        let variants = variants
            .iter()
            .map(|variant| ident(variant))
            .collect::<Result<Vec<_>, _>>()?;
        struct_items.push(syn::parse_quote! {
            #[allow(non_camel_case_types)]
            #[derive(::core::clone::Clone, ::core::marker::Copy, ::core::fmt::Debug,
                ::core::cmp::PartialEq, ::core::cmp::Eq,
                ::midnight_compact_runtime::CompactCellValue,
                ::midnight_compact_runtime::CompactEnum)]
            pub enum #name { #(#variants),* }
        });
    }
    let mut alias_names = HashSet::new();
    let mut alias_reexports = Vec::new();
    for alias in aliases {
        located(alias.source.as_ref(), || {
            let name = ident(&alias.name)?;
            let rendered_name = name.to_string();
            let normalized_name = rendered_name.strip_prefix("r#").unwrap_or(&rendered_name);
            // Root alias reexports and the generated slots module share Rust's
            // type namespace. A raw identifier or Compact `$` spelling cannot
            // evade this collision, but a contract without slots owns the name.
            if has_ledger_slots_module && normalized_name == "ledger_slots" {
                return Err(RenderError::ConflictingTypeAlias(alias.name.clone()));
            }
            if !alias_names.insert(alias.name.as_str())
                || struct_definitions.contains_key(&alias.name)
                || enum_definitions.contains_key(&alias.name)
                || matches!(
                    alias.name.as_str(),
                    "runtime" | "types" | "pure_circuits" | "ledger_contract"
                )
            {
                return Err(RenderError::ConflictingTypeAlias(alias.name.clone()));
            }
            let ty = declaration_type(&alias.ty)?;
            struct_items
                .push(syn::parse_quote!(#[allow(non_camel_case_types)] pub type #name = #ty;));
            alias_reexports.push(name);
            Ok(())
        })?;
    }
    let types_module: Option<syn::Item> = if struct_items.is_empty() {
        None
    } else {
        Some(syn::parse_quote! {
            #[allow(non_snake_case, non_camel_case_types, unused_mut, unused_variables)]
            pub mod types {
                #(#struct_items)*
            }
        })
    };
    let alias_exports: Option<syn::Item> = if alias_reexports.is_empty() {
        None
    } else {
        Some(syn::parse_quote!(pub use types::{#(#alias_reexports),*};))
    };
    Ok(TypeDeclarations {
        module: types_module,
        alias_exports,
    })
}
