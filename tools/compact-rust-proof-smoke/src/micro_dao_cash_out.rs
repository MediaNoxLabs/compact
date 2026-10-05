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
//! Original cash_out from seeded final DAO/coin state, not a proved voting lifecycle.
use super::micro_dao_cash_out_support as support;
use super::*;
use compact_rust_test_center_micro_dao_fixture::{
    ledger_contract as c, ledger_slots as slots, types,
};
use midnight_compact_runtime as runtime;
use midnight_zswap::{Input, Offer, Output};
use runtime::transaction::{
    OfferBackedObservedState, OfferBindingOptions, OfferPlacement, PersistentOutputAllocation,
    RecordedCall,
};

pub(super) fn run(root: &Path) -> Result<(), Box<dyn Error>> {
    const NAME: &str = "cash_out";
    let mut rng = StdRng::seed_from_u64(0x0212_0000);
    let seeded = support::seeded(&support::Settings::default())?;
    let mut pot = slots::pot.inspect(seeded.query.state.get_ref())?;
    pot.mt_index = BoundedUint::new(1)?;
    let seeded = slots::pot.write(seeded, pot.clone())?.context;
    let deploy = make_deploy(root, NAME, seeded.query.state.get_ref().clone(), &mut rng)?;
    let address = deploy.address();
    let prior = deploy.initial_state.clone();
    let mut fees = super::qualified_coin_funding::fee_funded_state(&mut rng)?;
    fees.ledger.contract = fees.ledger.contract.insert(address, prior.clone());
    let coin = runtime::ledger::coin_info_from_compact(pot.nonce, pot.color, pot.value.value());
    let padding = runtime::ledger::coin_info_from_compact(
        FixedBytes::new([88; 32]),
        FixedBytes::new([42; 32]),
        17,
    );
    // Sequential offline genesis outputs fix a nonzero historical pot index.
    // These seeds do not prove the application's prior deposit/vote lifecycle.
    for (expected, coin) in [(0, padding), (1, coin)] {
        let genesis: Offer<ProofPreimage, DefaultDB> = Offer::new(
            vec![],
            vec![Output::new_contract_owned(&mut rng, &coin, None, address)?],
            vec![],
        )
        .unwrap();
        let (next, indices) = fees.ledger.zswap.try_apply(&genesis, None)?;
        assert_eq!(
            indices
                .get(&coin.commitment(&runtime::ledger::CoinRecipient::Contract(address)))
                .copied(),
            Some(expected)
        );
        fees.ledger.zswap =
            midnight_storage_core::arena::Sp::new(next.post_block_update(fees.time));
    }
    let ledger = fees.ledger.clone();
    let start = ledger.zswap.first_free;
    assert_eq!(start, 2);
    let key = runtime::ledger::CoinPublicKey(runtime::ledger::HashOutput([7; 32]));
    let provisional = c::cash_out(
        runtime::context::CircuitContext::from_contract_state(
            support::Private { calls: 0 },
            address,
            &prior,
        )
        .with_coin_public_key(key),
    )?;
    let sent = provisional.context.circuit_zswap().outputs()[0].coin;
    let input = Input::new_contract_owned(
        &mut rng,
        &coin.qualify(1),
        Some(1),
        address,
        &ledger.zswap.coin_coms,
    )?;
    let spent = input.nullifier;
    let untouched = Input::new_contract_owned(
        &mut rng,
        &padding.qualify(0),
        Some(1),
        address,
        &ledger.zswap.coin_coms,
    )?
    .nullifier;
    let output = Output::new(&mut rng, &sent, Some(1), &key, None)?;
    let output_commitment = output.coin_com;
    let offer: Offer<ProofPreimage, DefaultDB> =
        Offer::new(vec![input], vec![output], vec![]).unwrap();
    let (_, indices) = ledger.zswap.try_apply(&offer, None)?;
    let bound = OfferBackedObservedState::with_options(
        ObservedContractState::new(
            address,
            prior.clone(),
            Observation {
                transaction_hash: [0; 32],
                block_hash: [0; 32],
                block_height: 0,
            },
        )
        .with_coin_public_key(key),
        &ledger,
        offer.clone(),
        OfferBindingOptions::default()
            .with_output_allocation(PersistentOutputAllocation::CanonicalOfferIndices)
            .with_offer_placement(OfferPlacement::Fallible(
                std::num::NonZeroU16::new(1).unwrap(),
            )),
    )?;
    let native = c::cash_out(
        bound
            .observed()
            .circuit_context(support::Private { calls: 0 }),
    )?;
    let recorded = c::recorded::cash_out(
        bound
            .observed()
            .circuit_context(support::Private { calls: 0 }),
    )?;
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
    assert_eq!(recorded.execution.result, native.result);
    assert_eq!(recorded.execution.private_transcript_outputs.len(), 3);
    assert_eq!(
        recorded.execution.context.query.call_context.com_indices,
        indices
    );
    assert_eq!(
        recorded.execution.context.circuit_zswap().next_index(),
        start + 1
    );
    assert_eq!(
        recorded.execution.context.circuit_zswap().outputs()[0].provisional_index,
        start
    );
    let returned = &recorded.execution.result;
    assert_eq!(returned.value.value(), 99);
    assert_eq!(returned.nonce.0, sent.nonce.0.0);
    assert_eq!(returned.color.0, coin.type_.0.0);
    let verifier: VerifierKey = tagged_deserialize(&mut BufReader::new(File::open(
        root.join("keys/cash_out.verifier"),
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
    let Transaction::Standard(structure) = &tx else {
        panic!("standard transaction");
    };
    assert!(structure.guaranteed_coins.is_none());
    assert_eq!(*structure.fallible_coins.get(&1).unwrap(), offer);
    let resolver = super::qualified_coin_funding::fee_resolver(root, NAME)?;
    let params = MidnightDataProvider::new(FetchMode::OnDemand, OutputMode::Log, vec![])?;
    let provider = LocalProvingProvider {
        rng: StdRng::seed_from_u64(0x0212_5052),
        resolver: &resolver,
        params: &params,
    };
    let proven = futures_executor::block_on(
        tx.prove(provider, &INITIAL_PARAMETERS.cost_model.runtime_cost_model),
    )?;
    let sealed = proven.seal(StdRng::seed_from_u64(0x0212_5345));
    let sealed = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(fees.balance_tx(rng, sealed, &resolver))?;
    tagged_serialize(
        &sealed,
        &mut File::create(root.join("cash-out-sealed.bin"))?,
    )?;
    let Transaction::Standard(structure) = &sealed else {
        panic!("standard transaction");
    };
    assert!(structure.guaranteed_coins.is_none());
    assert_eq!(structure.fallible_coins.iter().count(), 1);
    assert!(structure.fallible_coins.contains_key(&1));
    assert_eq!(structure.intents.iter().count(), 2);
    let dust = structure
        .intents
        .iter()
        .find(|row| *row.0 != 1)
        .expect("separate Dust intent");
    assert!(dust.1.actions.is_empty());
    assert!(dust.1.dust_actions.is_some());
    assert!(dust.1.guaranteed_unshielded_offer.is_none());
    assert!(dust.1.fallible_unshielded_offer.is_none());
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
    // Prove from round7, apply against roundMAX. The pinned VM reaches the late
    // increment and reports ArithmeticOverflow; earlier fallible resets roll back.
    let mut changed = ledger.clone();
    let ctx = runtime::context::CircuitContext::from_contract_state((), address, &prior);
    let changed_data = runtime::ledger::write_cell(&ctx.query, 6, u64::MAX, None, &ctx.cost_model)?
        .context
        .state;
    let mut changed_contract = prior.clone();
    changed_contract.data = changed_data.clone();
    changed.contract = changed.contract.insert(address, changed_contract);
    let (failed_after, failure) = changed.apply(&verified, &context);
    println!("cash_out changed-round fallible result: {failure:?}");
    assert!(
        matches!(failure,TransactionResult::PartialSuccess(ref phases,_) if phases.get(&0).is_some_and(Result::is_ok) && matches!(phases.get(&1), Some(Err(midnight_ledger::error::TransactionInvalid::Transcript(
            midnight_onchain_runtime::error::TranscriptRejected::Execution(
                midnight_onchain_runtime::vm_error::OnchainProgramError::ArithmeticOverflow
            )
        ))))),
        "{failure:?}"
    );
    assert_eq!(failed_after.zswap, changed.zswap);
    assert_eq!(
        failed_after.contract.get(&address).unwrap().data,
        changed_data
    );
    assert_ne!(failed_after.dust, changed.dust);
    assert_ne!(failed_after.replay_protection, changed.replay_protection);
    let (updated, outcome) = ledger.apply(&verified, &context);
    assert!(
        matches!(outcome, TransactionResult::Success(_)),
        "{outcome:?}"
    );
    assert_eq!(
        updated.contract.get(&address).unwrap().data,
        native.context.query.state
    );
    assert_eq!(updated.zswap.first_free, start + 1);
    assert_eq!(
        updated
            .zswap
            .coin_coms
            .index(start)
            .map(|(hash, owner)| (hash, owner.as_ref().map(|owner| **owner))),
        Some((output_commitment.0, None))
    );
    assert!(updated.zswap.nullifiers.contains_key(&spent));
    assert!(!updated.zswap.nullifiers.contains_key(&untouched));
    assert!(
        matches!(updated.zswap.try_apply(&offer,None),Err(midnight_zswap::error::TransactionInvalid::NullifierAlreadyPresent(found)) if found==spent)
    );
    let state = updated.contract.get(&address).unwrap().data.get_ref();
    assert_eq!(slots::state.inspect(state)?, types::LedgerState::setup);
    assert_eq!(slots::round.inspect(state)?, 8);
    assert_eq!(slots::yes.inspect(state)?, 0);
    assert_eq!(slots::no.inspect(state)?, 0);
    assert_eq!(slots::topic.inspect(state)?, types::Maybe::default());
    assert_eq!(
        slots::beneficiary.inspect(state)?,
        types::MaybeCompact1::default()
    );
    assert_eq!(
        slots::pot.inspect(state)?,
        types::QualifiedShieldedCoinInfo::default()
    );
    assert!(!slots::pot_has_coin.inspect(state)?);
    assert_eq!(
        slots::committed_votes.inspect(state)?.first_free()?.value(),
        0
    );
    assert!(slots::committed_participants.inspect(state)?.is_empty());
    assert!(slots::revealed_participants.inspect(state)?.is_empty());
    assert_eq!(
        slots::organizer.inspect(state)?,
        slots::organizer.inspect(prior.data.get_ref())?
    );
    assert_eq!(
        slots::costs.inspect(state)?,
        slots::costs.inspect(prior.data.get_ref())?
    );
    println!(
        "original cash_out: nonzero historical pot99 at index1 to user output index2, default-strict proof/apply, complete reset and saved sent result, exact nullifier replay refusal; seeded prior DAO and separate Dust"
    );
    Ok(())
}
