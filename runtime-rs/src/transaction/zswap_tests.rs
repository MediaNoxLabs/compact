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

use super::*;
use crate::ledger::{
    CoinInfo, CoinNonce, CoinRecipient, HashOutput, ShieldedTokenType, StateValue,
};
use crate::recording::RecordingFrame;
use midnight_onchain_state::state::ContractMaintenanceAuthority;
use midnight_storage::arena::Sp;
use midnight_zswap::{Input, Output};
use rand::{SeedableRng, rngs::StdRng};

pub(super) fn coin(nonce: u8) -> CoinInfo {
    CoinInfo {
        nonce: CoinNonce(HashOutput([nonce; 32])),
        type_: ShieldedTokenType(HashOutput([2; 32])),
        value: 42,
    }
}
pub(super) fn setup() -> (
    OfferBackedObservedState,
    crate::ledger::QualifiedCoinInfo,
    CoinInfo,
    CoinRecipient,
) {
    let mut rng = StdRng::seed_from_u64(188);
    let address = ContractAddress::default();
    let contract = ContractState::new(
        StateValue::Array(Default::default()),
        Default::default(),
        ContractMaintenanceAuthority::default(),
    );
    let mut ledger = LedgerState::new("local-test");
    ledger.contract = ledger.contract.insert(address, contract.clone());
    let seed = coin(1);
    let output = Output::new_contract_owned(&mut rng, &seed, None, address).unwrap();
    let offer = Offer::new(vec![], vec![output], vec![]).unwrap();
    let (state, _) = ledger.zswap.try_apply(&offer, None).unwrap();
    ledger.zswap = Sp::new(state.post_block_update(Timestamp::from_secs(0)));
    let input = Input::new_contract_owned(
        &mut rng,
        &seed.qualify(0),
        None,
        address,
        &ledger.zswap.coin_coms,
    )
    .unwrap();
    let next = coin(3);
    let output = Output::new_contract_owned(&mut rng, &next, None, address).unwrap();
    let offer = Offer::new(vec![input], vec![output], vec![]).unwrap();
    let bound = OfferBackedObservedState::new(
        ObservedContractState::new(
            address,
            contract,
            Observation {
                transaction_hash: [0; 32],
                block_hash: [0; 32],
                block_height: 0,
            },
        ),
        &ledger,
        offer,
    )
    .unwrap();
    (
        bound,
        seed.qualify(0),
        next,
        CoinRecipient::Contract(address),
    )
}
fn recorded(
    bound: &OfferBackedObservedState,
    input: crate::ledger::QualifiedCoinInfo,
    output: CoinInfo,
    recipient: CoinRecipient,
) -> RecordedCircuitResult<(), ()> {
    RecordingFrame::new(bound.observed().circuit_context(()))
        .create_zswap_input(input)
        .create_zswap_output(output, recipient)
        .unwrap()
        .kernel_self()
        .unwrap()
        .0
        .finish(())
}
#[test]
fn exact_intents_accept_and_reject_index_plan_and_context_changes() {
    let (bound, input, output, recipient) = setup();
    let good = recorded(&bound, input, output, recipient.clone());
    assert_eq!(bound.reconcile(&good), Ok(()));
    assert_eq!(
        good.execution.private_transcript_outputs,
        vec![AlignedValue::from(()), AlignedValue::from(())]
    );
    let mut wrong_index = input;
    wrong_index.mt_index = 1;
    assert_eq!(
        bound.reconcile(&recorded(&bound, wrong_index, output, recipient.clone())),
        Err(ZswapIntentError::InputIndexMismatch)
    );
    let mut wrong_coin = input;
    wrong_coin.nonce = CoinNonce(HashOutput([9; 32]));
    assert_eq!(
        bound.reconcile(&recorded(&bound, wrong_coin, output, recipient.clone())),
        Err(ZswapIntentError::InputIndexMismatch)
    );
    let mut changed = recorded(&bound, input, output, recipient.clone());
    changed.execution.context.query.call_context.com_indices = Map::new();
    assert_eq!(
        bound.reconcile(&changed),
        Err(ZswapIntentError::AllocationMismatch)
    );
    let mut erased = recorded(&bound, input, output, recipient.clone());
    erased.execution.context = bound.observed().circuit_context(());
    assert_eq!(
        bound.reconcile(&erased),
        Err(ZswapIntentError::AllocationMismatch)
    );
    let duplicate = RecordingFrame::new(bound.observed().circuit_context(()))
        .create_zswap_input(input)
        .create_zswap_input(input)
        .create_zswap_output(output, recipient.clone())
        .unwrap()
        .finish(());
    assert_eq!(
        bound.reconcile(&duplicate),
        Err(ZswapIntentError::InputMismatch)
    );
    let missing = RecordingFrame::new(bound.observed().circuit_context(()))
        .create_zswap_input(input)
        .finish(());
    assert_eq!(
        bound.reconcile(&missing),
        Err(ZswapIntentError::OutputMismatch)
    );
    let mut context = bound.observed().circuit_context(());
    let initial = context.query.call_context.com_indices.clone();
    assert!(matches!(
        context.create_zswap_output(coin(4), recipient.clone()),
        Err(crate::CompactError::ZswapOfferOutputMismatch)
    ));
    assert_eq!(context.query.call_context.com_indices, initial);
    assert!(context.circuit_zswap().is_empty());
    context
        .create_zswap_output(output, recipient.clone())
        .unwrap();
    assert!(matches!(
        context.create_zswap_output(output, recipient),
        Err(crate::CompactError::ZswapOfferOutputMismatch)
    ));
    assert_eq!(context.circuit_zswap().outputs().len(), 1);
    assert_eq!(context.query.call_context.com_indices, initial);
}
#[test]
fn empty_plan_legacy_offer_and_exact_owner_nullifier_scope() {
    let (mut bound, input, output, recipient) = setup();
    let empty = RecordingFrame::new(bound.observed().circuit_context(()))
        .kernel_self()
        .unwrap()
        .0
        .finish(());
    assert_eq!(bound.reconcile(&empty), Ok(()));
    let good = recorded(&bound, input, output, recipient);
    let mut changed_input = bound.offer.inputs.get(0).unwrap().clone();
    changed_input.contract_address = None;
    bound.offer.inputs = vec![changed_input].into_iter().collect();
    assert_eq!(bound.reconcile(&good), Err(ZswapIntentError::InputMismatch));
    assert_eq!(bound.reconcile(&empty), Ok(()));
}

#[test]
fn explicit_wallet_funding_requires_exact_upstream_input_and_nonempty_plan() {
    let (mut bound, _input, _output, _recipient) = setup();
    assert!(matches!(
        WalletFundingInputs::<DefaultDB>::from_inputs(vec![]),
        Err(ZswapIntentError::WalletFundingMismatch)
    ));
    let contract_input = bound.offer.inputs.get(0).unwrap().clone();
    assert!(matches!(
        WalletFundingInputs::from_inputs(vec![contract_input.clone()]),
        Err(ZswapIntentError::WalletFundingOwner)
    ));
    let mut selected = contract_input;
    selected.contract_address = None;
    let mut foreign_owned = selected.clone();
    foreign_owned.contract_address =
        Some(Sp::new(crate::ledger::ContractAddress(HashOutput([8; 32]))));
    assert!(matches!(
        WalletFundingInputs::from_inputs(vec![foreign_owned]),
        Err(ZswapIntentError::WalletFundingOwner)
    ));
    assert!(matches!(
        WalletFundingInputs::from_inputs(vec![selected.clone(), selected.clone()]),
        Err(ZswapIntentError::WalletFundingDuplicate)
    ));
    let funding = WalletFundingInputs::from_inputs(vec![selected.clone()]).unwrap();
    assert_eq!(
        funding.validate_offer(&bound.offer),
        Err(ZswapIntentError::WalletFundingMismatch)
    );
    let original_offer = bound.offer.clone();
    bound.offer.inputs = vec![selected.clone()].into_iter().collect();
    assert_eq!(funding.validate_offer(&bound.offer), Ok(()));
    let mut extra_selection = selected.clone();
    extra_selection.nullifier = crate::ledger::CoinNullifier(HashOutput([10; 32]));
    let extra_funding =
        WalletFundingInputs::from_inputs(vec![selected.clone(), extra_selection]).unwrap();
    assert_eq!(
        extra_funding.validate_offer(&bound.offer),
        Err(ZswapIntentError::WalletFundingMismatch)
    );
    let mut altered_proof = selected.clone();
    std::sync::Arc::make_mut(&mut altered_proof.proof)
        .private_transcript
        .push(midnight_transient_crypto::curve::Fr::from(1));
    bound.offer.inputs = vec![altered_proof].into_iter().collect();
    assert_eq!(
        funding.validate_offer(&bound.offer),
        Err(ZswapIntentError::WalletFundingMismatch)
    );
    let mut replaced = selected.clone();
    replaced.merkle_tree_root = original_offer.inputs.get(0).unwrap().merkle_tree_root;
    replaced.nullifier = crate::ledger::CoinNullifier(HashOutput([9; 32]));
    bound.offer.inputs = vec![replaced].into_iter().collect();
    assert_eq!(
        funding.validate_offer(&bound.offer),
        Err(ZswapIntentError::WalletFundingMismatch)
    );
    bound.offer.inputs = vec![selected.clone(), selected].into_iter().collect();
    assert_eq!(
        funding.validate_offer(&bound.offer),
        Err(ZswapIntentError::WalletFundingDuplicate)
    );
    bound.offer = original_offer;
    bound.wallet_funding = Some(funding);
    let empty = RecordingFrame::new(bound.observed().circuit_context(()))
        .kernel_self()
        .unwrap()
        .0
        .finish(());
    assert_eq!(
        bound.reconcile(&empty),
        Err(ZswapIntentError::WalletFundingEmptyPlan)
    );
}

#[test]
fn funded_reconciliation_requires_exact_disjoint_input_union_and_output_order() {
    // These tests exercise public reconciliation only. The proof smoke uses
    // LocalZswapState::spend and proves the actual wallet input.
    let (mut bound, contract_coin, output_coin, recipient) = setup();
    let mut wallet_input = bound.offer.inputs.get(0).unwrap().clone();
    wallet_input.contract_address = None;
    bound.offer.inputs = vec![wallet_input.clone()].into_iter().collect();
    bound.wallet_funding =
        Some(WalletFundingInputs::from_inputs(vec![wallet_input.clone()]).unwrap());
    let good = RecordingFrame::new(bound.observed().circuit_context(()))
        .create_zswap_output(output_coin, recipient.clone())
        .unwrap()
        .kernel_self()
        .unwrap()
        .0
        .finish(());
    assert_eq!(bound.reconcile(&good), Ok(()));

    let mut wrong = wallet_input.clone();
    wrong.nullifier = crate::ledger::CoinNullifier(HashOutput([9; 32]));
    bound.wallet_funding = Some(WalletFundingInputs::from_inputs(vec![wrong.clone()]).unwrap());
    assert_eq!(bound.reconcile(&good), Err(ZswapIntentError::InputMismatch));
    bound.wallet_funding =
        Some(WalletFundingInputs::from_inputs(vec![wallet_input.clone()]).unwrap());
    bound.offer.inputs = vec![wallet_input.clone(), wrong].into_iter().collect();
    assert_eq!(bound.reconcile(&good), Err(ZswapIntentError::InputMismatch));
    bound.offer.inputs = vec![wallet_input.clone()].into_iter().collect();
    let contract_claim = recorded(&bound, contract_coin, output_coin, recipient.clone());
    assert_eq!(
        bound.reconcile(&contract_claim),
        Err(ZswapIntentError::InputMismatch)
    );
    let original = bound.offer.clone();
    let mut extra = coin(7);
    extra.value = 1;
    let mut rng = StdRng::seed_from_u64(199);
    let extra_output =
        Output::new_contract_owned(&mut rng, &extra, None, bound.observed.address).unwrap();
    bound.offer.outputs = vec![extra_output.clone()].into_iter().collect();
    assert_eq!(
        bound.reconcile(&good),
        Err(ZswapIntentError::OutputMismatch)
    );
    bound.offer.outputs = vec![original.outputs.get(0).unwrap().clone(), extra_output]
        .into_iter()
        .collect();
    assert_eq!(
        bound.reconcile(&good),
        Err(ZswapIntentError::OutputMismatch)
    );
    bound.offer = original;
    let transient = midnight_zswap::Transient::new_from_contract_owned_output(
        &mut rng,
        &output_coin.qualify(0),
        None,
        bound.offer.outputs.get(0).unwrap().clone(),
    )
    .unwrap();
    bound.offer.transient = vec![transient].into_iter().collect();
    assert_eq!(
        bound.reconcile(&good),
        Err(ZswapIntentError::TransientsUnsupported)
    );
    bound.offer.transient = Default::default();
    let mut context = bound.observed().circuit_context(());
    context.query.call_context.com_indices = Default::default();
    let altered = RecordingFrame::new(context).finish(());
    assert_eq!(
        bound.reconcile(&altered),
        Err(ZswapIntentError::AllocationMismatch)
    );
}

#[test]
fn transient_extra_input_and_output_order_are_explicit_boundaries() {
    let (mut bound, input, output, recipient) = setup();
    let good = recorded(&bound, input, output, recipient.clone());
    let original = bound.offer.clone();
    let mut wrong_input = bound.offer.inputs.get(0).unwrap().clone();
    wrong_input.nullifier = crate::ledger::CoinNullifier(HashOutput([9; 32]));
    bound.offer.inputs = vec![wrong_input.clone()].into_iter().collect();
    assert_eq!(bound.reconcile(&good), Err(ZswapIntentError::InputMismatch));
    bound.offer.inputs = vec![original.inputs.get(0).unwrap().clone(), wrong_input]
        .into_iter()
        .collect();
    assert_eq!(bound.reconcile(&good), Err(ZswapIntentError::InputMismatch));
    bound.offer = original.clone();
    let mut rng = StdRng::seed_from_u64(189);
    let transient = midnight_zswap::Transient::new_from_contract_owned_output(
        &mut rng,
        &output.qualify(0),
        None,
        bound.offer.outputs.get(0).unwrap().clone(),
    )
    .unwrap();
    bound.offer.transient = vec![transient].into_iter().collect();
    assert_eq!(
        bound.reconcile(&good),
        Err(ZswapIntentError::TransientsUnsupported)
    );
    let mut ledger = LedgerState::new("local-test");
    ledger.contract = ledger
        .contract
        .insert(bound.observed.address, bound.observed.contract.clone());
    ledger.zswap = Sp::new(bound.zswap.clone());
    let canonical_transient = OfferBackedObservedState::with_options(
        ObservedContractState::new(
            bound.observed.address,
            bound.observed.contract.clone(),
            bound.observed.observation,
        ),
        &ledger,
        bound.offer.clone(),
        OfferBindingOptions::default()
            .with_output_allocation(PersistentOutputAllocation::CanonicalOfferIndices),
    );
    assert!(format!("{:?}", canonical_transient.err().unwrap()).contains("rejects transients"));
    let extra_coin = coin(4);
    let extra =
        Output::new_contract_owned(&mut rng, &extra_coin, None, bound.observed.address).unwrap();
    let offer = Offer::new(
        vec![original.inputs.get(0).unwrap().clone()],
        vec![original.outputs.get(0).unwrap().clone(), extra],
        vec![],
    )
    .unwrap();
    let observed = ObservedContractState::new(
        bound.observed.address,
        bound.observed.contract.clone(),
        bound.observed.observation,
    );
    let two = OfferBackedObservedState::new(observed, &ledger, offer).unwrap();
    let first = two.offer.outputs.get(0).unwrap().coin_com;
    let (first_coin, second_coin) = if first == output.commitment(&recipient) {
        (output, extra_coin)
    } else {
        (extra_coin, output)
    };
    let mut ctx = two.observed.circuit_context(());
    assert!(matches!(
        ctx.create_zswap_output(second_coin, recipient.clone()),
        Err(crate::CompactError::ZswapOfferOutputMismatch)
    ));
    ctx.create_zswap_input(input);
    ctx.create_zswap_output(first_coin, recipient.clone())
        .unwrap();
    let incomplete = RecordingFrame::new(ctx).kernel_self().unwrap().0.finish(());
    // Starting recording after intents is also forbidden; a caller cannot hide a prefix.
    assert_eq!(
        two.reconcile(&incomplete),
        Err(ZswapIntentError::AllocationMismatch)
    );
    let prefix = RecordingFrame::new(two.observed.circuit_context(()))
        .create_zswap_input(input)
        .create_zswap_output(first_coin, recipient)
        .unwrap()
        .finish(());
    assert_eq!(
        two.reconcile(&prefix),
        Err(ZswapIntentError::OutputMismatch)
    );
}

fn canonical_pair() -> (
    OfferBackedObservedState,
    crate::ledger::QualifiedCoinInfo,
    Vec<CoinInfo>,
) {
    let (old, input, first, _) = setup();
    let second = coin(4);
    let mut rng = StdRng::seed_from_u64(205);
    let offer = Offer::new(
        old.offer.inputs.iter_deref().cloned().collect(),
        vec![
            old.offer.outputs.get(0).unwrap().clone(),
            Output::new_contract_owned(&mut rng, &second, None, old.observed.address).unwrap(),
        ],
        vec![],
    )
    .unwrap();
    let mut ledger = LedgerState::new("local-test");
    ledger.zswap = Sp::new(old.zswap.clone());
    ledger.contract = ledger
        .contract
        .insert(old.observed.address, old.observed.contract.clone());
    let bound = OfferBackedObservedState::with_options(
        old.observed,
        &ledger,
        offer,
        OfferBindingOptions::default()
            .with_output_allocation(PersistentOutputAllocation::CanonicalOfferIndices),
    )
    .unwrap();
    let recipient = CoinRecipient::Contract(bound.observed.address);
    let mut coins = vec![first, second];
    coins.sort_by_key(|coin| coin.commitment(&recipient));
    (bound, input, coins)
}
fn pair_recorded(
    bound: &OfferBackedObservedState,
    input: crate::ledger::QualifiedCoinInfo,
    coins: &[CoinInfo],
) -> RecordedCircuitResult<(), ()> {
    let mut frame =
        RecordingFrame::new(bound.observed.circuit_context(())).create_zswap_input(input);
    for coin in coins {
        frame = frame
            .create_zswap_output(*coin, CoinRecipient::Contract(bound.observed.address))
            .unwrap();
    }
    frame.kernel_self().unwrap().0.finish(())
}
#[test]
fn canonical_indices_preserve_source_order_and_logical_progress() {
    let (bound, input, mut coins) = canonical_pair();
    let start = bound.zswap.first_free;
    assert!(start > 0);
    for reversed in [false, true] {
        if reversed {
            coins.reverse();
        }
        let recorded = pair_recorded(&bound, input, &coins);
        assert_eq!(bound.reconcile(&recorded), Ok(()));
        let plan = recorded.execution.context.circuit_zswap();
        assert_eq!(plan.next_index(), start + 2);
        assert_eq!(
            plan.outputs().iter().map(|x| x.coin).collect::<Vec<_>>(),
            coins
        );
        assert_eq!(
            plan.outputs()
                .iter()
                .map(|x| x.provisional_index)
                .collect::<Vec<_>>(),
            if reversed {
                vec![start + 1, start]
            } else {
                vec![start, start + 1]
            }
        );
        assert_eq!(
            recorded.execution.private_transcript_outputs,
            vec![AlignedValue::from(()); 3]
        );
        assert_eq!(
            recorded.execution.context.query.call_context.com_indices,
            bound.observed.com_indices
        );
        let replay = recorded
            .public
            .initial()
            .query(
                recorded.public.verify_ops(),
                None,
                &recorded.execution.context.cost_model,
            )
            .unwrap();
        assert_eq!(replay.context.state, recorded.execution.context.query.state);
    }
    // Existing constructor still uses exact order for the same complete offer.
    let mut ledger = LedgerState::new("local-test");
    ledger.zswap = Sp::new(bound.zswap.clone());
    ledger.contract = ledger
        .contract
        .insert(bound.observed.address, bound.observed.contract.clone());
    let exact = OfferBackedObservedState::new(
        ObservedContractState::new(
            bound.observed.address,
            bound.observed.contract.clone(),
            bound.observed.observation,
        ),
        &ledger,
        bound.offer.clone(),
    )
    .unwrap();
    assert!(matches!(
        exact
            .observed
            .circuit_context(())
            .create_zswap_output(coins[0], CoinRecipient::Contract(exact.observed.address)),
        Err(crate::CompactError::ZswapOfferOutputMismatch)
    ));
}
#[test]
fn canonical_output_bijection_and_sealed_allocation_reject_mutations() {
    let (mut bound, input, coins) = canonical_pair();
    let good = pair_recorded(&bound, input, &coins);
    let missing = pair_recorded(&bound, input, &coins[..1]);
    assert_eq!(
        bound.reconcile(&missing),
        Err(ZswapIntentError::OutputMismatch)
    );
    let empty = RecordingFrame::new(bound.observed.circuit_context(()))
        .kernel_self()
        .unwrap()
        .0
        .finish(());
    assert_eq!(
        bound.reconcile(&empty),
        Err(ZswapIntentError::CanonicalEmptyPlan)
    );
    let recipient = CoinRecipient::Contract(bound.observed.address);
    let mut context = bound.observed.circuit_context(());
    context
        .create_zswap_output(coins[0], recipient.clone())
        .unwrap();
    let before = context.circuit_zswap().clone();
    let map = context.query.call_context.com_indices.clone();
    for (coin, recipient) in [
        (coins[0], recipient.clone()),
        (coin(9), recipient.clone()),
        (
            coins[1],
            CoinRecipient::Contract(ContractAddress(crate::ledger::HashOutput([8; 32]))),
        ),
    ] {
        assert!(matches!(
            context.create_zswap_output(coin, recipient),
            Err(crate::CompactError::ZswapOfferOutputMismatch)
        ));
        assert_eq!(context.circuit_zswap(), &before);
        assert_eq!(context.query.call_context.com_indices, map);
    }
    for mutation in 0..5 {
        let mut altered = pair_recorded(&bound, input, &coins);
        let context = &mut altered.execution.context;
        match mutation {
            0 => context.query.call_context.com_indices = Map::new(),
            1 => context.circuit_zswap.next_index += 1,
            2 => context.circuit_zswap.outputs[0].provisional_index += 1,
            3 => context.circuit_zswap.allocation = crate::zswap::Allocation::Locked,
            _ => context.query.address = ContractAddress(crate::ledger::HashOutput([8; 32])),
        }
        assert_eq!(
            bound.reconcile(&altered),
            Err(ZswapIntentError::AllocationMismatch)
        );
    }
    // Public owner metadata must match the captured typed recipient, even when
    // commitment bytes are unchanged. Upstream proof checks remain required.
    let mut output = bound.offer.outputs.get(0).unwrap().clone();
    output.contract_address = None;
    bound.offer.outputs = vec![output, bound.offer.outputs.get(1).unwrap().clone()]
        .into_iter()
        .collect();
    assert_eq!(
        bound.reconcile(&good),
        Err(ZswapIntentError::OutputMismatch)
    );
}
#[test]
fn canonical_constructor_rejects_duplicates_non_normal_form_and_forged_owner() {
    let (bound, _, coins) = canonical_pair();
    let mut ledger = LedgerState::new("local-test");
    ledger.zswap = Sp::new(bound.zswap.clone());
    ledger.contract = ledger
        .contract
        .insert(bound.observed.address, bound.observed.contract.clone());
    let observe = || {
        ObservedContractState::new(
            bound.observed.address,
            bound.observed.contract.clone(),
            bound.observed.observation,
        )
    };
    let options = || {
        OfferBindingOptions::default()
            .with_output_allocation(PersistentOutputAllocation::CanonicalOfferIndices)
    };
    let output = bound.offer.outputs.get(0).unwrap().clone();
    let duplicate = Offer::new(vec![], vec![output.clone(), output], vec![]).unwrap();
    let error = OfferBackedObservedState::with_options(observe(), &ledger, duplicate, options())
        .err()
        .unwrap();
    assert!(format!("{error:?}").contains("CommitmentAlreadyPresent"));
    let mut reversed = bound.offer.clone();
    reversed.outputs = reversed
        .outputs
        .iter_deref()
        .cloned()
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    let error = OfferBackedObservedState::with_options(observe(), &ledger, reversed, options())
        .err()
        .unwrap();
    assert!(format!("{error:?}").contains("requires a normalized offer"));
    let mut forged = bound.offer.clone();
    let mut output = forged.outputs.get(0).unwrap().clone();
    output.contract_address = Some(Sp::new(ContractAddress(crate::ledger::HashOutput([8; 32]))));
    forged.outputs = vec![output, forged.outputs.get(1).unwrap().clone()]
        .into_iter()
        .collect();
    forged.normalize();
    let forged =
        OfferBackedObservedState::with_options(observe(), &ledger, forged, options()).unwrap();
    assert!(matches!(
        forged
            .observed
            .circuit_context(())
            .create_zswap_output(coins[0], CoinRecipient::Contract(bound.observed.address)),
        Err(crate::CompactError::ZswapOfferOutputMismatch)
    ));
    let (mut corrupt, _, coins) = canonical_pair();
    if let crate::zswap::Allocation::CanonicalOfferBound { outputs, .. } =
        &mut corrupt.observed.allocation
    {
        outputs[1].index = outputs[0].index;
    }
    assert!(matches!(
        corrupt
            .observed
            .circuit_context(())
            .create_zswap_output(coins[1], CoinRecipient::Contract(corrupt.observed.address)),
        Err(crate::CompactError::ZswapOfferOutputMismatch)
    ));
}

#[test]
fn public_prepare_preserves_default_and_funded_empty_call_boundaries() {
    let verifier: VerifierKey = StdRng::seed_from_u64(205).r#gen();
    for canonical in [false, true] {
        for empty_offer in [false, true] {
            let (base, _, _) = canonical_pair();
            let mut offer = base.offer.clone();
            if empty_offer {
                offer.inputs = Default::default();
                offer.outputs = Default::default();
                offer.deltas = Default::default();
            }
            let mut ledger = LedgerState::new("local-test");
            ledger.zswap = Sp::new(base.zswap.clone());
            ledger.contract = ledger
                .contract
                .insert(base.observed.address, base.observed.contract.clone());
            let mut bound = OfferBackedObservedState::with_options(
                ObservedContractState::new(
                    base.observed.address,
                    base.observed.contract.clone(),
                    base.observed.observation,
                ),
                &ledger,
                offer,
                OfferBindingOptions::default().with_output_allocation(if canonical {
                    PersistentOutputAllocation::CanonicalOfferIndices
                } else {
                    PersistentOutputAllocation::ExactIntentOrder
                }),
            )
            .unwrap();
            if empty_offer {
                assert!(bound.observed.com_indices.is_empty());
            }
            bound.observed.contract.operations = bound.observed.contract.operations.insert(
                EntryPointBuf(b"empty".to_vec()),
                ContractOperation::new(Some(verifier.clone())),
            );
            for query in [false, true] {
                let mut frame = RecordingFrame::new(bound.observed.circuit_context(()));
                if query {
                    frame = frame.kernel_self().unwrap().0;
                }
                let call = RecordedCall::new(bound.observed(), frame.finish(()), "empty", ());
                let result = bound.prepare(call, verifier.clone(), Fr::from(0));
                if canonical {
                    assert!(matches!(
                        result,
                        Err(ObservedCallError::ZswapIntent(
                            ZswapIntentError::CanonicalEmptyPlan
                        ))
                    ));
                } else if query {
                    assert!(result.is_ok());
                } else {
                    assert!(matches!(
                        result,
                        Err(ObservedCallError::Prepare(
                            PrepareCallError::EmptyTranscript
                        ))
                    ));
                }
            }
        }
    }
    let (mut funded, _, _, _) = setup();
    let mut wallet = funded.offer.inputs.get(0).unwrap().clone();
    wallet.contract_address = None;
    funded.wallet_funding = Some(WalletFundingInputs::from_inputs(vec![wallet]).unwrap());
    let call = RecordedCall::new(
        funded.observed(),
        RecordingFrame::new(funded.observed.circuit_context(())).finish(()),
        "empty",
        (),
    );
    assert!(matches!(
        funded.prepare(call, verifier, Fr::from(0)),
        Err(ObservedCallError::ZswapIntent(
            ZswapIntentError::WalletFundingEmptyPlan
        ))
    ));
}

#[test]
fn canonical_options_compose_with_explicit_wallet_input_selection() {
    // Structural policy composition only; actual wallet ownership is proved by
    // the existing funded-receive smoke, never inferred from owner=None here.
    let (base, _, mut coins) = canonical_pair();
    coins.reverse();
    let mut wallet = base.offer.inputs.get(0).unwrap().clone();
    wallet.contract_address = None;
    let offer = Offer::new(
        vec![wallet.clone()],
        base.offer.outputs.iter_deref().cloned().collect(),
        vec![],
    )
    .unwrap();
    let mut ledger = LedgerState::new("local-test");
    ledger.zswap = Sp::new(base.zswap.clone());
    ledger.contract = ledger
        .contract
        .insert(base.observed.address, base.observed.contract.clone());
    let observe = || {
        ObservedContractState::new(
            base.observed.address,
            base.observed.contract.clone(),
            base.observed.observation,
        )
    };
    let bound = OfferBackedObservedState::with_options(
        observe(),
        &ledger,
        offer.clone(),
        OfferBindingOptions::default()
            .with_output_allocation(PersistentOutputAllocation::CanonicalOfferIndices)
            .with_wallet_funding(WalletFundingInputs::from_inputs(vec![wallet.clone()]).unwrap()),
    )
    .unwrap();
    let record = || {
        let mut frame = RecordingFrame::new(bound.observed.circuit_context(()));
        for coin in &coins {
            frame = frame
                .create_zswap_output(*coin, CoinRecipient::Contract(bound.observed.address))
                .unwrap();
        }
        frame.kernel_self().unwrap().0.finish(())
    };
    assert_eq!(bound.reconcile(&record()), Ok(()));
    let mut missing_selection = OfferBackedObservedState::with_options(
        observe(),
        &ledger,
        offer.clone(),
        OfferBindingOptions::default()
            .with_output_allocation(PersistentOutputAllocation::CanonicalOfferIndices),
    )
    .unwrap();
    // Same sealed output context, absent explicit wallet partition is refused.
    assert_eq!(
        missing_selection.reconcile(&record()),
        Err(ZswapIntentError::InputMismatch)
    );
    missing_selection.offer.outputs = vec![missing_selection.offer.outputs.get(0).unwrap().clone()]
        .into_iter()
        .collect();
    assert_eq!(
        missing_selection.reconcile(&record()),
        Err(ZswapIntentError::OutputMismatch)
    );
    let exact_funded = OfferBackedObservedState::with_wallet_funding(
        observe(),
        &ledger,
        offer,
        WalletFundingInputs::from_inputs(vec![wallet]).unwrap(),
    )
    .unwrap();
    assert!(matches!(
        exact_funded
            .observed
            .circuit_context(())
            .create_zswap_output(
                coins[0],
                CoinRecipient::Contract(exact_funded.observed.address)
            ),
        Err(crate::CompactError::ZswapOfferOutputMismatch)
    ));
}
