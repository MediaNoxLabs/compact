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

//! Cell assignments and qualified-coin cell writes.

use crate::coin_shapes::{qualified_coin_type, shielded_coin_type, shielded_recipient_type};
use crate::ident;
use crate::ir::{Expr, LedgerFieldKind, Type};
use crate::recorded::typed_plan::{
    CompositeDomain, Plan, Scope, cell_type, composition, guarded_deposit, optional_string,
    phase_reset, reset_payout, start_funding, unit_actions,
};

impl Plan<'_> {
    pub(in crate::recorded::typed_plan) fn lower_cell_write(
        &mut self,
        field: &str,
        index: u8,
        value: &Expr,
        scope: &Scope,
        steps: &mut Vec<syn::Stmt>,
    ) -> Option<()> {
        let LedgerFieldKind::Cell { ty } = &self.field(field, index)?.declaration else {
            return None;
        };
        if self.admission.effectful_field_cells && *ty != Type::Field {
            return None;
        }
        if self.admission.composite_domain == CompositeDomain::FundedShieldedMint
            && *ty != Type::Boolean
        {
            return None;
        }
        if self.admission.composite_domain == CompositeDomain::GuardedShieldedDeposit
            && !guarded_deposit::write_type(ty)
        {
            return None;
        }
        if self.admission.composite_domain == CompositeDomain::ResetShieldedPayout
            && !reset_payout::write_type(ty)
        {
            return None;
        }
        if self.admission.composite_domain == CompositeDomain::StartFunding
            && !start_funding::cell_type(ty)
        {
            return None;
        }
        if !cell_type(ty)
            && !(self.admission.composite_domain == CompositeDomain::JubjubValues
                && *ty == Type::JubjubPoint)
            && !(self.admission.composite_domain == CompositeDomain::ResetShieldedPayout
                && reset_payout::write_type(ty))
            && !(self.admission.composite_domain == CompositeDomain::GuardedShieldedDeposit
                && guarded_deposit::write_type(ty))
            && !(self.admission.composite_domain == CompositeDomain::StartFunding
                && start_funding::cell_type(ty))
            && !(self.admission.unit_actions && unit_actions::value_type(ty))
            && !(self.admission.composition_calls.is_some() && composition::write_type(ty))
            && !(self.admission.phase_reset && phase_reset::cell_type(ty))
            && !(ty == &Type::Field
                && (self.admission.effectful_field_cells
                    || self.admission.field_cell_slot.as_ref() == Some(&(field.to_owned(), index))))
        {
            return None;
        }
        let ty = ty.clone();
        let value = self.expression(value, scope, steps)?;
        if value.ty != ty {
            return None;
        }
        let value = value.value;
        let slot = ident(field).ok()?;
        steps.push(
            syn::parse_quote!(let frame = crate::ledger_slots::#slot.record_write(frame, #value)?;),
        );
        self.evidence.cell_writes += 1;
        if ty == Type::Field {
            self.evidence.field_cell_writes += 1;
        }
        if ty == Type::OpaqueBytes {
            self.evidence.opaque_cells += 1;
        }
        if optional_string(&ty) {
            self.evidence.optional_cells += 1;
        }
        Some(())
    }

    pub(in crate::recorded::typed_plan) fn lower_cell_coin_write(
        &mut self,
        field: &str,
        index: u8,
        coin: &Expr,
        recipient: &Expr,
        scope: &Scope,
        steps: &mut Vec<syn::Stmt>,
    ) -> Option<()> {
        let LedgerFieldKind::Cell { ty } = &self.qualified_cell_field(field, index)?.declaration
        else {
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
            syn::parse_quote!(let frame = crate::ledger_slots::#slot.record_write_coin(
            frame,
            runtime::ledger::coin_info_from_compact(#coin.nonce, #coin.color, #coin.value.value()),
            runtime::ledger::coin_recipient_from_compact(
                #recipient.is_left, #recipient.left.bytes, #recipient.right.bytes,
            ),
        )?;),
        );
        self.evidence.qualified_cell_writes += 1;
        Some(())
    }
}
