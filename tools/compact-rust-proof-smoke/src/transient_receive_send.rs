// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//   http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Genuine wallet funding → same-call receive/transient spend → full user output.
use super::*;
use compact_rust_transient_receive_send_oracle_fixture::{ledger_contract as c, types};
use midnight_compact_runtime as runtime;
use midnight_zswap::{Offer, Output, Transient};
use runtime::transaction::{
    ContractTransientCoins, OfferBackedObservedState, OfferBindingOptions,
    PersistentOutputAllocation,
};

pub(super) fn run(root: &Path) -> Result<(), Box<dyn Error>> {
    const NAME: &str = "receive_then_send";
    let mut rng = StdRng::seed_from_u64(0x0204_5052);
    let initial = c::initial_state(ConstructorContext::new(()))?;
    let deploy = make_deploy(root, NAME, initial.ledger_state.get_ref().clone(), &mut rng)?;
    let address = deploy.address();
    let mut fees = super::qualified_coin_funding::fee_funded_state(&mut rng)?;
    fees.ledger.contract = fees
        .ledger
        .contract
        .insert(address, deploy.initial_state.clone());
    let coin = types::ShieldedCoinInfo {
        nonce: FixedBytes::new([3; 32]),
        color: FixedBytes::new([2; 32]),
        value: BoundedUint::new(42)?,
    };
    let info = runtime::ledger::coin_info_from_compact(coin.nonce, coin.color, 42);
    let keys = midnight_zswap::keys::SecretKeys::from_rng_seed(&mut rng);
    let seed_coins = [
        runtime::ledger::coin_info_from_compact(FixedBytes::new([91; 32]), coin.color, 42),
        runtime::ledger::coin_info_from_compact(FixedBytes::new([92; 32]), coin.color, 1),
    ];
    let seed_outputs = seed_coins
        .iter()
        .map(|coin| {
            Output::new(
                &mut rng,
                coin,
                None,
                &keys.coin_public_key(),
                Some(keys.enc_public_key()),
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    let seed: Offer<ProofPreimage, DefaultDB> =
        Offer::new(vec![], seed_outputs, vec![]).ok_or("empty genesis seed")?;
    let (seeded, _) = fees.ledger.zswap.try_apply(&seed, None)?;
    fees.ledger.zswap = midnight_storage_core::arena::Sp::new(seeded.post_block_update(fees.time));
    let ledger = fees.ledger.clone();
    let start = ledger.zswap.first_free;
    assert!(start > 1);
    let wallet = midnight_zswap::local::State::<DefaultDB>::new().apply(&keys, &seed);
    assert_eq!(wallet.merkle_tree.root(), ledger.zswap.coin_coms.root());
    let seed_coin = *wallet
        .coins
        .iter()
        .find(|(_, coin)| coin.value == 42)
        .ok_or("wallet seed coin missing")?
        .1;
    let (_, wallet_input) = wallet.spend(&mut rng, &keys, &seed_coin, None)?;
    let wallet_nullifier = wallet_input.nullifier;
    let received = Output::new_contract_owned(&mut rng, &info, None, address)?;
    let transient = Transient::new_from_contract_owned_output(
        &mut rng,
        &info.qualify(0),
        None,
        received.clone(),
    )?;
    let recipient = types::Either {
        is_left: true,
        left: types::ZswapCoinPublicKey {
            bytes: FixedBytes::new([7; 32]),
        },
        right: types::ContractAddress::default(),
    };
    // Native provisional run derives the source's nonce chain. Its indices are
    // retained as a separate raw-mode result; no returned value is remapped.
    let mut native_context =
        c::initial_state(ConstructorContext::new(()))?.into_circuit_context(address);
    native_context.set_zswap_output_start(start)?;
    let provisional = c::receive_then_send(native_context, coin.clone(), recipient.clone())?;
    let sent = &provisional.result.sent;
    let sent_info =
        runtime::ledger::coin_info_from_compact(sent.nonce, sent.color, sent.value.value());
    let output = Output::new(
        &mut rng,
        &sent_info,
        None,
        &runtime::ledger::CoinPublicKey(runtime::ledger::HashOutput([7; 32])),
        None,
    )?;
    let sent_commitment = output.coin_com;
    let offer: Offer<ProofPreimage, DefaultDB> = Offer::new(
        vec![wallet_input.clone()],
        vec![output],
        vec![transient.clone()],
    )
    .ok_or("empty transient offer")?;
    let (_, indices) = ledger.zswap.try_apply(&offer, None)?;
    assert_eq!(indices.get(&sent_commitment), Some(&start));
    assert_eq!(indices.get(&transient.coin_com), Some(&(start + 1)));
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
    let options = |selected| -> Result<_, Box<dyn Error>> {
        Ok(OfferBindingOptions::default()
            .with_output_allocation(PersistentOutputAllocation::CanonicalOfferIndices)
            .with_wallet_funding(
                runtime::transaction::WalletFundingInputs::from_inputs(vec![wallet_input.clone()])
                    .map_err(|e| format!("funding {e:?}"))?,
            )
            .with_transient_coins(
                ContractTransientCoins::from_transients(vec![selected])
                    .map_err(|e| format!("transient {e:?}"))?,
            ))
    };
    let bound = OfferBackedObservedState::with_options(
        observe(),
        &ledger,
        offer.clone(),
        options(transient.clone())?,
    )?;
    let native = c::receive_then_send(
        bound.observed().circuit_context(()),
        coin.clone(),
        recipient.clone(),
    )?;
    let call = c::recorded::Contract.receive_then_send_call(
        bound.observed(),
        (),
        coin.clone(),
        recipient.clone(),
    )?;
    let recorded = call.recorded();
    assert_eq!(native.result, provisional.result);
    assert_eq!(native.result, recorded.execution.result);
    assert_eq!(native.gas_cost, recorded.execution.gas_cost);
    assert_eq!(
        native.private_transcript_outputs,
        recorded.execution.private_transcript_outputs
    );
    assert_eq!(
        native.context.circuit_zswap(),
        recorded.execution.context.circuit_zswap()
    );
    assert_eq!(
        native.context.query.state,
        recorded.execution.context.query.state
    );
    assert_eq!(
        native.context.query.effects,
        recorded.execution.context.query.effects
    );
    assert_eq!(recorded.execution.private_transcript_outputs.len(), 3);
    assert_eq!(recorded.public.verify_ops().len(), 24);
    let plan = native.context.circuit_zswap();
    assert_eq!(plan.inputs()[0], info.qualify(0));
    assert_eq!(
        plan.outputs()
            .iter()
            .map(|out| out.provisional_index)
            .collect::<Vec<_>>(),
        vec![start + 1, start]
    );
    assert_eq!(plan.next_index(), start + 2);
    assert_eq!(
        provisional
            .context
            .circuit_zswap()
            .outputs()
            .iter()
            .map(|out| out.provisional_index)
            .collect::<Vec<_>>(),
        vec![start, start + 1]
    );
    let replay = recorded.public.initial().query(
        recorded.public.verify_ops(),
        None,
        &native.context.cost_model,
    )?;
    assert_eq!(replay.context.state, native.context.query.state);
    assert_eq!(replay.context.effects, native.context.query.effects);
    // Same source VM, independently replayed with authoritative allocation.
    let raw = c::recorded::receive_then_send(
        {
            let mut context =
                c::initial_state(ConstructorContext::new(()))?.into_circuit_context(address);
            context.set_zswap_output_start(start)?;
            context
        },
        coin.clone(),
        recipient.clone(),
    )?;
    assert_eq!(raw.public.verify_ops(), recorded.public.verify_ops());
    let mut replay_context = raw.public.initial().clone();
    replay_context.call_context.com_indices = indices.clone();
    let independent =
        replay_context.query(raw.public.verify_ops(), None, &native.context.cost_model)?;
    assert_eq!(independent.context.state, native.context.query.state);
    assert_eq!(independent.context.effects, native.context.query.effects);
    let verifier: VerifierKey = tagged_deserialize(&mut BufReader::new(File::open(
        root.join("keys/receive_then_send.verifier"),
    )?))?;
    let prepared = bound.prepare(call, verifier.clone(), Fr::from(0))?;
    super::kernel_shielded_effects::prove_and_verify_call(
        root,
        NAME,
        prepared.prototype(),
        &verifier,
    )?;
    let resolver = super::qualified_coin_funding::fee_resolver(root, NAME)?;
    let params = MidnightDataProvider::new(FetchMode::OnDemand, OutputMode::Log, vec![])?;
    // Constructor success is not proof validity: wrong singleton index1
    // constructs an upstream preimage, but the actual transaction prover must
    // reject it despite unchanged commitment/nullifier/public singleton root.
    let wrong =
        Transient::new_from_contract_owned_output(&mut rng, &info.qualify(1), None, received)?;
    let mut wrong_offer = offer.clone();
    wrong_offer.transient = vec![wrong.clone()].into_iter().collect();
    let wrong_bound =
        OfferBackedObservedState::with_options(observe(), &ledger, wrong_offer, options(wrong)?)?;
    let wrong_call = c::recorded::Contract.receive_then_send_call(
        wrong_bound.observed(),
        (),
        coin,
        recipient,
    )?;
    let wrong_tx = wrong_bound
        .prepare(wrong_call, verifier, Fr::from(0))?
        .into_transaction(
            &mut rng,
            "local-test",
            Timestamp::from_secs(fees.time.to_secs() + 3600),
        );
    let wrong_provider = LocalProvingProvider {
        rng: StdRng::seed_from_u64(0x0204_4241),
        resolver: &resolver,
        params: &params,
    };
    let rejected = futures_executor::block_on(wrong_tx.prove(
        wrong_provider,
        &INITIAL_PARAMETERS.cost_model.runtime_cost_model,
    ));
    let failure = rejected
        .err()
        .ok_or("invalid transient singleton private path unexpectedly proved")?;
    let diagnostic = format!("{failure:?}");
    assert!(
        diagnostic.contains("Public transcript input mismatch for input 13;"),
        "unexpected failure instead of singleton Merkle-root constraint: {diagnostic}"
    );
    println!("malformed singleton-index1 proving rejection: {diagnostic}");
    let tx = prepared.into_transaction(
        &mut rng,
        "local-test",
        Timestamp::from_secs(fees.time.to_secs() + 3600),
    );
    let provider = LocalProvingProvider {
        rng: StdRng::seed_from_u64(0x0204_5052),
        resolver: &resolver,
        params: &params,
    };
    let proven = futures_executor::block_on(
        tx.prove(provider, &INITIAL_PARAMETERS.cost_model.runtime_cost_model),
    )?;
    let sealed = proven.seal(StdRng::seed_from_u64(0x0204_5345));
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
    assert_eq!(updated.zswap.first_free, start + 2);
    assert_eq!(
        updated.zswap.coin_coms.index(start).unwrap().0,
        sent_commitment.0
    );
    assert_eq!(
        updated.zswap.coin_coms.index(start + 1).unwrap().0,
        transient.coin_com.0
    );
    assert!(updated.zswap.nullifiers.contains_key(&wallet_nullifier));
    assert!(updated.zswap.nullifiers.contains_key(&transient.nullifier));
    assert!(
        matches!(updated.zswap.try_apply(&offer,None),Err(midnight_zswap::error::TransactionInvalid::NullifierAlreadyPresent(n)) if n==wallet_nullifier)
    );
    println!(
        "receive→full immediate send: genuine wallet42, full transient proof pair, final user42; actual indices[{},{start}], separate Dust, default strict proof+apply and replay rejection passed",
        start + 1
    );
    Ok(())
}
