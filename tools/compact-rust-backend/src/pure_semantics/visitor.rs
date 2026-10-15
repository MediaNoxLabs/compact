// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Borrowed full-fold traversal. Scoped visits clone binding metadata only.
//! No native dispatcher, declaration-body traversal or syntax fallback is reachable.
use super::{
    children::{Children, ScopedChildren},
    declarations::PureDeclarations,
    expression::fold_expression_in_session,
    scope::{LocalScope, TypeBoundsSession},
    sink::{ExpressionSink, Typed},
};
use crate::{
    RenderError,
    ir::{Expr, Type},
};

pub(crate) struct ExpressionVisitor<'call, 'ir, S: ExpressionSink, D>
where
    S::Binding: Clone,
{
    bindings: &'call TypeBoundsSession<'ir, S::Binding>,
    declarations: &'call D,
}
impl<'call, 'ir, S: ExpressionSink, D> ExpressionVisitor<'call, 'ir, S, D>
where
    S::Binding: Clone,
{
    pub(crate) fn new(
        bindings: &'call TypeBoundsSession<'ir, S::Binding>,
        declarations: &'call D,
    ) -> Self {
        Self {
            bindings,
            declarations,
        }
    }
}
impl<S: ExpressionSink, D: PureDeclarations> Children<S> for ExpressionVisitor<'_, '_, S, D>
where
    S::Binding: Clone,
{
    fn visit(&mut self, source: &Expr, sink: &mut S) -> Result<Typed<S::Value>, RenderError> {
        fold_expression_in_session(source, self.bindings, self.declarations, sink)
    }
}
impl<S: ExpressionSink, D: PureDeclarations> ScopedChildren<S> for ExpressionVisitor<'_, '_, S, D>
where
    S::Binding: Clone,
{
    fn visit_bound(
        &mut self,
        source: &Expr,
        source_key: &str,
        ty: &Type,
        binding: &S::Binding,
        sink: &mut S,
    ) -> Result<Typed<S::Value>, RenderError> {
        let mut scope = LocalScope::new(self.bindings);
        scope.insert(source_key, ty, binding.clone());
        fold_expression_in_session(source, scope.bindings(), self.declarations, sink)
    }
}
