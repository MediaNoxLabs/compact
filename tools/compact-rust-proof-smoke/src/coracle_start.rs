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
//! Original start, funded by two real wallet inputs. The blue fixture seeds a
//! prior red pot at its authoritative ledger index; it is not a proved game chain.
use super::*;
use compact_rust_test_center_coracle_fixture::{
    ledger_contract as c, ledger_slots as slots, types,
};
use midnight_compact_runtime as runtime;
use midnight_zswap::keys::SecretKeys;
use midnight_zswap::local::State as WalletState;
use midnight_zswap::{Input, Offer, Output, Transient};
use runtime::transaction::{
    ContractTransientCoins, OfferBackedObservedState, OfferBindingOptions, OfferPlacement,
    PersistentOutputAllocation, WalletFundingInputs,
};
use std::num::NonZeroU16;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct Private {
    calls: u64,
    board: Option<types::Committable>,
}

struct Witness;
impl c::TryWitnesses<Private> for Witness {
    fn local_secret_key(
        &self,
        ctx: WitnessContext<'_, Private, c::LedgerView<'_>>,
    ) -> Result<(Private, FixedBytes<32>), runtime::CompactError> {
        Ok((
            Private {
                calls: ctx.private_state.calls + 1,
                board: ctx.private_state.board.clone(),
            },
            FixedBytes::new([4; 32]),
        ))
    }
    fn fresh_nonce(
        &self,
        ctx: WitnessContext<'_, Private, c::LedgerView<'_>>,
    ) -> Result<(Private, Field), runtime::CompactError> {
        Ok((
            Private {
                calls: ctx.private_state.calls + 1,
                board: ctx.private_state.board.clone(),
            },
            Field::from(9u64),
        ))
    }
    fn local_set_board(
        &self,
        ctx: WitnessContext<'_, Private, c::LedgerView<'_>>,
        board: types::Committable,
    ) -> Result<(Private, ()), runtime::CompactError> {
        assert_eq!(board.nonce, Field::from(9u64));
        assert_eq!(board.contents.position, Field::from(4u64));
        Ok((
            Private {
                calls: ctx.private_state.calls + 1,
                board: Some(board),
            },
            (),
        ))
    }
    fn local_board(
        &self,
        _: WitnessContext<'_, Private, c::LedgerView<'_>>,
    ) -> Result<(Private, types::Committable), runtime::CompactError> {
        panic!("start must not read a prior private board")
    }
}

fn coin(nonce: u8, color: u8, value: u128) -> runtime::ledger::CoinInfo {
    runtime::ledger::coin_info_from_compact(
        FixedBytes::new([nonce; 32]),
        FixedBytes::new([color; 32]),
        value,
    )
}
fn compact_coin(value: runtime::ledger::CoinInfo) -> types::ShieldedCoinInfo {
    types::ShieldedCoinInfo {
        nonce: FixedBytes::new(value.nonce.0.0),
        color: FixedBytes::new(value.type_.0.0),
        value: BoundedUint::new(value.value).unwrap(),
    }
}
fn compact_qualified(
    value: runtime::ledger::QualifiedCoinInfo,
) -> types::QualifiedShieldedCoinInfo {
    types::QualifiedShieldedCoinInfo {
        nonce: FixedBytes::new(value.nonce.0.0),
        color: FixedBytes::new(value.type_.0.0),
        value: BoundedUint::new(value.value).unwrap(),
        mt_index: BoundedUint::new(value.mt_index.into()).unwrap(),
    }
}

pub(super) fn run(root: &Path) -> Result<(), Box<dyn Error>> {
    for blue in [false, true] {
        run_case(root, blue)?;
    }
    Ok(())
}

fn run_case(root: &Path, blue: bool) -> Result<(), Box<dyn Error>> {
    const NAME: &str = "start";
    let mut rng = StdRng::seed_from_u64(0x0213_0000 + u64::from(blue));
    let initial = c::initial_state(ConstructorContext::new(Private::default()))?;
    let deploy = make_deploy(root, NAME, initial.ledger_state.get_ref().clone(), &mut rng)?;
    let address = deploy.address();
    let mut fees = super::qualified_coin_funding::fee_funded_state(&mut rng)?;
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(fees.give_fee_token(&mut rng, 10));
    let wager = coin(3, 5, 17);
    let deposit = coin(2, 0, 100000);
    let historic = coin(1, 5, 17);
    let keys = SecretKeys::from_rng_seed(&mut rng);
    let placement = if blue {
        OfferPlacement::Fallible(NonZeroU16::new(1).unwrap())
    } else {
        OfferPlacement::Guaranteed
    };
    let segment = blue.then_some(1);
    let mut seeded_offers = vec![];
    if blue {
        seeded_offers.push(
            Offer::new(
                vec![],
                vec![Output::new_contract_owned(
                    &mut rng, &historic, None, address,
                )?],
                vec![],
            )
            .unwrap(),
        );
    }
    for value in [wager, deposit] {
        seeded_offers.push(
            Offer::new(
                vec![],
                vec![Output::new(
                    &mut rng,
                    &value,
                    None,
                    &keys.coin_public_key(),
                    Some(keys.enc_public_key()),
                )?],
                vec![],
            )
            .unwrap(),
        );
    }
    let mut wallet = WalletState::<DefaultDB>::new();
    let mut zswap = (*fees.ledger.zswap).clone();
    let mut historic_index = None;
    let mut wallet_indices = vec![];
    for (i, offer) in seeded_offers.iter().enumerate() {
        let (next, allocated) = zswap.try_apply(offer, None)?;
        let output = offer.outputs.iter().next().ok_or("seed output absent")?;
        let index = *allocated.get(&output.coin_com).ok_or("seed index absent")?;
        if blue && i == 0 {
            historic_index = Some(index);
        } else {
            wallet_indices.push(index);
        }
        zswap = next;
        wallet = wallet.apply(&keys, offer);
    }
    fees.ledger.zswap = midnight_storage_core::arena::Sp::new(zswap.post_block_update(fees.time));
    let mut prior = deploy.initial_state.clone();
    if blue {
        let historic = historic.qualify(historic_index.ok_or("historic index absent")?);
        let mut ctx = runtime::context::CircuitContext::from_contract_state(
            Private::default(),
            address,
            &prior,
        );
        ctx = slots::pot.write(ctx, compact_qualified(historic))?.context;
        ctx = slots::state.write(ctx, types::State::red_started)?.context;
        prior.data = ctx.query.state;
    }
    fees.ledger.contract = fees.ledger.contract.insert(address, prior.clone());
    let ledger = fees.ledger.clone();
    assert_eq!(wallet.merkle_tree.root(), ledger.zswap.coin_coms.root());
    let first_free = ledger.zswap.first_free;
    let mut wallet_inputs = vec![];
    for (value, index) in [(wager, wallet_indices[0]), (deposit, wallet_indices[1])] {
        let (_, input) = wallet.spend(&mut rng, &keys, &value.qualify(index), segment)?;
        wallet_inputs.push(input);
    }
    let selected_wallet = WalletFundingInputs::from_inputs(wallet_inputs.clone())
        .map_err(|e| format!("wallet selection {e:?}"))?;
    let mut inputs: Vec<Input<ProofPreimage, DefaultDB>> = wallet_inputs.clone();
    let mut nullifiers = inputs
        .iter()
        .map(|input| input.nullifier)
        .collect::<Vec<_>>();
    let mut transients = vec![];
    if blue {
        let historic = historic.qualify(historic_index.unwrap());
        let input = Input::new_contract_owned(
            &mut rng,
            &historic,
            segment,
            address,
            &ledger.zswap.coin_coms,
        )?;
        nullifiers.push(input.nullifier);
        inputs.push(input);
        let output = Output::new_contract_owned(&mut rng, &wager, segment, address)?;
        let transient = Transient::new_from_contract_owned_output(
            &mut rng,
            &wager.qualify(0),
            segment,
            output,
        )?;
        nullifiers.push(transient.nullifier);
        transients.push(transient);
    }
    let mut preview =
        runtime::context::CircuitContext::from_contract_state(Private::default(), address, &prior);
    preview.set_zswap_output_start(first_free)?;
    let preview = c::start(
        preview,
        &Witness,
        Field::from(4u64),
        compact_coin(wager),
        compact_coin(deposit),
    )?;
    assert_eq!(
        preview.result,
        if blue {
            types::Player::blue
        } else {
            types::Player::red
        }
    );
    let planned = preview.context.circuit_zswap();
    assert_eq!(planned.outputs().len(), if blue { 3 } else { 2 });
    let persistent = if blue {
        &planned.outputs()[1..]
    } else {
        planned.outputs()
    };
    let outputs = persistent
        .iter()
        .map(|output| Output::new_contract_owned(&mut rng, &output.coin, segment, address))
        .collect::<Result<Vec<_>, _>>()?;
    let offer: Offer<ProofPreimage, DefaultDB> =
        Offer::new(inputs, outputs, transients.clone()).ok_or("empty start offer")?;
    let (_, allocated) = ledger.zswap.try_apply(&offer, None)?;
    let mut options = OfferBindingOptions::default()
        .with_output_allocation(PersistentOutputAllocation::CanonicalOfferIndices)
        .with_offer_placement(placement)
        .with_wallet_funding(selected_wallet);
    if blue {
        options = options.with_transient_coins(
            ContractTransientCoins::from_transients_for_placement(transients, placement)
                .map_err(|e| format!("transient selection {e:?}"))?,
        );
    }
    let observed = ObservedContractState::new(
        address,
        prior.clone(),
        Observation {
            transaction_hash: [0; 32],
            block_hash: [0; 32],
            block_height: 0,
        },
    );
    let bound = OfferBackedObservedState::with_options(observed, &ledger, offer.clone(), options)?;
    let native = c::start(
        bound.observed().circuit_context(Private::default()),
        &Witness,
        Field::from(4u64),
        compact_coin(wager),
        compact_coin(deposit),
    )?;
    let generated = c::Contract::from(Witness);
    let call = generated.recording().start_call(
        bound.observed(),
        Private::default(),
        Field::from(4u64),
        compact_coin(wager),
        compact_coin(deposit),
    )?;
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
    assert_eq!(
        recorded.public.verify_ops().len(),
        if blue { 97 } else { 58 }
    );
    assert_eq!(
        recorded.execution.private_transcript_outputs.len(),
        if blue { 8 } else { 5 }
    );
    let mut replay_initial = recorded.public.initial().clone();
    replay_initial.call_context.com_indices = recorded
        .execution
        .context
        .query
        .call_context
        .com_indices
        .clone();
    let replay = replay_initial.query(
        recorded.public.verify_ops(),
        None,
        &recorded.execution.context.cost_model,
    )?;
    assert_eq!(replay.context.state, native.context.query.state);
    assert_eq!(replay.context.effects, native.context.query.effects);
    let plan = native.context.circuit_zswap();
    assert_eq!(plan.inputs().len(), if blue { 2 } else { 0 });
    assert_eq!(plan.outputs().len(), if blue { 3 } else { 2 });
    for output in native
        .context
        .circuit_zswap()
        .outputs()
        .iter()
        .skip(usize::from(blue))
    {
        let actual = allocated
            .get(&output.coin.commitment(&output.recipient))
            .ok_or("missing persistent allocation")?;
        assert_eq!(&output.provisional_index, actual);
        let entry = ledger.zswap.coin_coms.index(*actual);
        assert!(entry.is_none()); // allocation is new relative to the prior state
    }
    assert_eq!(
        slots::state.inspect(native.context.query.state.get_ref())?,
        if blue {
            types::State::blue_started
        } else {
            types::State::red_started
        }
    );
    let verifier: VerifierKey = tagged_deserialize(&mut BufReader::new(File::open(
        root.join("keys/start.verifier"),
    )?))?;
    let prepared = bound.prepare(call, verifier.clone(), Fr::from(0u64))?;
    assert_eq!(
        prepared.prototype().guaranteed_public_transcript.is_some(),
        !blue
    );
    assert_eq!(
        prepared.prototype().fallible_public_transcript.is_some(),
        blue
    );
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
        panic!("standard")
    };
    if blue {
        assert!(structure.guaranteed_coins.is_none());
        assert_eq!(*structure.fallible_coins.get(&1).unwrap(), offer);
    } else {
        assert!(structure.fallible_coins.is_empty());
        assert_eq!(**structure.guaranteed_coins.as_ref().unwrap(), offer);
    }
    let resolver = super::qualified_coin_funding::fee_resolver(root, NAME)?;
    let params = MidnightDataProvider::new(FetchMode::OnDemand, OutputMode::Log, vec![])?;
    let provider = LocalProvingProvider {
        rng: StdRng::seed_from_u64(0x0213_5052 + u64::from(blue)),
        resolver: &resolver,
        params: &params,
    };
    let proven = futures_executor::block_on(
        tx.prove(provider, &INITIAL_PARAMETERS.cost_model.runtime_cost_model),
    )?;
    let sealed = proven.seal(StdRng::seed_from_u64(0x0213_5345 + u64::from(blue)));
    let sealed = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(fees.balance_tx(rng, sealed, &resolver))?;
    tagged_serialize(
        &sealed,
        &mut File::create(root.join(format!(
            "start-{}-sealed.bin",
            if blue { "blue" } else { "red" }
        )))?,
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
    if blue {
        // The selected whole-fallible program begins with a red-started Cell
        // observation. A concurrent phase change must roll back all selected
        // wallet, historic, transient and persistent coin effects together.
        let changed_data = slots::state
            .write(
                runtime::context::CircuitContext::from_contract_state(
                    Private::default(),
                    address,
                    &prior,
                ),
                types::State::no_game,
            )?
            .context
            .query
            .state;
        let mut changed_contract = prior.clone();
        changed_contract.data = changed_data.clone();
        let mut changed = ledger.clone();
        changed.contract = changed.contract.insert(address, changed_contract);
        let (after, result) = changed.apply(&verified, &context);
        let selected_failed = match &result {
            TransactionResult::PartialSuccess(phases, _) => phases
                .get(&0)
                .is_some_and(Result::is_ok)
                && matches!(
                    phases.get(&1),
                    Some(Err(midnight_ledger::error::TransactionInvalid::Transcript(
                        midnight_onchain_runtime::error::TranscriptRejected::Execution(
                            midnight_onchain_runtime::vm_error::OnchainProgramError::ReadMismatch { .. }
                        )
                    )))
                ),
            _ => false,
        };
        assert!(selected_failed, "{result:?}");
        assert_eq!(after.zswap, changed.zswap);
        assert_eq!(after.contract.get(&address).unwrap().data, changed_data);
        for nullifier in &nullifiers {
            assert!(!after.zswap.nullifiers.contains_key(nullifier));
        }
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
    assert_eq!(
        updated.zswap.first_free,
        first_free + if blue { 3 } else { 2 }
    );
    for output in native
        .context
        .circuit_zswap()
        .outputs()
        .iter()
        .skip(usize::from(blue))
    {
        let commitment = output.coin.commitment(&output.recipient);
        let index = *allocated.get(&commitment).unwrap();
        let entry = updated.zswap.coin_coms.index(index).unwrap();
        assert_eq!(entry.0, commitment.0);
        assert_eq!(entry.1.as_ref().map(|owner| **owner), Some(address));
    }
    for nullifier in nullifiers {
        assert!(updated.zswap.nullifiers.contains_key(&nullifier));
    }
    assert!(matches!(
        updated.zswap.try_apply(&offer, None),
        Err(midnight_zswap::error::TransactionInvalid::NullifierAlreadyPresent(_))
    ));
    println!(
        "original Coracle.start blue={blue}: two exact wallet inputs, canonical persistent allocation, whole selected segment, default-strict proof/apply, seeded prior state"
    );
    Ok(())
}
