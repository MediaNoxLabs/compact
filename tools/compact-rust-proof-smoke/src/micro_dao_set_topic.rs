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
//! Original set_topic: guaranteed empty pot and whole-fallible funded merge.
//! Prior contract/coin state is explicitly seeded offline, not a proved DAO lifecycle.
use super::micro_dao_advance_support as support;
use super::*;
use compact_rust_test_center_micro_dao_fixture::{
    ledger_contract as c, ledger_slots as slots, types,
};
use midnight_compact_runtime as runtime;
use midnight_ledger::error::MalformedTransaction;
use midnight_zswap::{Input, Offer, Output, Transient};
use runtime::transaction::{
    ContractTransientCoins, OfferBackedObservedState, OfferBindingOptions, OfferPlacement,
    PersistentOutputAllocation, WalletFundingInputs,
};
use std::num::NonZeroU16;

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
    let name = "set_topic";
    let mut rng = StdRng::seed_from_u64(0x0211_0000 + u64::from(occupied));
    let h = runtime::ledger::coin_info_from_compact(
        FixedBytes::new([1; 32]),
        FixedBytes::new([0; 32]),
        17,
    );
    let w = runtime::ledger::coin_info_from_compact(
        FixedBytes::new([3; 32]),
        FixedBytes::new([0; 32]),
        10,
    );
    let initial = c::initial_state(
        ConstructorContext::new(support::Private { calls: 0 }),
        FixedBytes::new([4; 32]),
        types::Costs {
            seed_dust: BoundedUint::new(10)?,
            buy_in_dust: BoundedUint::new(3)?,
        },
    )?;
    let mut prior = initial.into_circuit_context(runtime::ledger::ContractAddress::default());
    if occupied {
        prior = slots::pot
            .write(prior, compact_qualified(h.qualify(0)))?
            .context;
        prior = slots::pot_has_coin.write(prior, true)?.context;
    }
    let deploy = make_deploy(root, name, prior.query.state.get_ref().clone(), &mut rng)?;
    let address = deploy.address();
    let mut fees = super::qualified_coin_funding::fee_funded_state(&mut rng)?;
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(fees.give_fee_token(&mut rng, 10));
    fees.ledger.contract = fees
        .ledger
        .contract
        .insert(address, deploy.initial_state.clone());
    let keys = midnight_zswap::keys::SecretKeys::from_rng_seed(&mut rng);
    // Offline prior-state fixture. Sequential seeds fix history at index0 and
    // wallet at index1 without a deployment-address/normalized-order cycle.
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
    let (state, _) = fees.ledger.zswap.try_apply(&seed_h, None)?;
    let (state, _) = state.try_apply(&seed_w, None)?;
    fees.ledger.zswap = midnight_storage_core::arena::Sp::new(state.post_block_update(fees.time));
    let ledger = fees.ledger.clone();
    assert_eq!(ledger.zswap.first_free, 2);
    let wallet = midnight_zswap::local::State::<DefaultDB>::new()
        .apply(&keys, &seed_h)
        .apply(&keys, &seed_w);
    assert_eq!(wallet.merkle_tree.root(), ledger.zswap.coin_coms.root());
    let segment = occupied.then_some(1);
    let placement = if occupied {
        OfferPlacement::Fallible(NonZeroU16::new(1).unwrap())
    } else {
        OfferPlacement::Guaranteed
    };
    let (_, wallet_input) = wallet.spend(&mut rng, &keys, &w.qualify(1), segment)?;
    assert_eq!(
        wallet_input.proof.public_transcript_outputs,
        vec![Fr::from(1), Fr::from(u64::from(occupied))]
    );
    let mut nullifiers = vec![wallet_input.nullifier];
    let funding = WalletFundingInputs::from_inputs(vec![wallet_input.clone()])
        .map_err(|e| format!("wallet selection {e:?}"))?;
    let mut inputs = vec![wallet_input];
    let mut transients = vec![];
    if occupied {
        let input = Input::new_contract_owned(
            &mut rng,
            &h.qualify(0),
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
    let beneficiary = types::ZswapCoinPublicKey {
        bytes: FixedBytes::new([5; 32]),
    };
    // Offline derivation uses a provisional context, not a locked observation.
    let mut provisional = runtime::context::CircuitContext::from_contract_state(
        support::Private { calls: 0 },
        address,
        &deploy.initial_state,
    );
    provisional.set_zswap_output_start(2)?;
    let provisional = c::set_topic(
        provisional,
        &support::Witness::new(support::Mode::Normal),
        "Proposal 🗳️".into(),
        beneficiary.clone(),
        compact_coin(w),
    )?;
    let pot = slots::pot.read(provisional.context)?.result;
    assert_eq!(pot.value.value(), if occupied { 27 } else { 10 });
    assert_eq!(pot.mt_index.value(), if occupied { 3 } else { 2 });
    let final_coin =
        runtime::ledger::coin_info_from_compact(pot.nonce, pot.color, pot.value.value());
    let output = Output::new_contract_owned(&mut rng, &final_coin, segment, address)?;
    let final_commitment = output.coin_com;
    let offer: Offer<ProofPreimage, DefaultDB> =
        Offer::new(inputs, vec![output], transients.clone()).unwrap();
    let (_, indices) = ledger.zswap.try_apply(&offer, None)?;
    assert_eq!(indices.get(&final_commitment), Some(&2));
    if occupied {
        assert_eq!(indices.get(&transients[0].coin_com), Some(&3));
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
    let bound = OfferBackedObservedState::with_options(observe(), &ledger, offer.clone(), options)?;
    let native = c::set_topic(
        bound
            .observed()
            .circuit_context(support::Private { calls: 0 }),
        &support::Witness::new(support::Mode::Normal),
        "Proposal 🗳️".into(),
        beneficiary.clone(),
        compact_coin(w),
    )?;
    let generated = c::Contract::from(support::Witness::new(support::Mode::Normal));
    let call = generated.recording().set_topic_call(
        bound.observed(),
        support::Private { calls: 0 },
        "Proposal 🗳️".into(),
        beneficiary,
        compact_coin(w),
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
        recorded.execution.context.private_state,
        native.context.private_state
    );
    assert_eq!(
        recorded.execution.private_transcript_outputs,
        native.private_transcript_outputs
    );
    assert_eq!(recorded.execution.gas_cost, native.gas_cost);
    assert_eq!(
        recorded.public.verify_ops().len(),
        if occupied { 71 } else { 44 }
    );
    assert_eq!(
        recorded.execution.private_transcript_outputs.len(),
        if occupied { 5 } else { 2 }
    );
    let replay = recorded.public.initial().query(
        recorded.public.verify_ops(),
        None,
        &recorded.execution.context.cost_model,
    )?;
    assert_eq!(replay.context.state, native.context.query.state);
    assert_eq!(replay.context.effects, native.context.query.effects);
    let verifier: VerifierKey = tagged_deserialize(&mut BufReader::new(File::open(
        root.join("keys/set_topic.verifier"),
    )?))?;
    let prepared = bound.prepare(call, verifier.clone(), Fr::from(0))?;
    assert_eq!(
        prepared.prototype().guaranteed_public_transcript.is_some(),
        !occupied
    );
    assert_eq!(
        prepared.prototype().fallible_public_transcript.is_some(),
        occupied
    );
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
    if occupied {
        assert!(structure.guaranteed_coins.is_none());
        assert_eq!(structure.fallible_coins.iter().count(), 1);
        assert_eq!(*structure.fallible_coins.get(&1).unwrap(), offer);
    } else {
        assert!(structure.fallible_coins.is_empty());
        assert_eq!(**structure.guaranteed_coins.as_ref().unwrap(), offer);
    }
    let resolver = super::qualified_coin_funding::fee_resolver(root, name)?;
    let params = MidnightDataProvider::new(FetchMode::OnDemand, OutputMode::Log, vec![])?;
    let provider = LocalProvingProvider {
        rng: StdRng::seed_from_u64(0x0211_5052 + u64::from(occupied)),
        resolver: &resolver,
        params: &params,
    };
    let proven = futures_executor::block_on(
        tx.prove(provider, &INITIAL_PARAMETERS.cost_model.runtime_cost_model),
    )?;
    let sealed = proven.seal(StdRng::seed_from_u64(0x0211_5345 + u64::from(occupied)));
    let sealed = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(fees.balance_tx(rng, sealed, &resolver))?;
    tagged_serialize(
        &sealed,
        &mut File::create(root.join(format!("set-topic-{occupied}-sealed.bin")))?,
    )?;
    let Transaction::Standard(structure) = &sealed else {
        panic!("standard")
    };
    if occupied {
        assert!(structure.guaranteed_coins.is_none());
        assert_eq!(structure.fallible_coins.iter().count(), 1);
        for row in structure.intents.iter() {
            if *row.0 != 1 {
                assert!(row.1.actions.is_empty());
                assert!(row.1.dust_actions.is_some());
                assert!(row.1.guaranteed_unshielded_offer.is_none());
                assert!(row.1.fallible_unshielded_offer.is_none());
            }
        }
        let mut misplaced = structure.clone();
        misplaced.guaranteed_coins = Some(midnight_storage_core::arena::Sp::new(
            (*misplaced.fallible_coins.get(&1).unwrap()).clone(),
        ));
        misplaced.fallible_coins = HashMap::new();
        let error = Transaction::Standard(misplaced)
            .well_formed(&ledger, WellFormedStrictness::default(), fees.time)
            .err()
            .ok_or("wrong placement accepted")?;
        assert!(
            matches!(
                error,
                MalformedTransaction::Zswap(midnight_zswap::error::MalformedOffer::InvalidProof(_))
            ),
            "{error:?}"
        );
    }
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
    if occupied {
        let mut changed = ledger.clone();
        let changed_data = slots::state
            .write(
                runtime::context::CircuitContext::from_contract_state(
                    (),
                    address,
                    &deploy.initial_state,
                ),
                types::LedgerState::commit,
            )?
            .context
            .query
            .state;
        let mut changed_contract = deploy.initial_state.clone();
        changed_contract.data = changed_data.clone();
        changed.contract = changed.contract.insert(address, changed_contract);
        let (after, result) = changed.apply(&verified, &context);
        assert!(
            matches!(result, TransactionResult::PartialSuccess(ref phases, _)
                if phases.get(&0).is_some_and(Result::is_ok)
                    && matches!(phases.get(&1), Some(Err(
                        midnight_ledger::error::TransactionInvalid::Transcript(
                            midnight_onchain_runtime::error::TranscriptRejected::Execution(
                                midnight_onchain_runtime::vm_error::OnchainProgramError::ReadMismatch { .. }
                            )
                        )
                    )))
            ),
            "{result:?}"
        );
        assert_eq!(after.zswap, changed.zswap);
        assert_eq!(after.contract.get(&address).unwrap().data, changed_data);
        assert_ne!(after.dust, changed.dust);
        assert_ne!(after.replay_protection, changed.replay_protection);
    }
    let (updated, result) = ledger.apply(&verified, &context);
    assert!(
        matches!(result, TransactionResult::Success(_)),
        "{result:?}"
    );
    assert_eq!(
        updated.contract.get(&address).unwrap().data,
        native.context.query.state
    );
    assert_eq!(updated.zswap.first_free, if occupied { 4 } else { 3 });
    assert_eq!(
        updated.zswap.coin_coms.index(2).unwrap().0,
        final_commitment.0
    );
    assert_eq!(
        updated
            .zswap
            .coin_coms
            .index(2)
            .unwrap()
            .1
            .as_ref()
            .map(|owner| **owner),
        Some(address)
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
    assert_eq!(pot.mt_index.value(), 2);
    assert_eq!(pot.value.value(), if occupied { 27 } else { 10 });
    println!(
        "original set_topic occupied={occupied}: default-strict proof/application, actual wallet funding, canonical qualified pot and replay refusal passed"
    );
    Ok(())
}
