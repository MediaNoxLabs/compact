// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Binary operation temporaries stay off recursive dispatch for unrelated families.
use super::super::validate;
use super::*;

impl Lowerer<'_> {
    pub(super) fn binary(
        &mut self,
        expression: &Expr,
        scope: &Scope,
        bindings: &mut Vec<Binding>,
        path: &str,
    ) -> Result<(Operation, Type), RecordingGap> {
        let (left, right) = match expression {
            Expr::Add { left, right }
            | Expr::Subtract { left, right }
            | Expr::Multiply { left, right }
            | Expr::Compare { left, right, .. }
            | Expr::Equal { left, right }
            | Expr::NotEqual { left, right }
            | Expr::UnsignedAdd { left, right, .. }
            | Expr::UnsignedSubtract { left, right, .. }
            | Expr::UnsignedMultiply { left, right, .. } => (left, right),
            _ => return Err(self.unsupported_expression(expression, path)),
        };
        let a = self.expression(left, scope, bindings, &format!("{path}.left"))?;
        let b = self.expression(right, scope, bindings, &format!("{path}.right"))?;
        Ok(match expression {
            Expr::Add { .. } | Expr::Subtract { .. } | Expr::Multiply { .. }
                if a.ty == Type::Field && b.ty == Type::Field =>
            {
                let operation = if matches!(expression, Expr::Multiply { .. }) {
                    Operation::Multiply(a, b)
                } else if matches!(expression, Expr::Subtract { .. }) {
                    Operation::Subtract(a, b)
                } else {
                    Operation::Add(a, b)
                };
                (operation, Type::Field)
            }
            Expr::Compare { operator, .. }
                if validate::bound(&a.ty).is_some() && validate::bound(&b.ty).is_some() =>
            {
                (
                    Operation::Compare {
                        operator: *operator,
                        left: a,
                        right: b,
                    },
                    Type::Boolean,
                )
            }
            Expr::Equal { .. } | Expr::NotEqual { .. }
                if validate::equality_type(&a.ty) && a.ty == b.ty =>
            {
                let operation = if matches!(expression, Expr::NotEqual { .. }) {
                    Operation::NotEqual(a, b)
                } else {
                    Operation::Equal(a, b)
                };
                (operation, Type::Boolean)
            }
            Expr::UnsignedAdd { max, .. }
                if validate::unsigned_add_type(&a.ty, &b.ty, &uint(max)) =>
            {
                (Operation::UnsignedAdd(a, b), uint(max))
            }
            Expr::UnsignedSubtract { max, .. } | Expr::UnsignedMultiply { max, .. }
                if validate::bound(&a.ty).is_some()
                    && validate::bound(&b.ty).is_some()
                    && validate::bound(&uint(max)).is_some() =>
            {
                let operation = match expression {
                    Expr::UnsignedSubtract { .. } => Operation::UnsignedSubtract(a, b),
                    Expr::UnsignedMultiply { .. } => Operation::UnsignedMultiply(a, b),
                    _ => unreachable!("unsigned arithmetic arm"),
                };
                (operation, uint(max))
            }
            _ => return Err(invalid(path, "invalid arithmetic operand types")),
        })
    }
}
