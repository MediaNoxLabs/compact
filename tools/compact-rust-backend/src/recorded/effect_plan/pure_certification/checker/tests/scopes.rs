// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

use super::*;

#[test]
fn raw_formals_sequential_let_and_nested_map_restore_distinct_raw_keys() {
    let map = Expr::VectorMap {
        parameter: parameter("r#value", Type::Field),
        source: Box::new(Expr::Vector {
            element: Type::Field,
            elements: vec![field()],
        }),
        body: Box::new(local(
            "x$0",
            Type::Field,
            reference("r#value"),
            reference("x$0"),
        )),
        result: Type::Field,
        length: 1,
    };
    let shadow = Expr::Let {
        bindings: vec![
            LocalBinding {
                name: "x$0".into(),
                ty: Type::Boolean,
                value: reference("x$0"),
            },
            LocalBinding {
                name: "x$0".into(),
                ty: Type::Field,
                value: reference("x_0"),
            },
        ],
        body: Box::new(Expr::Tuple {
            elements: vec![reference("x$0"), map],
        }),
    };
    let body = Expr::Tuple {
        elements: vec![
            shadow,
            reference("x$0"),
            reference("r#value"),
            reference("value"),
        ],
    };
    let result = Type::Tuple {
        elements: vec![
            Type::Tuple {
                elements: vec![
                    Type::Field,
                    Type::Vector {
                        element: Box::new(Type::Field),
                        length: 1,
                    },
                ],
            },
            Type::Boolean,
            Type::Boolean,
            Type::Field,
        ],
    };
    let circuit = source(
        vec![
            parameter("x$0", Type::Boolean),
            parameter("x_0", Type::Field),
            parameter("r#value", Type::Boolean),
            parameter("value", Type::Field),
        ],
        result,
        body,
    );
    compare(&circuit, &HashMap::new()).unwrap();
    refusal(
        &source(
            vec![],
            Type::Field,
            local("new$0", Type::Field, reference("new$0"), field()),
        ),
        RenderError::UnknownParameter("new$0".into()),
    );
}

#[test]
fn fresh_callee_formals_reject_caller_capture_and_signature_lookup_stays_separate() {
    let mut callee = source(vec![], Type::Field, reference("caller$only"));
    callee.name = "callee".into();
    let declarations = HashMap::from([("callee", &callee)]);
    assert_eq!(
        compare(&callee, &declarations),
        Err(RenderError::UnknownParameter("caller$only".into()))
    );
    // This single-body checker checks a Call signature; the Certifier separately
    // checks reachable callee bodies in fresh formal scopes, with no caller capture.
    let caller = source(
        vec![],
        Type::Field,
        local(
            "caller$only",
            Type::Field,
            field(),
            Expr::Call {
                name: "callee".into(),
                arguments: vec![],
            },
        ),
    );
    compare(&caller, &declarations).unwrap();
}
