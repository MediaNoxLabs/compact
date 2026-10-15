// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Actual DEFAULT near/over boundaries on the unchanged ordinary worker stack.
use crate::{
    RenderError,
    ir::{Contract, Expr, Parameter, PureCircuit, SCHEMA_VERSION, StructField, Type},
};
#[derive(Clone, Copy, Debug)]
enum Shape {
    TupleProjection,
    StructProjection,
    MapSource,
    MapBody,
    CallArgument,
    FoldInitial,
    FoldSource,
    MixedTupleHash,
}
fn field() -> Expr {
    Expr::FieldLiteral { value: "1".into() }
}
fn vector(value: Expr) -> Expr {
    Expr::Vector {
        element: Type::Field,
        elements: vec![value],
    }
}
fn parameter(name: &str) -> Parameter {
    Parameter {
        name: name.into(),
        ty: Type::Field,
    }
}
fn scalar_box() -> Type {
    Type::Struct {
        name: "ScalarBox".into(),
        fields: vec![StructField {
            name: "value".into(),
            ty: Type::Field,
        }],
    }
}
fn contract(shape: Shape, depth: usize) -> Contract {
    let mut body = if matches!(shape, Shape::MapBody) {
        Expr::Parameter {
            name: "item".into(),
        }
    } else if matches!(shape, Shape::MapSource) {
        vector(field())
    } else {
        field()
    };
    let mut map_body_result = Type::Field;
    for _ in 0..depth {
        body = match shape {
            Shape::TupleProjection => Expr::TupleIndex {
                value: Box::new(Expr::Tuple {
                    elements: vec![body],
                }),
                index: 0,
            },
            Shape::StructProjection => Expr::StructField {
                value: Box::new(Expr::StructLiteral {
                    ty: scalar_box(),
                    fields: vec![body],
                }),
                field: "value".into(),
                index: 0,
            },
            Shape::MapSource => Expr::VectorMap {
                parameter: parameter("item"),
                source: Box::new(body),
                body: Box::new(Expr::Parameter {
                    name: "item".into(),
                }),
                result: Type::Field,
                length: 1,
            },
            Shape::MapBody => {
                let result = map_body_result.clone();
                map_body_result = Type::Vector {
                    element: Box::new(result.clone()),
                    length: 1,
                };
                Expr::VectorMap {
                    parameter: parameter("item"),
                    source: Box::new(vector(field())),
                    body: Box::new(body),
                    result,
                    length: 1,
                }
            }
            Shape::CallArgument => Expr::Call {
                name: "identity".into(),
                arguments: vec![body],
            },
            Shape::FoldInitial => Expr::VectorFoldCall {
                name: "step".into(),
                initial: Box::new(body),
                source: Box::new(vector(field())),
                accumulator: Type::Field,
                element: Type::Field,
                length: 1,
            },
            Shape::FoldSource => Expr::VectorFoldCall {
                name: "step".into(),
                initial: Box::new(field()),
                source: Box::new(vector(body)),
                accumulator: Type::Field,
                element: Type::Field,
                length: 1,
            },
            Shape::MixedTupleHash => Expr::TupleIndex {
                value: Box::new(Expr::Tuple {
                    elements: vec![Expr::TransientHash {
                        value: Box::new(Expr::Coerce {
                            value: Box::new(body),
                            ty: Type::Field,
                        }),
                    }],
                }),
                index: 0,
            },
        };
    }
    let result = if matches!(shape, Shape::MapBody) {
        map_body_result
    } else if matches!(shape, Shape::MapSource) {
        Type::Vector {
            element: Box::new(Type::Field),
            length: 1,
        }
    } else {
        Type::Field
    };
    let mut circuits = vec![PureCircuit {
        source: None,
        internal: false,
        name: "probe".into(),
        parameters: vec![],
        result,
        body,
    }];
    if matches!(shape, Shape::CallArgument) {
        circuits.push(PureCircuit {
            source: None,
            internal: true,
            name: "identity".into(),
            parameters: vec![parameter("value")],
            result: Type::Field,
            body: Expr::Parameter {
                name: "value".into(),
            },
        });
    }
    if matches!(shape, Shape::FoldInitial | Shape::FoldSource) {
        circuits.push(PureCircuit {
            source: None,
            internal: true,
            name: "step".into(),
            parameters: vec![parameter("acc"), parameter("item")],
            result: Type::Field,
            body: Expr::Add {
                left: Box::new(Expr::Parameter { name: "acc".into() }),
                right: Box::new(Expr::Parameter {
                    name: "item".into(),
                }),
            },
        });
    }
    Contract {
        schema_version: SCHEMA_VERSION,
        type_aliases: vec![],
        ledger_fields: vec![],
        constructor: None,
        witnesses: vec![],
        circuits,
        stateful_circuits: vec![],
    }
}
#[test]
fn ordinary_two_mib_aggregate_and_mixed_workers_keep_actual_default_boundaries() {
    for shape in [
        Shape::TupleProjection,
        Shape::StructProjection,
        Shape::MapSource,
        Shape::MapBody,
        Shape::CallArgument,
        Shape::FoldInitial,
        Shape::FoldSource,
        Shape::MixedTupleHash,
    ] {
        let depth = (1..=64)
            .take_while(|d| crate::resource_limits::validate(&contract(shape, *d)).is_ok())
            .last()
            .expect("small aggregate is supported");
        assert!(
            (3..48).contains(&depth),
            "unexpected {shape:?} depth{depth}"
        );
        std::thread::Builder::new()
            .stack_size(2 * 1024 * 1024)
            .spawn(move || {
                assert!(
                    crate::render(&contract(shape, depth)).is_ok(),
                    "{shape:?} depth{depth}"
                );
                assert!(matches!(
                    crate::render(&contract(shape, depth + 1)),
                    Err(RenderError::ResourceLimit { .. })
                ));
                eprintln!(
                    "Aggregate worker {shape:?}: depth {depth} accepted, {} refused",
                    depth + 1
                );
            })
            .unwrap()
            .join()
            .unwrap();
    }
}
