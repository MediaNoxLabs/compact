// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Remaining Map operations preserve key-only metadata and scalar mutation policies.
use super::super::type_environment::TypeEnvironment;
use super::*;
#[path = "map_remaining/authority.rs"]
mod authority;
#[path = "map_remaining/composition.rs"]
mod composition;

fn key() -> Expr {
    Expr::Parameter { name: "key".into() }
}
fn contract(value: Type, kind: usize, direct: bool) -> Contract {
    let field = "table".to_owned();
    let index = 0;
    let (actions, result, returned) = match kind {
        0 => (
            vec![StateAction::MapInsertDefault {
                field,
                index,
                key: key(),
            }],
            Type::Unit,
            StateReturn::Unit,
        ),
        1 => (
            vec![StateAction::MapReset { field, index }],
            Type::Unit,
            StateReturn::Unit,
        ),
        2 => (
            vec![],
            validate::uint("18446744073709551615"),
            StateReturn::MapSize { field, index },
        ),
        _ => (
            vec![],
            Type::Boolean,
            if direct {
                StateReturn::MapIsEmpty { field, index }
            } else {
                StateReturn::Expression {
                    value: Expr::MapIsEmpty { field, index },
                }
            },
        ),
    };
    Contract {
        schema_version: SCHEMA_VERSION,
        type_aliases: vec![],
        constructor: None,
        circuits: vec![],
        witnesses: vec![],
        ledger_fields: vec![LedgerField {
            id: "table".into(),
            index: 0,
            path: vec![0],
            source: None,
            declaration: LedgerFieldKind::Map {
                key: Type::Boolean,
                value,
            },
        }],
        stateful_circuits: vec![StatefulCircuit {
            name: "map_action".into(),
            source: None,
            internal: false,
            parameters: vec![Parameter {
                name: "key".into(),
                ty: Type::Boolean,
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
        .position(|b| {
            matches!(
                b.operation,
                Operation::MapInsertDefault { .. }
                    | Operation::MapReset { .. }
                    | Operation::MapSize { .. }
                    | Operation::MapIsEmpty { .. }
            )
        })
        .unwrap()
}
fn slot(operation: &mut Operation) -> &mut MapSlot {
    match operation {
        Operation::MapInsertDefault { slot, .. }
        | Operation::MapReset { slot }
        | Operation::MapSize { slot }
        | Operation::MapIsEmpty { slot } => slot,
        _ => unreachable!("new Map operation"),
    }
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
fn four_signatures_and_existing_query_forms_render_the_exact_runtime_method() {
    for (kind, expected) in [
        "record_insert_default",
        "record_reset",
        "record_size",
        "record_is_empty",
    ]
    .into_iter()
    .enumerate()
    {
        for direct in [false, true] {
            let c = contract(Type::Field, kind, direct);
            let plan = checked(&c).unwrap();
            assert!(has_public_program(&plan));
            assert_eq!(
                methods(&plan)
                    .iter()
                    .filter(|m| m.starts_with("record_"))
                    .map(String::as_str)
                    .collect::<Vec<_>>(),
                vec![expected]
            );
            assert_eq!(
                plan.plan().body.bindings[index(plan.plan())].value.ty,
                c.stateful_circuits[0].result
            );
            crate::render_with_capabilities(&c).unwrap();
        }
    }
    let c: Contract = serde_json::from_str(include_str!(
        "../../../../tests/map-boolean-field-schema20-ir.json"
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
