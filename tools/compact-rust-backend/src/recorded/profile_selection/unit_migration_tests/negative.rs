// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use super::*;

#[test]
fn malformed_scope_slot_types_witnesses_and_calls_still_refuse() {
    for mode in 0..9 {
        let mut contract = subject();
        match mode {
            0 => contract.stateful_circuits[1].actions = vec![write("amount", 9, field())],
            1 => contract.ledger_fields[0].declaration = LedgerFieldKind::Counter,
            2 => {
                contract.stateful_circuits[1].actions = vec![write("amount", 0, parameter("seed"))]
            } // caller capture
            3 => {
                contract.stateful_circuits[1].actions = vec![write(
                    "amount",
                    0,
                    Expr::WitnessCall {
                        name: "w_amount".into(),
                        arguments: vec![field()],
                    },
                )]
            }
            4 => {
                contract.stateful_circuits[1].actions =
                    vec![write("amount", 0, project(record_value(), "amount", 1))]
            }
            5 => contract.stateful_circuits[1].actions = vec![write("amount", 0, boolean())],
            6 => {
                contract.stateful_circuits[0].return_value = StateReturn::Expression {
                    value: call("missing", vec![field()]),
                }
            }
            7 => contract.circuits.push(identity("store", Type::Field)),
            8 => {
                contract.stateful_circuits[1].actions = vec![write(
                    "amount",
                    0,
                    Expr::Sequence {
                        steps: vec![call("store", vec![field()])],
                        value: Box::new(field()),
                    },
                )]
            }
            _ => unreachable!(),
        }
        let context = Context::new(&contract);
        let root = &contract.stateful_circuits[0];
        assert!(context.old(root).is_none(), "old negative {mode}");
        assert!(
            matches!(
                context.algebra(root),
                Err(effect_plan::PlanError::Unsupported(_))
            ),
            "new negative {mode}"
        );
    }
}

#[test]
fn pure_helper_effects_remain_forbidden_even_in_an_unselected_branch() {
    let mut contract = subject();
    contract
        .circuits
        .iter_mut()
        .find(|c| c.name == "p_flag")
        .unwrap()
        .body = Expr::If {
        condition: Box::new(Expr::Boolean { value: false }),
        then: Box::new(equal(witness("w_amount"), field())),
        otherwise: Box::new(boolean()),
    };
    contract.stateful_circuits[1].actions = vec![write("flag", 1, call("p_flag", vec![boolean()]))];
    let context = Context::new(&contract);
    assert!(context.old(&contract.stateful_circuits[0]).is_none());
    assert!(matches!(
        context.algebra(&contract.stateful_circuits[0]),
        Err(effect_plan::PlanError::Unsupported(_))
    ));
}

#[test]
fn removed_root_helper_and_effect_counts_are_labeled_as_expansions() {
    for mode in 0..6 {
        let mut contract = subject();
        match mode {
            0 => {
                contract.stateful_circuits[0].parameters.clear();
                contract.stateful_circuits[0].return_value =
                    StateReturn::Expression { value: Expr::Unit };
            }
            1 => {
                contract.stateful_circuits[0].actions = vec![write("amount", 0, field())];
                contract.stateful_circuits[0].return_value =
                    StateReturn::Expression { value: Expr::Unit };
            }
            2 => {
                contract.stateful_circuits[0].parameters[0].ty = Type::Boolean;
                contract.stateful_circuits[0].return_value = StateReturn::Expression {
                    value: call("store", vec![field()]),
                };
            }
            3 => {
                contract.stateful_circuits[1].return_value =
                    StateReturn::Expression { value: Expr::Unit }
            }
            4 => contract.stateful_circuits[1].actions.clear(),
            5 => {
                let paths = crate::expected_ledger_paths(16);
                for (index, field) in contract.ledger_fields.iter_mut().enumerate() {
                    field.path = paths[index].clone();
                    field.index = paths[index][0];
                }
                for (index, path) in paths.into_iter().enumerate().skip(5) {
                    contract.ledger_fields.push(LedgerField {
                        source: None,
                        id: format!("padding_{index}"),
                        index: path[0],
                        path,
                        declaration: LedgerFieldKind::Cell { ty: Type::Field },
                    });
                }
            }
            _ => unreachable!(),
        }
        let context = Context::new(&contract);
        let root = &contract.stateful_circuits[0];
        assert!(
            context.old(root).is_none(),
            "expansion {mode} must not count as old inclusion"
        );
        assert!(context.algebra(root).is_ok(), "expansion {mode}");
        assert!(context.unit(root).unwrap().is_some(), "expansion {mode}");
        let rendered = crate::render_with_capabilities(&contract).unwrap();
        let capability = rendered
            .capabilities
            .circuits
            .iter()
            .find(|c| c.name == "root")
            .unwrap();
        assert!(
            capability.recorded && capability.observed_call,
            "expansion {mode}"
        );
    }
}

#[test]
fn legacy_unbuildable_tuple_equality_is_an_explicit_compatibility_exception() {
    for arity in [0, 1, 12, 13] {
        let mut contract = subject();
        let tuple = || Expr::Tuple {
            elements: vec![field(); arity],
        };
        contract.stateful_circuits[1].actions = vec![write("flag", 1, equal(tuple(), tuple()))];
        let context = Context::new(&contract);
        let root = &contract.stateful_circuits[0];
        assert!(
            context.old(root).is_some(),
            "legacy planner accepted tuple arity {arity}"
        );
        if arity <= 12 {
            included(&contract, "root", &format!("tuple-equality-{arity}"));
        } else {
            // Rust supplies no tuple PartialEq above 12. CoPS005 records this
            // invalid old generated output; it is not a supported language case.
            assert!(matches!(
                context.algebra(root),
                Err(effect_plan::PlanError::Unsupported(_))
            ));
        }
    }
}
