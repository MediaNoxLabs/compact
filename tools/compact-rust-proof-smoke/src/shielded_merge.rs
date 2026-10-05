// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Qualified merge and receive/immediate merge, with exact upstream input unions.
//! Prepared independently; register only after recorded APIs are delivered.
use super::*;
use compact_rust_shielded_merge_oracle_fixture::{ledger_contract as c, types};
use midnight_compact_runtime as runtime;
use midnight_zswap::{Input, Offer, Output, Transient};
use runtime::transaction::{
    ContractTransientCoins, OfferBackedObservedState, OfferBindingOptions,
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
    let value = compact_coin((&coin).into());
    types::QualifiedShieldedCoinInfo {
        nonce: value.nonce,
        color: value.color,
        value: value.value,
        mt_index: BoundedUint::new(coin.mt_index.into()).unwrap(),
    }
}
pub(super) fn run(root: &Path) -> Result<(), Box<dyn Error>> {
    for immediate in [false, true] {
        run_case(root, immediate)?;
    }
    Ok(())
}
fn run_case(root: &Path, immediate: bool) -> Result<(), Box<dyn Error>> {
    let name = if immediate {
        "receive_then_merge"
    } else {
        "merge_qualified"
    };
    let mut rng = StdRng::seed_from_u64(0x0207_0000 + u64::from(immediate));
    let initial = c::initial_state(ConstructorContext::new(()))?;
    let deploy = make_deploy(root, name, initial.ledger_state.get_ref().clone(), &mut rng)?;
    let address = deploy.address();
    let mut fees = super::qualified_coin_funding::fee_funded_state(&mut rng)?;
    // Three input-side proofs in the transient case use more fee capacity;
    // funding remains separate from the user's colored coin conservation.
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(fees.give_fee_token(&mut rng, 10));
    fees.ledger.contract = fees
        .ledger
        .contract
        .insert(address, deploy.initial_state.clone());
    let a = runtime::ledger::coin_info_from_compact(
        FixedBytes::new([1; 32]),
        FixedBytes::new([2; 32]),
        17,
    );
    let b = runtime::ledger::coin_info_from_compact(
        FixedBytes::new([3; 32]),
        FixedBytes::new([2; 32]),
        25,
    );
    let keys = midnight_zswap::keys::SecretKeys::from_rng_seed(&mut rng);
    let seed_a = Output::new_contract_owned(&mut rng, &a, None, address)?;
    let a_commitment = seed_a.coin_com;
    let seed_b = if immediate {
        Output::new(
            &mut rng,
            &b,
            None,
            &keys.coin_public_key(),
            Some(keys.enc_public_key()),
        )?
    } else {
        Output::new_contract_owned(&mut rng, &b, None, address)?
    };
    let b_commitment = seed_b.coin_com;
    // Explicit offline seeded-state prerequisite, not a proved deposit history.
    let seed: Offer<ProofPreimage, DefaultDB> =
        Offer::new(vec![], vec![seed_a, seed_b], vec![]).unwrap();
    let (seeded, seeded_indices) = fees.ledger.zswap.try_apply(&seed, None)?;
    fees.ledger.zswap = midnight_storage_core::arena::Sp::new(seeded.post_block_update(fees.time));
    let ledger = fees.ledger.clone();
    let start = ledger.zswap.first_free;
    assert_eq!(start, 2);
    let a_qualified = a.qualify(*seeded_indices.get(&a_commitment).unwrap());
    let b_qualified = b.qualify(*seeded_indices.get(&b_commitment).unwrap());
    let input_a = Input::new_contract_owned(
        &mut rng,
        &a_qualified,
        None,
        address,
        &ledger.zswap.coin_coms,
    )?;
    let mut expected_nullifiers = vec![input_a.nullifier];
    let mut inputs = vec![input_a];
    let mut transients = vec![];
    let mut options = OfferBindingOptions::default()
        .with_output_allocation(PersistentOutputAllocation::CanonicalOfferIndices);
    if immediate {
        let wallet = midnight_zswap::local::State::<DefaultDB>::new().apply(&keys, &seed);
        assert_eq!(wallet.merkle_tree.root(), ledger.zswap.coin_coms.root());
        let (_, wallet_input) = wallet.spend(&mut rng, &keys, &b_qualified, None)?;
        expected_nullifiers.push(wallet_input.nullifier);
        options = options.with_wallet_funding(
            WalletFundingInputs::from_inputs(vec![wallet_input.clone()])
                .map_err(|error| format!("wallet funding: {error:?}"))?,
        );
        inputs.push(wallet_input);
        let received = Output::new_contract_owned(&mut rng, &b, None, address)?;
        let transient =
            Transient::new_from_contract_owned_output(&mut rng, &b.qualify(0), None, received)?;
        expected_nullifiers.push(transient.nullifier);
        options = options.with_transient_coins(
            ContractTransientCoins::from_transients(vec![transient.clone()])
                .map_err(|error| format!("contract transient: {error:?}"))?,
        );
        transients.push(transient);
    } else {
        let input_b = Input::new_contract_owned(
            &mut rng,
            &b_qualified,
            None,
            address,
            &ledger.zswap.coin_coms,
        )?;
        expected_nullifiers.push(input_b.nullifier);
        inputs.push(input_b);
    }
    let a_compact = compact_qualified(a_qualified);
    let b_compact = compact_coin(b);
    let b_historical = compact_qualified(b_qualified);
    let mut context = c::initial_state(ConstructorContext::new(()))?.into_circuit_context(address);
    context.set_zswap_output_start(start)?;
    let provisional = if immediate {
        c::receive_then_merge(context, a_compact.clone(), b_compact.clone())?
    } else {
        c::merge_qualified(context, a_compact.clone(), b_historical.clone())?
    };
    assert_eq!(provisional.result.value.value(), 42);
    let merged = runtime::ledger::coin_info_from_compact(
        provisional.result.nonce,
        provisional.result.color,
        42,
    );
    let merged_output = Output::new_contract_owned(&mut rng, &merged, None, address)?;
    let merged_commitment = merged_output.coin_com;
    let offer: Offer<ProofPreimage, DefaultDB> =
        Offer::new(inputs, vec![merged_output], transients.clone()).unwrap();
    let (_, actual_indices) = ledger.zswap.try_apply(&offer, None)?;
    assert_eq!(actual_indices.get(&merged_commitment), Some(&start));
    if immediate {
        assert_eq!(
            actual_indices.get(&transients[0].coin_com),
            Some(&(start + 1))
        );
        assert_eq!(
            provisional.context.circuit_zswap().outputs()[0].provisional_index,
            start
        );
        assert_eq!(
            provisional.context.circuit_zswap().outputs()[1].provisional_index,
            start + 1
        );
    }
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
    };
    let bound = OfferBackedObservedState::with_options(observe(), &ledger, offer.clone(), options)?;
    let native = if immediate {
        c::receive_then_merge(
            bound.observed().circuit_context(()),
            a_compact.clone(),
            b_compact.clone(),
        )?
    } else {
        c::merge_qualified(
            bound.observed().circuit_context(()),
            a_compact.clone(),
            b_historical.clone(),
        )?
    };
    let call = if immediate {
        c::recorded::Contract.receive_then_merge_call(bound.observed(), (), a_compact, b_compact)?
    } else {
        c::recorded::Contract.merge_qualified_call(bound.observed(), (), a_compact, b_historical)?
    };
    let recorded = call.recorded();
    assert_eq!(recorded.execution.result, native.result);
    assert_eq!(native.result, provisional.result);
    assert_eq!(recorded.execution.gas_cost, native.gas_cost);
    assert_eq!(
        recorded.execution.private_transcript_outputs,
        native.private_transcript_outputs
    );
    assert_eq!(
        recorded.execution.context.circuit_zswap(),
        native.context.circuit_zswap()
    );
    assert_eq!(
        recorded.execution.context.query.state,
        native.context.query.state
    );
    assert_eq!(
        recorded.execution.context.query.effects,
        native.context.query.effects
    );
    assert_eq!(
        recorded.execution.private_transcript_outputs.len(),
        if immediate { 4 } else { 3 }
    );
    assert_eq!(
        recorded.public.verify_ops().len(),
        if immediate { 36 } else { 27 }
    );
    let replay = recorded.public.initial().query(
        recorded.public.verify_ops(),
        None,
        &recorded.execution.context.cost_model,
    )?;
    assert_eq!(replay.context.state, native.context.query.state);
    assert_eq!(replay.context.effects, native.context.query.effects);
    let verifier: VerifierKey = tagged_deserialize(&mut BufReader::new(File::open(
        root.join(format!("keys/{name}.verifier")),
    )?))?;
    let prepared = bound.prepare(call, verifier.clone(), Fr::from(0))?;
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
    let resolver = super::qualified_coin_funding::fee_resolver(root, name)?;
    let params = MidnightDataProvider::new(FetchMode::OnDemand, OutputMode::Log, vec![])?;
    let provider = LocalProvingProvider {
        rng: StdRng::seed_from_u64(0x0207_5052 + u64::from(immediate)),
        resolver: &resolver,
        params: &params,
    };
    let proven = futures_executor::block_on(
        tx.prove(provider, &INITIAL_PARAMETERS.cost_model.runtime_cost_model),
    )?;
    let sealed = proven.seal(StdRng::seed_from_u64(0x0207_5345 + u64::from(immediate)));
    let sealed = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(fees.balance_tx(rng, sealed, &resolver))?;
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
    assert_eq!(
        updated.zswap.first_free,
        start + if immediate { 2 } else { 1 }
    );
    assert_eq!(
        updated.zswap.coin_coms.index(start).unwrap().0,
        merged_commitment.0
    );
    assert_eq!(
        updated
            .zswap
            .coin_coms
            .index(start)
            .unwrap()
            .1
            .as_ref()
            .map(|owner| **owner),
        Some(address)
    );
    for nullifier in expected_nullifiers {
        assert!(updated.zswap.nullifiers.contains_key(&nullifier));
    }
    assert!(matches!(
        updated.zswap.try_apply(&offer, None),
        Err(midnight_zswap::error::TransactionInvalid::NullifierAlreadyPresent(_))
    ));
    println!(
        "{name}: seeded frontier2, genuine inputs, merged self42, separate Dust, default-strict proof/application and nullifier replay rejection passed"
    );
    Ok(())
}
