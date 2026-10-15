// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Map lexical overlays and Fold signature/coercion sequencing.
//! Static Fold checks initial before source; its output keeps source-first runtime
//! initialization. Explicit borrowed annotations retain their existing bounds.
use super::{CheckedAggregate, Output};
use crate::{
    RenderError,
    ir::{Expr, Parameter, Type},
    pure_semantics::{
        children::ScopedChildren,
        coercion::plan_coercion,
        declarations::PureDeclarations,
        identifier::check_name,
        sink::{AggregateSink, Typed},
    },
};

pub(super) struct MapInput<'a> {
    pub(super) parameter: &'a Parameter,
    pub(super) source: &'a Expr,
    pub(super) body: &'a Expr,
    pub(super) result: &'a Type,
    pub(super) length: usize,
}
pub(super) struct FoldInput<'a> {
    pub(super) name: &'a str,
    pub(super) initial: &'a Expr,
    pub(super) source: &'a Expr,
    pub(super) accumulator: &'a Type,
    pub(super) element: &'a Type,
    pub(super) length: usize,
}

pub(super) fn map<S: AggregateSink>(
    input: MapInput<'_>,
    children: &mut impl ScopedChildren<S>,
    sink: &mut S,
) -> Result<Typed<Output<S>>, RenderError> {
    let MapInput {
        parameter,
        source,
        body,
        result,
        length,
    } = input;
    let source = children.visit(source, sink)?;
    let target = Type::Vector {
        element: Box::new(parameter.ty.clone()),
        length,
    };
    let source_plan = plan_coercion(&source.ty, &target)?;
    let name = check_name(&parameter.name)?;
    let binding = sink.binding(&name);
    let body = children.visit_bound(body, &parameter.name, &parameter.ty, &binding, sink)?;
    if body.ty != *result {
        return Err(RenderError::TypeMismatch {
            expected: result.clone(),
            actual: body.ty,
        });
    }
    Ok(Typed {
        output: CheckedAggregate::Map {
            source: source.output,
            source_plan,
            binding,
            body: body.output,
            length,
        },
        ty: Type::Vector {
            element: Box::new(result.clone()),
            length,
        },
    })
}

pub(super) fn fold<S: AggregateSink>(
    input: FoldInput<'_>,
    children: &mut impl ScopedChildren<S>,
    declarations: &impl PureDeclarations,
    sink: &mut S,
) -> Result<Typed<Output<S>>, RenderError> {
    let FoldInput {
        name,
        initial,
        source,
        accumulator,
        element,
        length,
    } = input;
    let circuit = declarations
        .signature(name)
        .ok_or_else(|| RenderError::UnknownCircuit(name.to_owned()))?;
    if circuit.parameters.len() != 2
        || circuit.parameters[0].ty != *accumulator
        || circuit.parameters[1].ty != *element
        || circuit.result != accumulator
    {
        return Err(RenderError::TypeMismatch {
            expected: accumulator.clone(),
            actual: circuit.result.clone(),
        });
    }
    let initial = children.visit(initial, sink)?;
    let initial_plan = plan_coercion(&initial.ty, accumulator)?;
    let source = children.visit(source, sink)?;
    let target = Type::Vector {
        element: Box::new(element.clone()),
        length,
    };
    let source_plan = plan_coercion(&source.ty, &target)?;
    let function = check_name(name)?;
    Ok(Typed {
        output: CheckedAggregate::Fold {
            function,
            initial: initial.output,
            initial_plan,
            source: source.output,
            source_plan,
        },
        ty: accumulator.clone(),
    })
}
