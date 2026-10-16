// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Pure bridge bodies are checked separately from the old direct send grammar.
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

#[test]
fn whole_old_send_certifies_default_string_and_nested_crypto_helpers() {
    let mut contract = source();
    contract.circuits.push(pure(
        "empty_text",
        vec![],
        Type::OpaqueString,
        Expr::Default {
            ty: Type::OpaqueString,
        },
    ));
    contract.circuits.push(pure(
        "nested_hash",
        vec![],
        Type::Field,
        Expr::TransientCommit {
            value: Box::new(Expr::Tuple {
                elements: vec![field_literal(3), field_literal(7)],
            }),
            opening: Box::new(field_literal(11)),
        },
    ));
    contract.circuits.push(pure(
        "crypto_bridge",
        vec![],
        Type::Field,
        Expr::Let {
            bindings: vec![local(
                "empty",
                Type::OpaqueString,
                call("empty_text", vec![]),
            )],
            body: Box::new(Expr::TransientHash {
                value: Box::new(Expr::Tuple {
                    elements: vec![
                        call("nested_hash", vec![]),
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
            }),
        },
    ));
    prepend(
        &mut contract,
        vec![local(
            "pure_observation",
            Type::Field,
            call("crypto_bridge", vec![]),
        )],
    );
    probe(
        &contract,
        "send_to_self",
        "pure-default-and-crypto-bridge",
        true,
    );
}

#[test]
fn whole_old_send_certifies_a_vector_fold_through_a_pure_call() {
    let mut contract = source();
    // Identity accumulator is inside the old pure grammar; arithmetic Add is
    // deliberately not smuggled into an old-first premise.
    contract.circuits.push(pure(
        "keep_accumulator",
        vec![
            parameter("acc", Type::Field),
            parameter("item", Type::Field),
        ],
        Type::Field,
        reference("acc"),
    ));
    let vector = vector(Type::Field, 2);
    contract.circuits.push(pure(
        "fold_values",
        vec![parameter("items", vector.clone())],
        Type::Field,
        Expr::VectorFoldCall {
            name: "keep_accumulator".into(),
            initial: Box::new(field_literal(3)),
            source: Box::new(reference("items")),
            accumulator: Type::Field,
            element: Type::Field,
            length: 2,
        },
    ));
    prepend(
        &mut contract,
        vec![local(
            "folded",
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
        )],
    );
    probe(&contract, "send_to_self", "pure-vector-fold-bridge", true);
}

#[test]
fn whole_old_send_direct_aggregate_leaves_and_tuple_equality_compose() {
    let mut contract = source();
    let choice = Type::Enum {
        name: "Choice".into(),
        variants: vec!["First".into(), "Second".into()],
    };
    let product = Type::Struct {
        name: "LocalProduct".into(),
        fields: vec![
            StructField {
                name: "bytes".into(),
                ty: Type::Bytes { length: 4 },
            },
            StructField {
                name: "choice".into(),
                ty: choice.clone(),
            },
            StructField {
                name: "count".into(),
                ty: validate::uint("255"),
            },
        ],
    };
    let tuple = || Expr::Tuple {
        elements: (1..=12).map(field_literal).collect(),
    };
    prepend(
        &mut contract,
        vec![
            local(
                "choice",
                choice.clone(),
                Expr::EnumVariant {
                    ty: choice,
                    variant: "Second".into(),
                },
            ),
            local(
                "defaulted",
                product.clone(),
                Expr::Default {
                    ty: product.clone(),
                },
            ),
            local(
                "constructed",
                product.clone(),
                Expr::StructLiteral {
                    ty: product,
                    fields: vec![
                        Expr::BytesLiteral {
                            bytes: vec![1, 2, 3, 4],
                        },
                        reference("choice"),
                        Expr::UnsignedLiteral {
                            value: "255".into(),
                            max: "255".into(),
                        },
                    ],
                },
            ),
            local(
                "tuple_equal",
                Type::Boolean,
                Expr::Equal {
                    left: Box::new(tuple()),
                    right: Box::new(tuple()),
                },
            ),
        ],
    );
    probe(
        &contract,
        "send_to_self",
        "direct-leaves-defaults-and-equality",
        true,
    );
}
