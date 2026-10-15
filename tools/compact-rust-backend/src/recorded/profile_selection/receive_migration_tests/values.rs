// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use super::*;
fn default(ty: Type) -> Expr {
    Expr::Default { ty }
}
fn equal(value: Expr) -> Expr {
    Expr::Equal {
        left: Box::new(value.clone()),
        right: Box::new(value),
    }
}
#[test]
fn finite_old_value_grammar_is_included_without_audit_only_filtering() {
    let enumeration = Type::Enum {
        name: "ReceiveMode".into(),
        variants: vec!["Initial".into(), "Ready".into()],
    };
    let empty = Type::Struct {
        name: "ReceiveEmpty".into(),
        fields: vec![],
    };
    let product = structure("ReceiveField", Type::Field);
    let owned = structure("ReceiveOwned", Type::OpaqueString);
    let point = structure("ReceivePoint", Type::JubjubPoint);
    let vector = Type::Vector {
        element: Box::new(Type::Field),
        length: 2,
    };
    let pair = Expr::Tuple {
        elements: vec![field(3), field(4)],
    };
    let vector_value = Expr::Coerce {
        value: Box::new(pair.clone()),
        ty: vector.clone(),
    };
    let cases = vec![
        (
            "coin reference",
            crate::coin_shapes::shielded_coin_type(),
            parameter("coin"),
        ),
        ("bool", Type::Boolean, Expr::Boolean { value: true }),
        ("bytes", Type::Bytes { length: 32 }, bytes()),
        ("field", Type::Field, field(1)),
        (
            "unsigned",
            Type::Unsigned {
                max: "65535".into(),
            },
            number(1, "65535"),
        ),
        (
            "enum",
            enumeration.clone(),
            Expr::EnumVariant {
                ty: enumeration,
                variant: "Ready".into(),
            },
        ),
        ("default product", product.clone(), default(product.clone())),
        ("default empty", empty.clone(), default(empty.clone())),
        (
            "construct empty",
            empty.clone(),
            Expr::StructLiteral {
                ty: empty,
                fields: vec![],
            },
        ),
        (
            "construct product",
            product.clone(),
            Expr::StructLiteral {
                ty: product.clone(),
                fields: vec![field(8)],
            },
        ),
        (
            "tuple",
            Type::Tuple {
                elements: vec![Type::Field, Type::Field],
            },
            pair,
        ),
        (
            "empty tuple",
            Type::Tuple { elements: vec![] },
            Expr::Tuple { elements: vec![] },
        ),
        (
            "project",
            Type::Field,
            Expr::StructField {
                value: Box::new(default(product)),
                field: "value".into(),
                index: 0,
            },
        ),
        (
            "scalar conversion",
            Type::Field,
            Expr::Coerce {
                value: Box::new(number(2, "255")),
                ty: Type::Field,
            },
        ),
        (
            "identity conversion",
            Type::Field,
            Expr::Coerce {
                value: Box::new(field(2)),
                ty: Type::Field,
            },
        ),
        ("tuple vector", vector.clone(), vector_value.clone()),
        (
            "vector identity",
            vector.clone(),
            Expr::Coerce {
                value: Box::new(vector_value),
                ty: vector,
            },
        ),
        ("scalar equality", Type::Boolean, equal(field(3))),
        (
            "owned equality",
            Type::Boolean,
            equal(default(owned.clone())),
        ),
        (
            "point product equality",
            Type::Boolean,
            equal(default(point.clone())),
        ),
        (
            "point projection equality",
            Type::Boolean,
            equal(Expr::StructField {
                value: Box::new(default(point)),
                field: "value".into(),
                index: 0,
            }),
        ),
        (
            "lexical shadow and restore",
            Type::Field,
            local(
                "shadow",
                Type::Field,
                field(3),
                local(
                    "unused",
                    Type::Boolean,
                    local(
                        "shadow",
                        Type::Boolean,
                        Expr::Boolean { value: false },
                        parameter("shadow"),
                    ),
                    parameter("shadow"),
                ),
            ),
        ),
        (
            "owned projection and whole reuse",
            Type::Boolean,
            local(
                "owned",
                owned.clone(),
                default(owned.clone()),
                local(
                    "text",
                    Type::OpaqueString,
                    Expr::StructField {
                        value: Box::new(parameter("owned")),
                        field: "value".into(),
                        index: 0,
                    },
                    equal(parameter("owned")),
                ),
            ),
        ),
    ];
    assert_eq!(cases.len(), 23);
    for (label, ty, value) in cases {
        let mut c = source();
        prepend(&mut c, ty, value);
        included(&c, "accept", label);
    }
    for selected in [false, true] {
        let mut c = source();
        prepend(
            &mut c,
            Type::Boolean,
            Expr::If {
                condition: Box::new(Expr::Boolean { value: selected }),
                then: Box::new(Expr::Boolean { value: false }),
                otherwise: Box::new(Expr::If {
                    condition: Box::new(Expr::Boolean { value: !selected }),
                    then: Box::new(Expr::Boolean { value: true }),
                    otherwise: Box::new(Expr::Boolean { value: false }),
                }),
            },
        );
        included(&c, "accept", "Boolean nested branch");
    }
}
#[test]
fn transitive_pure_graph_and_owned_interface_use_complete_old_evaluation() {
    let mut c = source();
    let original = c
        .circuits
        .iter()
        .find(|p| p.name == "coinCommitment")
        .unwrap()
        .clone();
    let mut renamed = original.clone();
    renamed.name = "renamedCommitment".into();
    c.circuits
        .iter_mut()
        .find(|p| p.name == "coinCommitment")
        .unwrap()
        .body = Expr::Call {
        name: renamed.name.clone(),
        arguments: original
            .parameters
            .iter()
            .map(|p| parameter(&p.name))
            .collect(),
    };
    c.circuits.push(renamed);
    c.circuits.push(PureCircuit {
        source: None,
        name: "emptyText".into(),
        internal: true,
        parameters: vec![],
        result: Type::OpaqueString,
        body: default(Type::OpaqueString),
    });
    prepend(
        &mut c,
        Type::OpaqueString,
        Expr::Call {
            name: "emptyText".into(),
            arguments: vec![],
        },
    );
    included(
        &c,
        "accept",
        "transitive persistent hash and opaque pure interface",
    );
}
