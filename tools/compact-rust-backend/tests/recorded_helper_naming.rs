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
        result: Type::Unsigned {
            max: u64::MAX.to_string(),
        },
        return_value: StateReturn::CounterRead {
            field: "count".into(),
            index: 1,
        },
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
        ledger_fields: vec![
            LedgerField {
                source: None,
                id: "cell".into(),
                index: 0,
                path: vec![],
                declaration: LedgerFieldKind::Cell { ty: Type::Field },
            },
            LedgerField {
                source: None,
                id: "count".into(),
                index: 1,
                path: vec![],
                declaration: LedgerFieldKind::Counter,
            },
        ],
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
        // CounterRead keeps this complete root on actual legacy action dispatch.
        // The canonical Uint128 public parameter is no longer a routing boundary.
        let fixture = contract(helper_name, true, true);
        let source = render(&fixture).unwrap();
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

#[test]
fn checked_only_call_inlines_without_emitting_an_unused_helper() {
    let mut fixture = contract("inner", true, false);
    let caller = &mut fixture.stateful_circuits[1];
    caller.result = Type::Unit;
    caller.return_value = StateReturn::Unit;
    let rendered = render_with_capabilities(&fixture).unwrap();
    assert!(
        rendered
            .capabilities
            .circuits
            .iter()
            .all(|c| c.recorded && c.observed_call)
    );
    let functions = recorded_functions(&rendered.source);
    assert!(
        !functions
            .iter()
            .any(|f| f.sig.ident == "__compact_recorded_body_inner")
    );
    let outer = functions.iter().find(|f| f.sig.ident == "outer").unwrap();
    assert_one_witness_then_write(outer);
}

#[test]
fn mixed_callers_keep_the_used_helper_and_validate_its_transitive_body() {
    let mut fixture = contract("inner", true, false);
    let mut leaf = fixture.stateful_circuits[0].clone();
    leaf.name = "leaf".into();
    fixture.stateful_circuits[0].actions = vec![StateAction::CircuitCall {
        name: "leaf".into(),
        arguments: vec![Expr::Parameter {
            name: "seed".into(),
        }],
    }];
    fixture.stateful_circuits.push(leaf);
    let mut checked = fixture.stateful_circuits[1].clone();
    checked.name = "checked".into();
    checked.result = Type::Unit;
    checked.return_value = StateReturn::Unit;
    fixture.stateful_circuits.push(checked);
    let rendered = render_with_capabilities(&fixture).unwrap();
    assert!(
        rendered
            .capabilities
            .circuits
            .iter()
            .all(|c| c.recorded && c.observed_call)
    );
    let functions = recorded_functions(&rendered.source);
    let inner = functions
        .iter()
        .find(|f| f.sig.ident == "__compact_recorded_body_inner")
        .unwrap();
    let outer = functions.iter().find(|f| f.sig.ident == "outer").unwrap();
    let checked = functions.iter().find(|f| f.sig.ident == "checked").unwrap();
    assert!(
        quote::quote!(#outer)
            .to_string()
            .contains("__compact_recorded_body_inner")
    );
    assert!(
        !quote::quote!(#checked)
            .to_string()
            .contains("__compact_recorded_body_inner")
    );
    assert_one_witness_then_write(inner);
    assert_one_witness_then_write(checked);
    // The retained helper itself inlines its checked child, so only its live
    // out-of-line definition is necessary. The child is still fully validated.
    assert!(
        !functions
            .iter()
            .any(|f| f.sig.ident == "__compact_recorded_body_leaf")
    );
    fixture.stateful_circuits[2].actions = vec![StateAction::CircuitCall {
        name: "inner".into(),
        arguments: vec![Expr::Parameter {
            name: "seed".into(),
        }],
    }];
    assert!(
        render(&fixture).is_err(),
        "transitive recursion must not be pruned away"
    );
}

fn recorded_functions(source: &str) -> Vec<syn::ItemFn> {
    let file = syn::parse_file(source).unwrap();
    let ledger = file
        .items
        .into_iter()
        .find_map(|item| match item {
            syn::Item::Mod(module) if module.ident == "ledger_contract" => {
                Some(module.content.unwrap().1)
            }
            _ => None,
        })
        .expect("ledger_contract module");
    ledger
        .into_iter()
        .find_map(|item| match item {
            syn::Item::Mod(module) if module.ident == "recorded" => Some(
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
            ),
            _ => None,
        })
        .expect("recorded module")
}

fn assert_one_witness_then_write(function: &syn::ItemFn) {
    use syn::visit_mut::{self, VisitMut};
    #[derive(Default)]
    struct EffectCalls(Vec<String>);
    impl VisitMut for EffectCalls {
        fn visit_expr_method_call_mut(&mut self, call: &mut syn::ExprMethodCall) {
            let name = call.method.to_string();
            if matches!(
                name.as_str(),
                "try_witness_metered" | "secret" | "record_write"
            ) {
                self.0.push(name);
            }
            visit_mut::visit_expr_method_call_mut(self, call);
        }
    }
    let mut calls = EffectCalls::default();
    calls.visit_item_fn_mut(&mut function.clone());
    assert_eq!(
        calls.0,
        ["try_witness_metered", "secret", "record_write"],
        "{} must retain one metered witness callback before one Cell write",
        function.sig.ident
    );
}
