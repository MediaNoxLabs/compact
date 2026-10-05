// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Explicit trusted observations, without pretending to possess full chain state.
use super::*;
use midnight_storage::arena::Sp;
use midnight_transient_crypto::merkle_tree::{MerkleTree, MerkleTreeDigest};

/// Caller-supplied checkpoint facts. The network adapter must verify canonical
/// finality, endpoint identity and that the event/frontier belong to this block.
/// A constructor taking these values cannot authenticate those network claims.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CheckpointMetadata {
    pub observation: Observation,
    pub address: ContractAddress,
    pub network_id: String,
    pub ledger_version: String,
    pub zswap_root: MerkleTreeDigest,
    pub first_free: u64,
    pub final_zswap_event_id: u64,
}

/// Adapter evidence accompanying the exact serialized synchronized wallet state.
/// `applied_event_id` is a Zswap event ID, not a tree index or block height.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WalletCheckpointMetadata {
    pub block_hash: [u8; 32],
    pub block_height: u64,
    pub network_id: String,
    pub ledger_version: String,
    pub applied_event_id: u64,
    /// Adapter-computed SHA256 of the exact acquired serialized wallet bytes.
    /// Verify before decoding. Runtime retains this claim; it cannot validate
    /// the original encoding from the decoded state and does not reserialize it.
    pub wallet_state_sha256: [u8; 32],
}

/// Owned source values for internal checkpoint consistency checking. Wallet state
/// may contain private data; it is consumed and only its root/frontier retained.
pub struct TrustedObservationParts<D: DB = DefaultDB> {
    pub metadata: CheckpointMetadata,
    pub node_contract: ContractState<D>,
    pub contract_tree: MerkleTree<Option<Sp<ContractAddress, D>>, D>,
    pub wallet: midnight_zswap::local::State<D>,
    pub wallet_checkpoint: WalletCheckpointMetadata,
}

/// Checked immutable evidence, not a consensus proof. Construction validates
/// internal agreement only. The adapter authenticates its trusted endpoints.
pub struct TrustedObservationCheckpoint<D: DB = DefaultDB> {
    evidence: TrustedCheckpointEvidence,
    contract: ContractState<D>,
    tree: MerkleTree<Option<Sp<ContractAddress, D>>, D>,
}

/// Immutable public receipt for the observation policy; no wallet secrets.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TrustedCheckpointEvidence {
    metadata: CheckpointMetadata,
    wallet: WalletCheckpointMetadata,
}
impl TrustedCheckpointEvidence {
    pub fn metadata(&self) -> &CheckpointMetadata {
        &self.metadata
    }
    pub fn wallet(&self) -> &WalletCheckpointMetadata {
        &self.wallet
    }
}

/// Local admission policy retained through preparation. Neither variant alone
/// proves transaction validity or finality; node submission remains necessary.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OfferAdmission {
    /// The existing binder additionally applied the offer to a complete ledger
    /// snapshot, including its known-root/nullifier/commitment history checks.
    CompleteLedger,
    /// Only internally consistent trusted observations were available. Global
    /// history, prior nullifiers/commitments and concurrent freshness are left
    /// to node admission. The adapter verifies block/event/network provenance.
    TrustedObservation(Box<TrustedCheckpointEvidence>),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ObservationBindingError {
    NetworkMismatch,
    LedgerVersionMismatch,
    BlockMismatch,
    EventMismatch,
    TreeHeightMismatch,
    RootMismatch,
    FrontierMismatch,
    ContractMismatch,
    UnsupportedOffer,
    UnnormalizedOffer,
    DuplicateCommitment,
    DuplicateNullifier,
    InputRootMismatch,
    InputOwnerMismatch,
    WalletFundingMismatch,
    SegmentMismatch,
}
impl fmt::Display for ObservationBindingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "trusted observation binding rejected: {self:?}")
    }
}
impl Error for ObservationBindingError {}

impl<D: DB> TrustedObservationCheckpoint<D> {
    /// Check consistency of caller-trusted sources. Does not contact a node,
    /// authenticate arbitrary hashes, validate consensus, or infer history.
    pub fn from_trusted_sources(
        parts: TrustedObservationParts<D>,
    ) -> Result<Self, ObservationBindingError> {
        let TrustedObservationParts {
            metadata,
            node_contract,
            contract_tree,
            wallet,
            wallet_checkpoint,
        } = parts;
        if metadata.network_id.is_empty() || metadata.network_id != wallet_checkpoint.network_id {
            return Err(ObservationBindingError::NetworkMismatch);
        }
        if metadata.ledger_version != "8.0.3"
            || metadata.ledger_version != wallet_checkpoint.ledger_version
        {
            return Err(ObservationBindingError::LedgerVersionMismatch);
        }
        if metadata.observation.block_hash != wallet_checkpoint.block_hash
            || metadata.observation.block_height != wallet_checkpoint.block_height
        {
            return Err(ObservationBindingError::BlockMismatch);
        }
        if metadata.final_zswap_event_id != wallet_checkpoint.applied_event_id {
            return Err(ObservationBindingError::EventMismatch);
        }
        if contract_tree.height() != midnight_zswap::ZSWAP_TREE_HEIGHT
            || wallet.merkle_tree.height() != midnight_zswap::ZSWAP_TREE_HEIGHT
        {
            return Err(ObservationBindingError::TreeHeightMismatch);
        }
        if contract_tree.rehash().root() != Some(metadata.zswap_root)
            || wallet.merkle_tree.rehash().root() != Some(metadata.zswap_root)
        {
            return Err(ObservationBindingError::RootMismatch);
        }
        if metadata.first_free != wallet.first_free
            || metadata.first_free > (1u64 << midnight_zswap::ZSWAP_TREE_HEIGHT)
        {
            return Err(ObservationBindingError::FrontierMismatch);
        }
        Ok(Self {
            evidence: TrustedCheckpointEvidence {
                metadata,
                wallet: wallet_checkpoint,
            },
            contract: node_contract,
            tree: contract_tree,
        })
    }
    pub fn evidence(&self) -> &TrustedCheckpointEvidence {
        &self.evidence
    }
}

/// A distinct observation-based offer binding. No fake LedgerState is built.
/// Preserves normalized upstream ordering and exact intent/funding checks, but
/// cannot locally establish global nullifier, prior commitment or root history.
/// Initially supports guaranteed persistent offers only; no transients/change
/// inference, input-only burns, fallible placement or automatic selection of wallet funding.
pub struct ObservationalOfferBackedState<D: DB = DefaultDB> {
    observed: ObservedContractState<D>,
    checkpoint: TrustedObservationCheckpoint<D>,
    offer: Offer<ProofPreimage, D>,
    wallet_funding: Option<WalletFundingInputs<D>>,
}
impl<D: DB> ObservationalOfferBackedState<D> {
    pub fn bind(
        mut observed: ObservedContractState<D>,
        checkpoint: TrustedObservationCheckpoint<D>,
        offer: Offer<ProofPreimage, D>,
        wallet_funding: Option<WalletFundingInputs<D>>,
    ) -> Result<Self, ObservationBindingError> {
        let metadata = &checkpoint.evidence.metadata;
        if observed.address != metadata.address
            || observed.observation != metadata.observation
            || observed.contract != checkpoint.contract
        {
            return Err(ObservationBindingError::ContractMismatch);
        }
        if !offer.transient.is_empty() || offer.outputs.is_empty() {
            return Err(ObservationBindingError::UnsupportedOffer);
        }
        let mut normalized = offer.clone();
        normalized.normalize();
        if normalized != offer {
            return Err(ObservationBindingError::UnnormalizedOffer);
        }
        if offer
            .inputs
            .iter_deref()
            .any(|i| i.proof.public_transcript_outputs != [Fr::from(1), Fr::from(0)])
            || offer
                .outputs
                .iter_deref()
                .any(|o| o.proof.public_transcript_outputs != [Fr::from(0)])
        {
            return Err(ObservationBindingError::SegmentMismatch);
        }
        if let Some(funding) = &wallet_funding {
            funding
                .validate_offer(&offer)
                .map_err(|_| ObservationBindingError::WalletFundingMismatch)?;
        }
        let mut nullifiers = std::collections::HashSet::new();
        for input in offer.inputs.iter_deref() {
            if !nullifiers.insert(input.nullifier) {
                return Err(ObservationBindingError::DuplicateNullifier);
            }
            if input.merkle_tree_root != metadata.zswap_root {
                return Err(ObservationBindingError::InputRootMismatch);
            }
            match &input.contract_address {
                Some(owner) if **owner != observed.address => {
                    return Err(ObservationBindingError::InputOwnerMismatch);
                }
                None if !wallet_funding
                    .as_ref()
                    .is_some_and(|f| f.inputs.iter().any(|selected| selected == input)) =>
                {
                    return Err(ObservationBindingError::WalletFundingMismatch);
                }
                _ => (),
            }
        }
        let end = metadata
            .first_free
            .checked_add(offer.outputs.len() as u64)
            .ok_or(ObservationBindingError::FrontierMismatch)?;
        if end > (1u64 << midnight_zswap::ZSWAP_TREE_HEIGHT) {
            return Err(ObservationBindingError::FrontierMismatch);
        }
        let mut indices = Map::new();
        let mut rows = Vec::with_capacity(offer.outputs.len());
        for (offset, output) in offer.outputs.iter_deref().enumerate() {
            if indices.contains_key(&output.coin_com) {
                return Err(ObservationBindingError::DuplicateCommitment);
            }
            let index = metadata.first_free + offset as u64;
            indices = indices.insert(output.coin_com, index);
            rows.push(crate::zswap::BoundOutput {
                commitment: output.coin_com,
                index,
                owner: output.contract_address.as_ref().map(|o| **o),
            });
        }
        observed.com_indices = indices;
        observed.allocation = crate::zswap::Allocation::CanonicalOfferBound {
            start: metadata.first_free,
            outputs: rows,
        };
        Ok(Self {
            observed,
            checkpoint,
            offer,
            wallet_funding,
        })
    }
    pub fn observed(&self) -> &ObservedContractState<D> {
        &self.observed
    }
    pub fn evidence(&self) -> &TrustedCheckpointEvidence {
        self.checkpoint.evidence()
    }
    fn reconciliation(&self) -> OfferReconciliation<'_, D> {
        OfferReconciliation {
            observed: &self.observed,
            offer: &self.offer,
            tree: &self.checkpoint.tree,
            frontier: self.checkpoint.evidence.metadata.first_free,
            wallet_funding: self.wallet_funding.as_ref(),
            transients: None,
            placement: OfferPlacement::Guaranteed,
        }
    }
    pub fn prepare<Private, Output: Into<AlignedValue>>(
        &self,
        call: RecordedCall<'_, Private, Output, D>,
        verifier: VerifierKey,
        communication_commitment_rand: Fr,
    ) -> Result<ObservationBoundPreparedCall<D>, ObservedCallError> {
        if !std::ptr::eq(call.observed, &self.observed)
            || call.recorded.public.initial().call_context.com_indices != self.observed.com_indices
        {
            return Err(ObservedCallError::OfferMismatch);
        }
        self.reconciliation()
            .reconcile(&call.recorded)
            .map_err(ObservedCallError::ZswapIntent)?;
        let prepared = call.prepare_inner(verifier, communication_commitment_rand, true)?;
        if prepared.fallible_public_transcript.is_some() {
            return Err(ObservedCallError::ZswapIntent(
                ZswapIntentError::OfferTranscriptPlacementMismatch,
            ));
        }
        Ok(ObservationBoundPreparedCall {
            inner: OfferBoundPreparedCall {
                call: prepared,
                offer: self.offer.clone(),
                placement: OfferPlacement::Guaranteed,
                admission: OfferAdmission::TrustedObservation(Box::new(
                    self.checkpoint.evidence.clone(),
                )),
            },
        })
    }
}

/// Prepared observation-bound call. The network identity is sealed to its
/// checkpoint; callers cannot replace it while constructing the transaction.
pub struct ObservationBoundPreparedCall<D: DB = DefaultDB> {
    inner: OfferBoundPreparedCall<D>,
}
impl<D: DB> ObservationBoundPreparedCall<D> {
    pub fn prototype(&self) -> &ContractCallPrototype<D> {
        self.inner.prototype()
    }
    pub fn admission(&self) -> &OfferAdmission {
        self.inner.admission()
    }
    pub fn into_transaction<R: Rng + CryptoRng + ?Sized>(
        self,
        rng: &mut R,
        ttl: Timestamp,
    ) -> Transaction<Signature, ProofPreimageMarker, PedersenRandomness, D> {
        let OfferAdmission::TrustedObservation(evidence) = &self.inner.admission else {
            unreachable!("private observation-only constructor")
        };
        let network = evidence.metadata.network_id.clone();
        self.inner.into_transaction(rng, network, ttl)
    }
}

#[cfg(test)]
#[path = "observation_tests.rs"]
mod tests;
