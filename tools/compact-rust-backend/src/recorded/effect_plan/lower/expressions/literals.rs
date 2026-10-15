// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Original literals bodies, without new checks or traversal.
use super::*;

impl Lowerer<'_> {
    pub(super) fn literal_operation(
        &mut self,
        expression: &Expr,
        _scope: &Scope,
        _bindings: &mut Vec<Binding>,
        path: &str,
    ) -> Result<(Operation, Type), RecordingGap> {
        let value = match expression {
            Expr::Unit => (Operation::Literal(Literal::Unit), Type::Unit),
            Expr::Boolean { value } => {
                (Operation::Literal(Literal::Boolean(*value)), Type::Boolean)
            }
            Expr::FieldLiteral { value } => (
                Operation::Literal(Literal::Field(
                    crate::field_literal_bytes(value).map_err(|e| invalid(path, e.to_string()))?,
                )),
                Type::Field,
            ),
            Expr::UnsignedLiteral { value, max } => {
                let ty = uint(max);
                let bound = validate::unsigned_bound(&ty)
                    .ok_or_else(|| invalid(path, "unsupported unsigned bound"))?;
                let bytes = crate::field_literal_bytes(value)
                    .map_err(|_| invalid(path, "noncanonical unsigned literal"))?;
                let literal = match bound {
                    crate::UnsignedMaximum::Small(max) => {
                        let low = u128::from_le_bytes(bytes[..16].try_into().expect("low limb"));
                        if bytes[16..].iter().any(|b| *b != 0) || low > max {
                            return Err(invalid(path, "unsigned literal outside bound"));
                        }
                        Literal::Unsigned(low)
                    }
                    crate::UnsignedMaximum::Wide { .. } => {
                        if !validate::wide_literal_fits(&bytes, &ty) {
                            return Err(invalid(path, "unsigned literal outside bound"));
                        }
                        Literal::WideUnsigned(bytes)
                    }
                };
                (Operation::Literal(literal), ty)
            }
            _ => unreachable!("expression dispatcher selects literals"),
        };
        Ok(value)
    }
}
