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
//! Imported by proof-bootstrap.py into an isolated copy of the normal proof
//! harness with the unedited generated acceptance crate as a real dependency.
use super::*;
use compact_contract_shielded::{ledger_contract as contract, types};
use midnight_compact_runtime as runtime;
use midnight_zswap::{Offer, Output, keys::SecretKeys};
use runtime::transaction::ObservationalOfferBackedState;

fn validate_bootstrap(
    address: runtime::ledger::ContractAddress,
    domain: runtime::ledger::HashOutput,
    amount: u64,
    coin: &runtime::ledger::CoinInfo,
    recipient: &runtime::ledger::CoinRecipient,
    commitment: runtime::ledger::CoinCommitment,
) -> Result<(), &'static str> {
    if coin.value != u128::from(amount) {
        return Err("bootstrap amount differs from supplied coin value");
    }
    if coin.type_ != address.custom_shielded_token_type(domain) {
        return Err("bootstrap coin color differs from deployed mint domain");
    }
    if !matches!(recipient, runtime::ledger::CoinRecipient::User(_)) {
        return Err("bootstrap recipient must be the live wallet");
    }
    if coin.commitment(recipient) != commitment {
        return Err("bootstrap commitment differs from coin and wallet recipient");
    }
    Ok(())
}

pub(super) fn run(root: &Path) -> Result<(), Box<dyn Error>> {
    const NAME: &str = "bootstrap";
    let mut rng = StdRng::seed_from_u64(0x0217_424f);
    let initial = contract::initial_state(ConstructorContext::new(()))?;
    let deploy = make_deploy(root, NAME, initial.ledger_state.get_ref().clone(), &mut rng)?;
    let address = deploy.address();
    let mut state = super::qualified_coin_funding::fee_funded_state(&mut rng)?;
    state.ledger.contract = state
        .ledger
        .contract
        .insert(address, deploy.initial_state.clone());
    let ledger = state.ledger.clone();
    let time = state.time;
    assert_eq!(
        ledger.zswap.first_free, 0,
        "bootstrap must not seed a shielded input"
    );
    let keys = SecretKeys::from_rng_seed(&mut rng);
    let domain = runtime::ledger::HashOutput([0x17; 32]);
    let coin = runtime::ledger::CoinInfo {
        nonce: runtime::ledger::CoinNonce(runtime::ledger::HashOutput([9; 32])),
        type_: address.custom_shielded_token_type(domain),
        value: 42,
    };
    let recipient = runtime::ledger::CoinRecipient::User(keys.coin_public_key());
    let commitment = coin.commitment(&recipient);
    validate_bootstrap(address, domain, 42, &coin, &recipient, commitment)?;
    assert_eq!(
        validate_bootstrap(address, domain, 41, &coin, &recipient, commitment),
        Err("bootstrap amount differs from supplied coin value")
    );
    let mut bad = coin;
    bad.type_ = runtime::ledger::ShieldedTokenType(runtime::ledger::HashOutput([0; 32]));
    assert_eq!(
        validate_bootstrap(address, domain, 42, &bad, &recipient, commitment),
        Err("bootstrap coin color differs from deployed mint domain")
    );
    assert_eq!(
        validate_bootstrap(
            address,
            domain,
            42,
            &coin,
            &recipient,
            runtime::ledger::CoinCommitment(runtime::ledger::HashOutput([0; 32]))
        ),
        Err("bootstrap commitment differs from coin and wallet recipient")
    );
    let output = Output::new(
        &mut rng,
        &coin,
        None,
        &keys.coin_public_key(),
        Some(keys.enc_public_key()),
    )?;
    let offer: Offer<ProofPreimage, DefaultDB> =
        Offer::new(vec![], vec![output], vec![]).ok_or("empty bootstrap offer")?;
    let observed = super::shielded_receive::observed(address, deploy.initial_state.clone());
    let checkpoint = super::observational_binding::checkpoint(&ledger, &observed)?;
    let bound = ObservationalOfferBackedState::bind(observed, checkpoint, offer, None)?;
    let call = contract::Contract::default().recording().bootstrap_call(
        bound.observed(),
        (),
        FixedBytes::new(domain.0),
        BoundedUint::new(42)?,
        types::ShieldedCoinInfo {
            nonce: FixedBytes::new(coin.nonce.0.0),
            color: FixedBytes::new(coin.type_.0.0),
            value: BoundedUint::new(42)?,
        },
        types::Either {
            is_left: true,
            left: types::ZswapCoinPublicKey {
                bytes: FixedBytes::new(keys.coin_public_key().0.0),
            },
            right: types::ContractAddress {
                bytes: FixedBytes::new([0; 32]),
            },
        },
        FixedBytes::new(commitment.0.0),
    )?;
    let verifier: VerifierKey = tagged_deserialize(&mut BufReader::new(File::open(
        root.join("keys/bootstrap.verifier"),
    )?))?;
    let prepared = bound.prepare(call, verifier.clone(), Fr::from(0))?;
    super::kernel_shielded_effects::prove_and_verify_call(
        root,
        NAME,
        prepared.prototype(),
        &verifier,
    )?;
    let tx = prepared.into_transaction(&mut rng, Timestamp::from_secs(time.to_secs() + 3600));
    let resolver = super::qualified_coin_funding::fee_resolver(root, NAME)?;
    let params = MidnightDataProvider::new(FetchMode::OnDemand, OutputMode::Log, vec![])?;
    let provider = LocalProvingProvider {
        rng: StdRng::seed_from_u64(217),
        resolver: &resolver,
        params: &params,
    };
    let proven = futures_executor::block_on(
        tx.prove(provider, &INITIAL_PARAMETERS.cost_model.runtime_cost_model),
    )?;
    let sealed = proven.seal(StdRng::seed_from_u64(218));
    let sealed = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(state.balance_tx(rng, sealed, &resolver))?;
    let verified = sealed.well_formed(&ledger, WellFormedStrictness::default(), time)?;
    let context = TransactionContext {
        ref_state: ledger.clone(),
        block_context: BlockContext {
            tblock: time,
            last_block_time: time,
            ..BlockContext::default()
        },
        whitelist: None,
    };
    let (updated, outcome) = ledger.apply(&verified, &context);
    if !matches!(outcome, TransactionResult::Success(_)) {
        return Err(format!("bootstrap apply failed: {outcome:?}").into());
    }
    assert_eq!(updated.zswap.first_free, 1);
    assert_eq!(
        updated.zswap.coin_coms.index(0).map(|(h, _)| h),
        Some(commitment.0)
    );
    let Transaction::Standard(accepted) = &sealed else {
        return Err("expected standard transaction".into());
    };
    let recovered = midnight_zswap::local::State::<DefaultDB>::new().apply(
        &keys,
        accepted
            .guaranteed_coins
            .as_ref()
            .ok_or("missing accepted output")?,
    );
    let owned = *recovered
        .coins
        .iter()
        .next()
        .ok_or("minted output not recovered by actual wallet keys")?
        .1;
    assert_eq!(recovered.coins.iter().count(), 1);
    assert_eq!(owned, coin.qualify(0));
    assert_eq!(
        recovered.merkle_tree.root(),
        updated.zswap.coin_coms.rehash().root()
    );
    println!(
        "acceptance bootstrap: builder amount/color/commitment refusals; generated mint/output/claim; default-strict proof and apply; encrypted token42 recovered by wallet at index0. Offline fixture, not live admission."
    );
    Ok(())
}
