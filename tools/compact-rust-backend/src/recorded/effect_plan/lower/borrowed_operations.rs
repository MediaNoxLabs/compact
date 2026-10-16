// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Original action/return inputs for the five formerly synthetic expressions.
//! This is a source descriptor, not a second expression dispatcher.
use super::*;

pub(super) enum LeafQuery<'source> {
    CellRead { field: &'source str, index: u8 },
    CounterRead { field: &'source str, index: u8 },
    SetSize { field: &'source str, index: u8 },
    SetIsEmpty { field: &'source str, index: u8 },
}

impl Lowerer<'_> {
    pub(super) fn borrowed_assertion(
        &mut self,
        action: &StateAction,
        condition: &Expr,
        message: &str,
        scope: &Scope,
        bindings: &mut Vec<Binding>,
        path: &str,
    ) -> Result<Value, RecordingGap> {
        // Preserve the old synthetic expression's spend, after the action spend.
        self.spend(path)?;
        let (operation, ty) = self
            .assertion_fields(condition, message, scope, bindings, path)
            .inspect_err(|_| refused(self.observer, RefusalSite::node(Owner::Action(action))))?;
        self.finish_borrowed_operation(operation, ty, bindings, path)
    }

    pub(super) fn borrowed_leaf_return(
        &mut self,
        returned: &StateReturn,
        query: LeafQuery<'_>,
        bindings: &mut Vec<Binding>,
        path: &str,
    ) -> Result<Value, RecordingGap> {
        // Exactly one expression spend; specialized return scope is unchanged.
        self.spend(path)?;
        let (operation, ty) = self.leaf_query_operation(query, path).inspect_err(|_| {
            refused(
                self.observer,
                RefusalSite::node(Owner::StateReturn(returned)),
            )
        })?;
        self.finish_borrowed_operation(operation, ty, bindings, path)
    }

    fn finish_borrowed_operation(
        &mut self,
        operation: Operation,
        ty: Type,
        bindings: &mut Vec<Binding>,
        path: &str,
    ) -> Result<Value, RecordingGap> {
        // This output Type is owned/derived, as in the ordinary expression path.
        // It has no authenticated original Type occurrence.
        self.declarations
            .types()
            .validate_at(&ty, path)
            .inspect_err(|_| refused(self.observer, RefusalSite::unavailable()))?;
        Ok(self.emit(operation, ty, bindings, path))
    }
}

#[cfg(test)]
mod tests;
