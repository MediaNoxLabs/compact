// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use super::*;
fn prepend(c: &mut Contract, ty: Type, value: Expr) {
    let body = returned_mut(c, "mergeCoin");
    *body = Expr::Let {
        bindings: vec![LocalBinding {
            name: "owned_probe".into(),
            ty,
            value,
        }],
        body: Box::new(body.clone()),
    };
}
#[test]
fn whole_merge_unused_owned_identity_calls_are_checked_in_fresh_scope() {
    for (label, ty, value) in [
        (
            "string",
            Type::OpaqueString,
            Expr::StructField {
                value: Box::new(Expr::Default {
                    ty: Type::Struct {
                        name: "Carrier".into(),
                        fields: vec![StructField {
                            name: "text".into(),
                            ty: Type::OpaqueString,
                        }],
                    },
                }),
                field: "text".into(),
                index: 0,
            },
        ),
        (
            "tuple13",
            Type::Tuple {
                elements: vec![Type::Field; 13],
            },
            Expr::Tuple {
                elements: (0..13)
                    .map(|i| Expr::FieldLiteral {
                        value: i.to_string(),
                    })
                    .collect(),
            },
        ),
    ] {
        let mut c = source();
        c.stateful_circuits.push(StatefulCircuit {
            source: None,
            name: "owned_identity".into(),
            internal: true,
            parameters: vec![Parameter {
                name: "a".into(),
                ty: ty.clone(),
            }],
            result: ty.clone(),
            actions: vec![],
            return_value: StateReturn::Expression {
                value: parameter("a"),
            },
        });
        prepend(
            &mut c,
            ty,
            Expr::Call {
                name: "owned_identity".into(),
                arguments: vec![value],
            },
        );
        both(&c, &format!("owned-{label}"));
    }
}
#[test]
fn whole_merge_pure_string_default_crosses_only_the_certified_pure_boundary() {
    let mut c = source();
    c.circuits.push(PureCircuit {
        source: None,
        name: "empty_owned".into(),
        internal: true,
        parameters: vec![],
        result: Type::OpaqueString,
        body: Expr::Default {
            ty: Type::OpaqueString,
        },
    });
    prepend(
        &mut c,
        Type::OpaqueString,
        Expr::Call {
            name: "empty_owned".into(),
            arguments: vec![],
        },
    );
    both(&c, "pure-string-default");
}

#[test]
fn whole_merge_repeated_owned_vector_calls_isolate_shadowed_formals() {
    let mut c = source();
    let ty = Type::Vector {
        element: Box::new(Type::OpaqueString),
        length: 2,
    };
    c.circuits.push(PureCircuit {
        source: None,
        name: "empty_vector_text".into(),
        internal: true,
        parameters: vec![],
        result: Type::OpaqueString,
        body: Expr::Default {
            ty: Type::OpaqueString,
        },
    });
    c.stateful_circuits.push(StatefulCircuit {
        source: None,
        name: "local_vector".into(),
        internal: true,
        parameters: vec![
            Parameter {
                name: "a".into(),
                ty: ty.clone(),
            },
            Parameter {
                name: "b".into(),
                ty: Type::Field,
            },
        ],
        result: ty.clone(),
        actions: vec![],
        return_value: StateReturn::Expression {
            value: Expr::Let {
                bindings: vec![LocalBinding {
                    name: "b".into(),
                    ty: Type::Field,
                    value: parameter("b"),
                }],
                body: Box::new(parameter("a")),
            },
        },
    });
    let initial = Expr::Coerce {
        value: Box::new(Expr::Tuple {
            elements: (0..2)
                .map(|_| Expr::Call {
                    name: "empty_vector_text".into(),
                    arguments: vec![],
                })
                .collect(),
        }),
        ty: ty.clone(),
    };
    let local = |name: &str, value: Expr| LocalBinding {
        name: name.into(),
        ty: ty.clone(),
        value,
    };
    let identity = |value: Expr, n: u64| Expr::Call {
        name: "local_vector".into(),
        arguments: vec![
            value,
            Expr::FieldLiteral {
                value: n.to_string(),
            },
        ],
    };
    let coin_ty = circuit_mut(&mut c, "mergeCoin").parameters[0].ty.clone();
    let body = returned_mut(&mut c, "mergeCoin");
    *body = Expr::Let {
        bindings: vec![
            LocalBinding {
                name: "saved_a".into(),
                ty: coin_ty.clone(),
                value: parameter("a"),
            },
            LocalBinding {
                name: "a".into(),
                ty: coin_ty,
                value: parameter("saved_a"),
            },
            local("first_vector", identity(initial, 11)),
            local("second_vector", identity(parameter("first_vector"), 13)),
            // Reuse the owned first argument after a prior call consumed its value.
            local("third_vector", identity(parameter("first_vector"), 17)),
        ],
        body: Box::new(body.clone()),
    };
    both(&c, "owned-vector-shadowing-and-reuse");
}
