// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//   http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Finite naming guarantees; no generated-output normalization or name rewrite.
use super::*;
use crate::ir::{
    Contract, LedgerField, LedgerFieldKind, Parameter, SCHEMA_VERSION, SourceLocation, StateAction,
    StateReturn, Type,
};
use syn::visit_mut::{self, VisitMut};

fn helper(name: &str) -> StatefulCircuit {
    StatefulCircuit {
        source: None,
        name: name.into(),
        internal: true,
        parameters: vec![],
        actions: vec![],
        result: Type::Unit,
        return_value: StateReturn::Unit,
    }
}

#[test]
fn recorded_helpers_are_deterministic_across_map_order_and_source_metadata() {
    let names = ["alpha", "beta", "__compact_recorded_body_alpha"];
    let declarations = names.map(helper);
    let forward = declarations.iter().map(|c| (c.name.as_str(), c)).collect();
    let reverse = declarations
        .iter()
        .rev()
        .map(|c| (c.name.as_str(), c))
        .collect();
    let first: Vec<_> = names
        .iter()
        .map(|n| recorded_helper_ident(n, &forward).unwrap().to_string())
        .collect();
    let second: Vec<_> = names
        .iter()
        .map(|n| recorded_helper_ident(n, &reverse).unwrap().to_string())
        .collect();
    assert_eq!(first, second);
    assert!(first[0].starts_with("__compact_recorded_body_1_x"));
    assert_eq!(first[1], "__compact_recorded_body_beta");
    let occupied: std::collections::HashSet<_> = names
        .iter()
        .map(|n| semantic_identifier(n).unwrap())
        .collect();
    let emitted: std::collections::HashSet<_> = first
        .iter()
        .map(|n| semantic_identifier(n).unwrap())
        .collect();
    assert_eq!(emitted.len(), names.len());
    assert!(occupied.is_disjoint(&emitted));
    let located: Vec<_> = declarations
        .iter()
        .cloned()
        .map(|mut c| {
            c.source = Some(SourceLocation {
                file: "/private/secret-path-sentinel/source.compact".into(),
                line: 999,
                column: 17,
            });
            c
        })
        .collect();
    let located = located.iter().map(|c| (c.name.as_str(), c)).collect();
    for (name, expected) in names.iter().zip(first) {
        assert_eq!(
            recorded_helper_ident(name, &located).unwrap().to_string(),
            expected
        );
    }
}

#[test]
fn parameter_fallbacks_preserve_one_namespace_for_reserved_raw_and_dollar_names() {
    let parameters = [
        "frame",
        "r#frame",
        "a$b",
        "a_b",
        "type",
        "r#type",
        "r#__compact_param_7",
        "frame",
    ]
    .map(|name| Parameter {
        name: name.into(),
        ty: Type::Field,
    });
    let allocate = || {
        parameter_idents(&parameters, &["frame"])
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
    };
    let first = allocate();
    assert_eq!(first, allocate());
    assert_eq!(
        first,
        [
            "__compact_param_0",
            "__compact_param_1",
            "a_b",
            "__compact_param_3",
            "r#type",
            "__compact_param_5",
            "r#__compact_param_7",
            "__compact_param_7_1"
        ]
    );
    let semantic: std::collections::HashSet<_> = first
        .iter()
        .map(|n| semantic_identifier(n).unwrap())
        .collect();
    assert_eq!(semantic.len(), parameters.len());
    assert!(!semantic.contains("frame"));
}

#[test]
fn normalized_public_circuit_collisions_refuse_instead_of_aliasing() {
    for names in [["a$b", "a_b"], ["type", "r#type"], ["foo", "r#foo"]] {
        assert!(matches!(
            validate_circuit_function_namespace(names.iter().map(|n| (*n, None))),
            Err(RenderError::ConflictingCircuitIdentifier { .. })
        ));
    }
}

#[derive(Default)]
struct InternalNames(Vec<String>);
impl VisitMut for InternalNames {
    fn visit_ident_mut(&mut self, ident: &mut syn::Ident) {
        let name = ident.to_string();
        if name.starts_with("__compact_") {
            self.0.push(name);
        }
    }
    fn visit_file_mut(&mut self, file: &mut syn::File) {
        visit_mut::visit_file_mut(self, file);
    }
}
fn internal_names(source: &str) -> Vec<String> {
    let mut names = InternalNames::default();
    names.visit_file_mut(&mut syn::parse_file(source).unwrap());
    names.0
}

#[test]
fn repeated_rendering_ignores_source_location_metadata_for_internal_names() {
    let mut contract = Contract {
        schema_version: SCHEMA_VERSION,
        type_aliases: vec![],
        ledger_fields: vec![LedgerField {
            id: "counter".into(),
            index: 0,
            path: vec![],
            source: None,
            declaration: LedgerFieldKind::Counter,
        }],
        constructor: None,
        witnesses: vec![],
        circuits: vec![],
        stateful_circuits: vec![StatefulCircuit {
            source: None,
            name: "reset".into(),
            internal: false,
            parameters: vec![],
            actions: vec![StateAction::CounterReset {
                field: "counter".into(),
                index: 0,
            }],
            result: Type::Unit,
            return_value: StateReturn::Unit,
        }],
    };
    let before = crate::render(&contract).unwrap();
    assert_eq!(before, crate::render(&contract).unwrap());
    let source = SourceLocation {
        file: "/private/secret-path-sentinel/reset.compact".into(),
        line: 1234,
        column: 56,
    };
    contract.ledger_fields[0].source = Some(source.clone());
    contract.stateful_circuits[0].source = Some(source);
    let after = crate::render(&contract).unwrap();
    let names = internal_names(&before);
    assert!(!names.is_empty(), "fixture must exercise generated names");
    assert_eq!(names, internal_names(&after));
    assert!(!after.contains("secret-path-sentinel"));
    assert_eq!(before, after);
}
