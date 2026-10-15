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

use super::StateExpressionContext;
use crate::source_diagnostics::{Owner, RefusalObserver, RefusalSite};
use std::{cell::RefCell, collections::HashMap};

#[derive(Default)]
struct Trace(RefCell<Vec<usize>>);
impl RefusalObserver for Trace {
    fn refused(&self, site: RefusalSite<'_>) {
        // Preflight may identify a Type. Semantic leaf controls below identify
        // actual Expr; no fake conversion from other owners is introduced.
        if let Some(Owner::Expr(source)) = site.owner {
            self.0
                .borrow_mut()
                .push(std::ptr::from_ref(source) as usize);
        }
    }
}
type Output = (String, Type, bool, bool, usize);
fn lower(source: &Contract, observer: Option<&dyn RefusalObserver>) -> Result<Output, RenderError> {
    // The actual production DEFAULT preflight runs before native assembly.
    let _preflight = crate::resource_limits::validate_with_observer(source, observer)?;
    let witnesses = HashMap::new();
    let stateful = HashMap::new();
    let ledger = HashMap::new();
    let pure = source
        .circuits
        .iter()
        .map(|c| (c.name.as_str(), c))
        .collect();
    let parameters = HashMap::new();
    let environment = StateExpressionContext {
        witnesses: &witnesses,
        circuits: &pure,
        stateful_circuits: &stateful,
        ledger_fields: &ledger,
        observer,
    };
    let mut statements = Vec::new();
    let mut next = 0;
    let mut query = false;
    // Direct entry makes native_frame's successful selective route impossible.
    let (rendered, ty, witness) = crate::stateful::expression::render_state_expression_observed(
        &source.circuits[0].body,
        &parameters,
        &mut statements,
        &mut next,
        &mut query,
        &environment,
    )?;
    Ok((
        quote::quote!(#(#statements)* #rendered).to_string(),
        ty,
        witness,
        query,
        next,
    ))
}
fn failed_leaf(body: &mut Expr, shape: Shape) -> &mut Expr {
    // A fixture-only borrowed descent through exactly the known frozen shape;
    // it is not a production traversal or source-address algorithm.
    let mut current = body;
    loop {
        current = match current {
            Expr::TupleIndex { value, .. } => value,
            Expr::Tuple { elements } => &mut elements[0],
            Expr::StructField { value, .. } => value,
            Expr::StructLiteral { fields, .. } => &mut fields[0],
            Expr::VectorMap { source, body, .. } => {
                if matches!(shape, Shape::MapBody) {
                    body
                } else {
                    source
                }
            }
            Expr::Call { arguments, .. } => &mut arguments[0],
            Expr::VectorFoldCall {
                initial, source, ..
            } => {
                if matches!(shape, Shape::FoldSource) {
                    source
                } else {
                    initial
                }
            }
            Expr::Vector { elements, .. } => &mut elements[0],
            Expr::TransientHash { value } | Expr::Coerce { value, .. } => value,
            Expr::FieldLiteral { .. } | Expr::Parameter { .. } => return current,
            _ => panic!("frozen fixture spine changed"),
        };
    }
}

#[test]
fn enabled_native_expression_observer_keeps_actual_default_two_mib_boundaries() {
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
            .expect("small frozen shape accepted");
        assert!(
            (3..48).contains(&depth),
            "unexpected {shape:?} depth{depth}"
        );
        std::thread::Builder::new().stack_size(2 * 1024 * 1024).spawn(move || {
            let accepted = contract(shape, depth);
            let trace = Trace::default();
            let observed = lower(&accepted, Some(&trace)).expect("enabled observer at actual near boundary");
            assert_eq!(observed, lower(&accepted, None).unwrap(), "{shape:?} output/type/effect/temporary continuity");
            assert!(trace.0.borrow().is_empty());
            let over = contract(shape, depth + 1);
            let observed = lower(&over, Some(&trace)).unwrap_err();
            assert!(matches!(observed, RenderError::ResourceLimit { .. }));
            assert_eq!(observed, lower(&over, None).unwrap_err());
            // Same accepted tree depth, real malformed innermost original leaf.
            let mut refused = contract(shape, depth);
            let leaf = failed_leaf(&mut refused.circuits[0].body, shape);
            *leaf = Expr::FieldLiteral { value: "bad".into() };
            let original = std::ptr::from_ref(leaf) as usize;
            let trace = Trace::default();
            let error = lower(&refused, Some(&trace)).unwrap_err();
            assert!(matches!(error, RenderError::InvalidFieldLiteral(ref value) if value == "bad"));
            assert_eq!(error, lower(&refused, None).unwrap_err());
            assert_eq!(trace.0.borrow().first(), Some(&original), "actual innermost borrowed leaf, not parent");
            eprintln!("Observed native worker {shape:?}: DEFAULT depth {depth} accepted, {} refused, malformed leaf captured", depth + 1);
        }).unwrap().join().unwrap();
    }
}
