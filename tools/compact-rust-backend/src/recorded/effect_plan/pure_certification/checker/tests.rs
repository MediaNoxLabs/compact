// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Focused cutover controls against the signed D4b bridge, not another checker.
use super::check;
use crate::RenderError;
use crate::ir::{Expr, LocalBinding, Parameter, PureCircuit, Type};
use std::collections::HashMap;

mod body;
mod old_bridge;
mod ordering;
mod scopes;
mod unit_sink;

fn field() -> Expr {
    Expr::FieldLiteral { value: "1".into() }
}
fn boolean() -> Expr {
    Expr::Boolean { value: true }
}
fn reference(name: &str) -> Expr {
    Expr::Parameter { name: name.into() }
}
fn parameter(name: &str, ty: Type) -> Parameter {
    Parameter {
        name: name.into(),
        ty,
    }
}
fn source(parameters: Vec<Parameter>, result: Type, body: Expr) -> PureCircuit {
    PureCircuit {
        source: None,
        name: "checked".into(),
        internal: true,
        parameters,
        result,
        body,
    }
}
fn local(name: &str, ty: Type, value: Expr, body: Expr) -> Expr {
    Expr::Let {
        bindings: vec![LocalBinding {
            name: name.into(),
            ty,
            value,
        }],
        body: Box::new(body),
    }
}
fn branch(condition: Expr, then: Expr, otherwise: Expr) -> Expr {
    Expr::If {
        condition: Box::new(condition),
        then: Box::new(then),
        otherwise: Box::new(otherwise),
    }
}
fn effect() -> Expr {
    Expr::WitnessCall {
        name: "hidden".into(),
        arguments: vec![],
    }
}
fn compare(source: &PureCircuit, pure: &HashMap<&str, &PureCircuit>) -> Result<(), RenderError> {
    let current = check(source, pure);
    assert_eq!(current, old_bridge::check(source, pure), "{}", source.name);
    current
}
fn refusal(source: &PureCircuit, expected: RenderError) {
    assert_eq!(compare(source, &HashMap::new()), Err(expected));
}
