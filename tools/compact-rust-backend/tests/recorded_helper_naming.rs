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

use compact_rust_backend::ir::{
    Contract, Expr, LedgerField, LedgerFieldKind, LocalBinding, Parameter, SCHEMA_VERSION,
    StateAction, StateReturn, StatefulCircuit, Type, WitnessDeclaration,
};
use compact_rust_backend::{RenderError, render, render_with_capabilities};

fn contract(helper_name: &str, caller: bool, collide: bool) -> Contract {
    let seed = Expr::Parameter {
        name: "seed".into(),
    };
    let helper = StatefulCircuit {
        source: None,
        internal: true,
        name: helper_name.into(),
        parameters: vec![Parameter {
            name: "seed".into(),
            ty: Type::Field,
        }],
        actions: vec![StateAction::Let {
            bindings: vec![LocalBinding {
                name: "value".into(),
                ty: Type::Field,
                value: Expr::WitnessCall {
                    name: "secret".into(),
                    arguments: vec![seed.clone()],
                },
            }],
            action: Box::new(StateAction::CellWrite {
                field: "cell".into(),
                index: 0,
                value: Expr::Parameter {
                    name: "value".into(),
                },
            }),
        }],
        result: Type::Unit,
        return_value: StateReturn::Unit,
    };
    let caller_circuit = StatefulCircuit {
        source: None,
        internal: false,
        name: "outer".into(),
        parameters: vec![Parameter {
            name: "seed".into(),
            ty: Type::Field,
        }],
        actions: vec![StateAction::CircuitCall {
            name: helper_name.into(),
            arguments: vec![seed],
        }],
        result: Type::Unit,
        return_value: StateReturn::Unit,
    };
    let mut stateful_circuits = vec![helper];
    if caller {
        stateful_circuits.push(caller_circuit.clone());
    }
    if collide {
        let mut collision = caller_circuit;
        let normalized = helper_name.strip_prefix("r#").unwrap_or(helper_name);
        collision.name = format!("__compact_recorded_body_{normalized}");
        stateful_circuits.push(collision);
    }
    Contract {
        schema_version: SCHEMA_VERSION,
        type_aliases: vec![],
        ledger_fields: vec![LedgerField {
            source: None,
            id: "cell".into(),
            index: 0,
            path: vec![],
            declaration: LedgerFieldKind::Cell { ty: Type::Field },
        }],
        constructor: None,
        witnesses: vec![WitnessDeclaration {
            source: None,
            name: "secret".into(),
            parameters: vec![Parameter {
                name: "seed".into(),
                ty: Type::Field,
            }],
            result: Type::Field,
        }],
        circuits: vec![],
        stateful_circuits,
    }
}

#[test]
fn raw_spelled_helper_records_without_changing_native_name() {
    let native = render(&contract("r#type", false, false)).unwrap();
    syn::parse_file(&native).unwrap();
    assert!(native.contains("pub(crate) fn r#type<"));

    let recorded = render_with_capabilities(&contract("r#type", true, false)).unwrap();
    syn::parse_file(&recorded.source).unwrap();
    assert!(recorded.source.contains("pub(crate) fn r#type<"));
    assert!(recorded.source.contains("fn __compact_recorded_body_type<"));
    assert!(
        recorded
            .capabilities
            .circuits
            .iter()
            .any(|c| c.name == "outer" && c.recorded)
    );
}

#[test]
fn compact_dollar_normalization_retains_existing_helper_spelling() {
    let source = render(&contract("a$b", true, false)).unwrap();
    syn::parse_file(&source).unwrap();
    assert!(source.contains("pub(crate) fn a_b<"));
    assert!(source.contains("fn __compact_recorded_body_a_b<"));
}

#[test]
fn helper_collision_uses_stable_fallback_for_normal_and_raw_spelling() {
    for helper_name in ["inner", "r#type"] {
        let source = render(&contract(helper_name, true, true)).unwrap();
        syn::parse_file(&source).unwrap();
        let suffix = helper_name
            .bytes()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        let index = if helper_name.starts_with("r#") { 2 } else { 1 };
        let fallback = format!("__compact_recorded_body_{index}_x{suffix}");
        assert!(source.contains(&format!("fn {fallback}<")), "{helper_name}");
        assert!(
            source.contains(&format!("let (frame, _) = {fallback}(")),
            "{helper_name}"
        );
    }
}

#[test]
fn raw_spelled_sibling_occupies_the_preferred_helper_name() {
    let mut contract = contract("r#type", true, true);
    contract.stateful_circuits[2].name = "r#__compact_recorded_body_type".into();
    let source = render(&contract).unwrap();
    syn::parse_file(&source).unwrap();
    assert!(source.contains("fn __compact_recorded_body_2_x722374797065<"));
    assert!(!source.contains("fn __compact_recorded_body_type<"));
}

#[test]
fn raw_spelled_sibling_also_occupies_the_fallback_name() {
    let mut contract = contract("inner", true, true);
    let mut fallback_collision = contract.stateful_circuits[1].clone();
    fallback_collision.name = "r#__compact_recorded_body_1_x696e6e6572".into();
    contract.stateful_circuits.push(fallback_collision);
    let source = render(&contract).unwrap();
    syn::parse_file(&source).unwrap();
    assert!(source.contains("fn __compact_recorded_body_1_x696e6e6572_<"));
    assert!(!source.contains("fn __compact_recorded_body_1_x696e6e6572<"));
}

#[test]
fn invalid_declaration_precedes_helper_planning() {
    let mut malformed = contract("r#type", true, false);
    malformed.stateful_circuits[1].name = "bad-name".into();
    assert_eq!(
        render(&malformed),
        Err(RenderError::InvalidIdentifier("bad-name".into()))
    );
}
