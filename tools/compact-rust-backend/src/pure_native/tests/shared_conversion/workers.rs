// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Check recursive family adapters against the unchanged public worker envelope.
use super::*;
use crate::ir::{Contract, SCHEMA_VERSION};

#[derive(Clone, Copy, Debug)]
enum Shape {
    Coerce,
    Cast,
    AggregateControl,
    LexicalCast,
}

fn contract(shape: Shape, depth: usize) -> Contract {
    let ty = match shape {
        Shape::Coerce | Shape::AggregateControl => Type::Field,
        _ => uint("255"),
    };
    let mut body = match shape {
        Shape::Coerce | Shape::AggregateControl => Expr::FieldLiteral { value: "1".into() },
        _ => Expr::UnsignedLiteral {
            value: "1".into(),
            max: "255".into(),
        },
    };
    for _ in 0..depth {
        body = match shape {
            Shape::Coerce => Expr::Coerce {
                value: Box::new(body),
                ty: ty.clone(),
            },
            Shape::Cast => Expr::UnsignedCast {
                value: Box::new(body),
                max: "255".into(),
            },
            Shape::AggregateControl => Expr::Coerce {
                value: Box::new(Expr::TupleIndex {
                    value: Box::new(Expr::Tuple {
                        elements: vec![branch(
                            boolean(true),
                            body,
                            Expr::FieldLiteral { value: "0".into() },
                        )],
                    }),
                    index: 0,
                }),
                ty: ty.clone(),
            },
            Shape::LexicalCast => local(
                "saved",
                ty.clone(),
                Expr::UnsignedCast {
                    value: Box::new(body),
                    max: "255".into(),
                },
                Expr::Coerce {
                    value: Box::new(reference("saved")),
                    ty: ty.clone(),
                },
            ),
        };
    }
    let mut circuit = pure("probe", ty);
    circuit.body = body;
    Contract {
        schema_version: SCHEMA_VERSION,
        type_aliases: Vec::new(),
        ledger_fields: Vec::new(),
        constructor: None,
        witnesses: Vec::new(),
        circuits: vec![circuit],
        stateful_circuits: Vec::new(),
    }
}

#[test]
fn ordinary_two_mib_conversion_and_mixed_workers_keep_the_actual_default_boundary() {
    for shape in [
        Shape::Coerce,
        Shape::Cast,
        Shape::AggregateControl,
        Shape::LexicalCast,
    ] {
        let depth = (1..=64)
            .take_while(|depth| crate::resource_limits::validate(&contract(shape, *depth)).is_ok())
            .last()
            .expect("small conversion cases are supported");
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
                    "Conversion worker {shape:?}: depth {depth} accepted, {} refused",
                    depth + 1
                );
            })
            .unwrap()
            .join()
            .unwrap();
    }
}
