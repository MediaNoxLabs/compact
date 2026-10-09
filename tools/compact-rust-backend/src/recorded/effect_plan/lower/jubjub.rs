// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Point operands are evaluated once, left to right, before creating an operation.
use super::*;

impl Lowerer<'_> {
    pub(super) fn jubjub(
        &mut self,
        expression: &Expr,
        scope: &Scope,
        bindings: &mut Vec<Binding>,
        path: &str,
    ) -> Result<(Operation, Type), RecordingGap> {
        let mut operand = |expression, name, expected| {
            let path = format!("{path}.{name}");
            let value = self.expression(expression, scope, bindings, &path)?;
            if value.ty != expected {
                return Err(invalid(
                    &path,
                    format!("expected {expected:?} Jubjub operand"),
                ));
            }
            Ok(value)
        };
        Ok(match expression {
            Expr::JubjubScalarFromNative { value } => (
                Operation::JubjubScalarFromNative(operand(value, "value", Type::Field)?),
                Type::Field,
            ),
            Expr::EcMulGenerator { scalar } => (
                Operation::EcMulGenerator(operand(scalar, "scalar", Type::Field)?),
                Type::JubjubPoint,
            ),
            Expr::EcMul { point, scalar } => {
                let point = operand(point, "point", Type::JubjubPoint)?;
                let scalar = operand(scalar, "scalar", Type::Field)?;
                (Operation::EcMul { point, scalar }, Type::JubjubPoint)
            }
            Expr::EcAdd { left, right } => {
                let left = operand(left, "left", Type::JubjubPoint)?;
                let right = operand(right, "right", Type::JubjubPoint)?;
                (Operation::EcAdd { left, right }, Type::JubjubPoint)
            }
            _ => return Err(RecordingGap::expression(expression, path.into())),
        })
    }
}
