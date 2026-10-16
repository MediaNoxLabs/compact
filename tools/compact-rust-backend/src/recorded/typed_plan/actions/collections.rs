// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//  	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Collection mutation effects and their original evidence accounting.
//!
//! Insert evidence is recorded before operand lowering, including refused attempts.
//! Other counters retain their original position after successful statement emission.

use crate::coin_shapes::{qualified_coin_type, shielded_coin_type, shielded_recipient_type};
use crate::ident;
use crate::ir::{Expr, LedgerFieldKind, StateAction, Type};
use crate::recorded::typed_plan::{
    CompositeDomain, Plan, Scope, bytes32_key, composition, wrapped_bytes32,
};

impl Plan<'_> {
    pub(in crate::recorded::typed_plan) fn lower_merkle_reset(
        &mut self,
        field: &str,
        index: u8,
        steps: &mut Vec<syn::Stmt>,
    ) -> Option<()> {
        if !matches!(
            self.field(field, index)?.declaration,
            LedgerFieldKind::MerkleTree {
                depth: 10,
                ty: Type::Bytes { length: 32 }
            }
        ) {
            return None;
        }
        let slot = ident(field).ok()?;
        steps.push(syn::parse_quote!(let frame = crate::ledger_slots::#slot.record_reset_to_default(frame)?;));
        self.evidence.tree_writes += 1;
        Some(())
    }

    pub(in crate::recorded::typed_plan) fn lower_collection_insert(
        &mut self,
        action: &StateAction,
        field: &str,
        index: u8,
        value: &Expr,
        scope: &Scope,
        steps: &mut Vec<syn::Stmt>,
    ) -> Option<()> {
        let declaration = self.field(field, index)?.clone();
        match (action, &declaration.declaration) {
            (
                StateAction::MerkleInsert { .. },
                LedgerFieldKind::MerkleTree {
                    ty: Type::Bytes { length: 32 },
                    ..
                },
            ) => self.evidence.tree_writes += 1,
            (StateAction::SetInsert { .. }, LedgerFieldKind::Set { ty })
                if bytes32_key(ty)
                    || (self.admission.composition_calls.is_some()
                        && *ty == Type::OpaqueString) =>
            {
                self.evidence.set_writes += 1
            }
            (StateAction::SetInsert { .. }, LedgerFieldKind::Set { ty })
                if *ty == qualified_coin_type() =>
            {
                self.evidence.qualified_set_writes += 1
            }
            (
                StateAction::HistoricMerkleInsert { .. },
                LedgerFieldKind::HistoricMerkleTree { ty, .. },
            ) if wrapped_bytes32(ty) => self.evidence.historic_writes += 1,
            _ => return None,
        }
        let ty = match &declaration.declaration {
            LedgerFieldKind::MerkleTree { ty, .. }
            | LedgerFieldKind::HistoricMerkleTree { ty, .. }
            | LedgerFieldKind::Set { ty } => ty.clone(),
            _ => return None,
        };
        let value = self.expression(value, scope, steps)?;
        if value.ty != ty {
            return None;
        }
        let slot = ident(field).ok()?;
        let value = value.value;
        steps.push(syn::parse_quote!(let frame = crate::ledger_slots::#slot.record_insert(frame, #value)?;));
        Some(())
    }

    pub(in crate::recorded::typed_plan) fn lower_set_remove(
        &mut self,
        field: &str,
        index: u8,
        value: &Expr,
        scope: &Scope,
        steps: &mut Vec<syn::Stmt>,
    ) -> Option<()> {
        let LedgerFieldKind::Set { ty } = &self.field(field, index)?.declaration else {
            return None;
        };
        if *ty != qualified_coin_type()
            && !(self.admission.composition_calls.is_some() && *ty == Type::OpaqueString)
        {
            return None;
        }
        let ty = ty.clone();
        let value = self.expression(value, scope, steps)?;
        if value.ty != ty {
            return None;
        }
        let slot = ident(field).ok()?;
        let value = value.value;
        steps.push(syn::parse_quote!(let frame = crate::ledger_slots::#slot.record_remove(frame, #value)?;));
        if ty == qualified_coin_type() {
            self.evidence.qualified_set_writes += 1;
        } else {
            self.evidence.set_writes += 1;
        }
        Some(())
    }

    pub(in crate::recorded::typed_plan) fn lower_map_insert(
        &mut self,
        field: &str,
        index: u8,
        key: &Expr,
        value: &Expr,
        scope: &Scope,
        steps: &mut Vec<syn::Stmt>,
    ) -> Option<()> {
        let LedgerFieldKind::Map {
            key: key_ty,
            value: value_ty,
        } = &self.field(field, index)?.declaration
        else {
            return None;
        };
        if *key_ty != Type::OpaqueString
            || !composition::checked_product_map(&self.field(field, index)?.declaration)
        {
            return None;
        }
        let value_ty = value_ty.clone();
        let key = self.expression(key, scope, steps)?;
        if key.ty != Type::OpaqueString {
            return None;
        }
        let value = self.expression(value, scope, steps)?;
        if value.ty != value_ty {
            return None;
        }
        let slot = ident(field).ok()?;
        let (key, value) = (key.value, value.value);
        steps.push(syn::parse_quote!(let frame = crate::ledger_slots::#slot.record_insert(frame, #key, #value)?;));
        Some(())
    }

    pub(in crate::recorded::typed_plan) fn lower_map_remove(
        &mut self,
        field: &str,
        index: u8,
        key: &Expr,
        scope: &Scope,
        steps: &mut Vec<syn::Stmt>,
    ) -> Option<()> {
        if !composition::checked_product_map(&self.field(field, index)?.declaration) {
            return None;
        }
        let key = self.expression(key, scope, steps)?;
        if key.ty != Type::OpaqueString {
            return None;
        }
        let slot = ident(field).ok()?;
        let key = key.value;
        steps.push(
            syn::parse_quote!(let frame = crate::ledger_slots::#slot.record_remove(frame, #key)?;),
        );
        Some(())
    }

    pub(in crate::recorded::typed_plan) fn lower_set_reset(
        &mut self,
        field: &str,
        index: u8,
        steps: &mut Vec<syn::Stmt>,
    ) -> Option<()> {
        let LedgerFieldKind::Set { ty } = &self.field(field, index)?.declaration else {
            return None;
        };
        if self.admission.composite_domain == CompositeDomain::ResetShieldedPayout
            && *ty != (Type::Bytes { length: 32 })
        {
            return None;
        }
        if *ty != qualified_coin_type()
            && !((self.admission.phase_reset
                || self.admission.composite_domain == CompositeDomain::ResetShieldedPayout)
                && *ty == (Type::Bytes { length: 32 }))
        {
            return None;
        }
        let slot = ident(field).ok()?;
        steps.push(syn::parse_quote!(let frame = crate::ledger_slots::#slot.record_reset(frame)?;));
        if *ty == qualified_coin_type() {
            self.evidence.qualified_set_writes += 1;
        } else {
            self.evidence.set_writes += 1;
        }
        Some(())
    }

    pub(in crate::recorded::typed_plan) fn lower_set_coin_insert(
        &mut self,
        field: &str,
        index: u8,
        coin: &Expr,
        recipient: &Expr,
        scope: &Scope,
        steps: &mut Vec<syn::Stmt>,
    ) -> Option<()> {
        let LedgerFieldKind::Set { ty } = &self.field(field, index)?.declaration else {
            return None;
        };
        if *ty != qualified_coin_type() {
            return None;
        }
        let coin = self.expression(coin, scope, steps)?;
        let recipient = self.expression(recipient, scope, steps)?;
        if coin.ty != shielded_coin_type() || recipient.ty != shielded_recipient_type() {
            return None;
        }
        let slot = ident(field).ok()?;
        let coin = coin.value;
        let recipient = recipient.value;
        steps.push(
            syn::parse_quote!(let frame = crate::ledger_slots::#slot.record_insert_coin(
            frame,
            runtime::ledger::coin_info_from_compact(#coin.nonce, #coin.color, #coin.value.value()),
            runtime::ledger::coin_recipient_from_compact(
                #recipient.is_left, #recipient.left.bytes, #recipient.right.bytes,
            ),
        )?;),
        );
        self.evidence.qualified_set_writes += 1;
        Some(())
    }
}
