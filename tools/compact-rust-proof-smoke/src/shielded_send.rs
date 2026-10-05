// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Prove unchanged qualified sendShielded and apply an exact funded offer.
use super::*;
use compact_rust_shielded_send_oracle_fixture::{ledger_contract as contract, types};
use midnight_compact_runtime as runtime;
use midnight_zswap::{Input, Offer, Output};
use runtime::transaction::{ObservedCallError, OfferBackedObservedState, ZswapIntentError};

pub(super) fn run(root: &Path) -> Result<(), Box<dyn Error>> {
    const NAME: &str = "send_to_self";
    let mut rng = StdRng::seed_from_u64(0x0203_0001);
    let initial = contract::initial_state(ConstructorContext::new(()))?;
    let deploy = make_deploy(root, NAME, initial.ledger_state.get_ref().clone(), &mut rng)?;
    let address = deploy.address();
    let mut fee_state = super::qualified_coin_funding::fee_funded_state(&mut rng)?;
    fee_state.ledger.contract = fee_state
        .ledger
        .contract
        .insert(address, deploy.initial_state.clone());
    let time = fee_state.time;
    let seed_info = runtime::ledger::coin_info_from_compact(
        FixedBytes::new([1; 32]),
        FixedBytes::new([2; 32]),
        42,
    );
    let seed_output = Output::new_contract_owned(&mut rng, &seed_info, None, address)?;
    let seed_commitment = seed_output.coin_com;
    let seed_offer: Offer<ProofPreimage, DefaultDB> =
        Offer::new(vec![], vec![seed_output], vec![]).ok_or("empty seed")?;
    let (seeded, indices) = fee_state.ledger.zswap.try_apply(&seed_offer, None)?;
    let seed_index = *indices
        .get(&seed_commitment)
        .ok_or("missing seeded index")?;
    fee_state.ledger.zswap = midnight_storage_core::arena::Sp::new(seeded.post_block_update(time));
    let ledger = fee_state.ledger.clone();
    let input = Input::new_contract_owned(
        &mut rng,
        &seed_info.qualify(seed_index),
        None,
        address,
        &ledger.zswap.coin_coms,
    )?;
    let nullifier = input.nullifier;
    let compact_input = types::QualifiedShieldedCoinInfo {
        nonce: FixedBytes::new([1; 32]),
        color: FixedBytes::new([2; 32]),
        value: BoundedUint::new(42)?,
        mt_index: BoundedUint::new(seed_index.into())?,
    };
    let observed = super::shielded_receive::observed(address, deploy.initial_state.clone());
    let mut preview =
        contract::initial_state(ConstructorContext::new(()))?.into_circuit_context(address);
    preview.set_zswap_output_start(ledger.zswap.first_free)?;
    let native = contract::send_to_self(preview, compact_input.clone(), BoundedUint::new(42)?)?;
    assert!(!native.result.change.is_some);
    let sent = &native.result.sent;
    let sent_info =
        runtime::ledger::coin_info_from_compact(sent.nonce, sent.color, sent.value.value());
    let output = Output::new_contract_owned(&mut rng, &sent_info, None, address)?;
    let commitment = output.coin_com;
    let offer: Offer<ProofPreimage, DefaultDB> =
        Offer::new(vec![input], vec![output], vec![]).ok_or("empty send")?;
    let (_, indices) = ledger.zswap.try_apply(&offer, None)?;
    let index = *indices.get(&commitment).ok_or("missing sent index")?;
    let bound = OfferBackedObservedState::new(observed, &ledger, offer.clone())?;
    let bound_native = contract::send_to_self(
        bound.observed().circuit_context(()),
        compact_input.clone(),
        BoundedUint::new(42)?,
    )?;
    assert_eq!(bound_native.result, native.result);
    assert_eq!(
        bound_native.context.circuit_zswap().inputs(),
        native.context.circuit_zswap().inputs()
    );
    assert_eq!(
        bound_native.context.circuit_zswap().outputs(),
        native.context.circuit_zswap().outputs()
    );
    assert_eq!(
        bound_native.context.circuit_zswap().next_index(),
        native.context.circuit_zswap().next_index()
    );
    let verifier: VerifierKey = tagged_deserialize(&mut BufReader::new(File::open(
        root.join("keys/send_to_self.verifier"),
    )?))?;
    let call = contract::Contract::default()
        .recording()
        .send_to_self_call(
            bound.observed(),
            (),
            compact_input.clone(),
            BoundedUint::new(42)?,
        )?;
    assert_eq!(call.recorded().execution.result, bound_native.result);
    assert_eq!(
        call.recorded().execution.context.query.effects,
        bound_native.context.query.effects
    );
    assert_eq!(
        call.recorded().execution.context.circuit_zswap(),
        bound_native.context.circuit_zswap()
    );
    let prepared = bound.prepare(call, verifier.clone(), Fr::from(0_u64))?;
    super::kernel_shielded_effects::prove_and_verify_call(
        root,
        NAME,
        prepared.prototype(),
        &verifier,
    )?;
    let mut wrong = compact_input.clone();
    wrong.mt_index = BoundedUint::new(index.into())?;
    let wrong_call = contract::Contract::default()
        .recording()
        .send_to_self_call(bound.observed(), (), wrong, BoundedUint::new(42)?)?;
    assert!(matches!(
        bound.prepare(wrong_call, verifier.clone(), Fr::from(0_u64)),
        Err(ObservedCallError::ZswapIntent(
            ZswapIntentError::InputIndexMismatch
        ))
    ));
    let partial = contract::Contract::default().recording().send_to_self_call(
        bound.observed(),
        (),
        compact_input,
        BoundedUint::new(17)?,
    );
    assert!(matches!(
        partial,
        Err(runtime::CompactError::ZswapOfferOutputMismatch)
    ));
    let tx = prepared.into_transaction(
        &mut rng,
        "local-test",
        Timestamp::from_secs(time.to_secs() + 3600),
    );
    let resolver = super::qualified_coin_funding::fee_resolver(root, NAME)?;
    let params = MidnightDataProvider::new(FetchMode::OnDemand, OutputMode::Log, vec![])?;
    let provider = LocalProvingProvider {
        rng: StdRng::seed_from_u64(0x0203_0002),
        resolver: &resolver,
        params: &params,
    };
    let proven = futures_executor::block_on(
        tx.prove(provider, &INITIAL_PARAMETERS.cost_model.runtime_cost_model),
    )?;
    let sealed = proven.seal(StdRng::seed_from_u64(0x0203_0003));
    let sealed = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(fee_state.balance_tx(rng.clone(), sealed, &resolver))?;
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
        return Err(format!("shielded send apply failed: {outcome:?}").into());
    }
    assert_eq!(
        updated.zswap.coin_coms.index(index).map(|(hash, _)| hash),
        Some(commitment.0)
    );
    assert_eq!(updated.zswap.first_free, index + 1);
    assert!(matches!(updated.zswap.try_apply(&offer, None),
        Err(midnight_zswap::error::TransactionInvalid::NullifierAlreadyPresent(found)) if found==nullifier));
    println!(
        "qualified full self send: exact offer, call proof, default strict Dust-funded ledger apply and nullifier replay rejection passed"
    );
    Ok(())
}
