// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Original assertions bodies, without new checks or traversal.
use super::super::*;

impl Lowerer<'_> {
    pub(in crate::recorded::effect_plan::lower::borrowed_operations) fn old_assertion_operation(
        &mut self,
        expression: &Expr,
        scope: &Scope,
        bindings: &mut Vec<Binding>,
        path: &str,
    ) -> Result<(Operation, Type), RecordingGap> {
        let value = match expression {
            Expr::Assert { condition, message } => {
                let condition =
                    self.expression(condition, scope, bindings, &format!("{path}.condition"))?;
                if condition.ty != Type::Boolean {
                    return Err(invalid(path, "assert requires Boolean"));
                }
                (
                    Operation::Assert {
                        condition,
                        message: message.clone(),
                    },
                    Type::Unit,
                )
            }
            _ => unreachable!("expression dispatcher selects assertions"),
        };
        Ok(value)
    }
}
