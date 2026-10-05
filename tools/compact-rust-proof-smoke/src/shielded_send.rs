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
    run_case(root, false)
}
pub(super) fn run_observational(root: &Path) -> Result<(), Box<dyn Error>> {
    run_case(root, true)
}
fn run_case(root: &Path, observation_policy: bool) -> Result<(), Box<dyn Error>> {
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
        compact_input.clone(),
        BoundedUint::new(17)?,
    );
    assert!(matches!(
        partial,
        Err(runtime::CompactError::ZswapOfferOutputMismatch)
    ));
    let tx = if observation_policy {
        use runtime::transaction::{ObservationalOfferBackedState, OfferAdmission};
        let observed = super::shielded_receive::observed(address, deploy.initial_state.clone());
        let checkpoint = super::observational_binding::checkpoint(&ledger, &observed)?;
        let binding =
            ObservationalOfferBackedState::bind(observed, checkpoint, offer.clone(), None)?;
        let call = contract::Contract::default()
            .recording()
            .send_to_self_call(binding.observed(), (), compact_input, BoundedUint::new(42)?)?;
        let observed_prepared = binding.prepare(call, verifier.clone(), Fr::from(0))?;
        assert!(matches!(
            observed_prepared.admission(),
            OfferAdmission::TrustedObservation(_)
        ));
        super::observational_binding::same_preimage(
            prepared.prototype(),
            observed_prepared.prototype(),
        )?;
        observed_prepared.into_transaction(&mut rng, Timestamp::from_secs(time.to_secs() + 3600))
    } else {
        prepared.into_transaction(
            &mut rng,
            "local-test",
            Timestamp::from_secs(time.to_secs() + 3600),
        )
    };
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
    if !observation_policy {
        partial_order_cases(
            root,
            &mut fee_state,
            address,
            deploy.initial_state.clone(),
            &verifier,
            &mut rng,
        )?;
    }
    println!(
        "qualified full self send: exact offer, call proof, default strict Dust-funded ledger apply and nullifier replay rejection passed"
    );
    Ok(())
}

fn partial_order_cases(
    root: &Path,
    fee_state: &mut midnight_ledger::test_utilities::TestState<DefaultDB>,
    address: runtime::ledger::ContractAddress,
    contract_state: ContractState<DefaultDB>,
    verifier: &VerifierKey,
    rng: &mut StdRng,
) -> Result<(), Box<dyn Error>> {
    use runtime::ledger::CoinRecipient;
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(fee_state.give_fee_token(rng, 10));
    let info = |byte| {
        runtime::ledger::coin_info_from_compact(
            FixedBytes::new([byte; 32]),
            FixedBytes::new([2; 32]),
            42,
        )
    };
    let mut same = None;
    let mut reversed = None;
    for byte in 3_u8..=96 {
        let qualified = types::QualifiedShieldedCoinInfo {
            nonce: FixedBytes::new([byte; 32]),
            color: FixedBytes::new([2; 32]),
            value: BoundedUint::new(42)?,
            mt_index: BoundedUint::new(0)?,
        };
        let preview =
            contract::initial_state(ConstructorContext::new(()))?.into_circuit_context(address);
        let result = contract::send_to_self(preview, qualified, BoundedUint::new(17)?)?.result;
        let sent = runtime::ledger::coin_info_from_compact(
            result.sent.nonce,
            result.sent.color,
            result.sent.value.value(),
        );
        let change = runtime::ledger::coin_info_from_compact(
            result.change.value.nonce,
            result.change.value.color,
            result.change.value.value.value(),
        );
        let sent_com = sent.commitment(&CoinRecipient::Contract(address));
        let change_com = change.commitment(&CoinRecipient::Contract(address));
        if sent_com < change_com && same.is_none() {
            same = Some(byte);
        }
        if sent_com > change_com && reversed.is_none() {
            reversed = Some(byte);
        }
        if same.is_some() && reversed.is_some() {
            break;
        }
    }
    let same = same.ok_or("no same-order partial nonce")?;
    let reversed = reversed.ok_or("no reverse-order partial nonce")?;
    let time = fee_state.time;
    let same_seed = Output::new_contract_owned(rng, &info(same), None, address)?;
    let same_com = same_seed.coin_com;
    let reverse_seed = Output::new_contract_owned(rng, &info(reversed), None, address)?;
    let reverse_com = reverse_seed.coin_com;
    let seed_offer: Offer<ProofPreimage, DefaultDB> =
        Offer::new(vec![], vec![same_seed, reverse_seed], vec![]).ok_or("empty partial seed")?;
    let (seeded, seed_indices) = fee_state.ledger.zswap.try_apply(&seed_offer, None)?;
    fee_state.ledger.zswap = midnight_storage_core::arena::Sp::new(seeded.post_block_update(time));
    let ledger = fee_state.ledger.clone();
    for (byte, seed_com, same_order) in [(same, same_com, true), (reversed, reverse_com, false)] {
        let seed_index = *seed_indices
            .get(&seed_com)
            .ok_or("partial seed index missing")?;
        let input = Input::new_contract_owned(
            rng,
            &info(byte).qualify(seed_index),
            None,
            address,
            &ledger.zswap.coin_coms,
        )?;
        assert_eq!(input.contract_address.as_deref(), Some(&address));
        let nullifier = input.nullifier;
        let qualified = types::QualifiedShieldedCoinInfo {
            nonce: FixedBytes::new([byte; 32]),
            color: FixedBytes::new([2; 32]),
            value: BoundedUint::new(42)?,
            mt_index: BoundedUint::new(seed_index.into())?,
        };
        let mut preview =
            contract::initial_state(ConstructorContext::new(()))?.into_circuit_context(address);
        preview.set_zswap_output_start(ledger.zswap.first_free)?;
        let native = contract::send_to_self(preview, qualified.clone(), BoundedUint::new(17)?)?;
        assert!(native.result.change.is_some);
        assert_eq!(native.result.change.value.value.value(), 25);
        let sent = runtime::ledger::coin_info_from_compact(
            native.result.sent.nonce,
            native.result.sent.color,
            native.result.sent.value.value(),
        );
        let change = runtime::ledger::coin_info_from_compact(
            native.result.change.value.nonce,
            native.result.change.value.color,
            native.result.change.value.value.value(),
        );
        let sent_output = Output::new_contract_owned(rng, &sent, None, address)?;
        let sent_com = sent_output.coin_com;
        let change_output = Output::new_contract_owned(rng, &change, None, address)?;
        let change_com = change_output.coin_com;
        assert_eq!(sent_output.contract_address.as_deref(), Some(&address));
        assert_eq!(change_output.contract_address.as_deref(), Some(&address));
        let offer: Offer<ProofPreimage, DefaultDB> =
            Offer::new(vec![input], vec![sent_output, change_output], vec![])
                .ok_or("empty partial offer")?;
        assert_eq!(
            offer
                .outputs
                .get(0)
                .ok_or("missing normalized output")?
                .coin_com
                == sent_com,
            same_order
        );
        let (_, indices) = ledger.zswap.try_apply(&offer, None)?;
        let sent_index = *indices.get(&sent_com).ok_or("missing sent index")?;
        let change_index = *indices.get(&change_com).ok_or("missing change index")?;
        let bound = OfferBackedObservedState::new(
            super::shielded_receive::observed(address, contract_state.clone()),
            &ledger,
            offer.clone(),
        )?;
        let call = contract::Contract::default().recording().send_to_self_call(
            bound.observed(),
            (),
            qualified,
            BoundedUint::new(17)?,
        );
        if !same_order {
            assert!(matches!(
                call,
                Err(runtime::CompactError::ZswapOfferOutputMismatch)
            ));
            println!("partial reverse-order offer rejected by exact output binding");
            continue;
        }
        let call = call?;
        assert_eq!(call.recorded().execution.result, native.result);
        assert_eq!(
            call.recorded().execution.context.circuit_zswap().outputs(),
            native.context.circuit_zswap().outputs()
        );
        let prepared = bound.prepare(call, verifier.clone(), Fr::from(0_u64))?;
        super::kernel_shielded_effects::prove_and_verify_call(
            root,
            "send_to_self",
            prepared.prototype(),
            verifier,
        )?;
        let tx = prepared.into_transaction(
            rng,
            "local-test",
            Timestamp::from_secs(time.to_secs() + 3600),
        );
        let resolver = super::qualified_coin_funding::fee_resolver(root, "send_to_self")?;
        let params = MidnightDataProvider::new(FetchMode::OnDemand, OutputMode::Log, vec![])?;
        let provider = LocalProvingProvider {
            rng: StdRng::seed_from_u64(0x0203_0017),
            resolver: &resolver,
            params: &params,
        };
        let proven = futures_executor::block_on(
            tx.prove(provider, &INITIAL_PARAMETERS.cost_model.runtime_cost_model),
        )?;
        let sealed = proven.seal(StdRng::seed_from_u64(0x0203_0018));
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
            return Err(format!("partial send apply failed: {outcome:?}").into());
        }
        assert_eq!(
            updated.contract.get(&address).unwrap().data.get_ref(),
            contract_state.data.get_ref()
        );
        assert_eq!(
            updated
                .zswap
                .coin_coms
                .index(sent_index)
                .map(|(hash, _)| hash),
            Some(sent_com.0)
        );
        assert_eq!(
            updated
                .zswap
                .coin_coms
                .index(change_index)
                .map(|(hash, _)| hash),
            Some(change_com.0)
        );
        assert_eq!(updated.zswap.first_free, ledger.zswap.first_free + 2);
        assert!(matches!(updated.zswap.try_apply(&offer, None),
            Err(midnight_zswap::error::TransactionInvalid::NullifierAlreadyPresent(found)) if found==nullifier));
        println!(
            "partial same-order send: two-output proof/default-strict apply/replay passed at {sent_index},{change_index}"
        );
    }
    Ok(())
}
