// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

use super::*;

#[test]
fn raw_lexical_keys_prior_initializers_and_outer_scope_restore() {
    let outer_ty = Type::Boolean;
    let bindings = HashMap::from([("r#x", (&outer_ty, ()))]);
    let source = Expr::Let {
        bindings: vec![
            LocalBinding {
                name: "r#x".into(),
                ty: Type::Boolean,
                value: reference("r#x"),
            },
            LocalBinding {
                name: "r#x".into(),
                ty: Type::Field,
                value: field("1"),
            },
        ],
        body: Box::new(local(
            "inner$0",
            Type::Field,
            reference("r#x"),
            reference("inner$0"),
        )),
    };
    let mut consumer = Consumer::default();
    assert_eq!(
        check(&source, &bindings, &HashMap::new(), &mut consumer).unwrap(),
        Type::Field
    );
    assert_eq!(consumer.locals, ["r#x", "r#x", "inner$0"]);
    assert_eq!(
        check(
            &reference("r#x"),
            &bindings,
            &HashMap::new(),
            &mut Consumer::default()
        )
        .unwrap(),
        Type::Boolean
    );
    assert!(error(&reference("inner$0"), &bindings).contains("UnknownParameter"));
    let failing = local("r#x", Type::Field, field("1"), reference("missing"));
    assert!(error(&failing, &bindings).contains("UnknownParameter"));
    assert_eq!(bindings["r#x"].0, &Type::Boolean);
}

#[test]
fn complete_unit_consumer_reuses_all_families_with_map_call_and_fold() {
    let field_sum = Expr::Add {
        left: Box::new(field("1")),
        right: Box::new(field("2")),
    };
    let converted = Expr::Coerce {
        value: Box::new(field_sum),
        ty: Type::Field,
    };
    let hashed = Expr::TransientHash {
        value: Box::new(converted),
    };
    let map = Expr::VectorMap {
        parameter: Parameter {
            name: "element".into(),
            ty: Type::Field,
        },
        source: Box::new(Expr::Vector {
            element: Type::Field,
            elements: vec![hashed],
        }),
        body: Box::new(local(
            "local",
            Type::Field,
            reference("element"),
            branch(boolean(true), reference("local"), field("3")),
        )),
        result: Type::Field,
        length: 1,
    };
    let function = PureCircuit {
        source: None,
        internal: false,
        name: "combine".into(),
        parameters: vec![
            Parameter {
                name: "acc".into(),
                ty: Type::Field,
            },
            Parameter {
                name: "element".into(),
                ty: Type::Field,
            },
        ],
        result: Type::Field,
        // Deliberately not a pure body: signature lookup does not inspect it.
        body: Expr::KernelSelf { ty: Type::Field },
    };
    let declarations = HashMap::from([("combine", &function)]);
    let source = Expr::Sequence {
        steps: vec![assertion(boolean(true))],
        value: Box::new(Expr::Tuple {
            elements: vec![
                Expr::VectorFoldCall {
                    name: "combine".into(),
                    initial: Box::new(field("0")),
                    source: Box::new(map),
                    accumulator: Type::Field,
                    element: Type::Field,
                    length: 1,
                },
                Expr::Call {
                    name: "combine".into(),
                    arguments: vec![field("4"), field("5")],
                },
            ],
        }),
    };
    let mut consumer = Consumer::default();
    assert_eq!(
        check(&source, &HashMap::new(), &declarations, &mut consumer).unwrap(),
        Type::Tuple {
            elements: vec![Type::Field, Type::Field]
        }
    );
    for family in [
        "leaf",
        "conversion",
        "arithmetic",
        "crypto",
        "aggregate",
        "control",
    ] {
        assert!(consumer.outputs.contains(&family));
    }
    assert!(
        consumer.visits.iter().all(|key| consumer
            .visits
            .iter()
            .filter(|other| *other == key)
            .count()
            == 1)
    );
    assert!(
        error(&Expr::KernelSelf { ty: Type::Field }, &HashMap::new())
            .contains("EffectfulExpression")
    );
}
