// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Borrowed pure action calls; shared signature/argument semantics run once.
use crate::{
    RenderError,
    ir::{Expr, StateAction, Type},
    pure_native::sink::NativeSink,
    pure_semantics::{
        aggregate::{CallRefusal, fold_borrowed_call},
        sink::{ExpressionSink, OrdinaryView},
        visitor::ExpressionVisitor,
    },
    source_diagnostics::{Owner, RefusalSite},
    stateful::NativeExpressionContext,
};
use std::collections::HashMap;

pub(super) fn render(
    action: &StateAction,
    name: &str,
    arguments: &[Expr],
    parameters: &HashMap<&str, (&Type, syn::Ident)>,
    environment: &NativeExpressionContext<'_, '_>,
) -> Result<syn::Expr, RenderError> {
    let observer = environment.observer;
    let mut child_refusal = |source: &Expr, _: &RenderError| {
        if let Some(observer) = observer {
            observer.refused(RefusalSite::node(Owner::Expr(source)));
        }
    };
    let mut call_refusal = |site: CallRefusal<'_>| {
        if let Some(observer) = observer {
            observer.refused(match site {
                CallRefusal::Name => RefusalSite::field(Owner::Action(action), "name"),
                CallRefusal::Argument(source) => RefusalSite::node(Owner::Expr(source)),
            });
        }
    };
    let mut sink = if observer.is_some() {
        NativeSink::with_refusal_callback(&mut child_refusal)
    } else {
        NativeSink::new()
    };
    let callback = if observer.is_some() {
        Some(&mut call_refusal as &mut crate::pure_semantics::aggregate::CallRefusalCallback<'_>)
    } else {
        None
    };
    let typed = fold_borrowed_call(
        name,
        arguments,
        &mut ExpressionVisitor::new(parameters, environment.circuits),
        environment.circuits,
        &mut sink,
        callback,
    )?;
    // A Call is never an EmptyUnit literal/default. Preserve the normal fold's
    // output-only ordinary view before discarding the action's result type.
    let view = if typed.ty == Type::Unit {
        OrdinaryView::ExecutableUnit
    } else {
        OrdinaryView::Value
    };
    Ok(sink.ordinary(view, typed.output).into_expression())
}

#[cfg(test)]
mod tests;
