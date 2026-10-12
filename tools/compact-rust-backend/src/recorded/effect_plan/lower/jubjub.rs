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
            Expr::EcNeg { value } => (
                Operation::EcNeg(operand(value, "value", Type::JubjubPoint)?),
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

// Keep coordinate-only temporaries off each recursive arithmetic lowering frame.
impl Lowerer<'_> {
    pub(super) fn jubjub_coordinate(
        &mut self,
        expression: &Expr,
        scope: &Scope,
        bindings: &mut Vec<Binding>,
        path: &str,
    ) -> Result<(Operation, Type), RecordingGap> {
        let (input, x_coordinate) = match expression {
            Expr::JubjubPointX { value } => (value, true),
            Expr::JubjubPointY { value } => (value, false),
            _ => return Err(RecordingGap::expression(expression, path.into())),
        };
        let operand_path = format!("{path}.value");
        let value = self.expression(input, scope, bindings, &operand_path)?;
        if value.ty != Type::JubjubPoint {
            return Err(invalid(
                &operand_path,
                "expected JubjubPoint Jubjub operand",
            ));
        }
        let operation = if x_coordinate {
            Operation::JubjubPointX(value)
        } else {
            Operation::JubjubPointY(value)
        };
        Ok((operation, Type::Field))
    }
}
