// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Explicit single-offer placement. Source transcripts and upstream proofs are
//! retained unchanged; this policy never retargets a proof or infers a segment.
use super::*;
use std::num::NonZeroU16;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum OfferPlacement {
    /// Existing guaranteed offer placement; the call intent is at segment 1.
    #[default]
    Guaranteed,
    /// One wholly fallible call and its entire persistent offer at this segment.
    /// No guaranteed Zswap offer or additional intent is constructed.
    Fallible(NonZeroU16),
}
impl OfferPlacement {
    pub(super) fn validate_offer<D: DB>(
        self,
        offer: &Offer<ProofPreimage, D>,
        options: &OfferBindingOptions<D>,
    ) -> Result<(), ZswapIntentError> {
        let Self::Fallible(segment) = self else {
            return Ok(());
        };
        if options.output_allocation != PersistentOutputAllocation::CanonicalOfferIndices
            || options.wallet_funding.is_some()
            || options.transients.is_some()
            || !offer.transient.is_empty()
            || (offer.inputs.is_empty() && offer.outputs.is_empty())
        {
            return Err(ZswapIntentError::OfferPlacementUnsupported);
        }
        // Exact pinned ledger8 return shapes; segment() alone accepts malformed
        // vectors with a valid last element and maps overflow to None.
        let segment = Fr::from(u64::from(segment.get()));
        if offer
            .inputs
            .iter_deref()
            .any(|input| input.proof.public_transcript_outputs != [Fr::from(1), segment])
            || offer
                .outputs
                .iter_deref()
                .any(|output| output.proof.public_transcript_outputs != [segment])
        {
            return Err(ZswapIntentError::OfferSegmentMismatch);
        }
        Ok(())
    }
    pub(super) fn validate_prototype<D: DB>(
        self,
        call: &ContractCallPrototype<D>,
    ) -> Result<(), ZswapIntentError> {
        if matches!(self, Self::Fallible(_))
            && (call.guaranteed_public_transcript.is_some()
                || call.fallible_public_transcript.is_none())
        {
            return Err(ZswapIntentError::OfferTranscriptPlacementMismatch);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::recording::RecordingFrame;
    use rand::{SeedableRng, rngs::StdRng};

    fn options(n: u16) -> OfferBindingOptions {
        OfferBindingOptions::default()
            .with_output_allocation(PersistentOutputAllocation::CanonicalOfferIndices)
            .with_offer_placement(OfferPlacement::Fallible(NonZeroU16::new(n).unwrap()))
    }
    fn bound(
        n: u16,
    ) -> (
        OfferBackedObservedState,
        crate::ledger::QualifiedCoinInfo,
        crate::ledger::CoinInfo,
        crate::ledger::CoinRecipient,
    ) {
        let (base, input, output, recipient) = super::super::zswap_tests::setup();
        let mut ledger = LedgerState::new("local-test");
        ledger.zswap = midnight_storage::arena::Sp::new(base.zswap.clone());
        ledger.contract = ledger
            .contract
            .insert(base.observed.address, base.observed.contract.clone());
        let bound = OfferBackedObservedState::with_options(
            ObservedContractState::new(
                base.observed.address,
                base.observed.contract.clone(),
                base.observed.observation,
            ),
            &ledger,
            base.offer.retarget_segment(n),
            options(n),
        )
        .unwrap();
        (bound, input, output, recipient)
    }
    #[test]
    fn exact_segment_vectors_and_policy_restrictions() {
        let (base, _, _, _) = bound(7);
        let good = options(7);
        assert_eq!(good.placement.validate_offer(&base.offer, &good), Ok(()));
        for values in [
            vec![],
            vec![Fr::from(7)],
            vec![Fr::from(0), Fr::from(7)],
            vec![Fr::from(1), Fr::from(1)],
            vec![Fr::from(1), Fr::from(65536)],
            vec![Fr::from(9), Fr::from(1), Fr::from(7)],
        ] {
            let mut offer = base.offer.clone();
            let mut input = offer.inputs.get(0).unwrap().clone();
            std::sync::Arc::make_mut(&mut input.proof).public_transcript_outputs = values;
            offer.inputs = vec![input].into_iter().collect();
            assert_eq!(
                good.placement.validate_offer(&offer, &good),
                Err(ZswapIntentError::OfferSegmentMismatch)
            );
        }
        for values in [
            vec![],
            vec![Fr::from(1)],
            vec![Fr::from(65536)],
            vec![Fr::from(0), Fr::from(7)],
        ] {
            let mut offer = base.offer.clone();
            let mut output = offer.outputs.get(0).unwrap().clone();
            std::sync::Arc::make_mut(&mut output.proof).public_transcript_outputs = values;
            offer.outputs = vec![output].into_iter().collect();
            assert_eq!(
                good.placement.validate_offer(&offer, &good),
                Err(ZswapIntentError::OfferSegmentMismatch)
            );
        }
        let exact = options(7).with_output_allocation(PersistentOutputAllocation::ExactIntentOrder);
        assert_eq!(
            exact.placement.validate_offer(&base.offer, &exact),
            Err(ZswapIntentError::OfferPlacementUnsupported)
        );
        let wallet = options(7).with_wallet_funding(WalletFundingInputs { inputs: vec![] });
        assert_eq!(
            wallet.placement.validate_offer(&base.offer, &wallet),
            Err(ZswapIntentError::OfferPlacementUnsupported)
        );
        let mut rng = StdRng::seed_from_u64(206);
        let coin = super::super::zswap_tests::coin(6);
        let output = midnight_zswap::Output::new_contract_owned(
            &mut rng,
            &coin,
            None,
            base.observed.address,
        )
        .unwrap();
        let transient = midnight_zswap::Transient::new_from_contract_owned_output(
            &mut rng,
            &coin.qualify(0),
            None,
            output,
        )
        .unwrap();
        let selected = options(7).with_transient_coins(
            ContractTransientCoins::from_transients(vec![transient.clone()]).unwrap(),
        );
        assert_eq!(
            selected.placement.validate_offer(&base.offer, &selected),
            Err(ZswapIntentError::OfferPlacementUnsupported)
        );
        let mut containing = base.offer.clone();
        containing.transient = vec![transient].into_iter().collect();
        assert_eq!(
            good.placement.validate_offer(&containing, &good),
            Err(ZswapIntentError::OfferPlacementUnsupported)
        );
        let mut empty = base.offer.clone();
        empty.inputs = Default::default();
        empty.outputs = Default::default();
        assert_eq!(
            good.placement.validate_offer(&empty, &good),
            Err(ZswapIntentError::OfferPlacementUnsupported)
        );
        // The unchanged default policy never retargets or interprets these tags.
        assert_eq!(
            OfferPlacement::Guaranteed.validate_offer(&base.offer, &OfferBindingOptions::default()),
            Ok(())
        );
    }
    #[test]
    fn public_prepare_empty_and_guaranteed_refusal_and_non_one_transaction_key() {
        let (mut bound, input, output, recipient) = bound(7);
        let mut rng = StdRng::seed_from_u64(206);
        let verifier: VerifierKey = rng.r#gen();
        bound.observed.contract.operations = bound.observed.contract.operations.insert(
            EntryPointBuf(b"placement".to_vec()),
            ContractOperation::new(Some(verifier.clone())),
        );
        let empty = RecordingFrame::new(bound.observed().circuit_context(()))
            .kernel_self()
            .unwrap()
            .0
            .finish(());
        let call = RecordedCall::new(bound.observed(), empty, "placement", ());
        assert!(matches!(
            bound.prepare(call, verifier.clone(), Fr::from(0)),
            Err(ObservedCallError::ZswapIntent(
                ZswapIntentError::CanonicalEmptyPlan
            ))
        ));
        let record = || {
            RecordingFrame::new(bound.observed().circuit_context(()))
                .create_zswap_input(input)
                .create_zswap_output(output, recipient.clone())
                .unwrap()
                .kernel_self()
                .unwrap()
                .0
                .finish(())
        };
        let call = RecordedCall::new(bound.observed(), record(), "placement", ());
        assert!(matches!(
            bound.prepare(call, verifier.clone(), Fr::from(0)),
            Err(ObservedCallError::ZswapIntent(
                ZswapIntentError::OfferTranscriptPlacementMismatch
            ))
        ));
        // Structural construction test only: the actual original withdraw proof
        // exercises public whole-fallible preparation and ledger acceptance.
        let call = RecordedCall::new(bound.observed(), record(), "placement", ());
        let mut prototype = call.prepare_inner(verifier, Fr::from(0), true).unwrap();
        prototype.fallible_public_transcript = prototype.guaranteed_public_transcript.take();
        assert_eq!(bound.placement.validate_prototype(&prototype), Ok(()));
        let mut mixed = prototype.clone();
        mixed.guaranteed_public_transcript = mixed.fallible_public_transcript.clone();
        assert_eq!(
            bound.placement.validate_prototype(&mixed),
            Err(ZswapIntentError::OfferTranscriptPlacementMismatch)
        );
        let prepared = OfferBoundPreparedCall {
            call: prototype,
            offer: bound.offer.clone(),
            placement: bound.placement,
        };
        let retained = bound.offer.clone();
        let Transaction::Standard(tx) =
            prepared.into_transaction(&mut rng, "local-test", Timestamp::from_secs(100))
        else {
            panic!("standard transaction")
        };
        assert!(tx.guaranteed_coins.is_none());
        assert_eq!(
            tx.intents.iter().map(|row| *row.0).collect::<Vec<_>>(),
            vec![7]
        );
        assert_eq!(
            tx.fallible_coins
                .iter()
                .map(|row| *row.0)
                .collect::<Vec<_>>(),
            vec![7]
        );
        assert_eq!(*tx.fallible_coins.get(&7).unwrap(), retained);
    }
}
