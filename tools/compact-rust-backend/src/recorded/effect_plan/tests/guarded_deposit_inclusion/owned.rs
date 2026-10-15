// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use super::*;
#[test]
fn whole_deposit_repeated_owned_vector_calls_isolate_shadowed_formals() {
    let mut c = source(true);
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
    included(&c, "set_topic", "merge-owned-vector-shadowing-and-reuse");
}
