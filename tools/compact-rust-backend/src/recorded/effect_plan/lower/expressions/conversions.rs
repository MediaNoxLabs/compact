// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Original conversions bodies, without new checks or traversal.
use super::*;

impl Lowerer<'_> {
    pub(super) fn conversion_operation(
        &mut self,
        expression: &Expr,
        scope: &Scope,
        bindings: &mut Vec<Binding>,
        path: &str,
    ) -> Result<(Operation, Type), RecordingGap> {
        let value = match expression {
            Expr::FieldToBytes32 { value } => {
                let value = self.expression(value, scope, bindings, &format!("{path}.value"))?;
                if value.ty != Type::Field {
                    return Err(invalid(path, "FieldToBytes32 requires a Field operand"));
                }
                (Operation::FieldToBytes32(value), Type::Bytes { length: 32 })
            }
            Expr::UnsignedCast { value, max } => {
                let value = self.expression(value, scope, bindings, &format!("{path}.value"))?;
                if validate::unsigned_bound(&value.ty).is_none()
                    || validate::unsigned_bound(&uint(max)).is_none()
                {
                    return Err(invalid(path, "unsupported unsigned cast"));
                }
                (
                    Operation::Convert {
                        value,
                        kind: Conversion::UnsignedCast,
                    },
                    uint(max),
                )
            }
            _ => unreachable!("expression dispatcher selects conversions"),
        };
        Ok(value)
    }
}
