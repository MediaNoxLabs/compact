// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Certified pure bridges are distinct from the old direct merge visitor.
use super::*;
fn pure(name: &str, parameters: Vec<Parameter>, result: Type, body: Expr) -> PureCircuit {
    PureCircuit {
        source: None,
        name: name.into(),
        internal: true,
        parameters,
        result,
        body,
    }
}
fn field_literal(value: usize) -> Expr {
    Expr::FieldLiteral {
        value: value.to_string(),
    }
}
fn call(name: &str, arguments: Vec<Expr>) -> Expr {
    Expr::Call {
        name: name.into(),
        arguments,
    }
}
fn local(name: &str, ty: Type, value: Expr) -> LocalBinding {
    LocalBinding {
        name: name.into(),
        ty,
        value,
    }
}
fn prepend(c: &mut Contract, binding: LocalBinding) {
    let body = returned_mut(c, "mergeCoin");
    *body = Expr::Let {
        bindings: vec![binding],
        body: Box::new(body.clone()),
    };
}
#[test]
fn whole_deposit_certified_nested_pure_crypto_preserves_merge_effect_counts() {
    let mut c = source(true);
    c.circuits.push(pure(
        "nested_commit",
        vec![],
        Type::Field,
        Expr::TransientCommit {
            value: Box::new(Expr::Tuple {
                elements: vec![field_literal(3), field_literal(7)],
            }),
            opening: Box::new(field_literal(11)),
        },
    ));
    c.circuits.push(pure(
        "crypto_bridge",
        vec![],
        Type::Field,
        Expr::TransientHash {
            value: Box::new(Expr::Tuple {
                elements: vec![
                    call("nested_commit", vec![]),
                    Expr::DegradeToTransient {
                        value: Box::new(Expr::PersistentHash {
                            value: Box::new(Expr::Tuple {
                                elements: vec![
                                    Expr::BytesLiteral { bytes: vec![1; 32] },
                                    Expr::BytesLiteral { bytes: vec![2; 32] },
                                ],
                            }),
                        }),
                    },
                ],
            }),
        },
    ));
    prepend(
        &mut c,
        local("pure_crypto", Type::Field, call("crypto_bridge", vec![])),
    );
    included(&c, "set_topic", "merge-nested-pure-crypto");
}
#[test]
fn whole_deposit_fold_uses_the_certified_pure_call_boundary() {
    let mut c = source(true);
    c.circuits.push(pure(
        "keep_accumulator",
        vec![
            Parameter {
                name: "acc".into(),
                ty: Type::Field,
            },
            Parameter {
                name: "item".into(),
                ty: Type::Field,
            },
        ],
        Type::Field,
        parameter("acc"),
    ));
    let vector = Type::Vector {
        element: Box::new(Type::Field),
        length: 2,
    };
    c.circuits.push(pure(
        "fold_values",
        vec![Parameter {
            name: "items".into(),
            ty: vector.clone(),
        }],
        Type::Field,
        Expr::VectorFoldCall {
            name: "keep_accumulator".into(),
            initial: Box::new(field_literal(3)),
            source: Box::new(parameter("items")),
            accumulator: Type::Field,
            element: Type::Field,
            length: 2,
        },
    ));
    prepend(
        &mut c,
        local(
            "pure_fold",
            Type::Field,
            call(
                "fold_values",
                vec![Expr::Coerce {
                    value: Box::new(Expr::Tuple {
                        elements: vec![field_literal(7), field_literal(11)],
                    }),
                    ty: vector,
                }],
            ),
        ),
    );
    included(&c, "set_topic", "merge-pure-vector-fold");
}
