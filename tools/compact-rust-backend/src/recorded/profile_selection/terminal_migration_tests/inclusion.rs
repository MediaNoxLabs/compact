// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use super::*;

fn field_values() -> Vec<Expr> {
    vec![
        parameter("seed"),
        literal(),
        read(),
        Expr::Add {
            left: Box::new(read()),
            right: Box::new(parameter("seed")),
        },
        Expr::Coerce {
            value: Box::new(read()),
            ty: Type::Field,
        },
        Expr::Let {
            bindings: vec![LocalBinding {
                name: "seed".into(),
                ty: Type::Field,
                value: read(),
            }],
            body: Box::new(parameter("seed")),
        },
        Expr::WitnessCall {
            name: "next_value".into(),
            arguments: vec![read()],
        },
        call("pure_field", vec![parameter("seed")]),
        call("stateful_field", vec![parameter("seed")]),
    ]
}
fn helpers(contract: &mut Contract) {
    contract.circuits = vec![
        PureCircuit {
            source: None,
            name: "pure_field".into(),
            internal: true,
            parameters: vec![Parameter {
                name: "value".into(),
                ty: Type::Field,
            }],
            result: Type::Field,
            body: Expr::TransientHash {
                value: Box::new(parameter("value")),
            },
        },
        PureCircuit {
            source: None,
            name: "pure_boolean".into(),
            internal: true,
            parameters: vec![Parameter {
                name: "value".into(),
                ty: Type::Boolean,
            }],
            result: Type::Boolean,
            body: parameter("value"),
        },
    ];
    contract.stateful_circuits.push(StatefulCircuit {
        source: None,
        name: "stateful_field".into(),
        internal: true,
        parameters: vec![Parameter {
            name: "value".into(),
            ty: Type::Field,
        }],
        result: Type::Field,
        actions: vec![bind(
            "snapshot",
            Type::Field,
            read(),
            write(parameter("value")),
        )],
        return_value: StateReturn::Expression {
            value: Expr::Add {
                left: Box::new(parameter("snapshot")),
                right: Box::new(parameter("value")),
            },
        },
    });
}
fn wrap(value: Expr, mode: usize) -> StateAction {
    let action = match mode {
        0 => write(value),
        1 => sequence(vec![sequence(vec![]), write(value), sequence(vec![])]),
        2 => bind(
            "selected_value",
            Type::Field,
            value,
            write(parameter("selected_value")),
        ),
        3 => StateAction::If {
            condition: parameter("selected"),
            then: Box::new(write(value.clone())),
            otherwise: Box::new(write(value)),
        },
        4 => sequence(vec![
            StateAction::Assert {
                condition: Expr::Equal {
                    left: Box::new(parameter("seed")),
                    right: Box::new(parameter("seed")),
                },
                message: "ordered guard".into(),
            },
            write(value),
        ]),
        _ => unreachable!(),
    };
    // Each composition has its own read evidence, even when value is a literal.
    bind("prior", Type::Field, read(), action)
}

#[test]
fn bounded_old_values_and_actions_compose_with_checked_calls() {
    let mut count = 0;
    for value in field_values() {
        for mode in 0..5 {
            let mut contract = subject();
            helpers(&mut contract);
            contract.stateful_circuits[0].actions = vec![wrap(value.clone(), mode)];
            contract.stateful_circuits[0].return_value = StateReturn::Expression {
                value: parameter("prior"),
            };
            included(&contract, "root");
            count += 1;
        }
    }
    assert_eq!(
        count, 45,
        "bounded grammar witnesses, not exhaustive combinations"
    );
}

#[test]
fn every_old_boolean_value_rule_can_control_action_selection() {
    let conditions = vec![
        parameter("selected"),
        Expr::Boolean { value: true },
        Expr::Equal {
            left: Box::new(read()),
            right: Box::new(literal()),
        },
        Expr::Coerce {
            value: Box::new(parameter("selected")),
            ty: Type::Boolean,
        },
        Expr::If {
            condition: Box::new(parameter("selected")),
            then: Box::new(Expr::Boolean { value: true }),
            otherwise: Box::new(Expr::Boolean { value: false }),
        },
        Expr::Let {
            bindings: vec![LocalBinding {
                name: "flag".into(),
                ty: Type::Boolean,
                value: parameter("selected"),
            }],
            body: Box::new(parameter("flag")),
        },
        call("pure_boolean", vec![parameter("selected")]),
    ];
    assert_eq!(conditions.len(), 7);
    for condition in conditions {
        let mut contract = subject();
        helpers(&mut contract);
        contract.stateful_circuits[0].actions = vec![StateAction::If {
            condition,
            then: Box::new(write(read())),
            otherwise: Box::new(write(parameter("seed"))),
        }];
        included(&contract, "root");
    }
}

#[test]
fn terminal_shadowing_and_repeated_nested_calls_preserve_old_scopes() {
    let mut contract = subject();
    helpers(&mut contract);
    let nested = call(
        "stateful_field",
        vec![call(
            "stateful_field",
            vec![call("pure_field", vec![parameter("seed")])],
        )],
    );
    contract.stateful_circuits[0].actions = vec![bind(
        "seed",
        Type::Field,
        read(),
        sequence(vec![
            bind("seed", Type::Field, literal(), sequence(vec![])), // sibling does not escape
            bind(
                "seed",
                Type::Field,
                nested.clone(),
                sequence(vec![write(parameter("seed")), sequence(vec![])]),
            ),
        ]),
    )];
    contract.stateful_circuits[0].return_value = StateReturn::Expression {
        value: Expr::Add {
            left: Box::new(parameter("seed")),
            right: Box::new(nested),
        },
    };
    included(&contract, "root");
}

#[test]
fn witnessed_arguments_and_empty_selected_arm_remain_in_the_old_domain() {
    let mut contract = subject();
    helpers(&mut contract);
    contract.stateful_circuits[0].actions = vec![StateAction::If {
        condition: parameter("selected"),
        then: Box::new(sequence(vec![])),
        otherwise: Box::new(write(read())),
    }];
    let witness = Expr::WitnessCall {
        name: "next_value".into(),
        arguments: vec![parameter("seed")],
    };
    contract.stateful_circuits[0].return_value = StateReturn::Expression {
        value: call("pure_field", vec![witness]),
    };
    included(&contract, "root");
}
