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

//! Ordered native circuit intents. These are not wallet state or a validated offer.

use crate::ledger::{CoinInfo, CoinRecipient, QualifiedCoinInfo};

/// One policy-approved output allocated by the retained complete upstream offer.
/// Includes explicitly selected transients when that policy is enabled.
#[cfg(feature = "ledger-transaction")]
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct BoundOutput {
    pub commitment: crate::ledger::CoinCommitment,
    pub index: u64,
    pub owner: Option<crate::ledger::ContractAddress>,
}
#[cfg(feature = "ledger-transaction")]
impl BoundOutput {
    pub fn matches_recipient(&self, recipient: &CoinRecipient) -> bool {
        match recipient {
            CoinRecipient::User(_) => self.owner.is_none(),
            CoinRecipient::Contract(address) => self.owner == Some(*address),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) enum Allocation {
    #[default]
    Provisional,
    #[cfg(feature = "ledger-transaction")]
    Locked,
    #[cfg(feature = "ledger-transaction")]
    OfferBound {
        start: u64,
        outputs: Vec<(crate::ledger::CoinCommitment, u64)>,
    },
    #[cfg(feature = "ledger-transaction")]
    CanonicalOfferBound {
        start: u64,
        outputs: Vec<BoundOutput>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CircuitZswapOutput {
    /// Provisional in native mode; exact upstream allocation in offer-bound mode.
    pub provisional_index: u64,
    pub coin: CoinInfo,
    pub recipient: CoinRecipient,
}

/// Successful cross-kind intent order. Indices refer to the existing typed
/// vectors; this is provenance, not another execution representation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum IntentEvent {
    Input(usize),
    Output(usize),
}

/// Native execution's provisional output cursor and ordered circuit intents.
/// Ledger validation and offer reconciliation remain separate operations.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CircuitZswapPlan {
    pub(crate) next_index: u64,
    pub(crate) inputs: Vec<QualifiedCoinInfo>,
    pub(crate) events: Vec<IntentEvent>,
    pub(crate) outputs: Vec<CircuitZswapOutput>,
    pub(crate) allocation: Allocation,
}

impl CircuitZswapPlan {
    pub fn is_empty(&self) -> bool {
        self.inputs.is_empty() && self.outputs.is_empty() && self.events.is_empty()
    }

    /// Logical progress: start plus the number of source-ordered output intents.
    /// In canonical mode this is not the next intent's physical Merkle index.
    pub fn next_index(&self) -> u64 {
        self.next_index
    }
    pub fn inputs(&self) -> &[QualifiedCoinInfo] {
        &self.inputs
    }
    pub fn outputs(&self) -> &[CircuitZswapOutput] {
        &self.outputs
    }
    pub fn allocation_locked(&self) -> bool {
        self.allocation != Allocation::Provisional
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        CompactError,
        context::{
            CircuitContext, CircuitResult, ConstructorContext, ConstructorResult, RunningCost,
        },
        ledger::{self, ChargedState, ContractAddress, DefaultDB, StateValue},
        recording::RecordingFrame,
    };
    fn context() -> CircuitContext<()> {
        ConstructorResult::new(
            ConstructorContext::new(()),
            ChargedState::new(StateValue::<DefaultDB>::Array(
                vec![ledger::constructor_cell(false)].into(),
            )),
        )
        .into_circuit_context(ContractAddress::default())
    }
    fn output() -> (CoinInfo, CoinRecipient) {
        (
            ledger::coin_info_from_compact(
                crate::FixedBytes::new([1; 32]),
                crate::FixedBytes::new([2; 32]),
                42,
            ),
            CoinRecipient::Contract(ContractAddress::default()),
        )
    }
    #[test]
    fn provisional_cursor_checks_precede_effects_and_roundtrip_retains_plan() {
        let (coin, recipient) = output();
        let mut ctx = context();
        ctx.set_zswap_output_start(u64::MAX).unwrap();
        let before = ctx.query.clone();
        let plan = ctx.circuit_zswap().clone();
        assert!(matches!(
            ctx.create_zswap_output(coin, recipient.clone()),
            Err(CompactError::ZswapCursorOverflow)
        ));
        assert_eq!(
            ctx.query.call_context.com_indices,
            before.call_context.com_indices
        );
        assert_eq!(ctx.circuit_zswap(), &plan);
        ctx.set_zswap_output_start(7).unwrap();
        ctx.create_zswap_input(coin.qualify(3));
        ctx.create_zswap_output(coin, recipient.clone()).unwrap();
        ctx.create_zswap_output(coin, recipient.clone()).unwrap();
        assert_eq!(ctx.circuit_zswap().next_index(), 9);
        assert!(matches!(
            ctx.set_zswap_output_start(0),
            Err(CompactError::ZswapCursorAlreadyUsed)
        ));
        let plan = ctx.circuit_zswap().clone();
        let indices = ctx.query.call_context.com_indices.clone();
        let ctx = ctx
            .into_constructor_result()
            .into_circuit_context(ContractAddress::default());
        assert_eq!(ctx.circuit_zswap(), &plan);
        assert_eq!(ctx.query.call_context.com_indices, indices);
        assert!(ctx.zswap_state.coins.is_empty());
        assert!(ctx.zswap_state.pending_outputs.is_empty());
        assert_eq!(ctx.zswap_state.first_free, 0);
    }
    #[cfg(feature = "ledger-transaction")]
    #[test]
    fn observed_lock_survives_conversion_and_blocks_all_provisional_changes() {
        use crate::transaction::{Observation, ObservedContractState};
        use midnight_onchain_state::state::{ContractMaintenanceAuthority, ContractState};
        let state = context().query.state.get_ref().clone();
        let observed = ObservedContractState::new(
            ContractAddress::default(),
            ContractState::new(
                state,
                Default::default(),
                ContractMaintenanceAuthority::default(),
            ),
            Observation {
                transaction_hash: [0; 32],
                block_hash: [0; 32],
                block_height: 0,
            },
        );
        let mut ctx = observed
            .circuit_context(())
            .into_constructor_result()
            .into_circuit_context(ContractAddress::default());
        let (coin, recipient) = output();
        // Model an existing authoritative allocation; locking must preserve it.
        ctx.query.call_context.com_indices = ctx
            .query
            .call_context
            .com_indices
            .insert(coin.commitment(&recipient), 19);
        let plan = ctx.circuit_zswap().clone();
        let indices = ctx.query.call_context.com_indices.clone();
        assert!(matches!(
            ctx.set_zswap_output_start(7),
            Err(CompactError::ZswapAllocationLocked)
        ));
        assert!(matches!(
            ctx.create_zswap_output(coin, recipient.clone()),
            Err(CompactError::ZswapAllocationLocked)
        ));
        assert_eq!(ctx.circuit_zswap(), &plan);
        assert_eq!(ctx.query.call_context.com_indices, indices);
    }
    #[cfg(feature = "ledger-transaction")]
    #[test]
    fn local_helper_audit_rejects_allocation_mode_changes() {
        let rejected = RecordingFrame::new(context()).call_local(|ctx| {
            Ok(CircuitResult {
                context: ctx.lock_zswap_allocation(),
                result: (),
                gas_cost: RunningCost::ZERO,
                private_transcript_outputs: vec![],
            })
        });
        assert!(matches!(rejected, Err(CompactError::InvalidLedgerCell(_))));
    }

    #[test]
    fn local_helper_audit_rejects_input_cursor_and_output_effects() {
        for mode in 0..4 {
            let rejected = RecordingFrame::new(context()).call_local(|mut ctx| {
                let (coin, recipient) = output();
                match mode {
                    0 => ctx.create_zswap_input(coin.qualify(4)),
                    1 => ctx.set_zswap_output_start(7)?,
                    2 => ctx.create_zswap_output(coin, recipient.clone())?,
                    _ => ctx.circuit_zswap.events.push(IntentEvent::Input(0)),
                }
                Ok(CircuitResult {
                    context: ctx,
                    result: (),
                    gas_cost: RunningCost::ZERO,
                    private_transcript_outputs: vec![],
                })
            });
            assert!(matches!(rejected, Err(CompactError::InvalidLedgerCell(_))));
        }
    }
}
