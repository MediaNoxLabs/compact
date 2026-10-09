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

//! Ledger observations and their existing typed recording counters and policies.
//!
//! The shared Plan owns mutable state and profile admission. This module keeps
//! its original method body and recursively calls the same expression dispatcher.

use super::{
    CompositeDomain, Plan, Scope, TypedValue, bytes32_key, cell_type, composition, guarded_deposit,
    optional_string, phase_reset, reset_payout, shielded_payout, start_funding, unit_actions,
    wrapped_bytes32,
};
use crate::coin_shapes::qualified_coin_type;
use crate::ident;
use crate::ir::{Expr, LedgerFieldKind, Type};

impl Plan<'_> {
    pub(super) fn observation_expression(
        &mut self,
        expression: &Expr,
        scope: &Scope,
        steps: &mut Vec<syn::Stmt>,
    ) -> Option<TypedValue> {
        match expression {
            Expr::CounterLessThan {
                field,
                index,
                threshold,
            } => {
                if self.field(field, *index)?.declaration != LedgerFieldKind::Counter {
                    return None;
                }
                let threshold = self.expression(threshold, scope, steps)?;
                if threshold.ty
                    != (Type::Unsigned {
                        max: u64::MAX.to_string(),
                    })
                {
                    return None;
                }
                let threshold = threshold.value;
                let slot = ident(field).ok()?;
                let observed = self.fresh();
                steps.push(syn::parse_quote!(let (frame, #observed) = crate::ledger_slots::#slot.record_less_than(frame, (#threshold).value() as u64)?;));
                self.evidence.counter_comparisons += 1;
                self.bind(syn::parse_quote!(#observed), Type::Boolean, steps)
            }
            Expr::CounterRead { field, index } => {
                if self.field(field, *index)?.declaration != LedgerFieldKind::Counter {
                    return None;
                }
                let slot = ident(field).ok()?;
                let observed = self.fresh();
                steps.push(syn::parse_quote!(let (frame, #observed) = crate::ledger_slots::#slot.record_read(frame)?;));
                self.evidence.counter_reads += 1;
                if self.evaluation.scalar_body_depth > 0 {
                    self.evidence.scalar_counter_reads += 1;
                }
                self.bind(syn::parse_quote!(runtime::BoundedUint::<18446744073709551615>::new(#observed as u128)?), Type::Unsigned { max: "18446744073709551615".into() }, steps)
            }
            Expr::CellRead { field, index } => {
                let declaration = self.field(field, *index)?;
                let LedgerFieldKind::Cell { ty } = &declaration.declaration else {
                    return None;
                };
                if self.admission.composite_domain == CompositeDomain::ShieldedPayout
                    && !shielded_payout::cell_type(ty)
                {
                    return None;
                }
                if self.admission.composite_domain == CompositeDomain::ActionfulShieldedPayout
                    && !shielded_payout::actionful_cell_type(ty)
                {
                    return None;
                }
                if self.admission.effectful_field_cells && *ty != Type::Field {
                    return None;
                }
                if self.admission.read_only_assertions && *ty != Type::Boolean {
                    return None;
                }
                if matches!(
                    self.admission.composite_domain,
                    CompositeDomain::GuardedShieldedDeposit | CompositeDomain::FundedShieldedMint
                ) && !guarded_deposit::read_type(ty)
                {
                    return None;
                }
                if self.admission.composite_domain == CompositeDomain::ResetShieldedPayout
                    && !reset_payout::read_type(ty)
                {
                    return None;
                }
                if self.admission.composite_domain == CompositeDomain::StartFunding
                    && !start_funding::cell_type(ty)
                {
                    return None;
                }
                if !cell_type(ty)
                    && !(self.admission.composition_calls.is_some()
                        && composition::observed_value_type(ty))
                    && !(matches!(
                        self.admission.composite_domain,
                        CompositeDomain::GuardedShieldedDeposit
                            | CompositeDomain::FundedShieldedMint
                    ) && guarded_deposit::read_type(ty))
                    && !(self.admission.composite_domain == CompositeDomain::ResetShieldedPayout
                        && reset_payout::read_type(ty))
                    && !(self.admission.composite_domain == CompositeDomain::StartFunding
                        && start_funding::cell_type(ty))
                    && !(self.admission.composite_domain == CompositeDomain::ShieldedPayout
                        && shielded_payout::cell_type(ty))
                    && !(self.admission.composite_domain
                        == CompositeDomain::ActionfulShieldedPayout
                        && shielded_payout::actionful_cell_type(ty))
                    && !(self.admission.unit_actions && unit_actions::value_type(ty))
                    && !(self.admission.phase_reset && phase_reset::cell_type(ty))
                    && !(self.admission.read_only_assertions && *ty == Type::Boolean)
                    && !(ty == &Type::Field
                        && (self.admission.effectful_field_cells
                            || self.admission.composite_domain
                                == CompositeDomain::FieldObservations
                            || self.admission.field_cell_slot.as_ref()
                                == Some(&(field.clone(), *index))))
                {
                    return None;
                }
                let ty = ty.clone();
                self.evidence.cell_reads += 1;
                if optional_string(&ty) {
                    self.evidence.optional_cells += 1;
                }
                self.observe(field, "record_read", vec![], ty, steps)
            }
            Expr::SetMember {
                field,
                index,
                value,
            } => {
                let LedgerFieldKind::Set { ty } = &self.field(field, *index)?.declaration else {
                    return None;
                };
                let ty = ty.clone();
                if !bytes32_key(&ty)
                    && ty != qualified_coin_type()
                    && !(self.admission.composition_calls.is_some() && ty == Type::OpaqueString)
                {
                    return None;
                }
                if ty == qualified_coin_type() {
                    self.evidence.qualified_set_reads += 1;
                }
                let value = self.expression(value, scope, steps)?;
                if value.ty != ty {
                    return None;
                }
                self.observe(
                    field,
                    "record_member",
                    vec![value.value],
                    Type::Boolean,
                    steps,
                )
            }
            Expr::MapMember { field, index, key } if self.admission.composition_calls.is_some() => {
                self.composition_map_member(field, *index, key, scope, steps)
            }
            Expr::MapLookup { field, index, key }
                if self.admission.composition_calls.is_some()
                    && self
                        .admission
                        .composition_lookup_sites
                        .contains(&(expression as *const Expr)) =>
            {
                self.composition_map_lookup(field, *index, key, scope, steps)
            }
            Expr::SetSize { field, index } | Expr::SetIsEmpty { field, index } => {
                let LedgerFieldKind::Set { ty } = &self.field(field, *index)?.declaration else {
                    return None;
                };
                if *ty != qualified_coin_type() {
                    return None;
                }
                self.evidence.qualified_set_reads += 1;
                if matches!(expression, Expr::SetSize { .. }) {
                    let slot = ident(field).ok()?;
                    let name = self.fresh();
                    steps.push(syn::parse_quote!(let (frame, #name) = crate::ledger_slots::#slot.record_size(frame)?;));
                    self.bind(syn::parse_quote!(runtime::BoundedUint::<18446744073709551615>::new(#name as u128)?), Type::Unsigned { max: u64::MAX.to_string() }, steps)
                } else {
                    self.observe(field, "record_is_empty", vec![], Type::Boolean, steps)
                }
            }
            Expr::HistoricMerkleCheckRoot { field, index, root } => {
                if !matches!(&self.field(field, *index)?.declaration, LedgerFieldKind::HistoricMerkleTree { ty, .. } if wrapped_bytes32(ty))
                {
                    return None;
                }
                let root = self.expression(root, scope, steps)?;
                if !matches!(&root.ty, Type::Struct { fields, .. } if matches!(fields.as_slice(), [member] if member.ty == Type::Field))
                {
                    return None;
                }
                self.evidence.historic_roots += 1;
                self.observe(
                    field,
                    "record_check_root",
                    vec![root.value],
                    Type::Boolean,
                    steps,
                )
            }
            Expr::MerkleCheckRoot { field, index, root } => {
                if !matches!(
                    self.field(field, *index)?.declaration,
                    LedgerFieldKind::MerkleTree {
                        ty: Type::Bytes { length: 32 },
                        ..
                    }
                ) {
                    return None;
                }
                let root = self.expression(root, scope, steps)?;
                if !matches!(&root.ty, Type::Struct { fields, .. } if matches!(fields.as_slice(), [member] if member.ty == Type::Field))
                {
                    return None;
                }
                self.evidence.root_observations += 1;
                self.observe(
                    field,
                    "record_check_root",
                    vec![root.value],
                    Type::Boolean,
                    steps,
                )
            }
            _ => None,
        }
    }
}
