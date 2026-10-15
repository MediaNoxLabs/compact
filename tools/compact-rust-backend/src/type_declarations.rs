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
use crate::naming::semantic_identifier;
use crate::{RenderError, is_compact_struct_instantiation, located};
use crate::{
    native_header,
    source_diagnostics::{Owner, RefusalObserver, RefusalSite},
};
use proc_macro2::Span;
use std::collections::{BTreeMap, HashSet};
use syn::visit_mut::{self, VisitMut};

pub(crate) struct TypeDeclarations {
    pub(crate) module: Option<syn::Item>,
    pub(crate) alias_exports: Option<syn::Item>,
}
/// Original definitions recorded only at the existing successful first insertion.
/// Missing entries describe synthesized definitions, never clones to match.
pub(crate) type DefinitionOrigins<'ir> = BTreeMap<String, &'ir Type>;

// Types here share a namespace with arbitrary Compact source names. Bind only
// emitter-owned support paths; retain crate::types paths for source types.
fn declaration_type(ty: &Type) -> Result<syn::Type, RenderError> {
    declaration_type_observed(ty, None)
}
fn declaration_type_observed(
    ty: &Type,
    observer: Option<&dyn RefusalObserver>,
) -> Result<syn::Type, RenderError> {
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
    let mut syntax = native_header::carrier(ty, observer)?;
    SupportPaths.visit_type_mut(&mut syntax);
    Ok(syntax)
}

pub(crate) fn render(
    struct_definitions: &BTreeMap<String, Vec<StructField>>,
    enum_definitions: &BTreeMap<String, Vec<String>>,
    aliases: &[TypeAlias],
    has_ledger_slots_module: bool,
    has_merkle_witness_view: bool,
    origins: &DefinitionOrigins<'_>,
    observer: Option<&dyn RefusalObserver>,
) -> Result<TypeDeclarations, RenderError> {
    let mut struct_items = Vec::<syn::Item>::new();
    let has_merkle_path = struct_definitions
        .keys()
        .any(|name| is_compact_struct_instantiation(name, "MerkleTreePath"));
    for (name, fields) in struct_definitions {
        let original = origins.get(name).copied();
        let original_fields = original.and_then(|ty| match ty {
            Type::Struct { fields, .. } => Some(fields.as_slice()),
            _ => None,
        });
        let conversion = if has_merkle_witness_view
            && crate::implicit_types::is_witness_merkle_digest(name, fields)
        {
            Some("CompactMerkleTreeDigest")
        } else if has_merkle_path {
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
        let checked_name = original
            .and_then(|ty| match ty {
                Type::Struct { name, .. } => Some(name.as_str()),
                _ => None,
            })
            .unwrap_or(name);
        let name_site = original.map_or_else(RefusalSite::unavailable, |ty| {
            RefusalSite::field(Owner::Type(ty), "name")
        });
        let name = native_header::identifier(checked_name, name_site, observer)?;
        let fields = fields
            .iter()
            .enumerate()
            .map(|(index, field)| {
                // The existing first-insertion fields.clone() supplies this
                // relation; there is no equality gate or clone-pointer match.
                let original = original_fields.map(|fields| &fields[index]);
                let checked = original.unwrap_or(field);
                let name_site = original.map_or_else(RefusalSite::unavailable, |field| {
                    RefusalSite::field(Owner::StructField(field), "name")
                });
                let field_name = native_header::identifier(&checked.name, name_site, observer)?;
                let field_ty = if original.is_some() {
                    declaration_type_observed(&checked.ty, observer)?
                } else {
                    declaration_type(&checked.ty).inspect_err(|_| {
                        if let Some(observer) = observer {
                            observer.refused(RefusalSite::unavailable());
                        }
                    })?
                };
                Ok(syn::parse_quote!(pub #field_name: #field_ty))
            })
            .collect::<Result<Vec<syn::Field>, RenderError>>()?;
        let mut item: syn::ItemStruct = syn::parse_quote! {
            #[derive(::core::clone::Clone, ::core::fmt::Debug, ::core::default::Default,
                ::core::cmp::PartialEq, ::core::cmp::Eq,
                ::midnight_compact_runtime::CompactCellValue,
                ::midnight_compact_runtime::CompactStructRepr,
                ::midnight_compact_runtime::CompactHashInput)]
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
        let original = origins.get(name).copied();
        let checked_name = original
            .and_then(|ty| match ty {
                Type::Enum { name, .. } => Some(name.as_str()),
                _ => None,
            })
            .unwrap_or(name);
        let original_variants = original.and_then(|ty| match ty {
            Type::Enum { variants, .. } => Some(variants.as_slice()),
            _ => None,
        });
        let name_site = original.map_or_else(RefusalSite::unavailable, |ty| {
            RefusalSite::field(Owner::Type(ty), "name")
        });
        let name = native_header::identifier(checked_name, name_site, observer)?;
        let variants = variants
            .iter()
            .enumerate()
            .map(|(index, variant)| {
                let checked = original_variants
                    .map(|variants| &variants[index])
                    .unwrap_or(variant);
                let site = original.map_or_else(RefusalSite::unavailable, |ty| {
                    RefusalSite::element(Owner::Type(ty), "variants", index)
                });
                native_header::identifier(checked, site, observer)
            })
            .collect::<Result<Vec<_>, _>>()?;
        struct_items.push(syn::parse_quote! {
            #[allow(non_camel_case_types)]
            #[derive(::core::clone::Clone, ::core::marker::Copy, ::core::fmt::Debug,
                ::core::cmp::PartialEq, ::core::cmp::Eq,
                ::midnight_compact_runtime::CompactCellValue,
                ::midnight_compact_runtime::CompactEnum,
                ::midnight_compact_runtime::CompactHashInput)]
            pub enum #name { #(#variants),* }
        });
    }
    let named_types = struct_definitions
        .keys()
        .chain(enum_definitions.keys())
        .map(|name| semantic_identifier(name))
        .collect::<Result<HashSet<_>, _>>()?;
    let mut alias_names = HashSet::new();
    let mut alias_reexports = Vec::new();
    for alias in aliases {
        located(alias.source.as_ref(), || {
            let name = native_header::identifier(
                &alias.name,
                RefusalSite::field(Owner::Alias(alias), "name"),
                observer,
            )?;
            let normalized_name = semantic_identifier(&alias.name)?;
            // Root alias reexports and the generated slots module share Rust's
            // type namespace. A raw identifier or Compact `$` spelling cannot
            // evade this collision, but a contract without slots owns the name.
            if has_ledger_slots_module && normalized_name == "ledger_slots" {
                if let Some(observer) = observer {
                    observer.refused(RefusalSite::field(Owner::Alias(alias), "name"));
                }
                return Err(RenderError::ConflictingTypeAlias(alias.name.clone()));
            }
            if !alias_names.insert(normalized_name.clone())
                || named_types.contains(&normalized_name)
                || matches!(
                    normalized_name.as_str(),
                    "runtime" | "types" | "pure_circuits" | "ledger_contract"
                )
            {
                if let Some(observer) = observer {
                    observer.refused(RefusalSite::field(Owner::Alias(alias), "name"));
                }
                return Err(RenderError::ConflictingTypeAlias(alias.name.clone()));
            }
            let ty = declaration_type_observed(&alias.ty, observer)?;
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
