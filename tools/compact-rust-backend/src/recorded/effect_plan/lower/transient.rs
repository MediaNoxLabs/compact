// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Official transient crypto, independent of contract or helper shapes.
use super::super::validate;
use super::*;
impl Lowerer<'_> {
    pub(super) fn transient_crypto(
        &mut self,
        expression: &Expr,
        scope: &Scope,
        bindings: &mut Vec<Binding>,
        path: &str,
    ) -> Result<(Operation, Type), RecordingGap> {
        let (Expr::DegradeToTransient { value }
        | Expr::UpgradeFromTransient { value }
        | Expr::TransientHash { value }
        | Expr::HashToCurve { value }
        | Expr::TransientCommit { value, .. }) = expression
        else {
            unreachable!("transient crypto dispatcher");
        };
        let value = self.expression(value, scope, bindings, &format!("{path}.value"))?;
        match expression {
            Expr::DegradeToTransient { .. } if value.ty == (Type::Bytes { length: 32 }) => {
                Ok((Operation::DegradeToTransient(value), Type::Field))
            }
            Expr::UpgradeFromTransient { .. } if value.ty == Type::Field => Ok((
                Operation::UpgradeFromTransient(value),
                Type::Bytes { length: 32 },
            )),
            Expr::HashToCurve { .. } if validate::aligned_fab_type(&value.ty) => {
                Ok((Operation::HashToCurve(value), Type::JubjubPoint))
            }
            Expr::TransientHash { .. } if validate::aligned_fab_type(&value.ty) => {
                Ok((Operation::TransientHash(value), Type::Field))
            }
            Expr::TransientCommit { opening, .. } => {
                // Both operands are evaluated in native source order before
                // reporting a signature mismatch; the validator rechecks it.
                let opening =
                    self.expression(opening, scope, bindings, &format!("{path}.opening"))?;
                if !validate::aligned_fab_type(&value.ty) || opening.ty != Type::Field {
                    return Err(invalid(
                        path,
                        "transient commitment requires aligned FAB value and Field opening",
                    ));
                }
                Ok((Operation::TransientCommit { value, opening }, Type::Field))
            }
            _ => Err(invalid(path, "unsupported transient crypto operand type")),
        }
    }
}
