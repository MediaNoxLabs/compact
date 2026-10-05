// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//  http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Exact intent reconciliation and a contract-owned funded transfer.
use super::*;
use compact_rust_zswap_transfer_oracle_fixture::{ledger_contract as contract, types};
use midnight_compact_runtime as runtime;
use midnight_zswap::{Input, Offer, Output};
use runtime::transaction::{
    ObservedCallError, OfferBackedObservedState, PrepareCallError, RecordedCall, ZswapIntentError,
};

pub(super) fn run(root: &Path) -> Result<(), Box<dyn Error>> {
    const NAME: &str = "transfer";
    let mut rng = StdRng::seed_from_u64(0x0188_5452);
    let initial = contract::initial_state(ConstructorContext::new(()))?;
    let deploy = make_deploy(root, NAME, initial.ledger_state.get_ref().clone(), &mut rng)?;
    let address = deploy.address();
    let mut fee_state = super::qualified_coin_funding::fee_funded_state(&mut rng)?;
    fee_state.ledger.contract = fee_state
        .ledger
        .contract
        .insert(address, deploy.initial_state.clone());
    let time = fee_state.time;
    let seed_coin = runtime::ledger::coin_info_from_compact(
        FixedBytes::new([1; 32]),
        FixedBytes::new([2; 32]),
        42,
    );
    let seed_output = Output::new_contract_owned(&mut rng, &seed_coin, None, address)?;
    let seed_commitment = seed_output.coin_com;
    let seed_offer: Offer<ProofPreimage, DefaultDB> =
        Offer::new(vec![], vec![seed_output], vec![]).ok_or("empty seed offer")?;
    let (seeded, indices) = fee_state.ledger.zswap.try_apply(&seed_offer, None)?;
    let seed_index = *indices.get(&seed_commitment).ok_or("seed index missing")?;
    // Explicit offline genesis prerequisite; seal the root before contract spend.
    fee_state.ledger.zswap = midnight_storage_core::arena::Sp::new(seeded.post_block_update(time));
    let ledger = fee_state.ledger.clone();
    let input = Input::new_contract_owned(
        &mut rng,
        &seed_coin.qualify(seed_index),
        None,
        address,
        &ledger.zswap.coin_coms,
    )?;
    let nullifier = input.nullifier;
    let coin = runtime::ledger::coin_info_from_compact(
        FixedBytes::new([3; 32]),
        FixedBytes::new([2; 32]),
        42,
    );
    let recipient = runtime::ledger::CoinPublicKey(runtime::ledger::HashOutput([7; 32]));
    let output = Output::new(&mut rng, &coin, None, &recipient, None)?;
    let commitment = output.coin_com;
    let offer: Offer<ProofPreimage, DefaultDB> =
        Offer::new(vec![input], vec![output], vec![]).ok_or("empty transfer offer")?;
    let (_, indices) = ledger.zswap.try_apply(&offer, None)?;
    let index = *indices
        .get(&commitment)
        .ok_or("transfer allocation missing")?;
    let bound = OfferBackedObservedState::new(
        ObservedContractState::new(
            address,
            deploy.initial_state.clone(),
            Observation {
                transaction_hash: [0; 32],
                block_hash: [0; 32],
                block_height: 0,
            },
        ),
        &ledger,
        offer.clone(),
    )?;
    let qualified = types::QualifiedShieldedCoinInfo {
        nonce: FixedBytes::new([1; 32]),
        color: FixedBytes::new([2; 32]),
        value: BoundedUint::new(42)?,
        mt_index: BoundedUint::new(seed_index.into())?,
    };
    let compact_coin = types::ShieldedCoinInfo {
        nonce: FixedBytes::new([3; 32]),
        color: FixedBytes::new([2; 32]),
        value: BoundedUint::new(42)?,
    };
    let compact_recipient = types::Either {
        is_left: true,
        left: types::ZswapCoinPublicKey {
            bytes: FixedBytes::new([7; 32]),
        },
        right: types::ContractAddress {
            bytes: FixedBytes::new([0; 32]),
        },
    };
    let nul = FixedBytes::new(nullifier.0.0);
    let com = FixedBytes::new(commitment.0.0);
    let make_call = || {
        contract::recorded::Contract.transfer_call(
            bound.observed(),
            (),
            qualified.clone(),
            compact_coin.clone(),
            compact_recipient.clone(),
            nul,
            com,
        )
    };
    let verifier: VerifierKey = tagged_deserialize(&mut BufReader::new(File::open(
        root.join("keys/transfer.verifier"),
    )?))?;
    let unbound = make_call()?
        .prepare(verifier.clone(), Fr::from(0_u64))
        .err()
        .ok_or("unbound intents accepted")?;
    assert!(matches!(
        unbound,
        ObservedCallError::Prepare(PrepareCallError::UnboundZswapIntents)
    ));
    let native = contract::transfer(
        bound.observed().circuit_context(()),
        qualified.clone(),
        compact_coin.clone(),
        compact_recipient.clone(),
        nul,
        com,
    )?;
    let call = make_call()?;
    assert_eq!(
        call.recorded().execution.context.query.state.get_ref(),
        native.context.query.state.get_ref()
    );
    assert_eq!(
        call.recorded().execution.context.query.effects,
        native.context.query.effects
    );
    assert_eq!(
        call.recorded().execution.context.circuit_zswap(),
        native.context.circuit_zswap()
    );
    assert_eq!(
        call.recorded().execution.private_transcript_outputs,
        native.private_transcript_outputs
    );
    assert_eq!(call.recorded().execution.gas_cost, native.gas_cost);
    let expected_state = native.context.query.state.get_ref().clone();
    let prepared = bound.prepare(call, verifier.clone(), Fr::from(0_u64))?;
    super::kernel_shielded_effects::prove_and_verify_call(
        root,
        NAME,
        prepared.prototype(),
        &verifier,
    )?;
    // Altering publicly mutable context cannot erase the sealed intent snapshot.
    let mut changed = contract::recorded::transfer(
        bound.observed().circuit_context(()),
        qualified.clone(),
        compact_coin.clone(),
        compact_recipient.clone(),
        nul,
        com,
    )?;
    changed.execution.context = bound.observed().circuit_context(());
    let changed = RecordedCall::new(
        bound.observed(),
        changed,
        NAME,
        (
            qualified.clone(),
            compact_coin.clone(),
            compact_recipient.clone(),
            nul,
            com,
        ),
    );
    assert!(matches!(
        bound.prepare(changed, verifier.clone(), Fr::from(0_u64)),
        Err(ObservedCallError::ZswapIntent(
            ZswapIntentError::AllocationMismatch
        ))
    ));
    let mut wrong = qualified.clone();
    wrong.mt_index = BoundedUint::new(index.into())?;
    let wrong = contract::recorded::Contract.transfer_call(
        bound.observed(),
        (),
        wrong,
        compact_coin.clone(),
        compact_recipient.clone(),
        nul,
        com,
    )?;
    assert!(matches!(
        bound.prepare(wrong, verifier.clone(), Fr::from(0_u64)),
        Err(ObservedCallError::ZswapIntent(
            ZswapIntentError::InputIndexMismatch
        ))
    ));
    original_boundaries(
        &bound,
        &ledger,
        &offer,
        &qualified,
        &compact_coin,
        &compact_recipient,
        index,
        &verifier,
    )?;
    let wrong_claim = contract::recorded::Contract.transfer_call(
        bound.observed(),
        (),
        qualified.clone(),
        compact_coin.clone(),
        compact_recipient.clone(),
        FixedBytes::new([9; 32]),
        com,
    )?;
    let wrong_tx = bound
        .prepare(wrong_claim, verifier.clone(), Fr::from(0_u64))?
        .into_transaction(
            &mut rng,
            "local-test",
            Timestamp::from_secs(time.to_secs() + 3600),
        );
    let mut negative_strictness = WellFormedStrictness::default();
    negative_strictness.enforce_balancing = false;
    let error = wrong_tx
        .well_formed(&ledger, negative_strictness, time)
        .err()
        .ok_or("wrong Kernel nullifier claim accepted")?;
    assert!(matches!(
        error,
        midnight_ledger::error::MalformedTransaction::EffectsCheckFailure(
            midnight_ledger::error::EffectsCheckError::NullifiersNEClaimedNullifiers { .. }
        )
    ));
    println!(
        "wrong Kernel nullifier claim rejected by exact upstream effects check (balancing disabled only for isolated negative)"
    );
    let tx = prepared.into_transaction(
        &mut rng,
        "local-test",
        Timestamp::from_secs(time.to_secs() + 3600),
    );
    let resolver = super::qualified_coin_funding::fee_resolver(root, NAME)?;
    let params = MidnightDataProvider::new(FetchMode::OnDemand, OutputMode::Log, vec![])?;
    let provider = LocalProvingProvider {
        rng: StdRng::seed_from_u64(0x0188_5052),
        resolver: &resolver,
        params: &params,
    };
    let proven = futures_executor::block_on(
        tx.prove(provider, &INITIAL_PARAMETERS.cost_model.runtime_cost_model),
    )?;
    let sealed = proven.seal(StdRng::seed_from_u64(0x0188_5345));
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
        return Err(format!("funded transfer apply failed: {outcome:?}").into());
    }
    assert_eq!(
        updated.contract.get(&address).unwrap().data.get_ref(),
        &expected_state
    );
    assert_eq!(
        updated.zswap.coin_coms.index(index).map(|(hash, _)| hash),
        Some(commitment.0)
    );
    assert_eq!(updated.zswap.first_free, index + 1);
    let replay = updated
        .zswap
        .try_apply(&offer, None)
        .err()
        .ok_or("spent offer replay accepted")?;
    assert!(
        matches!(replay,midnight_zswap::error::TransactionInvalid::NullifierAlreadyPresent(found) if found==nullifier)
    );
    let mut read_context = bound.observed().circuit_context(());
    read_context.query.state = runtime::ledger::ChargedState::new(expected_state);
    let stored = contract::read_coin(read_context)?.result;
    assert_eq!(stored.mt_index.value(), u128::from(index));
    assert_eq!(stored.nonce, compact_coin.nonce);
    assert_eq!(stored.color, compact_coin.color);
    assert_eq!(stored.value, compact_coin.value);
    println!(
        "funded contract-owned input -> distinct output42 with exact intents, Kernel claims, Dust: default strictness + ledger apply passed at index {index}; nullifier replay rejected"
    );
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn original_boundaries(
    bound: &OfferBackedObservedState,
    ledger: &LedgerState<DefaultDB>,
    offer: &Offer<ProofPreimage, DefaultDB>,
    input: &types::QualifiedShieldedCoinInfo,
    coin: &types::ShieldedCoinInfo,
    recipient: &types::Either,
    index: u64,
    verifier: &VerifierKey,
) -> Result<(), Box<dyn Error>> {
    use compact_rust_native_zswap_intents_oracle_fixture::{
        ledger_contract as original, types as original_types,
    };
    let input = original_types::QualifiedShieldedCoinInfo {
        nonce: input.nonce,
        color: input.color,
        value: input.value,
        mt_index: input.mt_index,
    };
    let coin = original_types::ShieldedCoinInfo {
        nonce: coin.nonce,
        color: coin.color,
        value: coin.value,
    };
    let recipient = original_types::Either {
        is_left: recipient.is_left,
        left: original_types::ZswapCoinPublicKey {
            bytes: recipient.left.bytes,
        },
        right: original_types::ContractAddress {
            bytes: recipient.right.bytes,
        },
    };
    assert!(matches!(
        original::recorded::flow(
            bound.observed().circuit_context(()),
            input.clone(),
            coin.clone(),
            recipient.clone(),
            true
        ),
        Err(runtime::CompactError::ZswapOfferOutputMismatch)
    ));
    let raw = |selected| -> Result<_, runtime::CompactError> {
        let mut context = original::initial_state(ConstructorContext::new(()))?
            .into_circuit_context(bound.observed().address());
        context.set_zswap_output_start(index)?;
        original::recorded::flow(
            context,
            input.clone(),
            coin.clone(),
            recipient.clone(),
            selected,
        )
    };
    let duplicate = raw(true)?;
    assert_eq!(
        duplicate.execution.context.circuit_zswap().outputs().len(),
        2
    );
    assert_eq!(
        duplicate.execution.context.circuit_zswap().next_index(),
        index + 2
    );
    assert_eq!(
        original::read_coin(duplicate.execution.context)?
            .result
            .mt_index
            .value(),
        u128::from(index + 1)
    );
    let spec = |selected| {
        runtime::transaction::CallSpec::new(
            "flow",
            verifier.clone(),
            (input.clone(), coin.clone(), recipient.clone(), selected),
            Fr::from(0_u64),
        )
    };
    assert!(matches!(
        runtime::transaction::prepare_call(raw(true)?, spec(true)),
        Err(PrepareCallError::UnboundZswapIntents)
    ));
    assert!(matches!(
        runtime::transaction::prepare_call(raw(false)?, spec(false)),
        Err(PrepareCallError::EmptyTranscript)
    ));
    let duplicated = Offer::new(
        offer.inputs.iter_deref().cloned().collect(),
        vec![
            offer.outputs.get(0).unwrap().clone(),
            offer.outputs.get(0).unwrap().clone(),
        ],
        vec![],
    )
    .unwrap();
    let expected = offer.outputs.get(0).unwrap().coin_com;
    assert!(
        matches!(ledger.zswap.try_apply(&duplicated,None),Err(midnight_zswap::error::TransactionInvalid::CommitmentAlreadyPresent(found)) if found==expected)
    );
    println!(
        "original flow: two provisional outputs retained; bound duplicate rejected; raw unbound preparation refused; selected(false) exact EmptyTranscript; duplicate offer exact CommitmentAlreadyPresent; no proof claim for original flow"
    );
    Ok(())
}
