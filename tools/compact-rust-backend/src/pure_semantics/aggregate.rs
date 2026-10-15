// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Complete eight-operation aggregate family, including lexical Map and Call/Fold.
//! Borrowed source/declaration roots retain the existing DEFAULT-bound premises.
//! No extra private guard, callee body checker or cumulative budget is introduced.
use super::{
    carrier::CarrierSink,
    children::ScopedChildren,
    coercion::CoercionPlan,
    declarations::PureDeclarations,
    identifier::CheckedName,
    ownership::Ownership,
    sink::{AggregateSink, Typed},
};
use crate::{
    RenderError,
    ir::{Expr, Parameter, Type},
};
mod call;
pub(crate) use call::{CallRefusal, CallRefusalCallback};
mod literal;
mod projection;
mod vector;

pub(crate) enum AggregateSource<'a> {
    StructField {
        value: &'a Expr,
        field: &'a str,
        index: usize,
    },
    TupleIndex {
        value: &'a Expr,
        index: usize,
    },
    StructLiteral {
        ty: &'a Type,
        fields: &'a [Expr],
    },
    Tuple {
        elements: &'a [Expr],
    },
    Vector {
        element: &'a Type,
        elements: &'a [Expr],
    },
    VectorMap {
        parameter: &'a Parameter,
        source: &'a Expr,
        body: &'a Expr,
        result: &'a Type,
        length: usize,
    },
    VectorFoldCall {
        name: &'a str,
        initial: &'a Expr,
        source: &'a Expr,
        accumulator: &'a Type,
        element: &'a Type,
        length: usize,
    },
    Call {
        name: &'a str,
        arguments: &'a [Expr],
    },
}

pub(crate) enum CheckedAggregate<V, B, C> {
    StructField {
        value: V,
        field: CheckedName,
        ownership: Ownership,
    },
    TupleIndex {
        value: V,
        index: usize,
    },
    StructLiteral {
        name: CheckedName,
        fields: Vec<(CheckedName, V)>,
    },
    Tuple(Vec<V>),
    Vector(Vec<V>),
    EmptyVector(C),
    Map {
        source: V,
        source_plan: CoercionPlan,
        binding: B,
        body: V,
        length: usize,
    },
    Fold {
        function: CheckedName,
        initial: V,
        initial_plan: CoercionPlan,
        source: V,
        source_plan: CoercionPlan,
    },
    Call {
        function: CheckedName,
        arguments: Vec<V>,
    },
}

type Output<S> = CheckedAggregate<
    <S as super::sink::LeafSink>::Value,
    <S as super::sink::LeafSink>::Binding,
    <<S as super::sink::LeafSink>::Carriers as CarrierSink>::Value,
>;

pub(crate) fn fold_aggregate<S: AggregateSink, C: ScopedChildren<S>>(
    source: AggregateSource<'_>,
    children: &mut C,
    declarations: &impl PureDeclarations,
    sink: &mut S,
) -> Result<Typed<S::Value>, RenderError> {
    // One post-match projection; individual owners return only checked facts.
    let checked = match source {
        AggregateSource::StructField {
            value,
            field,
            index,
        } => projection::field(value, field, index, children, sink),
        AggregateSource::TupleIndex { value, index } => {
            projection::tuple(value, index, children, sink)
        }
        AggregateSource::StructLiteral { ty, fields } => {
            literal::structure(ty, fields, children, sink)
        }
        AggregateSource::Tuple { elements } => literal::tuple(elements, children, sink),
        AggregateSource::Vector { element, elements } => {
            literal::vector(element, elements, children, sink)
        }
        AggregateSource::VectorMap {
            parameter,
            source,
            body,
            result,
            length,
        } => vector::map(
            vector::MapInput {
                parameter,
                source,
                body,
                result,
                length,
            },
            children,
            sink,
        ),
        AggregateSource::VectorFoldCall {
            name,
            initial,
            source,
            accumulator,
            element,
            length,
        } => vector::fold(
            vector::FoldInput {
                name,
                initial,
                source,
                accumulator,
                element,
                length,
            },
            children,
            declarations,
            sink,
        ),
        AggregateSource::Call { name, arguments } => {
            call::check(name, arguments, children, declarations, sink)
        }
    };
    let checked = checked?;
    Ok(project(checked, sink))
}

/// Borrowed Action call entry. Signature checks use the same call owner once;
/// nested arguments retain the ordinary visitor and no Action-owned callback.
pub(crate) fn fold_borrowed_call<S: AggregateSink, C: ScopedChildren<S>>(
    name: &str,
    arguments: &[Expr],
    children: &mut C,
    declarations: &impl PureDeclarations,
    sink: &mut S,
    refusal: Option<&mut CallRefusalCallback<'_>>,
) -> Result<Typed<S::Value>, RenderError> {
    let checked = call::check_with_refusal(name, arguments, children, declarations, sink, refusal)?;
    Ok(project(checked, sink))
}

/// Common output projection, after all child traversal and semantic checks.
fn project<S: AggregateSink>(checked: Typed<Output<S>>, sink: &mut S) -> Typed<S::Value> {
    Typed {
        output: sink.aggregate(checked.output),
        ty: checked.ty,
    }
}

#[cfg(test)]
pub(crate) mod tests;
