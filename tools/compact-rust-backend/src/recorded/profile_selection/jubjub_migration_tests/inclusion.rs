// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use super::*;

fn scalar_values() -> Vec<Expr> {
    let reduced = Expr::JubjubScalarFromNative {
        value: Box::new(parameter("scalar")),
    };
    vec![
        parameter("scalar"),
        literal("0"),
        literal("1"),
        reduced.clone(),
        Expr::JubjubScalarFromNative {
            value: Box::new(reduced),
        },
        Expr::Let {
            bindings: vec![LocalBinding {
                name: "scalar".into(),
                ty: Type::Field,
                value: literal("2"),
            }],
            body: Box::new(parameter("scalar")),
        },
    ]
}
fn point_values() -> Vec<Expr> {
    let mut points = vec![parameter("point")];
    for scalar in scalar_values() {
        let generator = Expr::EcMulGenerator {
            scalar: Box::new(scalar.clone()),
        };
        points.push(generator.clone());
        points.push(Expr::EcMul {
            point: Box::new(parameter("point")),
            scalar: Box::new(scalar),
        });
        points.push(Expr::EcAdd {
            left: Box::new(parameter("point")),
            right: Box::new(generator),
        });
    }
    points.push(Expr::Let {
        bindings: vec![LocalBinding {
            name: "point".into(),
            ty: Type::JubjubPoint,
            value: Expr::EcMulGenerator {
                scalar: Box::new(literal("1")),
            },
        }],
        body: Box::new(Expr::EcAdd {
            left: Box::new(parameter("point")),
            right: Box::new(parameter("point")),
        }),
    });
    points
}
fn wrap(point: Expr, mode: usize) -> (StateAction, Expr) {
    match mode {
        0 => (write(point.clone()), point),
        1 => (
            sequence(vec![
                sequence(vec![]),
                write(point.clone()),
                sequence(vec![]),
            ]),
            point,
        ),
        2 => (
            bind(
                "selected",
                Type::JubjubPoint,
                point,
                write(parameter("selected")),
            ),
            parameter("selected"),
        ),
        3 => (
            bind(
                "point",
                Type::JubjubPoint,
                point.clone(),
                sequence(vec![
                    bind("point", Type::JubjubPoint, point.clone(), sequence(vec![])),
                    bind(
                        "selected",
                        Type::JubjubPoint,
                        point,
                        write(parameter("selected")),
                    ),
                ]),
            ),
            parameter("selected"),
        ),
        _ => unreachable!(),
    }
}

#[test]
fn bounded_compositions_cover_each_old_value_and_action_rule() {
    let mut count = 0;
    for point in point_values() {
        for mode in 0..4 {
            let mut contract = subject();
            let circuit = &mut contract.stateful_circuits[0];
            circuit.name = format!("finite_case_{count}");
            let (action, value) = wrap(point.clone(), mode);
            circuit.actions = vec![action];
            circuit.return_value = StateReturn::Expression { value };
            included(&contract, &contract.stateful_circuits[0]);
            count += 1;
        }
    }
    assert_eq!(
        count, 80,
        "finite grammar witnesses; not an exhaustive proof"
    );
}

#[test]
fn sequential_bindings_and_final_scope_are_not_shape_or_name_dependent() {
    let mut contract = subject();
    let circuit = &mut contract.stateful_circuits[0];
    circuit.parameters.push(Parameter {
        name: "unused".into(),
        ty: Type::Field,
    });
    circuit.actions = vec![StateAction::Let {
        bindings: vec![
            LocalBinding {
                name: "scalar".into(),
                ty: Type::Field,
                value: Expr::JubjubScalarFromNative {
                    value: Box::new(parameter("scalar")),
                },
            },
            LocalBinding {
                name: "point".into(),
                ty: Type::JubjubPoint,
                value: Expr::EcMulGenerator {
                    scalar: Box::new(parameter("scalar")),
                },
            },
        ],
        action: Box::new(sequence(vec![write(parameter("point")), sequence(vec![])])),
    }];
    circuit.return_value = StateReturn::Expression {
        value: parameter("point"),
    };
    contract.ledger_fields.push(LedgerField {
        source: None,
        id: "unused_counter".into(),
        index: 1,
        path: vec![1],
        declaration: LedgerFieldKind::Counter,
    });
    included(&contract, &contract.stateful_circuits[0]);
}

#[test]
fn earlier_action_and_expression_locals_do_not_escape() {
    let local = bind(
        "hidden",
        Type::JubjubPoint,
        parameter("point"),
        sequence(vec![]),
    );
    for actions in [
        vec![local.clone(), write(parameter("point"))],
        vec![sequence(vec![local, write(parameter("point"))])],
        vec![write(Expr::Let {
            bindings: vec![LocalBinding {
                name: "hidden".into(),
                ty: Type::JubjubPoint,
                value: parameter("point"),
            }],
            body: Box::new(parameter("hidden")),
        })],
    ] {
        let mut contract = subject();
        let circuit = &mut contract.stateful_circuits[0];
        circuit.actions = actions;
        circuit.return_value = StateReturn::Expression {
            value: parameter("hidden"),
        };
        let context = Context::new(&contract);
        let circuit = &contract.stateful_circuits[0];
        assert!(context.old(circuit).is_none());
        let Err(effect_plan::PlanError::Unsupported(gap)) = context.algebra(circuit) else {
            panic!("escaped local must refuse")
        };
        assert_eq!(gap.path, "return_value.value");
    }
}

#[test]
fn declaration_identity_and_annotation_errors_cannot_appear_as_inclusion() {
    for case in ["index", "type", "duplicate", "unknown", "annotation"] {
        let mut contract = subject();
        match case {
            "index" => contract.ledger_fields[0].index = 1,
            "type" => {
                contract.ledger_fields[0].declaration = LedgerFieldKind::Cell { ty: Type::Field }
            }
            "duplicate" => contract.stateful_circuits[0].parameters[1].name = "point".into(),
            "unknown" => {
                contract.stateful_circuits[0].return_value = StateReturn::Expression {
                    value: parameter("missing"),
                }
            }
            "annotation" => {
                contract.stateful_circuits[0].actions = vec![bind(
                    "wrong",
                    Type::JubjubPoint,
                    literal("1"),
                    write(parameter("point")),
                )]
            }
            _ => unreachable!(),
        }
        let context = Context::new(&contract);
        let circuit = &contract.stateful_circuits[0];
        assert!(context.old(circuit).is_none(), "{case}");
        assert!(
            matches!(
                context.algebra(circuit),
                Err(effect_plan::PlanError::Unsupported(_))
            ),
            "{case}"
        );
    }
}

#[test]
fn declaration_backed_full_path_is_a_separate_new_domain_candidate() {
    let mut contract = subject();
    let paths = crate::expected_ledger_paths(16);
    contract.ledger_fields[0].path = paths[0].clone();
    for (position, path) in paths.into_iter().enumerate().skip(1) {
        contract.ledger_fields.push(LedgerField {
            source: None,
            id: format!("unrelated_{position}"),
            index: path[0],
            path,
            declaration: LedgerFieldKind::Cell { ty: Type::Field },
        });
    }
    let context = Context::new(&contract);
    let circuit = &contract.stateful_circuits[0];
    assert!(
        context.old(circuit).is_none(),
        "old family requires a flat path"
    );
    let algebra = context
        .algebra(circuit)
        .expect("EA01 retains complete declaration paths");
    assert_eq!(
        selected_tokens(context.selected(circuit).unwrap()),
        Some(tokens(&algebra))
    );
    let rendered =
        crate::render_with_capabilities(&contract).expect("public native and recorded pipeline");
    assert!(rendered.capabilities.circuits[0].recorded);
    // IR compilation evidence; runtime nested-path behavior is a separate gate.
}
