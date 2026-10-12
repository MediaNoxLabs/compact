// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Construct-level laws, separate from retained example admission and runtime tests.
use super::*;
#[path = "vector_map/resources.rs"]
mod resources;
#[path = "vector_map/scopes.rs"]
mod scopes;
#[path = "vector_map/validation.rs"]
mod validation;

fn vector(element: Type, length: usize) -> Type {
    Type::Vector {
        element: Box::new(element),
        length,
    }
}
fn parameter(name: &str) -> Expr {
    Expr::Parameter { name: name.into() }
}
fn mapped(source: Expr, body: Expr, result: Type, length: usize) -> Expr {
    Expr::VectorMap {
        parameter: Parameter {
            name: "item".into(),
            ty: Type::Field,
        },
        source: Box::new(source),
        body: Box::new(body),
        result,
        length,
    }
}
fn circuit(length: usize, body: Expr) -> StatefulCircuit {
    StatefulCircuit {
        source: None,
        name: "mapped".into(),
        internal: false,
        parameters: vec![
            Parameter {
                name: "input".into(),
                ty: vector(Type::Field, length),
            },
            Parameter {
                name: "capture".into(),
                ty: Type::Field,
            },
        ],
        actions: vec![],
        result: vector(Type::Field, length),
        return_value: StateReturn::Expression {
            value: mapped(parameter("input"), body, Type::Field, length),
        },
    }
}
fn checked(c: &StatefulCircuit) -> CheckedPlan {
    check(c, &HashMap::new(), &HashMap::new()).unwrap()
}
fn contract(c: StatefulCircuit) -> Contract {
    Contract {
        schema_version: SCHEMA_VERSION,
        type_aliases: vec![],
        ledger_fields: vec![],
        constructor: None,
        witnesses: vec![],
        circuits: vec![],
        stateful_circuits: vec![c],
    }
}
fn addition() -> Expr {
    Expr::Add {
        left: Box::new(parameter("item")),
        right: Box::new(parameter("capture")),
    }
}
fn map_binding(plan: &mut Plan) -> &mut Binding {
    plan.body
        .bindings
        .iter_mut()
        .find(|b| matches!(b.operation, Operation::VectorMap { .. }))
        .unwrap()
}

#[test]
fn nonidentity_mapping_captures_and_tuple_coercion_render_one_loop() {
    for length in [0, 1, 4] {
        let mut c = circuit(length, addition());
        if let StateReturn::Expression {
            value: Expr::VectorMap { source, .. },
        } = &mut c.return_value
        {
            **source = if length == 0 {
                Expr::Vector {
                    elements: vec![],
                    element: Type::Field,
                }
            } else {
                Expr::Tuple {
                    elements: (0..length).map(|_| parameter("capture")).collect(),
                }
            };
        }
        let p = checked(&c);
        assert!(
            p.plan()
                .body
                .bindings
                .iter()
                .any(|b| matches!(b.operation, Operation::VectorMap { .. }))
        );
        let syntax = render::render(&p).unwrap();
        let steps = syntax.steps;
        let text = quote::quote!(#(#steps)*).to_string();
        assert_eq!(text.matches("for ").count(), 1);
        assert!(text.contains("into_array"));
        assert!(text.contains("Vector map preserves its length"));
        crate::render_with_capabilities(&contract(c)).unwrap();
    }
}

#[test]
fn empty_mapping_still_checks_body_result_capture_and_source_cardinality() {
    for mutation in ["capture", "result", "length", "formal", "empty_tuple"] {
        let mut c = circuit(0, addition());
        let StateReturn::Expression {
            value:
                Expr::VectorMap {
                    body,
                    result,
                    length,
                    parameter,
                    source,
                },
        } = &mut c.return_value
        else {
            unreachable!()
        };
        match mutation {
            "capture" => **body = super::vector_map::parameter("missing"),
            "result" => *result = Type::Boolean,
            "length" => *length = 1,
            "formal" => parameter.ty = Type::Boolean,
            "empty_tuple" => **source = Expr::Tuple { elements: vec![] },
            _ => unreachable!(),
        }
        assert!(
            check(&c, &HashMap::new(), &HashMap::new()).is_err(),
            "{mutation}"
        );
        assert!(
            crate::render_with_capabilities(&contract(c)).is_err(),
            "{mutation}"
        );
    }
}
