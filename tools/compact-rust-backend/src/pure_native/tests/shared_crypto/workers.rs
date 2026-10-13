// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Real public default boundaries on the unchanged ordinary worker stack.
use crate::{
    RenderError,
    ir::{Contract, Expr, LocalBinding, PureCircuit, SCHEMA_VERSION, Type},
};
#[derive(Clone, Copy, Debug)]
enum Shape {
    Hash,
    Commit,
    HashControl,
    CurveAggregate,
    PointAdd,
    PointNegate,
    LexicalCommit,
    Keccak,
}
fn field() -> Expr {
    Expr::FieldLiteral { value: "1".into() }
}
fn point() -> Expr {
    Expr::EcMulGenerator {
        scalar: Box::new(field()),
    }
}
fn contract(shape: Shape, depth: usize) -> Contract {
    let result = match shape {
        Shape::PointAdd | Shape::PointNegate => Type::JubjubPoint,
        Shape::Keccak => Type::Bytes { length: 32 },
        _ => Type::Field,
    };
    let mut body = match shape {
        Shape::PointAdd | Shape::PointNegate => point(),
        Shape::Keccak => Expr::BytesLiteral { bytes: vec![1; 32] },
        _ => field(),
    };
    for _ in 0..depth {
        body = match shape {
            Shape::Hash => Expr::TransientHash {
                value: Box::new(body),
            },
            Shape::Commit => Expr::TransientCommit {
                value: Box::new(body),
                opening: Box::new(field()),
            },
            Shape::HashControl => Expr::TransientHash {
                value: Box::new(Expr::If {
                    condition: Box::new(Expr::Boolean { value: true }),
                    then: Box::new(body),
                    otherwise: Box::new(field()),
                }),
            },
            Shape::CurveAggregate => Expr::JubjubPointX {
                value: Box::new(Expr::HashToCurve {
                    value: Box::new(Expr::Tuple {
                        elements: vec![
                            Expr::Coerce {
                                value: Box::new(body),
                                ty: Type::Field,
                            },
                            field(),
                        ],
                    }),
                }),
            },
            Shape::PointAdd => Expr::EcAdd {
                left: Box::new(body),
                right: Box::new(point()),
            },
            Shape::PointNegate => Expr::EcNeg {
                value: Box::new(body),
            },
            Shape::LexicalCommit => Expr::Let {
                bindings: vec![LocalBinding {
                    name: "saved".into(),
                    ty: Type::Field,
                    value: Expr::TransientHash {
                        value: Box::new(body),
                    },
                }],
                body: Box::new(Expr::TransientCommit {
                    value: Box::new(Expr::Parameter {
                        name: "saved".into(),
                    }),
                    opening: Box::new(field()),
                }),
            },
            Shape::Keccak => Expr::Keccak256 {
                value: Box::new(body),
            },
        };
    }
    Contract {
        schema_version: SCHEMA_VERSION,
        type_aliases: vec![],
        ledger_fields: vec![],
        constructor: None,
        witnesses: vec![],
        circuits: vec![PureCircuit {
            source: None,
            internal: false,
            name: "probe".into(),
            parameters: vec![],
            result,
            body,
        }],
        stateful_circuits: vec![],
    }
}
#[test]
fn ordinary_two_mib_crypto_and_mixed_workers_keep_the_actual_default_boundary() {
    for shape in [
        Shape::Hash,
        Shape::Commit,
        Shape::HashControl,
        Shape::CurveAggregate,
        Shape::PointAdd,
        Shape::PointNegate,
        Shape::LexicalCommit,
        Shape::Keccak,
    ] {
        let depth = (1..=64)
            .take_while(|d| crate::resource_limits::validate(&contract(shape, *d)).is_ok())
            .last()
            .expect("small crypto source is supported");
        assert!(
            (3..48).contains(&depth),
            "unexpected {shape:?} depth {depth}"
        );
        std::thread::Builder::new()
            .stack_size(2 * 1024 * 1024)
            .spawn(move || {
                assert!(
                    crate::render(&contract(shape, depth)).is_ok(),
                    "{shape:?} depth {depth}"
                );
                assert!(matches!(
                    crate::render(&contract(shape, depth + 1)),
                    Err(RenderError::ResourceLimit { .. })
                ));
                eprintln!(
                    "Crypto worker {shape:?}: depth {depth} accepted, {} refused",
                    depth + 1
                );
            })
            .unwrap()
            .join()
            .unwrap();
    }
}
