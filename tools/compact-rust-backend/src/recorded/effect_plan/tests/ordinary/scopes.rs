// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Only the final Sequence/Let continuation exports local names to the return.
use super::*;

#[test]
fn final_nested_bindings_shadow_parameters_and_keep_sequential_dependencies() {
    let action = binding(
        "value",
        literal("1"),
        sequence(vec![
            binding("value", literal("2"), sequence(vec![])),
            binding(
                "value",
                Expr::Add {
                    left: Box::new(parameter("value")),
                    right: Box::new(literal("3")),
                },
                sequence(vec![]),
            ),
        ]),
    );
    let mut circuit = candidate(vec![action], parameter("value"));
    circuit.parameters.push(Parameter {
        name: "value".into(),
        ty: Type::Field,
    });
    let checked = checked(&contract(), &circuit).unwrap();
    let plan = checked.plan();
    let result = plan
        .body
        .bindings
        .iter()
        .find(|b| b.value == plan.body.result)
        .unwrap();
    let Operation::Add(left, _) = &result.operation else {
        panic!("expected final local addition")
    };
    assert_eq!(
        left, &plan.body.bindings[0].value,
        "earlier sibling's shadow must be restored"
    );
    assert_ne!(plan.body.result, plan.parameters[0]);
}

#[test]
fn earlier_top_level_and_sequence_sibling_locals_do_not_escape() {
    let local = binding("hidden", literal("1"), sequence(vec![]));
    for actions in [
        vec![local.clone(), assert_true()],
        vec![sequence(vec![local.clone(), assert_true()])],
        vec![local.clone(), sequence(vec![])],
        vec![sequence(vec![local, sequence(vec![])])],
    ] {
        let circuit = candidate(actions, parameter("hidden"));
        assert_eq!(refusal(&circuit).path, "return_value.value");
    }
}

#[test]
fn branch_local_names_never_escape_but_enclosing_terminal_locals_remain() {
    for selected in [false, true] {
        let branch = StateAction::If {
            condition: Expr::Boolean { value: selected },
            then: Box::new(binding("value", literal("2"), sequence(vec![]))),
            otherwise: Box::new(binding("value", literal("3"), sequence(vec![]))),
        };
        let circuit = candidate(vec![branch.clone()], parameter("value"));
        assert_eq!(refusal(&circuit).path, "return_value.value");
        let circuit = candidate(
            vec![binding("value", literal("1"), branch)],
            parameter("value"),
        );
        let checked = checked(&contract(), &circuit).unwrap();
        assert_eq!(
            returned_field(&checked),
            crate::field_literal_bytes("1").unwrap()
        );
    }
}

#[test]
fn expression_local_names_do_not_join_the_action_continuation() {
    let circuit = candidate(
        vec![StateAction::Expression {
            value: Expr::Let {
                bindings: vec![LocalBinding {
                    name: "hidden".into(),
                    ty: Type::Field,
                    value: literal("1"),
                }],
                body: Box::new(Expr::Unit),
            },
        }],
        parameter("hidden"),
    );
    assert_eq!(refusal(&circuit).path, "return_value.value");
}

#[test]
fn empty_sequences_preserve_the_enclosing_scope_without_reviving_siblings() {
    for actions in [vec![], vec![sequence(vec![])]] {
        let mut circuit = candidate(actions, parameter("value"));
        circuit.parameters.push(Parameter {
            name: "value".into(),
            ty: Type::Field,
        });
        let checked = checked(&contract(), &circuit).unwrap();
        assert_eq!(checked.plan().body.result, checked.plan().parameters[0]);
        assert!(checked.plan().body.bindings.is_empty());
    }
    for body in [
        sequence(vec![]),
        sequence(vec![
            binding("value", literal("2"), sequence(vec![])),
            sequence(vec![]),
        ]),
    ] {
        let circuit = candidate(
            vec![binding("value", literal("1"), body)],
            parameter("value"),
        );
        let checked = checked(&contract(), &circuit).unwrap();
        assert_eq!(
            returned_field(&checked),
            crate::field_literal_bytes("1").unwrap()
        );
    }
}
