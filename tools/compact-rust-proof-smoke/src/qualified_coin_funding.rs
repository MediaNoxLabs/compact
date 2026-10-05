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

//! Explicit offline genesis funding for strict qualified-coin proof smokes.
//!
//! `MIDNIGHT_LEDGER_TEST_STATIC_DIR` is required by the upstream fee-token
//! fixture resolver. Contract, Zswap, and Dust proofs use their pinned material.
//! Genesis insertion and the upstream Night reward/Dust registration are
//! privileged fixture setup; default strictness applies to the resulting call.

use super::*;
use midnight_coin_structure::transfer::Recipient;
use midnight_compact_runtime as runtime;
use midnight_ledger::dust::{DUST_EXPECTED_FILES, DustResolver};
use midnight_ledger::prove::Resolver as LedgerResolver;
use midnight_ledger::test_utilities::TestState;
use midnight_zswap::keys::SecretKeys;
use midnight_zswap::local::State as LocalZswapState;
use midnight_zswap::prove::ZswapResolver;
use midnight_zswap::{Input, Offer, Output};

pub(super) struct FundedOffer {
    pub offer: Offer<ProofPreimage, DefaultDB>,
    pub spent_nullifier: midnight_coin_structure::coin::Nullifier,
    pub output_index: u64,
    pub output_commitment: midnight_coin_structure::coin::Commitment,
}

pub(super) fn seed_and_fund(
    rng: &mut StdRng,
    ledger: &mut LedgerState<DefaultDB>,
    output_coin: runtime::ledger::CoinInfo,
    recipient: runtime::ledger::CoinPublicKey,
    time: Timestamp,
) -> Result<FundedOffer, Box<dyn Error>> {
    let keys = SecretKeys::from_rng_seed(rng);
    let mut seed_nonce = [0_u8; 32];
    seed_nonce[..9].copy_from_slice(b"seed-coin");
    let seed_coin = runtime::ledger::CoinInfo {
        nonce: runtime::ledger::CoinNonce(runtime::ledger::HashOutput(seed_nonce)),
        type_: output_coin.type_,
        value: output_coin.value,
    };
    let seed_output = Output::new(
        rng,
        &seed_coin,
        None,
        &keys.coin_public_key(),
        Some(keys.enc_public_key()),
    )?;
    let seed_commitment = seed_output.coin_com;
    let seed_offer: Offer<ProofPreimage, DefaultDB> =
        Offer::new(vec![], vec![seed_output], vec![]).ok_or("empty genesis seed offer")?;
    let (seeded_zswap, seed_indices) = ledger.zswap.try_apply(&seed_offer, None)?;
    // Local genesis insertion is privileged. Seal its Merkle root as a block
    // update so the later input references an accepted historical root.
    ledger.zswap = midnight_storage_core::arena::Sp::new(seeded_zswap.post_block_update(time));
    let seed_index = *seed_indices
        .get(&seed_commitment)
        .ok_or("seed coin commitment was not allocated")?;
    let wallet = LocalZswapState::<DefaultDB>::new().apply(&keys, &seed_offer);
    if wallet.merkle_tree.root() != ledger.zswap.coin_coms.root() {
        return Err("seeded wallet and ledger Zswap trees differ".into());
    }
    let owned_coin = *wallet
        .coins
        .iter()
        .next()
        .ok_or("seed coin not recovered")?
        .1;
    if wallet.coins.iter().count() != 1 || owned_coin.mt_index != seed_index {
        return Err("seed coin index or ownership differs".into());
    }
    let (_, input): (_, Input<ProofPreimage, DefaultDB>) =
        wallet.spend(rng, &keys, &owned_coin, None)?;
    let spent_nullifier = input.nullifier;
    let output = Output::new(rng, &output_coin, None, &recipient, None)?;
    let output_commitment = output.coin_com;
    if output_commitment != output_coin.commitment(&Recipient::User(recipient)) {
        return Err("funded output commitment differs from coin and recipient".into());
    }
    let offer =
        Offer::new(vec![input], vec![output], vec![]).ok_or("funded offer unexpectedly empty")?;
    let (_, indices) = ledger.zswap.try_apply(&offer, None)?;
    let output_index = *indices
        .get(&output_commitment)
        .ok_or("funded output commitment was not allocated")?;
    if output_index != ledger.zswap.first_free {
        return Err("funded output index differs from ledger cursor".into());
    }
    Ok(FundedOffer {
        offer,
        spent_nullifier,
        output_index,
        output_commitment,
    })
}

pub(super) fn fee_funded_state(rng: &mut StdRng) -> Result<TestState<DefaultDB>, Box<dyn Error>> {
    std::env::var("MIDNIGHT_LEDGER_TEST_STATIC_DIR")
        .map_err(|_| "set MIDNIGHT_LEDGER_TEST_STATIC_DIR for the upstream fee funding fixture")?;
    let mut state = TestState::new(rng);
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(state.give_fee_token(rng, 1));
    // give_fee_token registers Dust generation, rewards Night, and advances
    // fixture time to the Dust cap. balance_tx later proves the fee spend.
    Ok(state)
}

pub(super) fn fee_resolver(
    root: &Path,
    circuit: &'static str,
) -> Result<LedgerResolver, Box<dyn Error>> {
    let zswap = ZswapResolver(MidnightDataProvider::new(
        FetchMode::Synchronous,
        OutputMode::Log,
        midnight_zswap::ZSWAP_EXPECTED_FILES.to_owned(),
    )?);
    let dust = DustResolver(MidnightDataProvider::new(
        FetchMode::Synchronous,
        OutputMode::Log,
        DUST_EXPECTED_FILES.to_owned(),
    )?);
    let root = root.to_owned();
    Ok(LedgerResolver::new(
        zswap,
        dust,
        Box::new(move |location| {
            let resolver = ArtifactResolver {
                root: root.clone(),
                circuit,
            };
            Box::pin(async move { resolver.resolve_key(location).await })
        }),
    ))
}
