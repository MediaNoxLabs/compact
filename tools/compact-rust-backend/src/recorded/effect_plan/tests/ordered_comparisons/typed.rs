// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Ordered comparison domain and SSA laws, independent of public root interfaces.
use super::*;
use crate::ir::ComparisonOperator;

#[path = "lowering.rs"]
mod lowering;
#[path = "validation.rs"]
mod validation;

const OPERATORS: [ComparisonOperator; 4] = [
    ComparisonOperator::Less,
    ComparisonOperator::LessEqual,
    ComparisonOperator::Greater,
    ComparisonOperator::GreaterEqual,
];
const U128_MAX: &str = "340282366920938463463374607431768211455";
const WIDE: &str = "340282366920938463463374607431768211456";
fn reference(name: &str) -> Expr {
    Expr::Parameter { name: name.into() }
}
fn compare(operator: ComparisonOperator, left: Expr, right: Expr) -> Expr {
    Expr::Compare {
        operator,
        left: Box::new(left),
        right: Box::new(right),
    }
}
fn root(operator: ComparisonOperator, left: Type, right: Type) -> StatefulCircuit {
    StatefulCircuit {
        source: None,
        name: "ordering".into(),
        internal: true,
        parameters: vec![
            Parameter {
                name: "left".into(),
                ty: left,
            },
            Parameter {
                name: "right".into(),
                ty: right,
            },
        ],
        result: Type::Boolean,
        actions: vec![],
        return_value: StateReturn::Expression {
            value: compare(operator, reference("left"), reference("right")),
        },
    }
}
fn checked(source: &StatefulCircuit) -> CheckedPlan {
    check(source, &HashMap::new(), &HashMap::new()).unwrap()
}
fn comparison(plan: &mut Plan) -> &mut Binding {
    plan.body
        .bindings
        .iter_mut()
        .find(|b| matches!(b.operation, Operation::Compare { .. }))
        .unwrap()
}
fn operand(binding: &mut Binding, side: usize) -> &mut Value {
    let Operation::Compare { left, right, .. } = &mut binding.operation else {
        panic!("expected ordered comparison")
    };
    if side == 0 { left } else { right }
}
fn authority(source: &StatefulCircuit) -> declarations::Declarations {
    let types = type_environment::TypeEnvironment::from_sources(
        source,
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
    )
    .unwrap();
    declarations::Declarations::default()
        .with_type_environment(types)
        .with_root(source)
}
fn unsupported_types() -> Vec<Type> {
    vec![
        Type::Unit,
        Type::Boolean,
        Type::Field,
        Type::JubjubPoint,
        Type::Bytes { length: 32 },
        Type::OpaqueString,
        validate::uint(WIDE),
    ]
}
