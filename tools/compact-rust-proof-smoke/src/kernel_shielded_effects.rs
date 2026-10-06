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

//! Call proofs preserve Kernel effects; transaction acceptance needs matching offers.

use super::*;
use compact_rust_kernel_shielded_effects_oracle_fixture::ledger_contract as contract;
use midnight_compact_runtime as runtime;
use midnight_compact_runtime::transaction::OfferBackedObservedState;
use midnight_ledger::error::{EffectsCheckError, MalformedTransaction};
use midnight_transient_crypto::proofs::{ProverKey, Zkir};
use midnight_zswap::{Offer, Output};

type Amount = BoundedUint<18446744073709551615>;
struct Witnesses;
impl contract::Witnesses<()> for Witnesses {
    fn next_domain(
        &self,
        _: WitnessContext<'_, (), contract::LedgerView<'_>>,
    ) -> ((), FixedBytes<32>) {
        ((), FixedBytes::new([3; 32]))
    }
    fn next_amount(&self, _: WitnessContext<'_, (), contract::LedgerView<'_>>) -> ((), Amount) {
        ((), Amount::new(13).expect("13 fits Uint64"))
    }
}

pub(super) fn run(root: &Path) -> Result<(), Box<dyn Error>> {
    for (name, selected) in [
        ("mint", false),
        ("nullifier", false),
        ("spend", false),
        ("claim_receive", false),
        ("batch", false),
        ("selected", false),
        ("selected", true),
        ("witness_order", false),
    ] {
        prove_case(root, name, selected)?;
    }
    funded_mint(root)
}

fn prove_case(root: &Path, name: &'static str, selected: bool) -> Result<(), Box<dyn Error>> {
    let mut rng = StdRng::seed_from_u64(0x0184_4341);
    let initial = contract::initial_state(ConstructorContext::new(()))?;
    let deploy = make_deploy(root, name, initial.ledger_state.get_ref().clone(), &mut rng)?;
    let observed = ObservedContractState::new(
        deploy.address(),
        deploy.initial_state.clone(),
        Observation {
            transaction_hash: [0; 32],
            block_hash: [0; 32],
            block_height: 0,
        },
    );
    let value = FixedBytes::new([2; 32]);
    let seven = Amount::new(7)?;
    let eleven = Amount::new(11)?;
    let generated = contract::Contract::from(Witnesses);
    let recording = generated.recording();
    let call = match name {
        "mint" => recording.mint_call(&observed, (), value, seven)?,
        "nullifier" => recording.nullifier_call(&observed, (), value)?,
        "spend" => recording.spend_call(&observed, (), value)?,
        "claim_receive" => recording.claim_receive_call(&observed, (), value)?,
        "batch" => recording.batch_call(&observed, (), value, value, seven, eleven)?,
        "selected" => recording.selected_call(&observed, (), selected, value)?,
        "witness_order" => recording.witness_order_call(&observed, ())?,
        _ => unreachable!(),
    };
    let verifier: VerifierKey = tagged_deserialize(&mut BufReader::new(File::open(
        root.join(format!("keys/{name}.verifier")),
    )?))?;
    if name == "selected" && !selected {
        if !matches!(
            call.prepare(verifier, Fr::from(0_u64)),
            Err(ObservedCallError::Prepare(
                runtime::transaction::PrepareCallError::EmptyTranscript
            ))
        ) {
            return Err(
                "zero-effect selected branch did not retain EmptyTranscript refusal".into(),
            );
        }
        println!(
            "selected(false) records/replays zero effects and retains exact EmptyTranscript preparation refusal"
        );
        return Ok(());
    }
    let prepared = call.prepare(verifier.clone(), Fr::from(0_u64))?;
    prove_and_verify_call(root, name, &prepared, &verifier)?;
    if matches!(name, "nullifier" | "spend" | "claim_receive" | "batch")
        || (name == "selected" && selected)
    {
        let intent: Intent<Signature, ProofPreimageMarker, PedersenRandomness, DefaultDB> =
            Intent::empty(&mut rng, Timestamp::from_secs(0)).add_call::<ProofPreimage>(prepared);
        let tx = Transaction::from_intents("local-test", HashMap::new().insert(1_u16, intent));
        let mut ledger = LedgerState::<DefaultDB>::new("local-test");
        ledger.contract = ledger
            .contract
            .insert(deploy.address(), deploy.initial_state);
        // Isolate mandatory coin-claim semantics from this deliberately fee-unfunded call.
        let mut strictness = WellFormedStrictness::default();
        strictness.enforce_balancing = false;
        let error = tx
            .well_formed(&ledger, strictness, Timestamp::from_secs(0))
            .err()
            .ok_or("unmatched Kernel claim unexpectedly passed transaction validation")?;
        let expected = match (&error, name) {
            (
                MalformedTransaction::EffectsCheckFailure(
                    EffectsCheckError::NullifiersNEClaimedNullifiers {
                        nullifiers,
                        claimed_nullifiers,
                    },
                ),
                "nullifier" | "batch",
            ) => {
                nullifiers.is_empty()
                    && claimed_nullifiers.len() == 1
                    && claimed_nullifiers[0].1
                        == runtime::ledger::CoinNullifier(runtime::ledger::HashOutput([2; 32]))
            }
            (
                MalformedTransaction::EffectsCheckFailure(
                    EffectsCheckError::CommitmentsNEClaimedShieldedReceives {
                        commitments,
                        claimed_shielded_receives,
                    },
                ),
                "claim_receive",
            ) => {
                commitments.is_empty()
                    && claimed_shielded_receives.len() == 1
                    && claimed_shielded_receives[0].1
                        == runtime::ledger::CoinCommitment(runtime::ledger::HashOutput([2; 32]))
            }
            (
                MalformedTransaction::EffectsCheckFailure(
                    EffectsCheckError::AllCommitmentsSubsetCheckFailure(missing),
                ),
                "spend" | "selected",
            ) => {
                missing.superset.is_empty()
                    && missing.subset
                        == vec![(
                            0,
                            runtime::ledger::CoinCommitment(runtime::ledger::HashOutput([2; 32])),
                        )]
            }
            _ => false,
        };
        if !expected {
            return Err(format!("unexpected {name} unmatched-claim rejection: {error:?}").into());
        }
        println!(
            "{name} call proof verified; unmatched-offer transaction rejected exactly: {error:?}"
        );
    } else {
        println!("{name} selected={selected} call proof verified; no transaction acceptance claim");
    }
    Ok(())
}

pub(super) fn prove_and_verify_call(
    root: &Path,
    name: &'static str,
    call: &ContractCallPrototype<DefaultDB>,
    verifier: &VerifierKey,
) -> Result<(), Box<dyn Error>> {
    prove_and_verify_call_measured(root, name, call, verifier).map(|_| ())
}

pub(super) fn prove_and_verify_call_measured(
    root: &Path,
    name: &'static str,
    call: &ContractCallPrototype<DefaultDB>,
    verifier: &VerifierKey,
) -> Result<usize, Box<dyn Error>> {
    let mut fields = Vec::new();
    call.input.value_only_field_repr(&mut fields);
    call.output.value_only_field_repr(&mut fields);
    let commitment = transient_commit(&fields[..], call.communication_commitment_rand);
    let preimage =
        match <ProofPreimage as ContractCallExt<DefaultDB>>::construct_proof(call, commitment) {
            ProofPreimageVersioned::V2(preimage) => preimage,
            _ => return Err("unexpected Kernel call proof version".into()),
        };
    let ir: IrSource = tagged_deserialize(&mut BufReader::new(File::open(
        root.join(format!("zkir/{name}.bzkir")),
    )?))?;
    let pk: ProverKey<IrSource> = tagged_deserialize(&mut BufReader::new(File::open(
        root.join(format!("keys/{name}.prover")),
    )?))?;
    let checked_skips = preimage.check(&ir)?;
    let params = MidnightDataProvider::new(FetchMode::OnDemand, OutputMode::Log, vec![])?;
    // Use upstream IR's exact public inputs, including conditional skip semantics.
    let (proof, mut inputs, proof_skips) = futures_executor::block_on(ir.prove(
        ChaCha20Rng::from_seed([0x84; 32]),
        &params,
        pk,
        &preimage,
    ))?;
    if proof_skips != checked_skips {
        return Err("Kernel proof/preimage skips differ".into());
    }
    verifier.verify(&PARAMS_VERIFIER, &proof, inputs.iter().copied())?;
    inputs[0] = if inputs[0] == Fr::from(0_u64) {
        Fr::from(1_u64)
    } else {
        Fr::from(0_u64)
    };
    if verifier
        .verify(&PARAMS_VERIFIER, &proof, inputs.iter().copied())
        .is_ok()
    {
        return Err("Kernel proof accepted changed binding".into());
    }
    println!(
        "{name} cryptographic proof verified ({} bytes), changed binding rejected",
        proof.0.len()
    );
    Ok(proof.0.len())
}

fn funded_mint(root: &Path) -> Result<(), Box<dyn Error>> {
    const NAME: &str = "mint";
    let mut rng = StdRng::seed_from_u64(0x0184_4d49);
    let initial = contract::initial_state(ConstructorContext::new(()))?;
    let deploy = make_deploy(root, NAME, initial.ledger_state.get_ref().clone(), &mut rng)?;
    let mut fee_state = super::qualified_coin_funding::fee_funded_state(&mut rng)?;
    fee_state.ledger.contract = fee_state
        .ledger
        .contract
        .insert(deploy.address(), deploy.initial_state.clone());
    let ledger = fee_state.ledger.clone();
    let time = fee_state.time;
    let domain = runtime::ledger::HashOutput([0x84; 32]);
    let coin = runtime::ledger::CoinInfo {
        nonce: runtime::ledger::CoinNonce(runtime::ledger::HashOutput([1; 32])),
        type_: deploy.address().custom_shielded_token_type(domain),
        value: 42,
    };
    let recipient = runtime::ledger::CoinPublicKey(runtime::ledger::HashOutput([7; 32]));
    let output = Output::new(&mut rng, &coin, None, &recipient, None)?;
    let commitment = output.coin_com;
    let offer: Offer<ProofPreimage, DefaultDB> =
        Offer::new(vec![], vec![output], vec![]).ok_or("empty minted output offer")?;
    let (_, indices) = ledger.zswap.try_apply(&offer, None)?;
    let index = *indices
        .get(&commitment)
        .ok_or("minted output not allocated")?;
    let bound = OfferBackedObservedState::new(
        ObservedContractState::new(
            deploy.address(),
            deploy.initial_state.clone(),
            Observation {
                transaction_hash: [0; 32],
                block_hash: [0; 32],
                block_height: 0,
            },
        ),
        &ledger,
        offer,
    )?;
    let call = contract::recorded::Contract.mint_call(
        bound.observed(),
        (),
        FixedBytes::new(domain.0),
        Amount::new(42)?,
    )?;
    let verifier: VerifierKey = tagged_deserialize(&mut BufReader::new(File::open(
        root.join("keys/mint.verifier"),
    )?))?;
    let tx = bound
        .prepare(call, verifier, Fr::from(0_u64))?
        .into_transaction(
            &mut rng,
            "local-test",
            Timestamp::from_secs(time.to_secs() + 3600),
        );
    let resolver = super::qualified_coin_funding::fee_resolver(root, NAME)?;
    let params = MidnightDataProvider::new(FetchMode::OnDemand, OutputMode::Log, vec![])?;
    let provider = LocalProvingProvider {
        rng: StdRng::seed_from_u64(0x0184_5052),
        resolver: &resolver,
        params: &params,
    };
    let proven = futures_executor::block_on(
        tx.prove(provider, &INITIAL_PARAMETERS.cost_model.runtime_cost_model),
    )?;
    let sealed = proven.seal(StdRng::seed_from_u64(0x0184_5345));
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
        return Err(format!("funded mint application failed: {outcome:?}").into());
    }
    if updated
        .contract
        .get(&deploy.address())
        .ok_or("mint contract disappeared")?
        .data
        != deploy.initial_state.data
    {
        return Err("Kernel mint changed public contract state".into());
    }
    if updated.zswap.first_free != index + 1
        || updated.zswap.coin_coms.index(index).map(|(hash, _)| hash) != Some(commitment.0)
    {
        return Err("minted output ledger index/commitment differs".into());
    }
    println!(
        "nonzero Kernel mint42 + matching minted output + Night-backed Dust passed default strict validation and ledger application at output index {index}"
    );
    Ok(())
}
