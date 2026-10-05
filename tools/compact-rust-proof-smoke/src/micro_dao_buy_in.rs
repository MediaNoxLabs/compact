// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Original buy_in: guaranteed wallet funding, optional historical merge and mint.
//! Prior contract/coin state is explicitly seeded offline, not a proved DAO lifecycle.
use super::*;
use compact_rust_test_center_micro_dao_fixture::{
    ledger_contract as c, ledger_slots as slots, types,
};
use midnight_compact_runtime as runtime;
use midnight_zswap::{Input, Offer, Output, Transient};
use runtime::transaction::{
    ContractTransientCoins, OfferBackedObservedState, OfferBindingOptions, OfferPlacement,
    PersistentOutputAllocation, WalletFundingInputs,
};

fn compact_coin(coin: runtime::ledger::CoinInfo) -> types::ShieldedCoinInfo {
    types::ShieldedCoinInfo {
        nonce: FixedBytes::new(coin.nonce.0.0),
        color: FixedBytes::new(coin.type_.0.0),
        value: BoundedUint::new(coin.value).unwrap(),
    }
}
fn compact_qualified(coin: runtime::ledger::QualifiedCoinInfo) -> types::QualifiedShieldedCoinInfo {
    let c = compact_coin((&coin).into());
    types::QualifiedShieldedCoinInfo {
        nonce: c.nonce,
        color: c.color,
        value: c.value,
        mt_index: BoundedUint::new(coin.mt_index.into()).unwrap(),
    }
}
pub(super) fn run(root: &Path) -> Result<(), Box<dyn Error>> {
    for occupied in [false, true] {
        run_case(root, occupied)?;
    }
    Ok(())
}
fn run_case(root: &Path, occupied: bool) -> Result<(), Box<dyn Error>> {
    let name = "buy_in";
    let mut rng = StdRng::seed_from_u64(0x0215_0000 + u64::from(occupied));
    let h = runtime::ledger::coin_info_from_compact(
        FixedBytes::new([1; 32]),
        FixedBytes::new([0; 32]),
        17,
    );
    let w = runtime::ledger::coin_info_from_compact(
        FixedBytes::new([3; 32]),
        FixedBytes::new([0; 32]),
        6,
    );
    let initial = c::initial_state(
        ConstructorContext::new(()),
        FixedBytes::new([4; 32]),
        types::Costs {
            seed_dust: BoundedUint::new(10)?,
            buy_in_dust: BoundedUint::new(3)?,
        },
    )?;
    let mut prior = initial.into_circuit_context(runtime::ledger::ContractAddress::default());
    if occupied {
        prior = slots::pot
            .write(prior, compact_qualified(h.qualify(1)))?
            .context;
        prior = slots::pot_has_coin.write(prior, true)?.context;
    }
    let deploy = make_deploy(root, name, prior.query.state.get_ref().clone(), &mut rng)?;
    let address = deploy.address();
    println!("buy_in occupied={occupied}: deployment prepared");
    let mut fees = super::qualified_coin_funding::fee_funded_state(&mut rng)?;
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(fees.give_fee_token(&mut rng, 10));
    fees.ledger.contract = fees
        .ledger
        .contract
        .insert(address, deploy.initial_state.clone());
    println!("buy_in occupied={occupied}: fee fixture prepared");
    let keys = midnight_zswap::keys::SecretKeys::from_rng_seed(&mut rng);
    // Offline prior-state fixture. Sequential seeds fix wallet at index0 and
    // history at nonzero index1 without a deployment-address/normalized-order cycle.
    let seed_h = Offer::new(
        vec![],
        vec![Output::new_contract_owned(&mut rng, &h, None, address)?],
        vec![],
    )
    .unwrap();
    let seed_w = Offer::new(
        vec![],
        vec![Output::new(
            &mut rng,
            &w,
            None,
            &keys.coin_public_key(),
            Some(keys.enc_public_key()),
        )?],
        vec![],
    )
    .unwrap();
    let (state, _) = fees.ledger.zswap.try_apply(&seed_w, None)?;
    let (state, _) = state.try_apply(&seed_h, None)?;
    fees.ledger.zswap = midnight_storage_core::arena::Sp::new(state.post_block_update(fees.time));
    let ledger = fees.ledger.clone();
    assert_eq!(ledger.zswap.first_free, 2);
    let wallet = midnight_zswap::local::State::<DefaultDB>::new()
        .apply(&keys, &seed_w)
        .apply(&keys, &seed_h);
    assert_eq!(wallet.merkle_tree.root(), ledger.zswap.coin_coms.root());
    let segment = None;
    let placement = OfferPlacement::Guaranteed;
    let (_, wallet_input) = wallet.spend(&mut rng, &keys, &w.qualify(0), segment)?;
    assert_eq!(
        wallet_input.proof.public_transcript_outputs,
        vec![Fr::from(1), Fr::from(0)]
    );
    let mut nullifiers = vec![wallet_input.nullifier];
    let funding = WalletFundingInputs::from_inputs(vec![wallet_input.clone()])
        .map_err(|e| format!("wallet selection {e:?}"))?;
    let mut inputs = vec![wallet_input];
    let mut transients = vec![];
    if occupied {
        let input = Input::new_contract_owned(
            &mut rng,
            &h.qualify(1),
            segment,
            address,
            &ledger.zswap.coin_coms,
        )?;
        nullifiers.push(input.nullifier);
        inputs.push(input);
        let received = Output::new_contract_owned(&mut rng, &w, segment, address)?;
        let transient =
            Transient::new_from_contract_owned_output(&mut rng, &w.qualify(0), segment, received)?;
        nullifiers.push(transient.nullifier);
        transients.push(transient);
    }
    println!("buy_in occupied={occupied}: funding components prepared");
    let observe = || {
        ObservedContractState::new(
            address,
            deploy.initial_state.clone(),
            Observation {
                transaction_hash: [0; 32],
                block_hash: [0; 32],
                block_height: 0,
            },
        )
        .with_coin_public_key(keys.coin_public_key())
    };
    // Offline derivation uses a provisional context, not a locked observation.
    let mut provisional =
        runtime::context::CircuitContext::from_contract_state((), address, &deploy.initial_state)
            .with_coin_public_key(keys.coin_public_key());
    provisional.set_zswap_output_start(2)?;
    let provisional = c::buy_in(provisional, compact_coin(w), BoundedUint::new(2)?)?;
    let minted = provisional.result.clone();
    assert_eq!(minted.value.value(), 2);
    assert_eq!(
        minted.color,
        c::dao_voting_token(runtime::context::CircuitContext::from_contract_state(
            (),
            address,
            &deploy.initial_state
        ))?
        .result
    );
    let mut nonce_domain = [0u8; 32];
    nonce_domain[..28].copy_from_slice(b"midnight:kernel:nonce_evolve");
    assert_eq!(
        minted.nonce,
        runtime::upgrade_from_transient(runtime::transient_hash((
            runtime::Field::from_le_bytes(&nonce_domain).unwrap(),
            runtime::Field::from(0u64),
            runtime::degrade_to_transient(FixedBytes::new(w.nonce.0.0))
        )))
    );
    let pot = slots::pot.read(provisional.context)?.result;
    assert_eq!(pot.value.value(), if occupied { 23 } else { 6 });
    assert_eq!(pot.mt_index.value(), if occupied { 3 } else { 2 });
    let final_coin =
        runtime::ledger::coin_info_from_compact(pot.nonce, pot.color, pot.value.value());
    let output = Output::new_contract_owned(&mut rng, &final_coin, segment, address)?;
    let final_commitment = output.coin_com;
    assert_eq!(runtime::ledger::HashOutput(minted.color.0), address.custom_shielded_token_type(runtime::ledger::HashOutput(compact_rust_test_center_micro_dao_fixture::pure_circuits::dao_token_domain_separator()?.0)).0);
    let minted_coin =
        runtime::ledger::coin_info_from_compact(minted.nonce, minted.color, minted.value.value());
    let minted_output = Output::new(
        &mut rng,
        &minted_coin,
        segment,
        &keys.coin_public_key(),
        Some(keys.enc_public_key()),
    )?;
    let minted_commitment = minted_output.coin_com;
    let offer: Offer<ProofPreimage, DefaultDB> =
        Offer::new(inputs, vec![output, minted_output], transients.clone()).unwrap();
    let (_, indices) = ledger.zswap.try_apply(&offer, None)?;
    let pot_index = *indices.get(&final_commitment).unwrap();
    let mint_index = *indices.get(&minted_commitment).unwrap();
    assert_eq!(
        [pot_index.min(mint_index), pot_index.max(mint_index)],
        [2, 3]
    );
    if occupied {
        assert_eq!(indices.get(&transients[0].coin_com), Some(&4));
    }
    let mut options = OfferBindingOptions::default()
        .with_output_allocation(PersistentOutputAllocation::CanonicalOfferIndices)
        .with_offer_placement(placement)
        .with_wallet_funding(funding);
    if occupied {
        options = options.with_transient_coins(
            ContractTransientCoins::from_transients_for_placement(transients, placement)
                .map_err(|e| format!("transient selection {e:?}"))?,
        );
    }
    println!("buy_in occupied={occupied}: canonical offer prepared");
    let bound = OfferBackedObservedState::with_options(observe(), &ledger, offer.clone(), options)?;
    let native = c::buy_in(
        bound.observed().circuit_context(()),
        compact_coin(w),
        BoundedUint::new(2)?,
    )?;
    let call = c::recorded::Contract.buy_in_call(
        bound.observed(),
        (),
        compact_coin(w),
        BoundedUint::new(2)?,
    )?;
    let recorded = call.recorded();
    assert_eq!(
        recorded.execution.context.query.state,
        native.context.query.state
    );
    assert_eq!(
        recorded.execution.context.query.effects,
        native.context.query.effects
    );
    assert_eq!(
        recorded.execution.context.circuit_zswap(),
        native.context.circuit_zswap()
    );
    assert_eq!(
        recorded.execution.private_transcript_outputs,
        native.private_transcript_outputs
    );
    assert_eq!(recorded.execution.gas_cost, native.gas_cost);
    assert_eq!(
        recorded.public.verify_ops().len(),
        if occupied { 81 } else { 54 }
    );
    assert_eq!(
        recorded.execution.private_transcript_outputs.len(),
        if occupied { 6 } else { 3 }
    );
    let replay = recorded.public.initial().query(
        recorded.public.verify_ops(),
        None,
        &recorded.execution.context.cost_model,
    )?;
    assert_eq!(replay.context.state, native.context.query.state);
    assert_eq!(replay.context.effects, native.context.query.effects);
    println!("buy_in occupied={occupied}: recording and replay passed");
    let verifier: VerifierKey = tagged_deserialize(&mut BufReader::new(File::open(
        root.join("keys/buy_in.verifier"),
    )?))?;
    println!("buy_in occupied={occupied}: verifier loaded");
    let prepared = bound.prepare(call, verifier.clone(), Fr::from(0))?;
    assert!(prepared.prototype().guaranteed_public_transcript.is_some());
    assert!(prepared.prototype().fallible_public_transcript.is_none());
    super::kernel_shielded_effects::prove_and_verify_call(
        root,
        name,
        prepared.prototype(),
        &verifier,
    )?;
    let tx = prepared.into_transaction(
        &mut rng,
        "local-test",
        Timestamp::from_secs(fees.time.to_secs() + 3600),
    );
    let Transaction::Standard(structure) = &tx else {
        panic!("standard")
    };
    assert!(structure.fallible_coins.is_empty());
    assert_eq!(**structure.guaranteed_coins.as_ref().unwrap(), offer);
    println!("buy_in occupied={occupied}: original call proof verified");
    let resolver = super::qualified_coin_funding::fee_resolver(root, name)?;
    let params = MidnightDataProvider::new(FetchMode::OnDemand, OutputMode::Log, vec![])?;
    let provider = LocalProvingProvider {
        rng: StdRng::seed_from_u64(0x0215_5052 + u64::from(occupied)),
        resolver: &resolver,
        params: &params,
    };
    let proven = futures_executor::block_on(
        tx.prove(provider, &INITIAL_PARAMETERS.cost_model.runtime_cost_model),
    )?;
    let sealed = proven.seal(StdRng::seed_from_u64(0x0215_5345 + u64::from(occupied)));
    let sealed = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(fees.balance_tx(rng, sealed, &resolver))?;
    tagged_serialize(
        &sealed,
        &mut File::create(root.join(format!("buy-in-{occupied}-sealed.bin")))?,
    )?;
    let verified = sealed.well_formed(&ledger, WellFormedStrictness::default(), fees.time)?;
    let context = TransactionContext {
        ref_state: ledger.clone(),
        block_context: BlockContext {
            tblock: fees.time,
            last_block_time: fees.time,
            ..BlockContext::default()
        },
        whitelist: None,
    };
    let (updated, result) = ledger.apply(&verified, &context);
    assert!(
        matches!(result, TransactionResult::Success(_)),
        "{result:?}"
    );
    assert_eq!(
        updated.contract.get(&address).unwrap().data,
        native.context.query.state
    );
    assert_eq!(updated.zswap.first_free, if occupied { 5 } else { 4 });
    assert_eq!(
        updated.zswap.coin_coms.index(pot_index).unwrap().0,
        final_commitment.0
    );
    assert_eq!(
        updated
            .zswap
            .coin_coms
            .index(pot_index)
            .unwrap()
            .1
            .as_ref()
            .map(|owner| **owner),
        Some(address)
    );
    assert_eq!(
        updated.zswap.coin_coms.index(mint_index).unwrap().0,
        minted_commitment.0
    );
    assert!(
        updated
            .zswap
            .coin_coms
            .index(mint_index)
            .unwrap()
            .1
            .is_none()
    );
    let after_wallet = wallet.apply(&keys, &offer);
    assert!(
        after_wallet
            .coins
            .iter()
            .any(|(_, coin)| *coin == minted_coin.qualify(mint_index))
    );
    for nullifier in nullifiers {
        assert!(updated.zswap.nullifiers.contains_key(&nullifier));
    }
    assert!(matches!(
        updated.zswap.try_apply(&offer, None),
        Err(midnight_zswap::error::TransactionInvalid::NullifierAlreadyPresent(_))
    ));
    let final_state = runtime::context::CircuitContext::from_contract_state(
        (),
        address,
        updated.contract.get(&address).unwrap(),
    );
    let pot = slots::pot.read(final_state)?.result;
    assert_eq!(pot.mt_index.value(), u128::from(pot_index));
    assert_eq!(pot.value.value(), if occupied { 23 } else { 6 });
    println!(
        "original buy_in occupied={occupied}: default-strict proof/application, actual wallet funding, canonical qualified pot, minted voting token and replay refusal passed"
    );
    Ok(())
}
