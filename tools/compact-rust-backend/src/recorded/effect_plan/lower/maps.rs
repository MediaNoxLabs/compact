// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Shared Map operations preserve declaration authority and key-before-value order.
use super::*;

pub(super) enum MapQuery<'a> {
    Member(&'a str, u8, &'a Expr),
    Lookup(&'a str, u8, &'a Expr),
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
    pub(super) fn map_query(
        &mut self,
        query: MapQuery<'_>,
        scope: &Scope,
        bindings: &mut Vec<Binding>,
        path: &str,
    ) -> Result<(Operation, Type), RecordingGap> {
        let lookup = matches!(query, MapQuery::Lookup(..));
        let (name, index, expression) = match query {
            MapQuery::Member(name, index, key) | MapQuery::Lookup(name, index, key) => {
                (name, index, key)
            }
        };
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
    pub(super) fn map_action(
        &mut self,
        action: &StateAction,
        scope: &Scope,
        bindings: &mut Vec<Binding>,
        path: &str,
    ) -> Result<(), RecordingGap> {
        let (name, index, key) = match action {
            StateAction::MapInsert {
                field, index, key, ..
            }
            | StateAction::MapRemove { field, index, key } => (field, *index, key),
            _ => unreachable!("Map action dispatch"),
        };
        let (slot, key_type, value_type) = self.map_slot(name, index, true, path)?;
        let key = self.map_operand(key, &key_type, scope, bindings, &format!("{path}.key"))?;
        let operation = if let StateAction::MapInsert { value, .. } = action {
            let value = self.map_operand(
                value,
                value_type.as_ref().expect("scalar mutation declaration"),
                scope,
                bindings,
                &format!("{path}.value"),
            )?;
            Operation::MapInsert { slot, key, value }
        } else {
            Operation::MapRemove { slot, key }
        };
        self.emit(operation, Type::Unit, bindings, path);
        Ok(())
    }
}
