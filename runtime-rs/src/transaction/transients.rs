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

//! Explicit upstream transient identities and cross-kind source provenance.
use super::{ContractAddress, DB, DefaultDB, Offer, ProofPreimage, ZswapIntentError};
use crate::ledger::{CoinInfo, CoinRecipient};
use crate::zswap::{CircuitZswapPlan, IntentEvent};
use midnight_coin_structure::transfer::SenderEvidence;
use midnight_transient_crypto::curve::Fr;
use midnight_zswap::Transient;
use std::collections::HashSet;

/// Caller-selected complete upstream transient coins. This is an identity and
/// intent policy, not proof validation. Both retained proofs are proved and
/// checked by the final ledger transaction. Only guaranteed-segment contract
/// transients are admitted; historical inputs keep their ordinary tree checks.
pub struct ContractTransientCoins<D: DB = DefaultDB> {
    coins: Vec<Transient<ProofPreimage, D>>,
}
impl<D: DB> ContractTransientCoins<D> {
    pub fn from_transients(
        coins: Vec<Transient<ProofPreimage, D>>,
    ) -> Result<Self, ZswapIntentError> {
        if coins.is_empty() {
            return Err(ZswapIntentError::TransientSelectionMismatch);
        }
        let mut commitments = HashSet::new();
        let mut nullifiers = HashSet::new();
        for coin in &coins {
            if coin.contract_address.is_none() {
                return Err(ZswapIntentError::TransientOwnerMismatch);
            }
            // Pinned ledger8 spend/output public return shapes. `segment()`
            // alone maps malformed conversions to None, so inspect both full
            // return shapes and the guaranteed-segment tag explicitly.
            if coin.proof_input.public_transcript_outputs != [Fr::from(1), Fr::from(0)]
                || coin.proof_output.public_transcript_outputs != [Fr::from(0)]
            {
                return Err(ZswapIntentError::TransientSegmentMismatch);
            }
            if !commitments.insert(coin.coin_com) || !nullifiers.insert(coin.nullifier) {
                return Err(ZswapIntentError::TransientDuplicate);
            }
        }
        Ok(Self { coins })
    }

    pub(super) fn validate_offer(
        &self,
        offer: &Offer<ProofPreimage, D>,
        address: ContractAddress,
    ) -> Result<(), ZswapIntentError> {
        if self.coins.len() != offer.transient.len() {
            return Err(ZswapIntentError::TransientSelectionMismatch);
        }
        for selected in &self.coins {
            if selected.contract_address.as_ref().map(|owner| **owner) != Some(address) {
                return Err(ZswapIntentError::TransientOwnerMismatch);
            }
            // Full Eq includes both complete proof preimages, ciphertext and
            // value commitments. Matching only coin_com/nullifier is unsound.
            if !offer.transient.iter_deref().any(|coin| coin == selected)
                || offer
                    .outputs
                    .iter_deref()
                    .any(|output| output.coin_com == selected.coin_com)
                || offer
                    .inputs
                    .iter_deref()
                    .any(|input| input.nullifier == selected.nullifier)
            {
                return Err(ZswapIntentError::TransientSelectionMismatch);
            }
        }
        let mut commitments = HashSet::new();
        let mut nullifiers = HashSet::new();
        for coin in offer.transient.iter_deref() {
            if !commitments.insert(coin.coin_com) || !nullifiers.insert(coin.nullifier) {
                return Err(ZswapIntentError::TransientDuplicate);
            }
        }
        Ok(())
    }

    /// Return the exact input positions accounted for by selected transients.
    /// Remaining inputs are reconciled as historical inputs by the caller.
    pub(super) fn reconcile_events(
        &self,
        plan: &CircuitZswapPlan,
        address: ContractAddress,
    ) -> Result<HashSet<usize>, ZswapIntentError> {
        let mut input_order = Vec::new();
        let mut output_order = Vec::new();
        for (order, event) in plan.events.iter().enumerate() {
            match *event {
                IntentEvent::Input(index)
                    if index == input_order.len() && index < plan.inputs.len() =>
                {
                    input_order.push(order)
                }
                IntentEvent::Output(index)
                    if index == output_order.len() && index < plan.outputs.len() =>
                {
                    output_order.push(order)
                }
                _ => return Err(ZswapIntentError::TransientEventMismatch),
            }
        }
        if input_order.len() != plan.inputs.len() || output_order.len() != plan.outputs.len() {
            return Err(ZswapIntentError::TransientEventMismatch);
        }
        let owner = CoinRecipient::Contract(address);
        let mut matched_inputs = HashSet::new();
        for transient in &self.coins {
            let outputs = plan
                .outputs
                .iter()
                .enumerate()
                .filter(|(_, output)| {
                    output.recipient == owner
                        && output.coin.commitment(&owner) == transient.coin_com
                })
                .collect::<Vec<_>>();
            let [(output_index, output)] = outputs.as_slice() else {
                return Err(ZswapIntentError::TransientInputMismatch);
            };
            let inputs = plan
                .inputs
                .iter()
                .enumerate()
                .filter(|(_, input)| {
                    CoinInfo::from(*input).commitment(&owner) == transient.coin_com
                })
                .collect::<Vec<_>>();
            let [(input_index, input)] = inputs.as_slice() else {
                return Err(ZswapIntentError::TransientInputMismatch);
            };
            if input.mt_index != 0
                || CoinInfo::from(*input) != output.coin
                || output.coin.nullifier(&SenderEvidence::Contract(address)) != transient.nullifier
                || output_order[*output_index] >= input_order[*input_index]
                || !matched_inputs.insert(*input_index)
            {
                return Err(ZswapIntentError::TransientInputMismatch);
            }
        }
        Ok(matched_inputs)
    }
}
