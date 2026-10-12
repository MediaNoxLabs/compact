// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Typed List operations: source normalization, immutable authority and ordered SSA.
use super::super::type_environment::TypeEnvironment;
use super::*;
#[path = "lists/authority.rs"]
mod authority;
#[path = "lists/composition.rs"]
mod composition;

fn maybe(element: Type, name: &str) -> Type {
    Type::Struct {
        name: name.into(),
        fields: vec![
            StructField {
                name: "is_some".into(),
                ty: Type::Boolean,
            },
            StructField {
                name: "value".into(),
                ty: element,
            },
        ],
    }
}
fn parameter() -> Expr {
    Expr::Parameter {
        name: "input".into(),
    }
}
fn contract(element: Type, kind: usize, direct: bool) -> Contract {
    let field = "items".to_owned();
    let index = 0;
    let (actions, result, returned) = match kind {
        0 => (
            vec![StateAction::ListPushFront {
                field,
                index,
                value: parameter(),
            }],
            Type::Unit,
            StateReturn::Unit,
        ),
        1 => (
            vec![StateAction::ListPopFront { field, index }],
            Type::Unit,
            StateReturn::Unit,
        ),
        2 => (
            vec![StateAction::ListReset { field, index }],
            Type::Unit,
            StateReturn::Unit,
        ),
        n => {
            let result = match n {
                3 => validate::uint("18446744073709551615"),
                4 => Type::Boolean,
                _ => maybe(element.clone(), "MaybeCompact7"),
            };
            let returned = if direct {
                match n {
                    3 => StateReturn::ListLength { field, index },
                    4 => StateReturn::ListIsEmpty { field, index },
                    _ => StateReturn::ListHead { field, index },
                }
            } else {
                StateReturn::Expression {
                    value: match n {
                        3 => Expr::ListLength { field, index },
                        4 => Expr::ListIsEmpty { field, index },
                        _ => Expr::ListHead {
                            field,
                            index,
                            ty: result.clone(),
                        },
                    },
                }
            };
            (vec![], result, returned)
        }
    };
    Contract {
        schema_version: SCHEMA_VERSION,
        type_aliases: vec![],
        constructor: None,
        circuits: vec![],
        witnesses: vec![],
        ledger_fields: vec![LedgerField {
            id: "items".into(),
            index: 0,
            path: vec![0],
            source: None,
            declaration: LedgerFieldKind::List {
                ty: element.clone(),
            },
        }],
        stateful_circuits: vec![StatefulCircuit {
            name: "list_action".into(),
            source: None,
            internal: false,
            parameters: vec![Parameter {
                name: "input".into(),
                ty: element,
            }],
            actions,
            result,
            return_value: returned,
        }],
    }
}
fn checked(c: &Contract) -> Result<CheckedPlan, PlanError> {
    check_with_calls(
        &c.stateful_circuits[0],
        &c.ledger_fields.iter().map(|f| (f.id.as_str(), f)).collect(),
        &c.witnesses.iter().map(|w| (w.name.as_str(), w)).collect(),
        &c.circuits.iter().map(|p| (p.name.as_str(), p)).collect(),
        &c.stateful_circuits
            .iter()
            .map(|s| (s.name.as_str(), s))
            .collect(),
    )
}
fn declarations(c: &Contract) -> declarations::Declarations {
    let ledger = c.ledger_fields.iter().map(|f| (f.id.as_str(), f)).collect();
    let witnesses = c.witnesses.iter().map(|w| (w.name.as_str(), w)).collect();
    let pure = c.circuits.iter().map(|p| (p.name.as_str(), p)).collect();
    let calls = c
        .stateful_circuits
        .iter()
        .map(|s| (s.name.as_str(), s))
        .collect();
    let types =
        TypeEnvironment::from_sources(&c.stateful_circuits[0], &ledger, &witnesses, &pure, &calls)
            .unwrap();
    declarations::Declarations::with_calls(&ledger, &witnesses, &pure, &calls)
        .with_type_environment(types)
}
fn index(plan: &Plan) -> usize {
    plan.body
        .bindings
        .iter()
        .position(|b| matches!(b.operation, Operation::List { .. }))
        .unwrap()
}
fn methods(plan: &CheckedPlan) -> Vec<String> {
    use syn::visit_mut::VisitMut;
    #[derive(Default)]
    struct Calls(Vec<String>);
    impl VisitMut for Calls {
        fn visit_expr_method_call_mut(&mut self, node: &mut syn::ExprMethodCall) {
            self.0.push(node.method.to_string());
            syn::visit_mut::visit_expr_method_call_mut(self, node);
        }
    }
    let mut calls = Calls::default();
    for mut statement in render(plan).unwrap().steps {
        calls.visit_stmt_mut(&mut statement);
    }
    calls.0
}

#[test]
fn all_six_signatures_and_both_query_forms_reuse_existing_runtime_methods() {
    let expected = [
        "record_push_front",
        "record_pop_front",
        "record_reset",
        "record_length",
        "record_is_empty",
        "record_head",
    ];
    for (kind, method) in expected.into_iter().enumerate() {
        for direct in [false, true] {
            let c = contract(Type::Field, kind, direct);
            let plan = checked(&c).unwrap();
            assert!(has_public_program(&plan));
            assert_eq!(
                methods(&plan)
                    .iter()
                    .filter(|m| m.starts_with("record_"))
                    .collect::<Vec<_>>(),
                vec![&method.to_owned()]
            );
            crate::render_with_capabilities(&c).unwrap();
            assert_eq!(
                plan.plan().body.bindings[index(plan.plan())].value.ty,
                c.stateful_circuits[0].result
            );
        }
    }
    // A complete genuine frontend contract exercises production dispatch, not only a synthetic plan.
    let c: Contract = serde_json::from_str(include_str!(
        "../../../../tests/adt-list-field-schema20-ir.json"
    ))
    .unwrap();
    for root in &c.stateful_circuits {
        let mut probe = c.clone();
        probe
            .stateful_circuits
            .retain(|candidate| candidate.name != root.name);
        probe.stateful_circuits.insert(0, root.clone());
        checked(&probe).unwrap();
    }
    crate::render_with_capabilities(&c).unwrap();
}
