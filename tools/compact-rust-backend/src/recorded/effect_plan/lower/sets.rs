// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Ordinary Set operands are exact declared values, evaluated before the operation.
use super::super::validate;
use super::*;

impl Lowerer<'_> {
    pub(super) fn set(
        &self,
        name: &str,
        index: u8,
        path: &str,
    ) -> Result<(SetSlot, Type), RecordingGap> {
        let (declaration, field) = self.field(name, index, path)?;
        let LedgerFieldKind::Set { ty } = &field.declaration else {
            return Err(invalid(path, "expected Set field"));
        };
        self.declarations.types().validate_at(ty, path)?;
        if !validate::set_element_type(ty) {
            return Err(invalid(path, "unsupported Set element type"));
        }
        Ok((SetSlot { declaration, index }, ty.clone()))
    }

    pub(super) fn set_member(
        &mut self,
        field: &str,
        index: u8,
        expression: &Expr,
        scope: &Scope,
        bindings: &mut Vec<Binding>,
        path: &str,
    ) -> Result<(Operation, Type), RecordingGap> {
        let (slot, value) = self.set_value(field, index, expression, scope, bindings, path)?;
        Ok((Operation::SetMember { slot, value }, Type::Boolean))
    }

    pub(super) fn set_value(
        &mut self,
        field: &str,
        index: u8,
        expression: &Expr,
        scope: &Scope,
        bindings: &mut Vec<Binding>,
        path: &str,
    ) -> Result<(SetSlot, Value), RecordingGap> {
        let (slot, ty) = self.set(field, index, path)?;
        let value_path = format!("{path}.value");
        let value = self.expression(expression, scope, bindings, &value_path)?;
        if value.ty != ty {
            return Err(invalid(
                &value_path,
                "Set operand type differs from declaration",
            ));
        }
        Ok((slot, value))
    }
}
