// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Ordered kernel effects use exact official ledger8 operand signatures.
use super::*;

impl Lowerer<'_> {
    pub(super) fn kernel(
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
                    format!("expected {expected:?} kernel operand"),
                ));
            }
            Ok(value)
        };
        let operation = match expression {
            Expr::KernelClaim { claim, value } => Operation::KernelClaim {
                kind: *claim,
                value: operand(value, "value", Type::Bytes { length: 32 })?,
            },
            Expr::KernelMintShielded { domain, amount } => {
                let domain = operand(domain, "domain", Type::Bytes { length: 32 })?;
                let amount = operand(amount, "amount", uint("18446744073709551615"))?;
                Operation::KernelMintShielded { domain, amount }
            }
            _ => unreachable!("kernel expression dispatch"),
        };
        Ok((operation, Type::Unit))
    }
}
