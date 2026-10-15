// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Original assertions bodies, without new checks or traversal.
use super::*;

impl Lowerer<'_> {
    pub(super) fn assertion_operation(
        &mut self,
        expression: &Expr,
        scope: &Scope,
        bindings: &mut Vec<Binding>,
        path: &str,
    ) -> Result<(Operation, Type), RecordingGap> {
        let value = match expression {
            Expr::Assert { condition, message } => {
                self.assertion_fields(condition, message, scope, bindings, path)?
            }
            _ => unreachable!("expression dispatcher selects assertions"),
        };
        Ok(value)
    }

    /// Shared original checks for source expressions and borrowed action operands.
    pub(in crate::recorded::effect_plan::lower) fn assertion_fields(
        &mut self,
        condition: &Expr,
        message: &str,
        scope: &Scope,
        bindings: &mut Vec<Binding>,
        path: &str,
    ) -> Result<(Operation, Type), RecordingGap> {
        let condition =
            self.expression(condition, scope, bindings, &format!("{path}.condition"))?;
        if condition.ty != Type::Boolean {
            return Err(invalid(path, "assert requires Boolean"));
        }
        Ok((
            Operation::Assert {
                condition,
                message: message.to_owned(),
            },
            Type::Unit,
        ))
    }
}
