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
use crate::ledger::{CoinRecipient, HashOutput};
use crate::recording::RecordingFrame;
use midnight_storage::arena::Sp;
use midnight_zswap::{Output, Transient};
use rand::{SeedableRng, rngs::StdRng};

// Structural reconciliation cases. Actual wallet/transient proofs and default
// strict ledger acceptance live in the generated source proof consumer.
fn setup_transient(
    historical: bool,
) -> (
    OfferBackedObservedState,
    crate::ledger::QualifiedCoinInfo,
    crate::ledger::CoinInfo,
    crate::ledger::CoinInfo,
) {
    let (old, history, received, _) = super::zswap_tests::setup();
    let sent = super::zswap_tests::coin(4);
    let mut rng = StdRng::seed_from_u64(204);
    let output =
        Output::new_contract_owned(&mut rng, &received, None, old.observed.address).unwrap();
    let transient =
        Transient::new_from_contract_owned_output(&mut rng, &received.qualify(0), None, output)
            .unwrap();
    let sent_output =
        Output::new_contract_owned(&mut rng, &sent, None, old.observed.address).unwrap();
    let offer = Offer::new(
        if historical {
            old.offer.inputs.iter_deref().cloned().collect()
        } else {
            vec![]
        },
        vec![sent_output],
        vec![transient.clone()],
    )
    .unwrap();
    let mut ledger = LedgerState::new("local-test");
    ledger.zswap = Sp::new(old.zswap);
    ledger.contract = ledger
        .contract
        .insert(old.observed.address, old.observed.contract.clone());
    let bound = OfferBackedObservedState::with_options(
        old.observed,
        &ledger,
        offer,
        OfferBindingOptions::default()
            .with_output_allocation(PersistentOutputAllocation::CanonicalOfferIndices)
            .with_transient_coins(
                ContractTransientCoins::from_transients(vec![transient]).unwrap(),
            ),
    )
    .unwrap();
    (bound, history, received, sent)
}
fn record(
    bound: &OfferBackedObservedState,
    received: crate::ledger::CoinInfo,
    sent: crate::ledger::CoinInfo,
    historical: Option<crate::ledger::QualifiedCoinInfo>,
    input_first: bool,
    index: u64,
    duplicate: bool,
) -> RecordedCircuitResult<(), ()> {
    let recipient = CoinRecipient::Contract(bound.observed.address);
    let mut frame = RecordingFrame::new(bound.observed.circuit_context(()));
    if let Some(coin) = historical {
        frame = frame.create_zswap_input(coin);
    }
    if input_first {
        frame = frame.create_zswap_input(received.qualify(index));
    }
    frame = frame
        .create_zswap_output(received, recipient.clone())
        .unwrap();
    if !input_first {
        frame = frame.create_zswap_input(received.qualify(index));
    }
    if duplicate {
        frame = frame.create_zswap_input(received.qualify(index));
    }
    frame
        .create_zswap_output(sent, recipient)
        .unwrap()
        .kernel_self()
        .unwrap()
        .0
        .finish(())
}
#[test]
fn transient_pairs_require_exact_causal_source_events_and_keep_historical_zero() {
    for historical in [false, true] {
        let (bound, history, received, sent) = setup_transient(historical);
        assert_eq!(history.mt_index, 0);
        let history = historical.then_some(history);
        let good = record(&bound, received, sent, history, false, 0, false);
        assert_eq!(bound.reconcile(&good), Ok(()));
        assert_eq!(
            good.execution.context.circuit_zswap.outputs[0].provisional_index,
            bound.zswap.first_free + 1
        );
        assert_eq!(
            good.execution.context.circuit_zswap.outputs[1].provisional_index,
            bound.zswap.first_free
        );
        for (first, index, duplicate) in [(true, 0, false), (false, 1, false), (false, 0, true)] {
            assert_eq!(
                bound.reconcile(&record(
                    &bound, received, sent, history, first, index, duplicate
                )),
                Err(ZswapIntentError::TransientInputMismatch)
            );
        }
        let mut wrong_history = history;
        if let Some(coin) = &mut wrong_history {
            coin.mt_index = bound.zswap.first_free;
            assert_eq!(
                bound.reconcile(&record(
                    &bound,
                    received,
                    sent,
                    wrong_history,
                    false,
                    0,
                    false
                )),
                Err(ZswapIntentError::InputIndexMismatch)
            );
        }
        let mut context = bound.observed.circuit_context(());
        context
            .create_zswap_output(received, CoinRecipient::Contract(bound.observed.address))
            .unwrap();
        let before = context.circuit_zswap.clone();
        assert!(
            context
                .create_zswap_output(received, CoinRecipient::Contract(bound.observed.address))
                .is_err()
        );
        assert_eq!(context.circuit_zswap, before); // failed output did not append provenance
        for corruption in 0..3 {
            let mut altered = record(&bound, received, sent, history, false, 0, false);
            match corruption {
                0 => altered.execution.context.circuit_zswap.events.clear(),
                1 => altered.execution.context.circuit_zswap.events.reverse(),
                _ => altered.execution.context.query.call_context.com_indices = Map::new(),
            }
            assert_eq!(
                bound.reconcile(&altered),
                Err(ZswapIntentError::AllocationMismatch)
            );
        }
    }
}
#[test]
fn transient_selection_binds_both_full_proofs_owner_segment_and_exact_offer_partition() {
    let (bound, _, _, _) = setup_transient(false);
    let selected = bound.offer.transient.get(0).unwrap().clone();
    let choose = || ContractTransientCoins::from_transients(vec![selected.clone()]).unwrap();
    assert!(matches!(
        ContractTransientCoins::<DefaultDB>::from_transients(vec![]),
        Err(ZswapIntentError::TransientSelectionMismatch)
    ));
    assert!(matches!(
        ContractTransientCoins::from_transients(vec![selected.clone(), selected.clone()]),
        Err(ZswapIntentError::TransientDuplicate)
    ));
    let mut missing = bound.offer.clone();
    missing.transient = Default::default();
    assert_eq!(
        choose().validate_offer(&missing, bound.observed.address),
        Err(ZswapIntentError::TransientSelectionMismatch)
    );
    for input_side in [false, true] {
        let mut changed = selected.clone();
        let proof = if input_side {
            &mut changed.proof_input
        } else {
            &mut changed.proof_output
        };
        std::sync::Arc::make_mut(proof).binding_input = Fr::from(1);
        let mut changed_offer = bound.offer.clone();
        changed_offer.transient = vec![changed].into_iter().collect();
        assert_eq!(
            choose().validate_offer(&changed_offer, bound.observed.address),
            Err(ZswapIntentError::TransientSelectionMismatch)
        );
    }
    assert!(matches!(
        ContractTransientCoins::from_transients(vec![selected.retarget_segment(1)]),
        Err(ZswapIntentError::TransientSegmentMismatch)
    ));
    let mut wrong_output_segment = selected.clone();
    std::sync::Arc::make_mut(&mut wrong_output_segment.proof_output).public_transcript_outputs =
        vec![Fr::from(1)];
    assert!(matches!(
        ContractTransientCoins::from_transients(vec![wrong_output_segment]),
        Err(ZswapIntentError::TransientSegmentMismatch)
    ));
    let mut no_owner = selected.clone();
    no_owner.contract_address = None;
    assert!(matches!(
        ContractTransientCoins::from_transients(vec![no_owner]),
        Err(ZswapIntentError::TransientOwnerMismatch)
    ));
    assert_eq!(
        choose().validate_offer(&bound.offer, ContractAddress(HashOutput([9; 32]))),
        Err(ZswapIntentError::TransientOwnerMismatch)
    );
    let mut overlap = bound.offer.clone();
    overlap.inputs = vec![selected.as_input()].into_iter().collect();
    assert_eq!(
        choose().validate_offer(&overlap, bound.observed.address),
        Err(ZswapIntentError::TransientSelectionMismatch)
    );
    let mut overlap = bound.offer.clone();
    overlap.outputs = vec![selected.as_output()].into_iter().collect();
    assert_eq!(
        choose().validate_offer(&overlap, bound.observed.address),
        Err(ZswapIntentError::TransientSelectionMismatch)
    );
}
#[test]
fn transient_options_require_explicit_canonical_mode_and_preserve_old_refusal() {
    let (bound, _, received, sent) = setup_transient(false);
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
    let transient = bound.offer.transient.get(0).unwrap().clone();
    assert!(
        OfferBackedObservedState::with_options(
            observe(),
            &ledger,
            bound.offer.clone(),
            OfferBindingOptions::default().with_transient_coins(
                ContractTransientCoins::from_transients(vec![transient]).unwrap()
            )
        )
        .is_err()
    );
    assert!(
        OfferBackedObservedState::with_options(
            observe(),
            &ledger,
            bound.offer.clone(),
            OfferBindingOptions::default()
                .with_output_allocation(PersistentOutputAllocation::CanonicalOfferIndices)
        )
        .is_err()
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
    let mut no_selection = setup_transient(false).0;
    no_selection.transients = None;
    let call = record(&no_selection, received, sent, None, false, 0, false);
    assert_eq!(
        no_selection.reconcile(&call),
        Err(ZswapIntentError::TransientsUnsupported)
    );
}

fn fallible_funded(
    historical: bool,
) -> (
    OfferBackedObservedState,
    crate::ledger::QualifiedCoinInfo,
    crate::ledger::CoinInfo,
    crate::ledger::CoinInfo,
) {
    let old = super::zswap_tests::setup().0;
    let mut ledger = LedgerState::new("local-test");
    let address = old.observed.address;
    ledger.contract = ledger
        .contract
        .insert(address, old.observed.contract.clone());
    let mut rng = StdRng::seed_from_u64(211);
    let keys = midnight_zswap::keys::SecretKeys::from_rng_seed(&mut rng);
    let mut history = super::zswap_tests::coin(1);
    history.value = 17;
    let mut received = super::zswap_tests::coin(2);
    received.value = 10;
    let mut sent = super::zswap_tests::coin(3);
    sent.value = if historical { 27 } else { 10 };
    let h = Output::new_contract_owned(&mut rng, &history, None, address).unwrap();
    let w = Output::new(
        &mut rng,
        &received,
        None,
        &keys.coin_public_key(),
        Some(keys.enc_public_key()),
    )
    .unwrap();
    let hc = h.coin_com;
    let wc = w.coin_com;
    let seed = Offer::new(vec![], vec![h, w], vec![]).unwrap();
    let (state, indices) = ledger.zswap.try_apply(&seed, None).unwrap();
    ledger.zswap = Sp::new(state.post_block_update(Timestamp::from_secs(0)));
    assert_eq!(ledger.zswap.first_free, 2);
    let history = history.qualify(*indices.get(&hc).unwrap());
    let wallet = midnight_zswap::local::State::<DefaultDB>::new().apply(&keys, &seed);
    let (_, wallet_input) = wallet
        .spend(
            &mut rng,
            &keys,
            &received.qualify(*indices.get(&wc).unwrap()),
            Some(7),
        )
        .unwrap();
    let input = midnight_zswap::Input::new_contract_owned(
        &mut rng,
        &history,
        Some(7),
        address,
        &ledger.zswap.coin_coms,
    )
    .unwrap();
    // Both owner classes have the same [1, segment] shape; ownership is
    // established by full selections and the actual historical path.
    assert!(wallet_input.contract_address.is_none());
    assert!(input.contract_address.is_some());
    assert_eq!(
        wallet_input.proof.public_transcript_outputs,
        input.proof.public_transcript_outputs
    );
    let received_output =
        Output::new_contract_owned(&mut rng, &received, Some(7), address).unwrap();
    let transient = Transient::new_from_contract_owned_output(
        &mut rng,
        &received.qualify(0),
        Some(7),
        received_output,
    )
    .unwrap();
    let output = Output::new_contract_owned(&mut rng, &sent, Some(7), address).unwrap();
    let offer = Offer::new(
        if historical {
            vec![input, wallet_input.clone()]
        } else {
            vec![wallet_input.clone()]
        },
        vec![output],
        vec![transient.clone()],
    )
    .unwrap();
    let placement = OfferPlacement::Fallible(std::num::NonZeroU16::new(7).unwrap());
    let bound = OfferBackedObservedState::with_options(
        old.observed,
        &ledger,
        offer,
        OfferBindingOptions::default()
            .with_output_allocation(PersistentOutputAllocation::CanonicalOfferIndices)
            .with_offer_placement(placement)
            .with_wallet_funding(WalletFundingInputs::from_inputs(vec![wallet_input]).unwrap())
            .with_transient_coins(
                ContractTransientCoins::from_transients_for_placement(vec![transient], placement)
                    .unwrap(),
            ),
    )
    .unwrap();
    (bound, history, received, sent)
}

#[test]
fn fallible_actual_wallet_history_transient_union_retains_causal_allocation() {
    let (mut bound, history, received, sent) = fallible_funded(true);
    let call = record(&bound, received, sent, Some(history), false, 0, false);
    assert_eq!(bound.reconcile(&call), Ok(()));
    let outputs = call.execution.context.circuit_zswap().outputs();
    assert_eq!(outputs[0].provisional_index, 3);
    assert_eq!(outputs[1].provisional_index, 2);
    let selected = bound.wallet_funding.take();
    assert_eq!(bound.reconcile(&call), Err(ZswapIntentError::InputMismatch));
    bound.wallet_funding = selected;
    let selected = bound.transients.take();
    assert_eq!(
        bound.reconcile(&call),
        Err(ZswapIntentError::TransientsUnsupported)
    );
    bound.transients = selected;
    assert_eq!(
        bound.reconcile(&record(
            &bound,
            received,
            sent,
            Some(history),
            true,
            0,
            false
        )),
        Err(ZswapIntentError::TransientInputMismatch)
    );
    assert_eq!(
        bound.reconcile(&record(
            &bound,
            received,
            sent,
            Some(history),
            false,
            1,
            false
        )),
        Err(ZswapIntentError::TransientInputMismatch)
    );
    let mut bad_history = history;
    bad_history.mt_index = 2;
    assert_eq!(
        bound.reconcile(&record(
            &bound,
            received,
            sent,
            Some(bad_history),
            false,
            0,
            false
        )),
        Err(ZswapIntentError::InputIndexMismatch)
    );
    bound.placement = OfferPlacement::Guaranteed;
    assert_eq!(
        bound.reconcile(&call),
        Err(ZswapIntentError::TransientSegmentMismatch)
    );
}

#[test]
fn fallible_selected_proofs_keep_exact_both_half_tags_and_full_identity() {
    let (bound, _, _, _) = fallible_funded(true);
    let placement = bound.placement;
    let transient = bound.offer.transient.get(0).unwrap().clone();
    assert!(matches!(
        ContractTransientCoins::from_transients(vec![transient.clone()]),
        Err(ZswapIntentError::TransientSegmentMismatch)
    ));
    let choose = || {
        ContractTransientCoins::from_transients_for_placement(vec![transient.clone()], placement)
            .unwrap()
    };
    assert_eq!(
        choose().validate_placement(OfferPlacement::Guaranteed),
        Err(ZswapIntentError::TransientSegmentMismatch)
    );
    for input in [true, false] {
        let bad_vectors = if input {
            vec![
                vec![],
                vec![Fr::from(7)],
                vec![Fr::from(0), Fr::from(7)],
                vec![Fr::from(1), Fr::from(0)],
                vec![Fr::from(1), Fr::from(65536)],
                vec![Fr::from(9), Fr::from(1), Fr::from(7)],
            ]
        } else {
            vec![
                vec![],
                vec![Fr::from(0)],
                vec![Fr::from(65536)],
                vec![Fr::from(0), Fr::from(7)],
            ]
        };
        for values in bad_vectors {
            let mut changed = transient.clone();
            let proof = if input {
                &mut changed.proof_input
            } else {
                &mut changed.proof_output
            };
            std::sync::Arc::make_mut(proof).public_transcript_outputs = values;
            assert!(matches!(
                ContractTransientCoins::from_transients_for_placement(vec![changed], placement),
                Err(ZswapIntentError::TransientSegmentMismatch)
            ));
        }
        let mut changed = transient.clone();
        let proof = if input {
            &mut changed.proof_input
        } else {
            &mut changed.proof_output
        };
        std::sync::Arc::make_mut(proof)
            .private_transcript
            .push(Fr::from(1));
        let mut offer = bound.offer.clone();
        offer.transient = vec![changed].into_iter().collect();
        assert_eq!(
            choose().validate_offer(&offer, bound.observed.address),
            Err(ZswapIntentError::TransientSelectionMismatch)
        );
    }
    let funding = bound.wallet_funding.as_ref().unwrap();
    let original = funding.inputs[0].clone();
    let mut changed = original.clone();
    std::sync::Arc::make_mut(&mut changed.proof)
        .private_transcript
        .push(Fr::from(1));
    let mut offer = bound.offer.clone();
    offer.inputs = offer
        .inputs
        .iter_deref()
        .map(|i| {
            if i.nullifier == original.nullifier {
                changed.clone()
            } else {
                i.clone()
            }
        })
        .collect();
    assert_eq!(
        funding.validate_offer(&offer),
        Err(ZswapIntentError::WalletFundingMismatch)
    );
}

#[test]
fn fallible_wallet_transient_without_historical_input_is_explicit() {
    let (bound, _, received, sent) = fallible_funded(false);
    let call = record(&bound, received, sent, None, false, 0, false);
    assert_eq!(bound.reconcile(&call), Ok(()));
    assert_eq!(bound.offer.inputs.len(), 1);
    assert!(
        bound
            .offer
            .inputs
            .get(0)
            .unwrap()
            .contract_address
            .is_none()
    );
    assert_eq!(bound.offer.transient.len(), 1);
    assert_eq!(call.execution.context.circuit_zswap().inputs().len(), 1);
}

#[test]
fn fallible_wallet_and_contract_input_vectors_are_both_exact() {
    let (bound, _, _, _) = fallible_funded(true);
    let transient = bound.offer.transient.get(0).unwrap().clone();
    let options = OfferBindingOptions::default()
        .with_offer_placement(bound.placement)
        .with_output_allocation(PersistentOutputAllocation::CanonicalOfferIndices)
        .with_wallet_funding(
            WalletFundingInputs::from_inputs(bound.wallet_funding.as_ref().unwrap().inputs.clone())
                .unwrap(),
        )
        .with_transient_coins(
            ContractTransientCoins::from_transients_for_placement(vec![transient], bound.placement)
                .unwrap(),
        );
    assert_eq!(
        bound.placement.validate_offer(&bound.offer, &options),
        Ok(())
    );
    for owner in [true, false] {
        for values in [
            vec![],
            vec![Fr::from(7)],
            vec![Fr::from(0), Fr::from(7)],
            vec![Fr::from(1), Fr::from(0)],
            vec![Fr::from(1), Fr::from(65536)],
            vec![Fr::from(9), Fr::from(1), Fr::from(7)],
        ] {
            let mut offer = bound.offer.clone();
            offer.inputs = offer
                .inputs
                .iter_deref()
                .map(|input| {
                    let mut input = input.clone();
                    if input.contract_address.is_some() == owner {
                        std::sync::Arc::make_mut(&mut input.proof).public_transcript_outputs =
                            values.clone();
                    }
                    input
                })
                .collect();
            assert_eq!(
                bound.placement.validate_offer(&offer, &options),
                Err(ZswapIntentError::OfferSegmentMismatch)
            );
        }
    }
}
