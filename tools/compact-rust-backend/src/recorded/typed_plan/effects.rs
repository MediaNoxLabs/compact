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

//! Witness, kernel and shielded effects in the existing typed recording order.
//!
//! The shared Plan owns mutable state and profile admission. This module keeps
//! its original method body and recursively calls the same expression dispatcher.

use super::{
    CompositeDomain, Plan, Scope, TypedValue, contract_address_type, shielded_payout, start_funding,
};
use crate::ir::{Expr, Type};
use crate::recorded::{intent_effect, recordable_cell_type};
use crate::{ident, rust_type};

impl Plan<'_> {
    pub(super) fn effect_expression(
        &mut self,
        expression: &Expr,
        scope: &Scope,
        steps: &mut Vec<syn::Stmt>,
    ) -> Option<TypedValue> {
        match expression {
            Expr::KernelMintShielded { domain, amount }
                if self.admission.composite_domain == CompositeDomain::FundedShieldedMint =>
            {
                self.funded_mint(expression, domain, amount, scope, steps)
            }
            Expr::CreateZswapInput { .. }
            | Expr::CreateZswapOutput { .. }
            | Expr::KernelClaim { .. }
                if self.admission.composite_domain.intents() =>
            {
                let operands: Vec<&Expr> = match expression {
                    Expr::CreateZswapInput { coin } => vec![coin],
                    Expr::CreateZswapOutput { coin, recipient } => vec![coin, recipient],
                    Expr::KernelClaim { value, .. } => vec![value],
                    _ => unreachable!(),
                };
                let operands = operands
                    .into_iter()
                    .map(|operand| {
                        let value = self.expression(operand, scope, steps)?;
                        Some((value.ty, value.value))
                    })
                    .collect::<Option<Vec<_>>>()?;
                let effect = intent_effect::emit(expression, &operands)?;
                self.evidence.intent_effects += usize::from(effect.intent);
                self.evidence.intent_queries += usize::from(effect.public_query);
                self.evidence.zswap_inputs +=
                    usize::from(matches!(expression, Expr::CreateZswapInput { .. }));
                self.evidence.zswap_outputs +=
                    usize::from(matches!(expression, Expr::CreateZswapOutput { .. }));
                steps.push(effect.statement);
                self.bind(syn::parse_quote!(()), Type::Unit, steps)
            }
            Expr::KernelSelf { ty } if *ty == contract_address_type() => {
                self.evidence.kernel_self_reads += 1;
                let rust_ty = rust_type(ty).ok()?;
                let observed = self.fresh();
                steps.push(syn::parse_quote!(let (frame, #observed) = frame.kernel_self()?;));
                self.bind(
                    syn::parse_quote!(#rust_ty {
                        bytes: runtime::ledger::contract_address_bytes(&#observed)
                    }),
                    ty.clone(),
                    steps,
                )
            }
            Expr::WitnessCall { name, arguments } => {
                let declaration = *self.declarations.witnesses.get(name.as_str())?;
                if self.admission.composite_domain == CompositeDomain::ResetShieldedPayout {
                    return None;
                }
                if self.admission.composite_domain == CompositeDomain::GuardedShieldedDeposit
                    && (declaration.result != (Type::Bytes { length: 32 })
                        || !declaration.parameters.is_empty())
                {
                    return None;
                }
                if self.admission.composite_domain == CompositeDomain::ShieldedPayout
                    && declaration.result != (Type::Bytes { length: 32 })
                {
                    return None;
                }
                if self.admission.composite_domain == CompositeDomain::ActionfulShieldedPayout
                    && !shielded_payout::witness_type(&declaration.result)
                {
                    return None;
                }
                if self.admission.composite_domain == CompositeDomain::StartFunding
                    && !start_funding::witness_type(&declaration.result)
                {
                    return None;
                }
                if arguments.len() != declaration.parameters.len()
                    || !(matches!(declaration.result, Type::Unit | Type::OpaqueBytes)
                        || recordable_cell_type(&declaration.result))
                {
                    return None;
                }
                let args = self.arguments(arguments, &declaration.parameters, scope, steps)?;
                let method = ident(name).ok()?;
                self.evidence.witness_calls += 1;
                let observed = self.fresh();
                let ty = rust_type(&declaration.result).ok()?;
                let frame_method = crate::hash_input::witness_method(&declaration.result);
                steps.push(syn::parse_quote! {
                    let (frame, #observed): (_, #ty) = frame.#frame_method(|context, meter| {
                        witnesses.#method(context.witness_context_with(super::LedgerView {
                            state: context.query.state.get_ref(), meter,
                        }), #(#args),*)
                    })?;
                });
                Some(TypedValue {
                    ty: declaration.result.clone(),
                    value: syn::parse_quote!(#observed),
                })
            }
            Expr::NativeWitnessCall {
                builtin: builtin @ crate::ir::NativeWitnessBuiltin::OwnPublicKey,
            } if matches!(
                self.admission.composite_domain,
                CompositeDomain::ShieldedPayout
                    | CompositeDomain::ActionfulShieldedPayout
                    | CompositeDomain::FundedShieldedMint
                    | CompositeDomain::ResetShieldedPayout
            ) =>
            {
                let ty = builtin.result_type();
                let rust_ty = rust_type(&ty).ok()?;
                let observed = self.fresh();
                steps.push(
                    syn::parse_quote!(let (frame, #observed) = frame.own_coin_public_key()?;),
                );
                self.bind(
                    syn::parse_quote!(#rust_ty { bytes: runtime::FixedBytes::new(#observed) }),
                    ty,
                    steps,
                )
            }
            Expr::Call { name, arguments } => self.call(name, arguments, scope, steps),
            _ => None,
        }
    }
}
