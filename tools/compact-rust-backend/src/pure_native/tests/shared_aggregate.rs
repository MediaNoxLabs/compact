// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Full eight-family old/native/real() outcomes with independent ownership facts.
//! The parent reserves workers.rs and source-bound execution after author HOLD.
use super::*;
use crate::pure_semantics::aggregate::tests::protocol::{Consumer, Visitor};
mod call_fold;
mod literals;
mod map;
mod nested;
mod old_retained;
mod ownership;
mod projection;

fn compare(
    source: &Expr,
    parameters: &HashMap<&str, (&Type, syn::Ident)>,
    circuits: &HashMap<&str, &PureCircuit>,
) -> Result<Type, RenderError> {
    let current = observe(source, parameters, circuits);
    assert_eq!(
        current,
        old_oracle::observe(source, parameters, circuits),
        "{source:?}"
    );
    let bindings = parameters
        .iter()
        .map(|(name, (ty, _))| (*name, (*ty, ())))
        .collect();
    let checked = Visitor::new(bindings, circuits)
        .check(source, &mut Consumer::default())
        .map(|value| value.ty);
    let native = current.map(|(_, _, ty)| ty);
    assert_eq!(checked, native, "generic consumer {source:?}");
    native
}
fn field(value: &str) -> Expr {
    Expr::FieldLiteral {
        value: value.into(),
    }
}
fn uint(max: &str) -> Type {
    Type::Unsigned { max: max.into() }
}
fn vector_type(element: Type, length: usize) -> Type {
    Type::Vector {
        element: Box::new(element),
        length,
    }
}
fn vector(elements: Vec<Expr>) -> Expr {
    Expr::Vector {
        element: Type::Field,
        elements,
    }
}
fn structure(name: &str, fields: Vec<(&str, Type)>) -> Type {
    Type::Struct {
        name: name.into(),
        fields: fields
            .into_iter()
            .map(|(name, ty)| StructField {
                name: name.into(),
                ty,
            })
            .collect(),
    }
}
fn declaration(name: &str, parameters: Vec<Type>, result: Type) -> PureCircuit {
    PureCircuit {
        source: None,
        internal: false,
        name: name.into(),
        parameters: parameters
            .into_iter()
            .enumerate()
            .map(|(index, ty)| Parameter {
                name: format!("arg{index}"),
                ty,
            })
            .collect(),
        result,
        body: Expr::KernelSelf { ty: Type::Field },
    }
}
fn folding(
    name: &str,
    initial: Expr,
    source: Expr,
    accumulator: Type,
    element: Type,
    length: usize,
) -> Expr {
    Expr::VectorFoldCall {
        name: name.into(),
        initial: Box::new(initial),
        source: Box::new(source),
        accumulator,
        element,
        length,
    }
}
#[test]
#[should_panic(expected = "private pure-expression dispatcher selects this family")]
fn wrong_family_private_aggregate_adapter_preserves_misuse_contract() {
    let _ = crate::pure_aggregate_expression(&Expr::Unit, &HashMap::new(), &HashMap::new());
}

mod runtime_probe;
mod workers;
