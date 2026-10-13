// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

use super::*;

#[test]
fn complete_body_composes_every_family_controls_map_call_and_fold() {
    let mut reducer = source(
        vec![parameter("a", Type::Field), parameter("b", Type::Field)],
        Type::Field,
        Expr::Add {
            left: Box::new(reference("a")),
            right: Box::new(reference("b")),
        },
    );
    reducer.name = "combine".into();
    let declarations = HashMap::from([("combine", &reducer)]);
    let converted = Expr::Coerce {
        value: Box::new(Expr::Add {
            left: Box::new(field()),
            right: Box::new(field()),
        }),
        ty: Type::Field,
    };
    let map = Expr::VectorMap {
        parameter: parameter("element$raw", Type::Field),
        source: Box::new(Expr::Vector {
            element: Type::Field,
            elements: vec![Expr::TransientHash {
                value: Box::new(converted),
            }],
        }),
        body: Box::new(local(
            "r#local",
            Type::Field,
            reference("element$raw"),
            branch(boolean(), reference("r#local"), field()),
        )),
        result: Type::Field,
        length: 1,
    };
    let body = Expr::Sequence {
        steps: vec![Expr::Assert {
            condition: Box::new(boolean()),
            message: "checked".into(),
        }],
        value: Box::new(Expr::Tuple {
            elements: vec![
                Expr::VectorFoldCall {
                    name: "combine".into(),
                    initial: Box::new(field()),
                    source: Box::new(map),
                    accumulator: Type::Field,
                    element: Type::Field,
                    length: 1,
                },
                Expr::Call {
                    name: "combine".into(),
                    arguments: vec![field(), field()],
                },
            ],
        }),
    };
    compare(
        &source(
            vec![],
            Type::Tuple {
                elements: vec![Type::Field, Type::Field],
            },
            body,
        ),
        &declarations,
    )
    .unwrap();
    compare(&reducer, &declarations).unwrap();
}

#[test]
fn dead_unused_equal_and_zero_length_children_are_still_checked() {
    let cases = [
        local("unused", Type::Field, effect(), field()),
        branch(boolean(), field(), effect()),
        branch(boolean(), effect(), effect()),
        Expr::VectorMap {
            parameter: parameter("item$raw", Type::Field),
            source: Box::new(Expr::Vector {
                element: Type::Field,
                elements: vec![],
            }),
            body: Box::new(effect()),
            result: Type::Field,
            length: 0,
        },
    ];
    for body in cases {
        refusal(
            &source(vec![], Type::Field, body),
            RenderError::EffectfulExpression,
        );
    }
    let empty = Expr::VectorMap {
        parameter: parameter("item$raw", Type::Field),
        source: Box::new(Expr::Vector {
            element: Type::Field,
            elements: vec![],
        }),
        body: Box::new(reference("missing")),
        result: Type::Field,
        length: 0,
    };
    refusal(
        &source(vec![], Type::Field, empty),
        RenderError::UnknownParameter("missing".into()),
    );
}

#[test]
fn unit_parameter_projection_call_and_coercion_have_the_same_body_type() {
    let mut unit = source(vec![], Type::Unit, Expr::Unit);
    unit.name = "unit_call".into();
    let declarations = HashMap::from([("unit_call", &unit)]);
    let projection = Expr::TupleIndex {
        value: Box::new(Expr::Tuple {
            elements: vec![Expr::Unit],
        }),
        index: 0,
    };
    let body = Expr::Sequence {
        steps: vec![
            reference("u$0"),
            projection,
            Expr::Call {
                name: "unit_call".into(),
                arguments: vec![],
            },
            Expr::Coerce {
                value: Box::new(reference("u$0")),
                ty: Type::Unit,
            },
        ],
        value: Box::new(Expr::Default { ty: Type::Unit }),
    };
    compare(
        &source(vec![parameter("u$0", Type::Unit)], Type::Unit, body),
        &declarations,
    )
    .unwrap();
}
