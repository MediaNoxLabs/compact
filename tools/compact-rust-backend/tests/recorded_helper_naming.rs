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

#[path = "support/recorded_helper_contract.rs"]
mod helper_contract;
use helper_contract::contract;

// Keep the former legacy identity-map caller as a checked composition positive.
// Private name allocation is tested against the real planner before filtering.
fn mapped_contract(helper_name: &str, caller: bool, collide: bool) -> Contract {
    let mut fixture = contract(helper_name, caller, collide);
    let scalar = Type::Unsigned { max: "255".into() };
    for circuit in &mut fixture.stateful_circuits {
        if !matches!(circuit.return_value, StateReturn::CounterRead { .. }) {
            continue;
        }
        let actions = std::mem::take(&mut circuit.actions);
        circuit.actions = vec![StateAction::Let {
            bindings: vec![LocalBinding {
                name: "mapped_for_legacy".into(),
                ty: Type::Vector {
                    element: Box::new(scalar.clone()),
                    length: 1,
                },
                value: Expr::VectorMap {
                    parameter: Parameter {
                        name: "item".into(),
                        ty: scalar.clone(),
                    },
                    source: Box::new(Expr::Tuple {
                        elements: vec![Expr::UnsignedLiteral {
                            value: "1".into(),
                            max: "255".into(),
                        }],
                    }),
                    body: Box::new(Expr::Parameter {
                        name: "item".into(),
                    }),
                    result: scalar.clone(),
                    length: 1,
                },
            }],
            action: Box::new(StateAction::Sequence { actions }),
        }];
    }
    fixture
}

#[test]
fn raw_spelled_helper_records_without_changing_native_name() {
    let native = render(&mapped_contract("r#type", false, false)).unwrap();
    syn::parse_file(&native).unwrap();
    assert!(native.contains("pub(crate) fn r#type<"));

    let recorded = render_with_capabilities(&mapped_contract("r#type", true, false)).unwrap();
    syn::parse_file(&recorded.source).unwrap();
    assert!(recorded.source.contains("pub(crate) fn r#type<"));
    assert_no_private_helpers(&recorded.source);
    assert_one_witness_then_write(
        recorded_functions(&recorded.source)
            .iter()
            .find(|f| f.sig.ident == "outer")
            .unwrap(),
    );
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
    let source = render(&mapped_contract("a$b", true, false)).unwrap();
    syn::parse_file(&source).unwrap();
    assert!(source.contains("pub(crate) fn a_b<"));
    assert_no_private_helpers(&source);
}

#[test]
fn helper_collision_preserves_public_names_while_checked_callers_inline() {
    for helper_name in ["inner", "r#type"] {
        let fixture = mapped_contract(helper_name, true, true);
        let source = render(&fixture).unwrap();
        syn::parse_file(&source).unwrap();
        assert_no_private_helpers(&source);
        let functions = recorded_functions(&source);
        let public_name = format!(
            "__compact_recorded_body_{}",
            helper_name.strip_prefix("r#").unwrap_or(helper_name)
        );
        assert!(
            functions
                .iter()
                .any(|function| function.sig.ident == public_name
                    && matches!(function.vis, syn::Visibility::Public(_)))
        );
        assert_one_witness_then_write(
            functions
                .iter()
                .find(|function| function.sig.ident == "outer")
                .unwrap(),
        );
    }
}

#[test]
fn raw_spelled_sibling_occupies_the_preferred_helper_name() {
    let mut contract = mapped_contract("r#type", true, true);
    contract.stateful_circuits[2].name = "r#__compact_recorded_body_type".into();
    let source = render(&contract).unwrap();
    syn::parse_file(&source).unwrap();
    assert_no_private_helpers(&source);
    assert!(
        recorded_functions(&source)
            .iter()
            .any(
                |function| function.sig.ident == "r#__compact_recorded_body_type"
                    && matches!(function.vis, syn::Visibility::Public(_))
            )
    );
}

#[test]
fn raw_spelled_sibling_also_occupies_the_fallback_name() {
    let mut contract = mapped_contract("inner", true, true);
    let mut fallback_collision = contract.stateful_circuits[1].clone();
    fallback_collision.name = "r#__compact_recorded_body_1_x696e6e6572".into();
    contract.stateful_circuits.push(fallback_collision);
    let source = render(&contract).unwrap();
    syn::parse_file(&source).unwrap();
    assert_no_private_helpers(&source);
    assert!(
        recorded_functions(&source)
            .iter()
            .any(
                |function| function.sig.ident == "r#__compact_recorded_body_1_x696e6e6572"
                    && matches!(function.vis, syn::Visibility::Public(_))
            )
    );
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
fn mapped_and_plain_callers_inline_and_validate_their_transitive_body() {
    let mut fixture = mapped_contract("inner", true, false);
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
    // Exercise both mapped and plain callers; both now compose checked effects.
    // Preserve their original transitive helper call.
    let StateAction::Let { action, .. } = checked.actions.remove(0) else {
        unreachable!()
    };
    let StateAction::Sequence { actions } = *action else {
        unreachable!()
    };
    checked.actions = actions;
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
    assert_no_private_helpers(&rendered.source);
    let outer = functions.iter().find(|f| f.sig.ident == "outer").unwrap();
    let checked = functions.iter().find(|f| f.sig.ident == "checked").unwrap();
    assert_one_witness_then_write(outer);
    assert_one_witness_then_write(checked);
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

fn assert_no_private_helpers(source: &str) {
    assert!(recorded_functions(source).iter().all(|function| {
        !function
            .sig
            .ident
            .to_string()
            .starts_with("__compact_recorded_body_")
            || !matches!(function.vis, syn::Visibility::Inherited)
    }));
}
