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
//! Offline differential observation evidence. These projections come from an
//! actual complete test ledger; they are not live wallet/indexer observations.
use super::*;
use midnight_compact_runtime::transaction::{
    CheckpointMetadata, ObservationBoundPreparedCall, OfferBoundPreparedCall,
    TrustedObservationCheckpoint, TrustedObservationParts, WalletCheckpointMetadata,
};
use midnight_transient_crypto::merkle_tree::MerkleTreeCollapsedUpdate;

pub(super) fn checkpoint(
    ledger: &LedgerState<DefaultDB>,
    observed: &ObservedContractState,
) -> Result<TrustedObservationCheckpoint, Box<dyn Error>> {
    let frontier = ledger.zswap.first_free;
    let wallet: midnight_zswap::local::State<DefaultDB> = if frontier == 0 {
        midnight_zswap::local::State::new()
    } else {
        midnight_zswap::local::State::new().apply_collapsed_update(
            &MerkleTreeCollapsedUpdate::new(&ledger.zswap.coin_coms, 0, frontier - 1)?,
        )?
    };
    // Offline adapter analogue: hash the exact serialized snapshot before
    // decoding, never reconstruct a raw acquisition hash inside the runtime.
    let mut wallet_bytes = vec![];
    tagged_serialize(&wallet, &mut wallet_bytes)?;
    let wallet_state_sha256 = midnight_base_crypto::hash::persistent_hash(&wallet_bytes).0;
    let wallet = tagged_deserialize(&mut &wallet_bytes[..])?;
    let observation = observed.observation();
    Ok(TrustedObservationCheckpoint::from_trusted_sources(
        TrustedObservationParts {
            metadata: CheckpointMetadata {
                observation,
                address: observed.address(),
                network_id: "local-test".into(),
                ledger_version: "8.0.3".into(),
                zswap_root: ledger
                    .zswap
                    .coin_coms
                    .rehash()
                    .root()
                    .ok_or("unhashed tree")?,
                first_free: frontier,
                final_zswap_event_id: 1,
            },
            node_contract: ledger
                .contract
                .get(&observed.address())
                .ok_or("missing contract")?
                .clone(),
            contract_tree: ledger.zswap.filter(&[observed.address()]),
            wallet,
            wallet_checkpoint: WalletCheckpointMetadata {
                block_hash: observation.block_hash,
                block_height: observation.block_height,
                network_id: "local-test".into(),
                ledger_version: "8.0.3".into(),
                applied_event_id: 1,
                wallet_state_sha256,
            },
        },
    )?)
}

pub(super) fn same_preimage(
    a: &ContractCallPrototype<DefaultDB>,
    b: &ContractCallPrototype<DefaultDB>,
) -> Result<(), Box<dyn Error>> {
    let encode = |call| -> Result<Vec<u8>, Box<dyn Error>> {
        let proof =
            <ProofPreimage as ContractCallExt<DefaultDB>>::construct_proof(call, Fr::from(0));
        let mut bytes = vec![];
        tagged_serialize(&proof, &mut bytes)?;
        Ok(bytes)
    };
    assert_eq!(encode(a)?, encode(b)?);
    Ok(())
}

pub(super) trait ProofBoundCall {
    fn prototype(&self) -> &ContractCallPrototype<DefaultDB>;
    fn into_test_transaction(
        self,
        rng: &mut StdRng,
        ttl: Timestamp,
    ) -> Transaction<Signature, ProofPreimageMarker, PedersenRandomness, DefaultDB>;
}
impl ProofBoundCall for OfferBoundPreparedCall {
    fn prototype(&self) -> &ContractCallPrototype<DefaultDB> {
        self.prototype()
    }
    fn into_test_transaction(
        self,
        rng: &mut StdRng,
        ttl: Timestamp,
    ) -> Transaction<Signature, ProofPreimageMarker, PedersenRandomness, DefaultDB> {
        self.into_transaction(rng, "local-test", ttl)
    }
}
impl ProofBoundCall for ObservationBoundPreparedCall {
    fn prototype(&self) -> &ContractCallPrototype<DefaultDB> {
        self.prototype()
    }
    fn into_test_transaction(
        self,
        rng: &mut StdRng,
        ttl: Timestamp,
    ) -> Transaction<Signature, ProofPreimageMarker, PedersenRandomness, DefaultDB> {
        self.into_transaction(rng, ttl)
    }
}
