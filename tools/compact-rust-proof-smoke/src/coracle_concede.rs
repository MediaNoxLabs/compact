// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Original concede from explicitly seeded prior game and funded coin state.
use super::coracle_guess_support as support;
use super::*;
use compact_rust_test_center_coracle_fixture::{
    ledger_contract as c, ledger_slots as slots, types,
};
use midnight_compact_runtime as runtime;
use midnight_zswap::{Input, Offer, Output};
use runtime::transaction::OfferBackedObservedState;

pub(super) fn run(root: &Path) -> Result<(), Box<dyn Error>> {
    for red in [true, false] {
        run_color(root, red)?;
    }
    Ok(())
}

fn run_color(root: &Path, red: bool) -> Result<(), Box<dyn Error>> {
    const NAME: &str = "concede";
    let mut rng = StdRng::seed_from_u64(0x0209_0000 + u64::from(red));
    let settings = support::Settings {
        red,
        dead: true,
        board: 1,
        ..Default::default()
    };
    let seeded = support::seeded(&settings)?;
    let deploy = make_deploy(root, NAME, seeded.query.state.get_ref().clone(), &mut rng)?;
    let address = deploy.address();
    let mut fees = super::qualified_coin_funding::fee_funded_state(&mut rng)?;
    let coins = [(3, 42), (4, 17), (5, 19)].map(|(nonce, value)| {
        runtime::ledger::coin_info_from_compact(
            FixedBytes::new([nonce; 32]),
            FixedBytes::new([2; 32]),
            value,
        )
    });
    let genesis = coins
        .iter()
        .map(|coin| Output::new_contract_owned(&mut rng, coin, None, address))
        .collect::<Result<Vec<_>, _>>()?;
    let genesis: Offer<ProofPreimage, DefaultDB> =
        Offer::new(vec![], genesis, vec![]).ok_or("empty genesis")?;
    let (zswap, indices) = fees.ledger.zswap.try_apply(&genesis, None)?;
    fees.ledger.zswap = midnight_storage_core::arena::Sp::new(zswap.post_block_update(fees.time));
    let qualified = coins.map(|coin| {
        coin.qualify(
            *indices
                .get(&coin.commitment(&runtime::ledger::CoinRecipient::Contract(address)))
                .unwrap(),
        )
    });
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
        let compact = types::QualifiedShieldedCoinInfo {
            nonce: FixedBytes::new(coin.nonce.0.0),
            color: FixedBytes::new(coin.type_.0.0),
            value: BoundedUint::new(coin.value)?,
            mt_index: BoundedUint::new(coin.mt_index.into())?,
        };
        ctx = slot.write(ctx, compact)?.context;
    }
    let mut prior = deploy.initial_state.clone();
    prior.data = ctx.query.state;
    fees.ledger.contract = fees.ledger.contract.insert(address, prior.clone());
    let ledger = fees.ledger.clone();
    let start = ledger.zswap.first_free;
    let selected = if red { 1 } else { 2 };
    let untouched = if red { 2 } else { 1 };
    let key = runtime::ledger::CoinPublicKey(runtime::ledger::HashOutput([11; 32]));
    let input = Input::new_contract_owned(
        &mut rng,
        &qualified[selected],
        Some(1),
        address,
        &ledger.zswap.coin_coms,
    )?;
    let spent = input.nullifier;
    let unspent = Input::new_contract_owned(
        &mut rng,
        &qualified[untouched],
        Some(1),
        address,
        &ledger.zswap.coin_coms,
    )?
    .nullifier;
    let mut preview = runtime::context::CircuitContext::from_contract_state(
        support::Private::default(),
        address,
        &prior,
    )
    .with_coin_public_key(key);
    preview.set_zswap_output_start(start)?;
    let provisional = c::concede(preview, &support::Witness::new(settings.clone()))?;
    let output_coin = runtime::ledger::coin_info_from_compact(
        provisional.result.nonce,
        provisional.result.color,
        provisional.result.value.value(),
    );
    let output = Output::new(&mut rng, &output_coin, Some(1), &key, None)?;
    let commitment = output.coin_com;
    let offer: Offer<ProofPreimage, DefaultDB> =
        Offer::new(vec![input], vec![output], vec![]).ok_or("empty concede")?;
    let (_, indices) = ledger.zswap.try_apply(&offer, None)?;
    let allocated = *indices.get(&commitment).ok_or("missing output index")?;
    let observe = ObservedContractState::new(
        address,
        prior.clone(),
        Observation {
            transaction_hash: [0; 32],
            block_hash: [0; 32],
            block_height: 0,
        },
    )
    .with_coin_public_key(key);
    let bound = OfferBackedObservedState::new(observe, &ledger, offer.clone())?;
    let native = c::concede(
        bound
            .observed()
            .circuit_context(support::Private::default()),
        &support::Witness::new(settings.clone()),
    )?;
    assert_eq!(native.result, provisional.result);
    assert_eq!(native.gas_cost, provisional.gas_cost);
    assert_eq!(native.context.query.state, provisional.context.query.state);
    assert_eq!(
        native.context.query.effects,
        provisional.context.query.effects
    );
    assert_eq!(
        native.context.circuit_zswap().inputs(),
        provisional.context.circuit_zswap().inputs()
    );
    assert_eq!(
        native.context.circuit_zswap().outputs(),
        provisional.context.circuit_zswap().outputs()
    );
    let witness = support::Witness::new(settings.clone());
    let generated = c::Contract::from(witness);
    let call = generated
        .recording()
        .concede_call(bound.observed(), support::Private::default())?;
    let recorded = call.recorded();
    assert_eq!(recorded.execution.result, native.result);
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
    assert_eq!(recorded.execution.private_transcript_outputs.len(), 6);
    assert_eq!(recorded.public.verify_ops().len(), 42);
    let replay = recorded.public.initial().query(
        recorded.public.verify_ops(),
        None,
        &recorded.execution.context.cost_model,
    )?;
    assert_eq!(replay.context.state, native.context.query.state);
    assert_eq!(replay.context.effects, native.context.query.effects);
    let plan = recorded.execution.context.circuit_zswap();
    assert_eq!(plan.inputs().len(), 1);
    assert_eq!(plan.outputs().len(), 1);
    assert_eq!(plan.outputs()[0].provisional_index, allocated);
    assert_eq!(plan.next_index(), start + 1);
    assert_eq!(
        recorded.execution.context.query.call_context.com_indices,
        indices
    );
    assert_eq!(
        slots::state.inspect(native.context.query.state.get_ref())?,
        if red {
            types::State::blue_wins
        } else {
            types::State::red_wins
        }
    );
    for slot in [slots::pot, slots::red_deposit, slots::blue_deposit] {
        assert_eq!(
            slot.clone().inspect(native.context.query.state.get_ref())?,
            slot.inspect(prior.data.get_ref())?
        );
    }
    let verifier: VerifierKey = tagged_deserialize(&mut BufReader::new(File::open(
        root.join("keys/concede.verifier"),
    )?))?;
    let prepared = bound.prepare(call, verifier.clone(), Fr::from(0u64))?;
    assert!(prepared.prototype().guaranteed_public_transcript.is_some());
    assert!(prepared.prototype().fallible_public_transcript.is_none());
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
        rng: StdRng::seed_from_u64(0x0209_5052 + u64::from(red)),
        resolver: &resolver,
        params: &params,
    };
    let proven = futures_executor::block_on(
        tx.prove(provider, &INITIAL_PARAMETERS.cost_model.runtime_cost_model),
    )?;
    let sealed = proven.seal(StdRng::seed_from_u64(0x0209_5345 + u64::from(red)));
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
    assert_eq!(updated.zswap.first_free, start + 1);
    assert_eq!(
        updated.zswap.coin_coms.index(allocated).unwrap().0,
        commitment.0
    );
    assert!(
        updated
            .zswap
            .coin_coms
            .index(allocated)
            .unwrap()
            .1
            .is_none()
    );
    assert!(updated.zswap.nullifiers.contains_key(&spent));
    assert!(!updated.zswap.nullifiers.contains_key(&unspent));
    assert!(matches!(updated.zswap.try_apply(&offer, None),
        Err(midnight_zswap::error::TransactionInvalid::NullifierAlreadyPresent(found)) if found == spent));
    println!(
        "original Coracle.concede red={red}: ordered winner write then one full-value payout, guaranteed proof/apply, selected nullifier spent; offline seeded prior game and separate Dust"
    );
    Ok(())
}
