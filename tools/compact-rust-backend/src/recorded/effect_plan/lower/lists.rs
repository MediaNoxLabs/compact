// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Ordered List effects, bound to immutable declarations rather than circuit shapes.
use super::*;

pub(super) enum Query<'a> {
    Length,
    IsEmpty,
    Head(&'a Type),
}
impl Lowerer<'_> {
    /// One dispatcher result keeps query-specific temporaries off the recursive
    /// expression frame; the three variants have identical declaration authority.
    pub(super) fn list_expression(
        &self,
        expression: &Expr,
        path: &str,
    ) -> Result<(Operation, Type), RecordingGap> {
        let (field, index, query) = match expression {
            Expr::ListLength { field, index } => (field, *index, Query::Length),
            Expr::ListIsEmpty { field, index } => (field, *index, Query::IsEmpty),
            Expr::ListHead { field, index, ty } => (field, *index, Query::Head(ty)),
            _ => unreachable!("List expression dispatch"),
        };
        self.list_query(field, index, query, path)
    }

    fn list_slot(
        &self,
        name: &str,
        index: u8,
        path: &str,
    ) -> Result<(ListSlot, Type), RecordingGap> {
        let (declaration, _) = self.field(name, index, path)?;
        let slot = ListSlot { declaration, index };
        let element = super::super::validate::lists::element(self.declarations, slot, path)?;
        Ok((slot, element.clone()))
    }
    pub(super) fn list_query(
        &self,
        field: &str,
        index: u8,
        query: Query<'_>,
        path: &str,
    ) -> Result<(Operation, Type), RecordingGap> {
        let (slot, element) = self.list_slot(field, index, path)?;
        let (operation, ty) = match query {
            Query::Length => (ListOperation::Length, uint("18446744073709551615")),
            Query::IsEmpty => (ListOperation::IsEmpty, Type::Boolean),
            Query::Head(result) => {
                super::super::validate::lists::head_type(
                    self.declarations,
                    &element,
                    result,
                    path,
                )?;
                (ListOperation::Head, result.clone())
            }
        };
        Ok((Operation::List { slot, operation }, ty))
    }
    pub(super) fn list_action(
        &mut self,
        action: &StateAction,
        scope: &Scope,
        bindings: &mut Vec<Binding>,
        path: &str,
    ) -> Result<(), RecordingGap> {
        let (field, index) = match action {
            StateAction::ListPushFront { field, index, .. }
            | StateAction::ListPopFront { field, index }
            | StateAction::ListReset { field, index } => (field, *index),
            _ => unreachable!("List action dispatch"),
        };
        let (slot, element) = self.list_slot(field, index, path)?;
        let operation = match action {
            StateAction::ListPushFront { value, .. } => {
                let path = format!("{path}.value");
                let value = self.expression(value, scope, bindings, &path)?;
                if value.ty != element {
                    return Err(invalid(&path, "List operand type differs from declaration"));
                }
                ListOperation::PushFront(value)
            }
            StateAction::ListPopFront { .. } => ListOperation::PopFront,
            _ => ListOperation::Reset,
        };
        self.emit(
            Operation::List { slot, operation },
            Type::Unit,
            bindings,
            path,
        );
        Ok(())
    }
}
