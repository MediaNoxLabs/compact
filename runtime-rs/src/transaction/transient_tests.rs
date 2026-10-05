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
