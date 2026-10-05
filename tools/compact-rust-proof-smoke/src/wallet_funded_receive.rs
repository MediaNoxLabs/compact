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

//! Explicit upstream wallet input funding of an unchanged receiveShielded call.
use super::*;
use compact_rust_shielded_receive_oracle_fixture::ledger_contract as contract;
use midnight_compact_runtime as runtime;
use midnight_zswap::error::TransactionInvalid;
use midnight_zswap::{Offer, Output};
use runtime::transaction::{OfferBackedObservedState, WalletFundingInputs, ZswapIntentError};

pub(super) fn run(root: &Path) -> Result<(), Box<dyn Error>> {
    let mut rng = StdRng::seed_from_u64(0x0199_0000);
    let initial = contract::initial_state(ConstructorContext::new(()))?;
    let deploy = make_deploy(
        root,
        "accept",
        initial.ledger_state.get_ref().clone(),
        &mut rng,
    )?;
    let address = deploy.address();
    let verifier: VerifierKey = tagged_deserialize(&mut BufReader::new(File::open(
        root.join("keys/accept.verifier"),
    )?))?;
    let mut state = super::qualified_coin_funding::fee_funded_state(&mut rng)?;
    let coin = super::shielded_receive::coin(3, 42);
    let info = runtime::ledger::coin_info_from_compact(coin.nonce, coin.color, 42);
    let funded = super::qualified_coin_funding::seed_and_fund(
        &mut rng,
        &mut state.ledger,
        info,
        runtime::ledger::CoinPublicKey(runtime::ledger::HashOutput([7; 32])),
        Timestamp::from_secs(0),
    )?;
    state.ledger.contract = state
        .ledger
        .contract
        .insert(address, deploy.initial_state.clone());
    let input = funded
        .offer
        .inputs
        .iter_deref()
        .next()
        .cloned()
        .ok_or("wallet spend input missing")?;
    let output = Output::new_contract_owned(&mut rng, &info, None, address)?;
    let commitment = output.coin_com;
    let offer = Offer::new(vec![input.clone()], vec![output], vec![])
        .ok_or("wallet funded receive offer missing")?;
    let generated = contract::Contract::default();

    // The old constructor continues to require an exact contract-owned input
    // intent. A caller must opt into the explicit wallet selection.
    let default_bound = OfferBackedObservedState::new(
        super::shielded_receive::observed(address, deploy.initial_state.clone()),
        &state.ledger,
        offer.clone(),
    )?;
    let default_call =
        generated
            .recording()
            .accept_call(default_bound.observed(), (), coin.clone())?;
    assert!(matches!(
        default_bound.prepare(default_call, verifier.clone(), Fr::from(0)),
        Err(ObservedCallError::ZswapIntent(
            ZswapIntentError::InputMismatch
        ))
    ));

    let mut wrong_input = input.clone();
    wrong_input.nullifier = runtime::ledger::CoinNullifier(runtime::ledger::HashOutput([9; 32]));
    let wrong_selection = WalletFundingInputs::from_inputs(vec![wrong_input])
        .map_err(|error| format!("wrong funding selection unexpectedly rejected: {error:?}"))?;
    assert!(
        OfferBackedObservedState::with_wallet_funding(
            super::shielded_receive::observed(address, deploy.initial_state.clone()),
            &state.ledger,
            offer.clone(),
            wrong_selection,
        )
        .is_err()
    );
    assert!(matches!(
        WalletFundingInputs::from_inputs(vec![input.clone(), input.clone()]),
        Err(ZswapIntentError::WalletFundingDuplicate)
    ));

    let funding = WalletFundingInputs::from_inputs(vec![input.clone()])
        .map_err(|error| format!("wallet funding selection rejected: {error:?}"))?;
    let bound = OfferBackedObservedState::with_wallet_funding(
        super::shielded_receive::observed(address, deploy.initial_state.clone()),
        &state.ledger,
        offer.clone(),
        funding,
    )?;
    let other_bound = OfferBackedObservedState::with_wallet_funding(
        super::shielded_receive::observed(address, deploy.initial_state),
        &state.ledger,
        offer.clone(),
        WalletFundingInputs::from_inputs(vec![input.clone()])
            .map_err(|error| format!("second funding selection rejected: {error:?}"))?,
    )?;
    let other_call = generated
        .recording()
        .accept_call(other_bound.observed(), (), coin.clone())?;
    assert!(matches!(
        bound.prepare(other_call, verifier.clone(), Fr::from(0)),
        Err(ObservedCallError::OfferMismatch)
    ));
    let native = contract::accept(bound.observed().circuit_context(()), coin.clone())?;
    let call = generated
        .recording()
        .accept_call(bound.observed(), (), coin)?;
    assert_eq!(call.recorded().execution.result, ());
    assert_eq!(native.result, ());
    assert_eq!(
        native.context.query.state,
        call.recorded().execution.context.query.state
    );
    assert_eq!(
        native.context.query.effects,
        call.recorded().execution.context.query.effects
    );
    assert_eq!(native.gas_cost, call.recorded().execution.gas_cost);
    let replay = call.recorded().public.initial().query(
        call.recorded().public.verify_ops(),
        None,
        &call.recorded().execution.context.cost_model,
    )?;
    assert_eq!(replay.context.state, native.context.query.state);
    assert_eq!(replay.context.effects, native.context.query.effects);
    // Replay executes the flattened public program as one query, while
    // native/recorded charge the helper's two source query calls. The
    // original-source fixture checks exact TS replay gas at its own address;
    // this funded call uses a deployed address and offer-derived index.
    let replay_compute = serde_json::json!(replay.gas_cost)["computeTime"]
        .as_u64()
        .unwrap();
    let native_compute = serde_json::json!(native.gas_cost)["computeTime"]
        .as_u64()
        .unwrap();
    assert!(replay_compute > 0 && replay_compute < native_compute);
    assert_eq!(call.recorded().public.verify_ops().len(), 9);
    assert_eq!(
        call.recorded().execution.private_transcript_outputs.len(),
        1
    );
    assert_eq!(
        call.recorded()
            .execution
            .context
            .circuit_zswap()
            .outputs()
            .len(),
        1
    );
    assert!(
        call.recorded()
            .execution
            .context
            .circuit_zswap()
            .inputs()
            .is_empty()
    );
    assert_eq!(
        call.recorded()
            .execution
            .context
            .query
            .call_context
            .com_indices
            .get(&commitment),
        Some(&state.ledger.zswap.first_free)
    );
    let prepared = bound.prepare(call, verifier.clone(), Fr::from(0))?;
    let output_index = state.ledger.zswap.first_free;
    let updated = super::composite_zswap::apply_bound(
        root,
        "accept",
        prepared,
        state,
        rng,
        &verifier,
        commitment,
        output_index,
        address,
    )?;
    assert!(
        updated
            .zswap
            .nullifiers
            .contains_key(&funded.spent_nullifier)
    );
    assert!(matches!(
        updated.zswap.try_apply(&offer, None),
        Err(TransactionInvalid::NullifierAlreadyPresent(nullifier))
            if nullifier == funded.spent_nullifier
    ));
    println!(
        "wallet-funded receive: input42 -> contract output42 proven and applied at default strictness; wallet nullifier consumed and replay rejected"
    );
    Ok(())
}
