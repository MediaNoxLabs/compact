// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Signature-only calls, with arity and immediate argument mismatch precedence.
use super::{CheckedAggregate, Output};
use crate::{
    RenderError,
    ir::Expr,
    pure_semantics::{
        children::Children,
        declarations::PureDeclarations,
        identifier::check_name,
        sink::{AggregateSink, Typed},
    },
};

pub(crate) enum CallRefusal<'source> {
    Name,
    Argument(&'source Expr),
}
pub(crate) type CallRefusalCallback<'callback> =
    dyn for<'source> FnMut(CallRefusal<'source>) + 'callback;

pub(super) fn check<S: AggregateSink>(
    name: &str,
    arguments: &[Expr],
    children: &mut impl Children<S>,
    declarations: &impl PureDeclarations,
    sink: &mut S,
) -> Result<Typed<Output<S>>, RenderError> {
    check_with_refusal(name, arguments, children, declarations, sink, None)
}

/// Only this call boundary receives a callback. Child visits use the ordinary
/// semantic entry, so nested calls never borrow an outer Action's name site.
pub(super) fn check_with_refusal<S: AggregateSink>(
    name: &str,
    arguments: &[Expr],
    children: &mut impl Children<S>,
    declarations: &impl PureDeclarations,
    sink: &mut S,
    mut refusal: Option<&mut CallRefusalCallback<'_>>,
) -> Result<Typed<Output<S>>, RenderError> {
    let circuit = declarations.signature(name).ok_or_else(|| {
        refuse(&mut refusal, CallRefusal::Name);
        RenderError::UnknownCircuit(name.to_owned())
    })?;
    if arguments.len() != circuit.parameters.len() {
        refuse(&mut refusal, CallRefusal::Name);
        return Err(RenderError::ArgumentCount {
            circuit: name.to_owned(),
            expected: circuit.parameters.len(),
            actual: arguments.len(),
        });
    }
    let mut values = Vec::with_capacity(arguments.len());
    for (source, parameter) in arguments.iter().zip(circuit.parameters) {
        let value = children.visit(source, sink)?;
        if value.ty != parameter.ty {
            refuse(&mut refusal, CallRefusal::Argument(source));
            return Err(RenderError::TypeMismatch {
                expected: parameter.ty.clone(),
                actual: value.ty,
            });
        }
        values.push(value.output);
    }
    let function = check_name(name).inspect_err(|_| refuse(&mut refusal, CallRefusal::Name))?;
    Ok(Typed {
        output: CheckedAggregate::Call {
            function,
            arguments: values,
        },
        ty: circuit.result.clone(),
    })
}

fn refuse(callback: &mut Option<&mut CallRefusalCallback<'_>>, site: CallRefusal<'_>) {
    if let Some(callback) = callback.as_mut() {
        callback(site);
    }
}
