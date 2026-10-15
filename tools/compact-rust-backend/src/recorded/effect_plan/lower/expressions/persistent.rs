// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Original persistent bodies, without new checks or traversal.
use super::*;

impl Lowerer<'_> {
    pub(super) fn persistent_operation(
        &mut self,
        expression: &Expr,
        scope: &Scope,
        bindings: &mut Vec<Binding>,
        path: &str,
    ) -> Result<(Operation, Type), RecordingGap> {
        let value = match expression {
            Expr::PersistentHash { value } | Expr::Keccak256 { value } => {
                let value = self.expression(value, scope, bindings, &format!("{path}.value"))?;
                let operation = if matches!(expression, Expr::Keccak256 { .. }) {
                    if !validate::keccak_input_type(&value.ty) {
                        return Err(invalid(path, "Keccak requires a supported FAB value"));
                    }
                    Operation::Keccak256(value)
                } else {
                    if !validate::fixed_aligned_fab_type(&value.ty) {
                        return Err(invalid(
                            path,
                            "persistent hash requires a supported aligned FAB value",
                        ));
                    }
                    Operation::PersistentHash(value)
                };
                (operation, Type::Bytes { length: 32 })
            }
            Expr::PersistentCommit { value, opening } => {
                let value = self.expression(value, scope, bindings, &format!("{path}.value"))?;
                let opening =
                    self.expression(opening, scope, bindings, &format!("{path}.opening"))?;
                if !validate::fixed_aligned_fab_type(&value.ty)
                    || opening.ty != (Type::Bytes { length: 32 })
                {
                    return Err(self.unsupported_expression_reason(
                        expression,
                        path,
                        "persistent commitment requires a fixed aligned FAB value and Bytes32 opening",
                    ));
                }
                (
                    Operation::PersistentCommit { value, opening },
                    Type::Bytes { length: 32 },
                )
            }
            _ => unreachable!("expression dispatcher selects persistent"),
        };
        Ok(value)
    }
}
