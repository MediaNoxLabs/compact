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

pub(super) fn check<S: AggregateSink>(
    name: &str,
    arguments: &[Expr],
    children: &mut impl Children<S>,
    declarations: &impl PureDeclarations,
    sink: &mut S,
) -> Result<Typed<Output<S>>, RenderError> {
    let circuit = declarations
        .signature(name)
        .ok_or_else(|| RenderError::UnknownCircuit(name.to_owned()))?;
    if arguments.len() != circuit.parameters.len() {
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
            return Err(RenderError::TypeMismatch {
                expected: parameter.ty.clone(),
                actual: value.ty,
            });
        }
        values.push(value.output);
    }
    let function = check_name(name)?;
    Ok(Typed {
        output: CheckedAggregate::Call {
            function,
            arguments: values,
        },
        ty: circuit.result.clone(),
    })
}
