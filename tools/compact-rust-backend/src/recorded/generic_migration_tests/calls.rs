// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use super::*;
fn pure(name: &str, parameters: Vec<Parameter>, result: Type, body: Expr) -> PureCircuit {
    PureCircuit {
        source: None,
        internal: true,
        name: name.into(),
        parameters,
        result,
        body,
    }
}

#[test]
fn complete_control_pure_and_stateful_helper_domains_use_real_discovery() {
    let mut c = cell_contract();
    c.circuits.push(pure(
        "sum",
        vec![parameter("x", Type::Field)],
        Type::Field,
        Expr::Add {
            left: Box::new(var("x")),
            right: Box::new(field(1)),
        },
    ));
    c.stateful_circuits[0].result = Type::Field;
    c.stateful_circuits[0].return_value = StateReturn::Expression {
        value: Expr::Call {
            name: "sum".into(),
            arguments: vec![Expr::CellRead {
                field: "cell".into(),
                index: 0,
            }],
        },
    };
    included(&c, "root", "pure Field return after action");
    c.stateful_circuits[0].actions.clear();
    c.stateful_circuits[0].return_value = StateReturn::Expression {
        value: Expr::If {
            condition: Box::new(Expr::Boolean { value: true }),
            then: Box::new(Expr::Call {
                name: "sum".into(),
                arguments: vec![field(1)],
            }),
            otherwise: Box::new(Expr::Call {
                name: "sum".into(),
                arguments: vec![field(2)],
            }),
        },
    };
    included(&c, "root", "both native pure return branches");
    c.stateful_circuits[0].return_value = StateReturn::Expression {
        value: Expr::Add {
            left: Box::new(Expr::CellRead {
                field: "cell".into(),
                index: 0,
            }),
            right: Box::new(field(1)),
        },
    };
    included(&c, "root", "read-bearing Field expression");
    c.stateful_circuits[0].result = Type::Boolean;
    c.stateful_circuits[0].return_value = StateReturn::Expression {
        value: Expr::Equal {
            left: Box::new(Expr::CellRead {
                field: "cell".into(),
                index: 0,
            }),
            right: Box::new(field(1)),
        },
    };
    included(&c, "root", "read-bearing Boolean expression");

    let mut c = cell_contract();
    let mut helper = c.stateful_circuits[0].clone();
    helper.name = "inner".into();
    helper.internal = true;
    c.witnesses.push(WitnessDeclaration {
        source: None,
        name: "notice".into(),
        parameters: vec![],
        result: Type::Field,
    });
    helper.actions.push(StateAction::Expression {
        value: Expr::WitnessCall {
            name: "notice".into(),
            arguments: vec![],
        },
    });
    c.stateful_circuits.push(helper);
    let mut outer = c.stateful_circuits[1].clone();
    outer.name = "outer".into();
    outer.actions = vec![StateAction::CircuitCall {
        name: "inner".into(),
        arguments: vec![],
    }];
    c.stateful_circuits.push(outer);
    c.stateful_circuits[0].actions = vec![StateAction::If {
        condition: Expr::Boolean { value: true },
        then: Box::new(StateAction::Sequence {
            actions: vec![
                StateAction::CircuitCall {
                    name: "outer".into(),
                    arguments: vec![],
                },
                assertion(),
            ],
        }),
        otherwise: Box::new(StateAction::CircuitCall {
            name: "inner".into(),
            arguments: vec![],
        }),
    }];
    let cx = Context::new(&c);
    let (names, items) = generic_reference::plan_recorded_helpers(
        &c.stateful_circuits,
        &cx.ledger,
        &cx.witnesses,
        &cx.pure,
        &cx.circuits,
    )
    .unwrap();
    assert!(names.contains("inner") && names.contains("outer") && items.len() == 2);
    included(
        &c,
        "root",
        "real transitive helper discovery and discarded Field witness",
    );
    let mut recursive = c.clone();
    recursive.stateful_circuits[1].actions = vec![StateAction::CircuitCall {
        name: "outer".into(),
        arguments: vec![],
    }];
    assert!(crate::render_with_capabilities(&recursive).is_err());

    // Generic pure binding alternatives, with real canonical declarations.
    let mut c = cell_contract();
    let bytes = Type::Bytes { length: 32 };
    let u = Type::Unsigned { max: "255".into() };
    let vector = Type::Vector {
        length: 2,
        element: Box::new(Type::Field),
    };
    let product = Type::Struct {
        name: "Product".into(),
        fields: vec![StructField {
            name: "value".into(),
            ty: Type::Field,
        }],
    };
    c.stateful_circuits[0].parameters = vec![parameter("product", product.clone())];
    c.circuits = vec![
        pure("uint", vec![parameter("x", u.clone())], u.clone(), var("x")),
        pure(
            "bytes",
            vec![],
            bytes.clone(),
            Expr::BytesLiteral { bytes: vec![0; 32] },
        ),
        pure(
            "product",
            vec![parameter("x", product.clone())],
            product.clone(),
            var("x"),
        ),
        pure(
            "vector",
            vec![],
            vector.clone(),
            Expr::Vector {
                element: Type::Field,
                elements: vec![field(1), field(2)],
            },
        ),
        pure(
            "flag",
            vec![parameter("x", Type::Boolean)],
            Type::Boolean,
            Expr::Sequence {
                steps: vec![Expr::Assert {
                    condition: Box::new(var("x")),
                    message: "flag".into(),
                }],
                value: Box::new(var("x")),
            },
        ),
        pure(
            "guard",
            vec![parameter("x", Type::Field)],
            Type::Unit,
            Expr::Sequence {
                steps: vec![Expr::Assert {
                    condition: Box::new(Expr::NotEqual {
                        left: Box::new(var("x")),
                        right: Box::new(field(0)),
                    }),
                    message: "nonzero".into(),
                }],
                value: Box::new(Expr::Unit),
            },
        ),
    ];
    c.stateful_circuits[0].actions = vec![StateAction::Let {
        bindings: vec![
            binding(
                "u",
                u,
                Expr::Call {
                    name: "uint".into(),
                    arguments: vec![Expr::UnsignedLiteral {
                        value: "1".into(),
                        max: "255".into(),
                    }],
                },
            ),
            binding(
                "b",
                bytes,
                Expr::Call {
                    name: "bytes".into(),
                    arguments: vec![],
                },
            ),
            binding(
                "p",
                product,
                Expr::Call {
                    name: "product".into(),
                    arguments: vec![var("product")],
                },
            ),
            binding(
                "v",
                vector,
                Expr::Call {
                    name: "vector".into(),
                    arguments: vec![],
                },
            ),
            binding(
                "f",
                Type::Boolean,
                Expr::Call {
                    name: "flag".into(),
                    arguments: vec![Expr::Boolean { value: true }],
                },
            ),
        ],
        action: Box::new(StateAction::Sequence {
            actions: vec![
                StateAction::PureCall {
                    name: "guard".into(),
                    arguments: vec![field(1)],
                },
                write(),
            ],
        }),
    }];
    included(&c, "root", "generic pure binding alternatives");
}
