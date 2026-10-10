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
        // Bind each expression once, preserving coin-before-recipient effects.
        let coin_path = format!("{path}.coin");
        let coin = self.expression(coin, scope, bindings, &coin_path)?;
        if coin.ty != shielded_coin_type() {
            return Err(invalid(
                &coin_path,
                "coin write requires canonical ShieldedCoinInfo",
            ));
        }
        let recipient_path = format!("{path}.recipient");
        let recipient = self.expression(recipient, scope, bindings, &recipient_path)?;
        if recipient.ty != shielded_recipient_type() {
            return Err(invalid(
                &recipient_path,
                "coin write requires its canonical ledger8 recipient type",
            ));
        }
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
}
