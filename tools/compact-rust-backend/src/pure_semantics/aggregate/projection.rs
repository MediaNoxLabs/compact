// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Projection validation and ownership facts, distinct from operation tuple caps.
use super::{CheckedAggregate, Output};
use crate::{
    RenderError,
    ir::{Expr, Type},
    pure_semantics::{
        children::Children,
        identifier::check_name,
        ownership::{Ownership, copy_type},
        sink::{AggregateSink, Typed},
    },
};

pub(super) fn field<S: AggregateSink>(
    source: &Expr,
    field: &str,
    index: usize,
    children: &mut impl Children<S>,
    sink: &mut S,
) -> Result<Typed<Output<S>>, RenderError> {
    let value = children.visit(source, sink)?;
    let Type::Struct { fields, .. } = value.ty else {
        return Err(RenderError::InvalidStructField(field.to_owned()));
    };
    let declaration = fields
        .get(index)
        .filter(|declaration| declaration.name == field)
        .ok_or_else(|| RenderError::InvalidStructField(field.to_owned()))?;
    let field = check_name(field)?;
    let ownership = if copy_type(&declaration.ty) {
        Ownership::Copy
    } else {
        Ownership::Owned
    };
    Ok(Typed {
        output: CheckedAggregate::StructField {
            value: value.output,
            field,
            ownership,
        },
        ty: declaration.ty.clone(),
    })
}
pub(super) fn tuple<S: AggregateSink>(
    source: &Expr,
    index: usize,
    children: &mut impl Children<S>,
    sink: &mut S,
) -> Result<Typed<Output<S>>, RenderError> {
    let value = children.visit(source, sink)?;
    let Type::Tuple { elements } = value.ty else {
        return Err(RenderError::ExpectedTuple(value.ty));
    };
    let ty = elements
        .get(index)
        .ok_or(RenderError::InvalidTupleIndex(index))?
        .clone();
    Ok(Typed {
        output: CheckedAggregate::TupleIndex {
            value: value.output,
            index,
        },
        ty,
    })
}
