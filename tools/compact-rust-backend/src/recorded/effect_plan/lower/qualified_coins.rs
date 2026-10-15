// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Qualified storage writes leave commitment allocation to the official ledger.
use super::*;
use crate::coin_shapes::{qualified_coin_type, shielded_coin_type, shielded_recipient_type};

impl Lowerer<'_> {
    pub(super) fn cell_write_coin(
        &mut self,
        action: &StateAction,
        scope: &Scope,
        bindings: &mut Vec<Binding>,
        path: &str,
    ) -> Result<(), RecordingGap> {
        let StateAction::CellWriteCoin {
            field,
            index,
            coin,
            recipient,
        } = action
        else {
            unreachable!("qualified Cell write dispatch")
        };
        let (slot, declared) = self.cell(field, *index, path)?;
        if declared != qualified_coin_type() {
            return Err(invalid(
                path,
                "coin write requires Cell<QualifiedShieldedCoinInfo>",
            ));
        }
        let (coin, recipient) =
            self.qualified_coin_operands(coin, recipient, scope, bindings, path, "coin write")?;
        self.emit(
            Operation::CellWriteCoin {
                slot,
                coin,
                recipient,
            },
            Type::Unit,
            bindings,
            path,
        );
        Ok(())
    }

    pub(super) fn set_insert_coin(
        &mut self,
        action: &StateAction,
        scope: &Scope,
        bindings: &mut Vec<Binding>,
        path: &str,
    ) -> Result<(), RecordingGap> {
        let StateAction::SetInsertCoin {
            field,
            index,
            coin,
            recipient,
        } = action
        else {
            unreachable!("qualified Set insertion dispatch")
        };
        let (slot, declared) = self.set(field, *index, path)?;
        if declared != qualified_coin_type() {
            return Err(invalid(
                path,
                "coin insertion requires Set<QualifiedShieldedCoinInfo>",
            ));
        }
        let (coin, recipient) =
            self.qualified_coin_operands(coin, recipient, scope, bindings, path, "coin insertion")?;
        self.emit(
            Operation::SetInsertCoin {
                slot,
                coin,
                recipient,
            },
            Type::Unit,
            bindings,
            path,
        );
        Ok(())
    }

    fn qualified_coin_operands(
        &mut self,
        coin: &Expr,
        recipient: &Expr,
        scope: &Scope,
        bindings: &mut Vec<Binding>,
        path: &str,
        operation: &str,
    ) -> Result<(Value, Value), RecordingGap> {
        // Bind each expression once, preserving coin-before-recipient effects.
        let coin_path = format!("{path}.coin");
        let coin = self.expression(coin, scope, bindings, &coin_path)?;
        if coin.ty != shielded_coin_type() {
            return Err(invalid(
                &coin_path,
                format!("{operation} requires canonical ShieldedCoinInfo"),
            ));
        }
        let recipient_path = format!("{path}.recipient");
        let recipient = self.expression(recipient, scope, bindings, &recipient_path)?;
        if recipient.ty != shielded_recipient_type() {
            return Err(invalid(
                &recipient_path,
                format!("{operation} requires its canonical ledger8 recipient type"),
            ));
        }
        Ok((coin, recipient))
    }
}
