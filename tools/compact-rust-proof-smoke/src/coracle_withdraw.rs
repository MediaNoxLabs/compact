// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Original withdraw from offline seeded game/coin state, not a funded game lifecycle.
use super::coracle_withdraw_support as support;
use super::*;
use compact_rust_test_center_coracle_fixture::{ledger_contract as c, ledger_slots as slots};
use midnight_compact_runtime as runtime;
use midnight_ledger::error::MalformedTransaction;
use midnight_zswap::{Input, Offer, Output};
use runtime::transaction::{
    OfferBackedObservedState, OfferBindingOptions, OfferPlacement, PersistentOutputAllocation,
    RecordedCall,
};

pub(super) fn run(root: &Path) -> Result<(), Box<dyn Error>> {
    for (red, reversed) in [(true, false), (false, true)] {
        run_order(root, red, reversed)?;
    }
    Ok(())
}
fn run_order(root: &Path, red: bool, reversed: bool) -> Result<(), Box<dyn Error>> {
    const NAME: &str = "withdraw";
    let mut rng = StdRng::seed_from_u64(0x0206_0000 + u64::from(red));
    let mut settings = support::Settings {
        red,
        phase: if red {
            compact_rust_test_center_coracle_fixture::types::State::red_wins
        } else {
            compact_rust_test_center_coracle_fixture::types::State::blue_wins
        },
        ..Default::default()
    };
    let seeded = support::seeded(&settings)?;
    let deploy = make_deploy(root, NAME, seeded.query.state.get_ref().clone(), &mut rng)?;
    let address = deploy.address();
    let mut fees = super::qualified_coin_funding::fee_funded_state(&mut rng)?;
    let coins = [(3, 42), (4, 17), (5, 19)].map(|(n, value)| {
        runtime::ledger::coin_info_from_compact(
            FixedBytes::new([n; 32]),
            FixedBytes::new([2; 32]),
            value,
        )
    });
    let genesis = coins
        .iter()
        .map(|coin| Output::new_contract_owned(&mut rng, coin, None, address))
        .collect::<Result<Vec<_>, _>>()?;
    let genesis: Offer<ProofPreimage, DefaultDB> = Offer::new(vec![], genesis, vec![]).unwrap();
    let (zswap, seed_indices) = fees.ledger.zswap.try_apply(&genesis, None)?;
    fees.ledger.zswap = midnight_storage_core::arena::Sp::new(zswap.post_block_update(fees.time));
    let qualified = coins.map(|coin| {
        coin.qualify(
            *seed_indices
                .get(&coin.commitment(&runtime::ledger::CoinRecipient::Contract(address)))
                .unwrap(),
        )
    });
    // The prior contract state is explicitly seeded with the authoritative genesis indices.
    let mut ctx = runtime::context::CircuitContext::from_contract_state(
        support::Private::default(),
        address,
        &deploy.initial_state,
    );
    for (slot, coin) in [
        (slots::pot, qualified[0]),
        (slots::red_deposit, qualified[1]),
        (slots::blue_deposit, qualified[2]),
    ] {
        let compact = support::coin(coin.nonce.0.0[0], coin.value, coin.mt_index);
        ctx = slot.write(ctx, compact)?.context;
    }
    let mut prior = deploy.initial_state.clone();
    prior.data = ctx.query.state;
    fees.ledger.contract = fees.ledger.contract.insert(address, prior.clone());
    let ledger = fees.ledger.clone();
    let start = ledger.zswap.first_free;
    let selected = if red { 1 } else { 2 };
    let untouched = if red { 2 } else { 1 };
    let input = |rng: &mut StdRng, i: usize| {
        Input::new_contract_owned(
            rng,
            &qualified[i],
            Some(1),
            address,
            &ledger.zswap.coin_coms,
        )
    };
    let input0 = input(&mut rng, 0)?;
    let input1 = input(&mut rng, selected)?;
    let unspent = input(&mut rng, untouched)?.nullifier;
    let spent = [input0.nullifier, input1.nullifier];
    let (key, sent) = (1..=255)
        .find_map(|byte| {
            let key = runtime::ledger::CoinPublicKey(runtime::ledger::HashOutput([byte; 32]));
            let mut ctx = runtime::context::CircuitContext::from_contract_state(
                support::Private::default(),
                address,
                &prior,
            )
            .with_coin_public_key(key);
            ctx.set_zswap_output_start(start).unwrap();
            let out = c::withdraw(ctx, &support::Witness::new(settings.clone())).unwrap();
            let output = out.context.circuit_zswap().outputs();
            let sent = [output[0].coin, output[1].coin];
            let recipient = runtime::ledger::CoinRecipient::User(key);
            ((sent[0].commitment(&recipient) > sent[1].commitment(&recipient)) == reversed)
                .then_some((key, sent))
        })
        .ok_or("no recipient for desired normalized order")?;
    settings.key = Some(key.0.0);
    let outputs = sent
        .iter()
        .map(|coin| Output::new(&mut rng, coin, Some(1), &key, None))
        .collect::<Result<Vec<_>, _>>()?;
    let offer: Offer<ProofPreimage, DefaultDB> =
        Offer::new(vec![input0, input1], outputs, vec![]).unwrap();
    let (_, indices) = ledger.zswap.try_apply(&offer, None)?;
    let observe = || {
        ObservedContractState::new(
            address,
            prior.clone(),
            Observation {
                transaction_hash: [0; 32],
                block_hash: [0; 32],
                block_height: 0,
            },
        )
        .with_coin_public_key(key)
    };
    let bound = OfferBackedObservedState::with_options(
        observe(),
        &ledger,
        offer.clone(),
        OfferBindingOptions::default()
            .with_output_allocation(PersistentOutputAllocation::CanonicalOfferIndices)
            .with_offer_placement(OfferPlacement::Fallible(
                std::num::NonZeroU16::new(1).unwrap(),
            )),
    )?;
    let native = c::withdraw(
        bound
            .observed()
            .circuit_context(support::Private::default()),
        &support::Witness::new(settings.clone()),
    )?;
    let recorded = c::recorded::withdraw(
        bound
            .observed()
            .circuit_context(support::Private::default()),
        &support::Witness::new(settings.clone()),
    )?;
    assert_eq!(
        native.context.query.state,
        recorded.execution.context.query.state
    );
    assert_eq!(
        native.context.query.effects,
        recorded.execution.context.query.effects
    );
    assert_eq!(
        native.context.circuit_zswap(),
        recorded.execution.context.circuit_zswap()
    );
    assert_eq!(native.gas_cost, recorded.execution.gas_cost);
    assert_eq!(
        native.private_transcript_outputs,
        recorded.execution.private_transcript_outputs
    );
    assert_eq!(native.result, recorded.execution.result);
    assert_eq!(recorded.execution.context.query.state, prior.data);
    assert_eq!(recorded.execution.private_transcript_outputs.len(), 8);
    assert_eq!(recorded.execution.context.private_state.calls, 2);
    let actual = recorded
        .execution
        .context
        .circuit_zswap()
        .outputs()
        .iter()
        .map(|o| o.provisional_index)
        .collect::<Vec<_>>();
    assert_eq!(
        actual,
        if reversed {
            vec![start + 1, start]
        } else {
            vec![start, start + 1]
        }
    );
    assert_eq!(
        recorded.execution.context.circuit_zswap().next_index(),
        start + 2
    );
    assert_eq!(
        recorded.execution.context.query.call_context.com_indices,
        indices
    );
    let verifier: VerifierKey = tagged_deserialize(&mut BufReader::new(File::open(
        root.join("keys/withdraw.verifier"),
    )?))?;
    let call = RecordedCall::new(bound.observed(), recorded, NAME, ());
    let prepared = bound.prepare(call, verifier.clone(), Fr::from(0u64))?;
    assert!(prepared.prototype().guaranteed_public_transcript.is_none());
    assert!(prepared.prototype().fallible_public_transcript.is_some());
    super::kernel_shielded_effects::prove_and_verify_call(
        root,
        NAME,
        prepared.prototype(),
        &verifier,
    )?;
    let tx = prepared.into_transaction(
        &mut rng,
        "local-test",
        Timestamp::from_secs(fees.time.to_secs() + 3600),
    );
    let resolver = super::qualified_coin_funding::fee_resolver(root, NAME)?;
    let params = MidnightDataProvider::new(FetchMode::OnDemand, OutputMode::Log, vec![])?;
    let provider = LocalProvingProvider {
        rng: StdRng::seed_from_u64(0x0206_5052 + u64::from(reversed)),
        resolver: &resolver,
        params: &params,
    };
    let proven = futures_executor::block_on(
        tx.prove(provider, &INITIAL_PARAMETERS.cost_model.runtime_cost_model),
    )?;
    let sealed = proven.seal(StdRng::seed_from_u64(0x0206_5345));
    let sealed = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(fees.balance_tx(rng, sealed, &resolver))?;
    tagged_serialize(
        &sealed,
        &mut File::create(root.join(format!("withdraw-{red}-sealed.bin")))?,
    )?;
    let Transaction::Standard(structure) = &sealed else {
        panic!("standard transaction");
    };
    assert!(structure.guaranteed_coins.is_none());
    assert_eq!(structure.fallible_coins.iter().count(), 1);
    assert!(structure.fallible_coins.contains_key(&1));
    assert_eq!(structure.intents.iter().count(), 2);
    let dust_row = structure
        .intents
        .iter()
        .find(|row| *row.0 != 1)
        .expect("separate Dust intent");
    let dust_intent = &dust_row.1;
    assert!(dust_intent.actions.is_empty());
    assert!(dust_intent.dust_actions.is_some());
    assert!(dust_intent.guaranteed_unshielded_offer.is_none());
    assert!(dust_intent.fallible_unshielded_offer.is_none());
    assert!(structure.intents.contains_key(&1));
    // Moving already-proven segment-1 coins to guaranteed placement violates
    // their proof binding. The separate original segment-0 offer failure
    // (NullifiersNEClaimedNullifiers) is retained in the ADR206 evidence.
    let mut misplaced = structure.clone();
    misplaced.guaranteed_coins = Some(midnight_storage_core::arena::Sp::new(
        (*misplaced.fallible_coins.get(&1).unwrap()).clone(),
    ));
    misplaced.fallible_coins = HashMap::new();
    let bad = Transaction::Standard(misplaced)
        .well_formed(&ledger, WellFormedStrictness::default(), fees.time)
        .err()
        .ok_or("misplaced withdrawal unexpectedly valid")?;
    assert!(
        matches!(
            bad,
            MalformedTransaction::Zswap(midnight_zswap::error::MalformedOffer::InvalidProof(_))
        ),
        "{bad:?}"
    );
    println!("wrong guaranteed placement rejected exactly: {bad:?}");
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
    // Simulate a concurrent public-state change before application. Guaranteed
    // Dust/replay processing may persist; the whole fallible offer must roll back.
    let mut failed_before = ledger.clone();
    let changed = slots::state
        .write(
            runtime::context::CircuitContext::from_contract_state((), address, &prior),
            compact_rust_test_center_coracle_fixture::types::State::no_game,
        )?
        .context
        .query
        .state;
    let mut changed_contract = prior.clone();
    changed_contract.data = changed.clone();
    failed_before.contract = failed_before.contract.insert(address, changed_contract);
    let (failed_after, failure) = failed_before.apply(&verified, &context);
    if let TransactionResult::PartialSuccess(phases, _) = &failure {
        println!("fallible withdrawal after changed public state: {phases:?}");
    }
    assert!(
        matches!(failure, TransactionResult::PartialSuccess(ref phases, _) if phases.get(&0).is_some_and(Result::is_ok) && matches!(phases.get(&1), Some(Err(midnight_ledger::error::TransactionInvalid::Transcript(
            midnight_onchain_runtime::error::TranscriptRejected::Execution(
                midnight_onchain_runtime::vm_error::OnchainProgramError::ReadMismatch { .. }
            )
        )))))
    );
    assert_eq!(failed_after.zswap, failed_before.zswap);
    assert_eq!(failed_after.contract.get(&address).unwrap().data, changed);
    assert_ne!(failed_after.dust, failed_before.dust);
    assert_ne!(
        failed_after.replay_protection,
        failed_before.replay_protection
    );
    let (updated, outcome) = ledger.apply(&verified, &context);
    assert!(
        matches!(outcome, TransactionResult::Success(_)),
        "{outcome:?}"
    );

    assert_eq!(updated.contract.get(&address).unwrap().data, prior.data);
    assert_eq!(updated.zswap.first_free, start + 2);
    for (i, coin) in sent.iter().enumerate() {
        let com = coin.commitment(&runtime::ledger::CoinRecipient::User(key));
        assert_eq!(
            updated
                .zswap
                .coin_coms
                .index(actual[i])
                .map(|(hash, owner)| (hash, owner.as_ref().map(|o| **o))),
            Some((com.0, None))
        );
    }
    for nullifier in spent {
        assert!(updated.zswap.nullifiers.contains_key(&nullifier));
    }
    assert!(!updated.zswap.nullifiers.contains_key(&unspent));
    assert!(
        matches!(updated.zswap.try_apply(&offer,None),Err(midnight_zswap::error::TransactionInvalid::NullifierAlreadyPresent(found)) if spent.contains(&found))
    );
    println!(
        "original Coracle.withdraw red={red}, reversed={reversed}: two full-value inputs/outputs, default strict proof/apply, unchanged Cells and untouched deposit, exact replay nullifier rejection; offline seeded prior game/coins with separate Dust"
    );
    Ok(())
}
