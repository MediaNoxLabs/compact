// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! One complete pure expression fold, independent of the output consumer.
//! Native output and the later formal body checker share the same traversal.
use super::{
    aggregate::fold_aggregate,
    arithmetic::fold_arithmetic,
    control::fold_control,
    conversion::fold_conversion,
    crypto::fold_crypto,
    declarations::PureDeclarations,
    leaf::fold_leaf,
    scope::{Scope, TypeBoundsSession},
    sink::{ExpressionSink, OrdinaryView, Typed},
    visitor::ExpressionVisitor,
};
use crate::{
    RenderError,
    ir::{Expr, Type},
};
mod source;
use source::Source;

pub(crate) fn fold_expression<S: ExpressionSink, D: PureDeclarations>(
    expr: &Expr,
    bindings: &Scope<'_, S::Binding>,
    declarations: &D,
    sink: &mut S,
) -> Result<Typed<S::Value>, RenderError>
where
    S::Binding: Clone,
{
    let checked = TypeBoundsSession::new(bindings);
    fold_expression_in_session(expr, &checked, declarations, sink)
}

/// Recursive entry uses the same immutable binding session, never a fresh memo.
pub(crate) fn fold_expression_in_session<S: ExpressionSink, D: PureDeclarations>(
    expr: &Expr,
    bindings: &TypeBoundsSession<'_, S::Binding>,
    declarations: &D,
    sink: &mut S,
) -> Result<Typed<S::Value>, RenderError>
where
    S::Binding: Clone,
{
    #[cfg(test)]
    sink.observe_source(expr);
    // One ordinary result slot, with a direct return for complete control views.
    let typed = match source::classify(expr) {
        Source::Control(source) => {
            return fold_control(source, bindings, declarations, sink)
                .inspect_err(|error| sink.refused(expr, error));
        }
        Source::Leaf(source) => fold_leaf(source, bindings, sink),
        Source::Conversion(source) => fold_conversion(
            source,
            &mut ExpressionVisitor::new(bindings, declarations),
            sink,
        ),
        Source::Arithmetic(source) => fold_arithmetic(
            source,
            &mut ExpressionVisitor::new(bindings, declarations),
            sink,
        ),
        Source::Crypto(source) => fold_crypto(
            source,
            &mut ExpressionVisitor::new(bindings, declarations),
            sink,
        ),
        Source::Aggregate(source) => fold_aggregate(
            source,
            &mut ExpressionVisitor::new(bindings, declarations),
            declarations,
            sink,
        ),
    }
    .inspect_err(|error| sink.refused(expr, error))?;
    let view = if typed.ty == Type::Unit {
        if matches!(expr, Expr::Unit | Expr::Default { ty: Type::Unit }) {
            OrdinaryView::EmptyUnit
        } else {
            OrdinaryView::ExecutableUnit
        }
    } else {
        OrdinaryView::Value
    };
    Ok(Typed {
        output: sink.ordinary(view, typed.output),
        ty: typed.ty,
    })
}
