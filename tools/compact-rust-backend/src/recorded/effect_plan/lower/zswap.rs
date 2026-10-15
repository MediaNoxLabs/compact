// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Source-ordered private intents with exact ledger8 carrier signatures.
use super::*;
use crate::coin_shapes::{qualified_coin_type, shielded_coin_type, shielded_recipient_type};

impl Lowerer<'_> {
    pub(super) fn zswap(
        &mut self,
        expression: &Expr,
        scope: &Scope,
        bindings: &mut Vec<Binding>,
        path: &str,
    ) -> Result<(Operation, Type), RecordingGap> {
        let mut operand = |expression, member, expected| {
            let path = format!("{path}.{member}");
            let value = self.expression(expression, scope, bindings, &path)?;
            if value.ty != expected {
                return Err(invalid(
                    &path,
                    "Zswap intent requires its canonical ledger8 type",
                ));
            }
            Ok(value)
        };
        let operation = match expression {
            Expr::CreateZswapInput { coin } => Operation::CreateZswapInput {
                coin: operand(coin, "coin", qualified_coin_type())?,
            },
            Expr::CreateZswapOutput { coin, recipient } => {
                let coin = operand(coin, "coin", shielded_coin_type())?;
                let recipient = operand(recipient, "recipient", shielded_recipient_type())?;
                Operation::CreateZswapOutput { coin, recipient }
            }
            _ => unreachable!("Zswap expression dispatch"),
        };
        Ok((operation, Type::Unit))
    }
}
