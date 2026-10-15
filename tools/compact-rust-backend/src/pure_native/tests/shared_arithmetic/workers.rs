// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Public default boundaries on the unchanged ordinary worker stack.
use crate::{
    RenderError,
    ir::{ComparisonOperator, Contract, Expr, LocalBinding, PureCircuit, SCHEMA_VERSION, Type},
};

#[derive(Clone, Copy, Debug)]
enum Shape {
    Field,
    Unsigned,
    CoercedField,
    AggregateField,
    LexicalField,
    Equality,
    Ordered,
}
fn field() -> Expr {
    Expr::FieldLiteral { value: "1".into() }
}
fn unsigned() -> Expr {
    Expr::UnsignedLiteral {
        value: "1".into(),
        max: "255".into(),
    }
}
fn uint() -> Type {
    Type::Unsigned { max: "255".into() }
}
fn contract(shape: Shape, depth: usize) -> Contract {
    let result = match shape {
        Shape::Unsigned => uint(),
        Shape::Equality | Shape::Ordered => Type::Boolean,
        _ => Type::Field,
    };
    let mut body = match shape {
        Shape::Unsigned => unsigned(),
        Shape::Equality | Shape::Ordered => Expr::Boolean { value: true },
        _ => field(),
    };
    for _ in 0..depth {
        body = match shape {
            Shape::Field => Expr::Subtract {
                left: Box::new(field()),
                right: Box::new(body),
            },
            Shape::Unsigned => Expr::UnsignedAdd {
                max: "255".into(),
                left: Box::new(body),
                right: Box::new(unsigned()),
            },
            Shape::CoercedField => Expr::Add {
                left: Box::new(Expr::Coerce {
                    value: Box::new(body),
                    ty: Type::Field,
                }),
                right: Box::new(field()),
            },
            Shape::AggregateField => Expr::Multiply {
                left: Box::new(Expr::TupleIndex {
                    value: Box::new(Expr::Tuple {
                        elements: vec![body],
                    }),
                    index: 0,
                }),
                right: Box::new(field()),
            },
            Shape::LexicalField => Expr::Let {
                bindings: vec![LocalBinding {
                    name: "saved".into(),
                    ty: Type::Field,
                    value: body,
                }],
                body: Box::new(Expr::Add {
                    left: Box::new(Expr::Parameter {
                        name: "saved".into(),
                    }),
                    right: Box::new(field()),
                }),
            },
            Shape::Equality => Expr::Equal {
                left: Box::new(body),
                right: Box::new(Expr::Boolean { value: true }),
            },
            Shape::Ordered => Expr::Compare {
                operator: ComparisonOperator::LessEqual,
                left: Box::new(Expr::If {
                    condition: Box::new(body),
                    then: Box::new(unsigned()),
                    otherwise: Box::new(Expr::UnsignedLiteral {
                        value: "0".into(),
                        max: "255".into(),
                    }),
                }),
                right: Box::new(unsigned()),
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
fn ordinary_two_mib_arithmetic_and_mixed_workers_keep_the_actual_default_boundary() {
    for shape in [
        Shape::Field,
        Shape::Unsigned,
        Shape::CoercedField,
        Shape::AggregateField,
        Shape::LexicalField,
        Shape::Equality,
        Shape::Ordered,
    ] {
        let depth = (1..=64)
            .take_while(|d| crate::resource_limits::validate(&contract(shape, *d)).is_ok())
            .last()
            .expect("small arithmetic supported");
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
                    "Arithmetic worker {shape:?}: depth {depth} accepted, {} refused",
                    depth + 1
                );
            })
            .unwrap()
            .join()
            .unwrap();
    }
}
