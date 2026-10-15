// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Native validity, old helper selection, current checked admission and Rust
//! liveness are distinct premises. Synthetic laws aren't frontend/proof claims.
use super::*;
use crate::ir::{Contract, Parameter, SCHEMA_VERSION, StructField};
use quote::ToTokens;

#[path = "helper_discovery_tests/boundaries.rs"]
mod boundaries;
#[path = "helper_discovery_tests/declarations.rs"]
mod declarations;
#[path = "helper_discovery_tests/graph.rs"]
mod graph;
#[path = "helper_discovery_tests/source.rs"]
mod source;

fn value(ty: &Type) -> Expr {
    Expr::Default { ty: ty.clone() }
}
fn fixture(result: Type, returned: StateReturn, internal: bool) -> Contract {
    Contract {
        schema_version: SCHEMA_VERSION,
        type_aliases: vec![],
        constructor: None,
        witnesses: vec![],
        circuits: vec![],
        ledger_fields: vec![],
        stateful_circuits: vec![
            StatefulCircuit {
                source: None,
                name: "callee".into(),
                internal,
                parameters: vec![],
                actions: vec![],
                result,
                return_value: returned,
            },
            StatefulCircuit {
                source: None,
                name: "root".into(),
                internal: false,
                parameters: vec![],
                actions: vec![StateAction::CircuitCall {
                    name: "callee".into(),
                    arguments: vec![],
                }],
                result: Type::Unit,
                return_value: StateReturn::Unit,
            },
        ],
    }
}
struct Context<'a> {
    ledger: HashMap<&'a str, &'a LedgerField>,
    witnesses: HashMap<&'a str, &'a WitnessDeclaration>,
    pure: HashMap<&'a str, &'a PureCircuit>,
    circuits: HashMap<&'a str, &'a StatefulCircuit>,
}
impl<'a> Context<'a> {
    fn new(c: &'a Contract) -> Self {
        Self {
            ledger: c.ledger_fields.iter().map(|v| (v.id.as_str(), v)).collect(),
            witnesses: c.witnesses.iter().map(|v| (v.name.as_str(), v)).collect(),
            pure: c.circuits.iter().map(|v| (v.name.as_str(), v)).collect(),
            circuits: c
                .stateful_circuits
                .iter()
                .map(|v| (v.name.as_str(), v))
                .collect(),
        }
    }
    fn plan(
        &self,
        c: &Contract,
        old: bool,
    ) -> Result<(HashSet<String>, Vec<syn::Item>), RenderError> {
        let planner = if old {
            helpers_reference::plan_recorded_helpers
        } else {
            helpers::plan_recorded_helpers
        };
        planner(
            &c.stateful_circuits,
            &self.ledger,
            &self.witnesses,
            &self.pure,
            &self.circuits,
        )
    }
}
fn normalized(item: &syn::Item) -> String {
    item.to_token_stream().to_string()
}
fn recorded_functions(c: &Contract) -> Vec<syn::ItemFn> {
    let rendered = crate::render_with_capabilities(c).unwrap();
    let file = syn::parse_file(&rendered.source).unwrap();
    let contract = file
        .items
        .into_iter()
        .find_map(|item| {
            if let syn::Item::Mod(module) = item
                && module.ident == "ledger_contract"
            {
                Some(module.content.unwrap().1)
            } else {
                None
            }
        })
        .unwrap();
    contract
        .into_iter()
        .find_map(|item| {
            if let syn::Item::Mod(module) = item
                && module.ident == "recorded"
            {
                Some(
                    module
                        .content
                        .unwrap()
                        .1
                        .into_iter()
                        .filter_map(|item| {
                            if let syn::Item::Fn(function) = item {
                                Some(function)
                            } else {
                                None
                            }
                        })
                        .collect(),
                )
            } else {
                None
            }
        })
        .unwrap()
}
