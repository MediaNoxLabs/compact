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

//! Typed witness declarations and contract ledger projection syntax.

use std::collections::{HashMap, HashSet};

use proc_macro2::Span;

use crate::ir::{LedgerField, LedgerFieldKind, WitnessDeclaration};
use crate::{RenderError, ident, located, rust_type};

pub(crate) struct WitnessSyntax<'a> {
    pub declarations: HashMap<&'a str, &'a WitnessDeclaration>,
    pub trait_methods: Vec<syn::TraitItemFn>,
    pub ledger_view_methods: Vec<syn::ImplItemFn>,
}

pub(crate) fn build<'a>(
    witnesses: &'a [WitnessDeclaration],
    ledger_fields: &[&LedgerField],
) -> Result<WitnessSyntax<'a>, RenderError> {
    let mut declarations = HashMap::new();
    let mut trait_methods = Vec::new();
    for witness in witnesses {
        located(witness.source.as_ref(), || {
            let name = ident(&witness.name)?;
            if declarations
                .insert(witness.name.as_str(), witness)
                .is_some()
            {
                return Err(RenderError::DuplicateWitness(witness.name.clone()));
            }
            let mut args = Vec::<syn::FnArg>::new();
            let mut parameter_names = HashSet::new();
            for (index, parameter) in witness.parameters.iter().enumerate() {
                ident(&parameter.name)?;
                if !parameter_names.insert(parameter.name.as_str()) {
                    return Err(RenderError::DuplicateParameter(parameter.name.clone()));
                }
                let parameter_name =
                    syn::Ident::new(&format!("__compact_param_{index}"), Span::call_site());
                let ty = rust_type(&parameter.ty)?;
                args.push(syn::parse_quote!(#parameter_name: #ty));
            }
            let result = rust_type(&witness.result)?;
            trait_methods.push(syn::parse_quote! {
                fn #name(
                    &self,
                    context: runtime::context::WitnessContext<'_, Private, LedgerView<'_>>,
                    #(#args),*
                ) -> (Private, #result);
            });
            Ok(())
        })?;
    }

    let mut ledger_view_methods = Vec::new();
    if !witnesses.is_empty() {
        for field in ledger_fields {
            located(field.source.as_ref(), || {
                let name = ident(&field.id)?;
                let index = syn::LitInt::new(&field.index.to_string(), Span::call_site());
                let path = field
                    .physical_path()
                    .iter()
                    .map(|part| syn::LitInt::new(&part.to_string(), Span::call_site()))
                    .collect::<Vec<_>>();
                match &field.declaration {
                    LedgerFieldKind::Cell { ty } => {
                        let ty = rust_type(ty)?;
                        ledger_view_methods.push(syn::parse_quote! {
                            pub fn #name(&self) -> Result<#ty, runtime::CompactError> {
                                self.meter.read_cell::<#ty>(&[#(#path),*])
                            }
                        });
                    }
                    LedgerFieldKind::Counter => {
                        let max = syn::LitInt::new(&u64::MAX.to_string(), Span::call_site());
                        ledger_view_methods.push(syn::parse_quote! {
                        pub fn #name(&self) -> Result<runtime::BoundedUint<#max>, runtime::CompactError> {
                            let value = self.meter.read_cell::<u64>(&[#(#path),*])?;
                            runtime::BoundedUint::<#max>::new(value as u128)
                        }
                    });
                    }
                    LedgerFieldKind::Set { ty } => {
                        let ty = rust_type(ty)?;
                        let view: syn::Expr = if path.len() == 1 {
                            syn::parse_quote!(runtime::ledger::set_view::<#ty, _>(self.state, #index))
                        } else {
                            syn::parse_quote!(runtime::ledger::set_view_at_path::<#ty, _>(self.state, &[#(#path),*]))
                        };
                        ledger_view_methods.push(syn::parse_quote! {
                        pub fn #name(&self) -> Result<runtime::ledger::SetView<'a, #ty, runtime::ledger::DefaultDB>, runtime::CompactError> {
                            #view
                        }
                    });
                    }
                    LedgerFieldKind::Map { key, value } => {
                        let key = rust_type(key)?;
                        let value = rust_type(value)?;
                        let view: syn::Expr = if path.len() == 1 {
                            syn::parse_quote!(runtime::ledger::map_view::<#key, #value, _>(self.state, #index))
                        } else {
                            syn::parse_quote!(runtime::ledger::map_view_at_path::<#key, #value, _>(self.state, &[#(#path),*]))
                        };
                        ledger_view_methods.push(syn::parse_quote! {
                        pub fn #name(&self) -> Result<runtime::ledger::MapView<'a, #key, #value, runtime::ledger::DefaultDB>, runtime::CompactError> {
                            #view
                        }
                    });
                    }
                    LedgerFieldKind::List { ty } => {
                        let ty = rust_type(ty)?;
                        ledger_view_methods.push(syn::parse_quote! {
                        pub fn #name(&self) -> Result<runtime::ledger::ListView<'a, #ty, runtime::ledger::DefaultDB>, runtime::CompactError> {
                            runtime::ledger::list_view::<#ty, _>(self.state, #index)
                        }
                    });
                    }
                    LedgerFieldKind::HistoricMerkleTree { .. } => {
                        ledger_view_methods.push(syn::parse_quote! {
                        pub fn #name(&self) -> Result<runtime::ledger::HistoricMerkleTreeView<'a, runtime::ledger::DefaultDB>, runtime::CompactError> {
                            runtime::ledger::historic_merkle_tree_view_at_path(self.state, &[#(#path),*])
                        }
                    });
                    }
                    LedgerFieldKind::MerkleTree { .. } => {
                        ledger_view_methods.push(syn::parse_quote! {
                        pub fn #name(&self) -> Result<runtime::ledger::MerkleTreeView<'a, runtime::ledger::DefaultDB>, runtime::CompactError> {
                            runtime::ledger::merkle_tree_view_at_path(self.state, &[#(#path),*])
                        }
                    });
                    }
                }
                Ok(())
            })?;
        }
    }

    Ok(WitnessSyntax {
        declarations,
        trait_methods,
        ledger_view_methods,
    })
}
