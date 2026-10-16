// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Shared Map operations preserve declaration authority and key-before-value order.
use super::*;

pub(super) enum MapQuery<'a> {
    Member(&'a str, u8, &'a Expr),
    Lookup(&'a str, u8, &'a Expr),
    Size(&'a str, u8),
    IsEmpty(&'a str, u8),
}
impl Lowerer<'_> {
    fn map_slot(
        &self,
        name: &str,
        index: u8,
        scalar: bool,
        path: &str,
    ) -> Result<(MapSlot, Type, Option<Type>), RecordingGap> {
        let (declaration, _) = self.field(name, index, path)?;
        let slot = MapSlot { declaration, index };
        let (key, value) =
            super::super::validate::maps::types(self.declarations, slot, scalar, path)?;
        Ok((slot, key.clone(), scalar.then(|| value.clone())))
    }
    fn map_operand(
        &mut self,
        expression: &Expr,
        ty: &Type,
        scope: &Scope,
        bindings: &mut Vec<Binding>,
        path: &str,
    ) -> Result<Value, RecordingGap> {
        let value = self.expression(expression, scope, bindings, path)?;
        if value.ty != *ty {
            return Err(invalid(path, "Map operand type differs from declaration"));
        }
        Ok(value)
    }
    /// Share one recursive expression dispatcher temporary across Map queries.
    pub(super) fn map_expression(
        &mut self,
        expression: &Expr,
        scope: &Scope,
        bindings: &mut Vec<Binding>,
        path: &str,
    ) -> Result<(Operation, Type), RecordingGap> {
        let query = match expression {
            Expr::MapMember { field, index, key } => MapQuery::Member(field, *index, key),
            Expr::MapLookup { field, index, key } => MapQuery::Lookup(field, *index, key),
            Expr::MapIsEmpty { field, index } => MapQuery::IsEmpty(field, *index),
            _ => {
                return Err(invalid(
                    path,
                    "Map expression dispatch requires a Map query",
                ));
            }
        };
        self.map_query(query, scope, bindings, path)
    }
    pub(super) fn map_query(
        &mut self,
        query: MapQuery<'_>,
        scope: &Scope,
        bindings: &mut Vec<Binding>,
        path: &str,
    ) -> Result<(Operation, Type), RecordingGap> {
        match query {
            MapQuery::Size(name, index) | MapQuery::IsEmpty(name, index) => {
                // Metadata queries never decode V; nested native Map nodes remain valid.
                let (slot, _, _) = self.map_slot(name, index, false, path)?;
                Ok(if matches!(query, MapQuery::Size(..)) {
                    (Operation::MapSize { slot }, uint("18446744073709551615"))
                } else {
                    (Operation::MapIsEmpty { slot }, Type::Boolean)
                })
            }
            MapQuery::Member(name, index, expression)
            | MapQuery::Lookup(name, index, expression) => {
                let lookup = matches!(query, MapQuery::Lookup(..));
                let (slot, key_type, value_type) = self.map_slot(name, index, lookup, path)?;
                let key = self.map_operand(
                    expression,
                    &key_type,
                    scope,
                    bindings,
                    &format!("{path}.key"),
                )?;
                Ok(if lookup {
                    (
                        Operation::MapLookup { slot, key },
                        value_type.expect("scalar lookup declaration"),
                    )
                } else {
                    (Operation::MapMember { slot, key }, Type::Boolean)
                })
            }
        }
    }
    pub(super) fn map_action(
        &mut self,
        action: &StateAction,
        scope: &Scope,
        bindings: &mut Vec<Binding>,
        path: &str,
    ) -> Result<(), RecordingGap> {
        let (name, index) = match action {
            StateAction::MapInsert { field, index, .. }
            | StateAction::MapRemove { field, index, .. }
            | StateAction::MapInsertDefault { field, index, .. }
            | StateAction::MapReset { field, index } => (field, *index),
            _ => return Err(invalid(path, "Map action dispatch requires a Map mutation")),
        };
        // Only removal is key-only. Reset deliberately retains the runtime V:CellValue bound.
        let scalar = !matches!(action, StateAction::MapRemove { .. });
        let (slot, key_type, value_type) = self.map_slot(name, index, scalar, path)?;
        let operation = match action {
            StateAction::MapReset { .. } => Operation::MapReset { slot },
            StateAction::MapInsert { key, value, .. } => {
                let key =
                    self.map_operand(key, &key_type, scope, bindings, &format!("{path}.key"))?;
                let value = self.map_operand(
                    value,
                    value_type.as_ref().expect("scalar mutation declaration"),
                    scope,
                    bindings,
                    &format!("{path}.value"),
                )?;
                Operation::MapInsert { slot, key, value }
            }
            StateAction::MapInsertDefault { key, .. } | StateAction::MapRemove { key, .. } => {
                let key =
                    self.map_operand(key, &key_type, scope, bindings, &format!("{path}.key"))?;
                if matches!(action, StateAction::MapInsertDefault { .. }) {
                    if !super::super::validate::default_value_type(
                        value_type.as_ref().expect("scalar default declaration"),
                    ) {
                        return Err(invalid(
                            path,
                            "Map default insertion requires a defaultable value",
                        ));
                    }
                    Operation::MapInsertDefault { slot, key }
                } else {
                    Operation::MapRemove { slot, key }
                }
            }
            _ => return Err(invalid(path, "Map action dispatch requires a Map mutation")),
        };
        self.emit(operation, Type::Unit, bindings, path);
        Ok(())
    }
}
