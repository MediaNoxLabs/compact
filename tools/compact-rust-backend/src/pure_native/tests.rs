// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Frozen output, observable execution and one-visit contracts for Unit views.
use super::{
    control::{self, IfFacts},
    fragment::{self, NativeFragment},
    visitation,
};
use crate::{
    RenderError,
    ir::{Expr, LocalBinding, Parameter, PureCircuit, StructField, Type},
};
use quote::ToTokens;
use std::collections::HashMap;

mod differential;
mod execution;
mod item_assembly;
mod old_oracle;
mod prepared;
mod shared_aggregate;
mod shared_arithmetic;
mod shared_conversion;
mod shared_crypto;
mod shared_leaf;
mod visits;
mod workers;

fn reference(name: &str) -> Expr {
    Expr::Parameter { name: name.into() }
}
fn call(name: &str) -> Expr {
    Expr::Call {
        name: name.into(),
        arguments: Vec::new(),
    }
}
fn boolean(value: bool) -> Expr {
    Expr::Boolean { value }
}
fn branch(condition: Expr, then: Expr, otherwise: Expr) -> Expr {
    Expr::If {
        condition: Box::new(condition),
        then: Box::new(then),
        otherwise: Box::new(otherwise),
    }
}
fn assertion(condition: Expr) -> Expr {
    Expr::Assert {
        condition: Box::new(condition),
        message: "guard".into(),
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
fn pure(name: &str, result: Type) -> PureCircuit {
    let body = if result == Type::Boolean {
        boolean(false)
    } else {
        Expr::Unit
    };
    PureCircuit {
        source: None,
        internal: false,
        name: name.into(),
        parameters: Vec::new(),
        result,
        body,
    }
}
fn observe(
    source: &Expr,
    parameters: &HashMap<&str, (&Type, syn::Ident)>,
    circuits: &HashMap<&str, &PureCircuit>,
) -> Result<(String, String, Type), RenderError> {
    let (fragment, actual) = crate::native_fragment_with_calls(source, parameters, circuits)?;
    let (expression, statements) = fragment.into_views();
    Ok((
        expression.to_token_stream().to_string(),
        quote::quote!(#(#statements)*).to_string(),
        actual,
    ))
}
