// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Test-only complete () consumer. This is not production certificate admission.
use super::*;
use crate::ir::{Parameter, PureCircuit};
use crate::pure_semantics::expression::fold_expression;
use std::collections::HashMap;
mod consumer;
mod ordering;
mod scopes;
use consumer::Consumer;
fn reference(name: &str) -> Expr {
    Expr::Parameter { name: name.into() }
}
fn boolean(value: bool) -> Expr {
    Expr::Boolean { value }
}
fn field(value: &str) -> Expr {
    Expr::FieldLiteral {
        value: value.into(),
    }
}
fn assertion(condition: Expr) -> Expr {
    Expr::Assert {
        condition: Box::new(condition),
        message: "guard".into(),
    }
}
fn branch(condition: Expr, then: Expr, otherwise: Expr) -> Expr {
    Expr::If {
        condition: Box::new(condition),
        then: Box::new(then),
        otherwise: Box::new(otherwise),
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
fn check(
    source: &Expr,
    bindings: &Scope<'_, ()>,
    declarations: &HashMap<&str, &PureCircuit>,
    consumer: &mut Consumer,
) -> Result<Type, RenderError> {
    fold_expression(source, bindings, declarations, consumer).map(|typed| typed.ty)
}
fn error(source: &Expr, bindings: &Scope<'_, ()>) -> String {
    format!(
        "{:?}",
        check(source, bindings, &HashMap::new(), &mut Consumer::default())
            .err()
            .unwrap()
    )
}
