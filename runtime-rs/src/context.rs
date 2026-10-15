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

//! Compact-level envelopes around ledger query and shielded state.
//!
//! Ledger state, query, cost, and wallet Zswap values reuse upstream types.
//! This module also owns a separate ordered circuit intent plan, ties execution
//! to private state, and carries ownership across constructor and circuit calls.

mod cell;
mod counter;
mod frame;
mod list;
mod local_boundary;
mod map;
mod set;
mod trees;
mod witness;

pub use frame::CircuitFrame;
pub use witness::{WitnessContext, WitnessReadMeter};

pub use midnight_base_crypto::cost_model::RunningCost;
use midnight_base_crypto::fab::AlignedValue;
use midnight_base_crypto::hash::HashOutput;
pub use midnight_coin_structure::coin::PublicKey as CoinPublicKey;
use midnight_onchain_vm::cost_model::{CostModel, INITIAL_COST_MODEL};
use midnight_zswap::local::State as ZswapLocalState;

use crate::ledger::{
    ChargedState, ContractAddress, ContractState, DB, DefaultDB, QueryContext, StateValue,
};
use crate::{CompactError, ledger};

pub struct ConstructorContext<Private, D: DB = DefaultDB> {
    pub private_state: Private,
    pub zswap_state: ZswapLocalState<D>,
    pub(crate) circuit_zswap: crate::CircuitZswapPlan,
}

impl<Private, D: DB> ConstructorContext<Private, D> {
    pub fn circuit_zswap(&self) -> &crate::CircuitZswapPlan {
        &self.circuit_zswap
    }
    pub fn new(private_state: Private) -> Self {
        Self {
            private_state,
            zswap_state: ZswapLocalState::default(),
            circuit_zswap: crate::CircuitZswapPlan::default(),
        }
    }
}

pub struct ConstructorResult<Private, D: DB = DefaultDB> {
    pub ledger_state: ChargedState<D>,
    pub private_state: Private,
    pub zswap_state: ZswapLocalState<D>,
    pub(crate) circuit_zswap: crate::CircuitZswapPlan,
}

impl<Private, D: DB> ConstructorResult<Private, D> {
    pub fn circuit_zswap(&self) -> &crate::CircuitZswapPlan {
        &self.circuit_zswap
    }
    pub fn new(context: ConstructorContext<Private, D>, ledger_state: ChargedState<D>) -> Self {
        Self {
            ledger_state,
            private_state: context.private_state,
            zswap_state: context.zswap_state,
            circuit_zswap: context.circuit_zswap,
        }
    }

    pub fn into_circuit_context(self, address: ContractAddress) -> CircuitContext<Private, D> {
        let mut query = QueryContext::new(self.ledger_state, address);
        for output in self.circuit_zswap.outputs() {
            query.call_context.com_indices = query.call_context.com_indices.insert(
                output.coin.commitment(&output.recipient),
                output.provisional_index,
            );
        }
        CircuitContext {
            private_state: self.private_state,
            query,
            zswap_state: self.zswap_state,
            circuit_zswap: self.circuit_zswap,
            coin_public_key: None,
            cost_model: INITIAL_COST_MODEL.clone(),
            gas_limit: None,
        }
    }
}

pub struct CircuitContext<Private, D: DB = DefaultDB> {
    pub private_state: Private,
    pub query: QueryContext<D>,
    pub zswap_state: ZswapLocalState<D>,
    pub(crate) circuit_zswap: crate::CircuitZswapPlan,
    coin_public_key: Option<CoinPublicKey>,
    pub cost_model: CostModel,
    /// Ledger-8 VM guard for each individual query, including witness reads.
    /// CircuitResult::gas_cost separately sums the accepted query costs.
    pub gas_limit: Option<RunningCost>,
}

pub struct CircuitResult<Private, Output, D: DB = DefaultDB> {
    pub context: CircuitContext<Private, D>,
    pub result: Output,
    /// Cost of the operations represented by this result. Generated circuits
    /// add the cost of each VM query rather than retaining only the last one.
    pub gas_cost: RunningCost,
    pub private_transcript_outputs: Vec<AlignedValue>,
}

impl<Private, D: DB> CircuitContext<Private, D> {
    /// Associate the callers pinned ledger coin key with native witness calls.
    pub fn with_coin_public_key(mut self, key: CoinPublicKey) -> Self {
        self.coin_public_key = Some(key);
        self
    }

    /// Use the 32-byte Compact representation of a ledger coin public key.
    pub fn with_coin_public_key_bytes(self, bytes: [u8; 32]) -> Self {
        self.with_coin_public_key(CoinPublicKey(HashOutput(bytes)))
    }

    pub fn kernel_claim_zswap_nullifier(
        self,
        value: ledger::CoinNullifier,
    ) -> Result<CircuitResult<Private, (), D>, CompactError> {
        self.kernel_claim(ledger::KernelClaim::Nullifier(value))
    }
    pub fn kernel_claim_zswap_coin_spend(
        self,
        value: ledger::CoinCommitment,
    ) -> Result<CircuitResult<Private, (), D>, CompactError> {
        self.kernel_claim(ledger::KernelClaim::CoinSpend(value))
    }
    pub fn kernel_claim_zswap_coin_receive(
        self,
        value: ledger::CoinCommitment,
    ) -> Result<CircuitResult<Private, (), D>, CompactError> {
        self.kernel_claim(ledger::KernelClaim::CoinReceive(value))
    }
    fn kernel_claim(
        mut self,
        claim: ledger::KernelClaim,
    ) -> Result<CircuitResult<Private, (), D>, CompactError> {
        let result =
            ledger::query_kernel_claim(&self.query, claim, self.gas_limit, &self.cost_model)?;
        self.query = result.context;
        Ok(CircuitResult {
            context: self,
            result: (),
            gas_cost: result.gas_cost,
            private_transcript_outputs: vec![],
        })
    }
    /// Record a shielded mint effect; transaction validation still governs supply/balancing.
    pub fn kernel_mint_shielded(
        mut self,
        domain: ledger::HashOutput,
        amount: u64,
    ) -> Result<CircuitResult<Private, (), D>, CompactError> {
        let result = ledger::query_kernel_mint_shielded(
            &self.query,
            domain,
            amount,
            self.gas_limit,
            &self.cost_model,
        )?;
        self.query = result.context;
        Ok(CircuitResult {
            context: self,
            result: (),
            gas_cost: result.gas_cost,
            private_transcript_outputs: vec![],
        })
    }

    /// The native circuit intent log, separate from the wallet's Zswap state.
    pub fn circuit_zswap(&self) -> &crate::CircuitZswapPlan {
        &self.circuit_zswap
    }

    /// Configure a provisional native output cursor before producing outputs.
    pub fn set_zswap_output_start(&mut self, index: u64) -> Result<(), CompactError> {
        if self.circuit_zswap.allocation_locked() {
            return Err(CompactError::ZswapAllocationLocked);
        }
        if !self.circuit_zswap.outputs.is_empty() {
            return Err(CompactError::ZswapCursorAlreadyUsed);
        }
        self.circuit_zswap.next_index = index;
        Ok(())
    }

    #[cfg(all(test, feature = "ledger-transaction"))]
    pub(crate) fn lock_zswap_allocation(mut self) -> Self {
        self.circuit_zswap.allocation = crate::zswap::Allocation::Locked;
        self
    }

    /// Append an input intent; this does not spend a wallet coin or validate an offer.
    pub fn create_zswap_input(&mut self, coin: ledger::QualifiedCoinInfo) {
        self.circuit_zswap
            .events
            .push(crate::zswap::IntentEvent::Input(
                self.circuit_zswap.inputs.len(),
            ));
        self.circuit_zswap.inputs.push(coin);
    }

    /// Append an output intent with a provisional native index or an exact offer-bound index.
    /// Ordinary observations remain locked; authoritative commitment maps are never modified.
    pub fn create_zswap_output(
        &mut self,
        coin: ledger::CoinInfo,
        recipient: ledger::CoinRecipient,
    ) -> Result<(), CompactError> {
        use crate::zswap::Allocation;
        let commitment = coin.commitment(&recipient);
        let index = match &self.circuit_zswap.allocation {
            #[cfg(feature = "ledger-transaction")]
            Allocation::Locked => return Err(CompactError::ZswapAllocationLocked),
            #[cfg(feature = "ledger-transaction")]
            Allocation::OfferBound { outputs, .. } => {
                if outputs.get(self.circuit_zswap.outputs.len())
                    != Some(&(commitment, self.circuit_zswap.next_index))
                    || self
                        .query
                        .call_context
                        .com_indices
                        .get(&commitment)
                        .copied()
                        != Some(self.circuit_zswap.next_index)
                {
                    return Err(CompactError::ZswapOfferOutputMismatch);
                }
                self.circuit_zswap.next_index
            }
            #[cfg(feature = "ledger-transaction")]
            Allocation::CanonicalOfferBound { outputs, .. } => {
                let row = outputs
                    .iter()
                    .find(|row| row.commitment == commitment)
                    .ok_or(CompactError::ZswapOfferOutputMismatch)?;
                if !row.matches_recipient(&recipient)
                    || self
                        .circuit_zswap
                        .outputs
                        .iter()
                        .any(|intent| intent.coin.commitment(&intent.recipient) == commitment)
                    || self
                        .query
                        .call_context
                        .com_indices
                        .get(&commitment)
                        .copied()
                        != Some(row.index)
                {
                    return Err(CompactError::ZswapOfferOutputMismatch);
                }
                row.index
            }
            Allocation::Provisional => self.circuit_zswap.next_index,
        };
        let next = self
            .circuit_zswap
            .next_index
            .checked_add(1)
            .ok_or(CompactError::ZswapCursorOverflow)?;
        if self.circuit_zswap.allocation == Allocation::Provisional {
            self.query.call_context.com_indices = self
                .query
                .call_context
                .com_indices
                .insert(commitment, self.circuit_zswap.next_index);
        }
        self.circuit_zswap.next_index = next;
        self.circuit_zswap
            .events
            .push(crate::zswap::IntentEvent::Output(
                self.circuit_zswap.outputs.len(),
            ));
        self.circuit_zswap.outputs.push(crate::CircuitZswapOutput {
            provisional_index: index,
            coin,
            recipient,
        });
        Ok(())
    }

    /// Native `ownPublicKey()` reads the execution identity, not Zswap state.
    pub fn own_coin_public_key(&self) -> Result<[u8; 32], CompactError> {
        self.coin_public_key
            .map(|key| key.0.0)
            .ok_or(CompactError::MissingCoinPublicKey)
    }

    /// Observe kernel.self() with the compiler's metered address query.
    pub fn kernel_self(
        mut self,
    ) -> Result<CircuitResult<Private, ContractAddress, D>, CompactError> {
        let result = ledger::query_kernel_self(&self.query, self.gas_limit, &self.cost_model)?;
        self.query = result.context;
        let address = self.query.address;
        Ok(CircuitResult {
            context: self,
            result: address,
            gas_cost: result.gas_cost,
            private_transcript_outputs: Vec::new(),
        })
    }

    /// Start a new circuit call from an upstream ledger contract snapshot.
    ///
    /// The caller must associate this state with `address` and establish its
    /// network provenance. This conversion does not attest finality or fetch a
    /// full ledger snapshot.
    pub fn from_contract_state(
        private_state: Private,
        address: ContractAddress,
        contract: &ContractState<D>,
    ) -> Self {
        Self {
            private_state,
            query: QueryContext::new(contract.data.clone(), address),
            zswap_state: ZswapLocalState::default(),
            circuit_zswap: crate::CircuitZswapPlan::default(),
            coin_public_key: None,
            cost_model: INITIAL_COST_MODEL.clone(),
            gas_limit: None,
        }
    }

    pub fn into_constructor_result(self) -> ConstructorResult<Private, D> {
        ConstructorResult {
            ledger_state: self.query.state,
            private_state: self.private_state,
            zswap_state: self.zswap_state,
            circuit_zswap: self.circuit_zswap,
        }
    }

    pub fn witness_context(&self) -> WitnessContext<'_, Private, &StateValue<D>> {
        WitnessContext {
            ledger: self.query.state.get_ref(),
            private_state: &self.private_state,
            contract_address: &self.query.address,
        }
    }

    pub fn witness_context_with<'a, Ledger>(
        &'a self,
        ledger: Ledger,
    ) -> WitnessContext<'a, Private, Ledger> {
        WitnessContext {
            ledger,
            private_state: &self.private_state,
            contract_address: &self.query.address,
        }
    }
}
