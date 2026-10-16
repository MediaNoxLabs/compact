// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Real no-syntax prior/aggregate owners; generic control remains future work.
use super::*;
use crate::ir::{Parameter, PureCircuit, StructField};
use std::collections::HashMap;
mod call_fold_order;
mod literal_order;
pub(crate) mod protocol;
mod scoped_map;
mod successes;
use protocol::{Consumer, Visitor};

fn parameter(name: &str) -> Expr {
    Expr::Parameter { name: name.into() }
}
fn field(value: &str) -> Expr {
    Expr::FieldLiteral {
        value: value.into(),
    }
}
fn vector_type(element: Type, length: usize) -> Type {
    Type::Vector {
        element: Box::new(element),
        length,
    }
}
fn uint(max: &str) -> Type {
    Type::Unsigned { max: max.into() }
}
fn circuit(name: &str, parameters: Vec<Type>, result: Type) -> PureCircuit {
    PureCircuit {
        source: None,
        internal: false,
        name: name.into(),
        parameters: parameters
            .into_iter()
            .enumerate()
            .map(|(index, ty)| Parameter {
                name: format!("p{index}"),
                ty,
            })
            .collect(),
        result,
        body: Expr::Unit,
    }
}
fn check(
    source: &Expr,
    bindings: HashMap<&str, (&Type, ())>,
    declarations: &HashMap<&str, &PureCircuit>,
) -> Result<Type, RenderError> {
    let mut visitor = Visitor::new(bindings, declarations);
    visitor
        .check(source, &mut Consumer::default())
        .map(|checked| checked.ty)
}
