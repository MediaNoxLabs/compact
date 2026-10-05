// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//  http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Canonical frame steps for already evaluated, exactly typed effect operands.
//! This leaf emitter owns no scopes, witnesses, call graph or evaluation order.
use super::*;
use crate::ir::KernelClaimKind;

pub(super) struct EffectStep {
    pub statement: syn::Stmt,
    pub intent: bool,
    pub public_query: bool,
}

pub(super) fn emit(effect: &Expr, operands: &[(Type, syn::Expr)]) -> Option<EffectStep> {
    let (statement, intent, public_query) = match (effect, operands) {
        (Expr::CreateZswapInput { .. }, [(ty, coin)])
            if *ty == crate::stateful::qualified_coin_type() =>
        {
            (
                syn::parse_quote! { let frame = frame.create_zswap_input(
                    runtime::ledger::coin_info_from_compact(#coin.nonce, #coin.color, #coin.value.value()).qualify(#coin.mt_index.value() as u64)
                ); },
                true,
                false,
            )
        }
        (Expr::CreateZswapOutput { .. }, [(coin_ty, coin), (recipient_ty, recipient)])
            if *coin_ty == crate::stateful::shielded_coin_type()
                && *recipient_ty == crate::stateful::shielded_recipient_type() =>
        {
            (
                syn::parse_quote! { let frame = frame.create_zswap_output(
                    runtime::ledger::coin_info_from_compact(#coin.nonce, #coin.color, #coin.value.value()),
                    runtime::ledger::coin_recipient_from_compact(#recipient.is_left, #recipient.left.bytes, #recipient.right.bytes)
                )?; },
                true,
                false,
            )
        }
        (Expr::KernelClaim { claim, .. }, [(Type::Bytes { length: 32 }, value)]) => {
            let (variant, carrier) = match claim {
                KernelClaimKind::Nullifier => ("Nullifier", "CoinNullifier"),
                KernelClaimKind::CoinSpend => ("CoinSpend", "CoinCommitment"),
                KernelClaimKind::CoinReceive => ("CoinReceive", "CoinCommitment"),
            };
            let variant = ident(variant).ok()?;
            let carrier = ident(carrier).ok()?;
            (
                syn::parse_quote! { let frame = frame.kernel_claim(runtime::ledger::KernelClaim::#variant(
                    runtime::ledger::#carrier(runtime::ledger::HashOutput((#value).into_array()))
                ))?; },
                false,
                true,
            )
        }
        (
            Expr::KernelMintShielded { .. },
            [
                (Type::Bytes { length: 32 }, domain),
                (Type::Unsigned { max }, amount),
            ],
        ) if max == &u64::MAX.to_string() => (
            syn::parse_quote! { let frame = frame.kernel_mint_shielded(
                runtime::ledger::HashOutput((#domain).into_array()), (#amount).value() as u64,
            )?; },
            false,
            true,
        ),
        _ => return None,
    };
    Some(EffectStep {
        statement,
        intent,
        public_query,
    })
}
