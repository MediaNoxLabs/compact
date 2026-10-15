// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Ordered literal children; only an empty vector validates its carrier annotation.
use super::{CheckedAggregate, Output};
use crate::{
    RenderError,
    ir::{Expr, Type},
    pure_semantics::{
        carrier::fold_carrier,
        children::Children,
        identifier::check_name,
        sink::{AggregateSink, Typed},
    },
};

pub(super) fn structure<S: AggregateSink>(
    ty: &Type,
    fields: &[Expr],
    children: &mut impl Children<S>,
    sink: &mut S,
) -> Result<Typed<Output<S>>, RenderError> {
    let Type::Struct {
        name,
        fields: declarations,
    } = ty
    else {
        return Err(RenderError::InvalidStructField("<literal>".into()));
    };
    if fields.len() != declarations.len() {
        return Err(RenderError::InvalidStructField(name.clone()));
    }
    let mut values = Vec::with_capacity(fields.len());
    for (source, declaration) in fields.iter().zip(declarations) {
        let value = children.visit(source, sink)?;
        if value.ty != declaration.ty {
            return Err(RenderError::TypeMismatch {
                expected: declaration.ty.clone(),
                actual: value.ty,
            });
        }
        let field = check_name(&declaration.name)?;
        values.push((field, value.output));
    }
    let name = check_name(name)?;
    Ok(Typed {
        output: CheckedAggregate::StructLiteral {
            name,
            fields: values,
        },
        ty: ty.clone(),
    })
}
pub(super) fn tuple<S: AggregateSink>(
    elements: &[Expr],
    children: &mut impl Children<S>,
    sink: &mut S,
) -> Result<Typed<Output<S>>, RenderError> {
    let values = elements
        .iter()
        .map(|element| children.visit(element, sink))
        .collect::<Result<Vec<_>, _>>()?;
    let (outputs, types) = values
        .into_iter()
        .map(|value| (value.output, value.ty))
        .unzip();
    Ok(Typed {
        output: CheckedAggregate::Tuple(outputs),
        ty: Type::Tuple { elements: types },
    })
}
pub(super) fn vector<S: AggregateSink>(
    element: &Type,
    elements: &[Expr],
    children: &mut impl Children<S>,
    sink: &mut S,
) -> Result<Typed<Output<S>>, RenderError> {
    let mut values = Vec::new();
    for source in elements {
        let value = children.visit(source, sink)?;
        if value.ty != *element {
            return Err(RenderError::TypeMismatch {
                expected: element.clone(),
                actual: value.ty,
            });
        }
        values.push(value.output);
    }
    let output = if values.is_empty() {
        CheckedAggregate::EmptyVector(fold_carrier(element, sink.carriers())?)
    } else {
        CheckedAggregate::Vector(values)
    };
    Ok(Typed {
        output,
        ty: Type::Vector {
            element: Box::new(element.clone()),
            length: elements.len(),
        },
    })
}
