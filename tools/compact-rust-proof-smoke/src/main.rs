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

//! Prove emitted Counter, Cell, Set, Map, List, plain/historic Merkle, and enum Cell artifacts in offline ledger-8 transactions.
//!
//! The proof uses the ledger-derived statement from a generated counter VM
//! program and checks it against the fixture's known ZKIR encoding. The call
//! commitment uses the value-field encoding from ledger-8's Intent::add_call.
//! Deployment combines the generated constructor state with the emitted key.

mod adt_list_bytes;
mod adt_list_enum;
mod adt_list_field;
mod adt_list_vector_field_4;
mod adt_set_enum;
mod adt_set_qualified_coin_info;
mod adt_set_vector;
mod asset_custody_grant;
mod asset_dispatch;
mod asset_freshness;
mod asset_grant_effective;
mod asset_record_write;
mod asset_removal;
mod asset_stored_record_fresh;
mod asset_watch_write;
mod asset_writable;
mod bboard;
mod boolean_pair_assert;
mod canonical_output_order;
mod closed_pure_field;
mod composite_zswap;
mod coracle_concede;
mod coracle_guess;
#[path = "../../../tests-rust-backend/test-center-coracle/support/guess.rs"]
mod coracle_guess_support;
mod coracle_start;
mod coracle_withdraw;
#[path = "../../../tests-rust-backend/test-center-coracle/support/withdraw.rs"]
mod coracle_withdraw_support;
mod counter_less_than;
mod counter_scenario;
mod effect_plan_counter;
mod effect_plan_counter_only;
mod effect_plan_jubjub;
#[cfg(feature = "experimental-recording-plan")]
mod effect_plan_prototype;
mod effect_plan_pure_calls;
mod effect_plan_rich_values;
mod effect_plan_stateful_calls;
mod effect_plan_transient_crypto;
mod effect_plan_unsigned_arithmetic;
mod effect_plan_wide_source;
mod effectful_return;
mod effectful_scenario;
mod election_commit;
#[path = "../../../tests-rust-backend/election-oracle/support/commit.rs"]
mod election_membership_support;
mod election_reveal;
mod election_topic;
mod field_observation;
mod field_pair_hash;
mod field_to_bytes32;
mod guarded_point_read;
mod guarded_recording;
mod impure_field_helper;
mod inline_type_scope;
mod kernel_shielded_effects;
mod let_return;
mod literal_bytes_field;
mod merkle_hash;
mod merkle_indexed;
mod merkle_reset;
mod merkle_root;
mod merkle_verify;
mod micro_dao_advance;
use micro_dao_cash_out_support::advance as micro_dao_advance_support;
mod did_deactivate;
#[path = "../../../tests-rust-backend/did-adoption/support/witness.rs"]
mod did_deactivate_support;
mod did_digest_reducer;
mod did_point_lifecycle;
mod did_primitive_reducers;
mod did_public_interchange;
mod direct_dispatch;
mod micro_dao_buy_in;
mod micro_dao_cash_out;
#[path = "../../../tests-rust-backend/test-center-micro-dao/support/cash_out.rs"]
mod micro_dao_cash_out_support;
mod micro_dao_reveal;
#[path = "../../../tests-rust-backend/test-center-micro-dao/support/reveal.rs"]
mod micro_dao_reveal_support;
mod micro_dao_set_topic;
mod micro_dao_token;
mod micro_dao_vote_commit;
mod mixed_width_recording;
mod observational_binding;
mod opaque_string_map;
mod opaque_string_set;
mod pair_hash_cell;
mod persistent_commit;
mod positional_cells;
mod positional_counters;
mod positional_inputs;
mod positional_keys;
mod positional_lists;
mod positional_maps;
mod positional_merkle;
mod positional_sets;
mod positional_suite;
mod positional_unsigned;
mod positional_witnesses;
mod proof_material;
mod pure_assert_call;
mod pure_field_arguments;
mod qualified_coin_cell;
mod qualified_coin_funding;
mod qualified_coin_set;
mod root_let_action_return;
mod schnorr_attestation;
mod shielded_merge;
mod shielded_receive;
mod shielded_send;
mod source_preimage;
mod stateful_assert;
mod stateful_pair_hash;
mod stateful_pure_return;
mod stateful_struct;
mod struct_constructor_cell;
mod terminal_lexical_return;
#[path = "../../../tests-rust-backend/terminal-lexical-return-oracle/support/terminal.rs"]
mod terminal_lexical_return_support;
mod test_center_counter;
mod test_center_welcome;
mod transient_receive_send;
mod unsigned_recording;
mod unused_field_reads;
mod vector_map;
mod wallet_funded_receive;
mod witness_assert;
mod witness_vector_action;
mod witness_vector_let;
mod zerocash_mint;
mod zerocash_spend;
mod zswap_transfer;

use std::env;
use std::error::Error;
use std::fs::{self, File};
use std::io::{self, BufReader};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use compact_rust_cell_struct_fixture::ledger_contract as composite_cell_contract;
use compact_rust_cell_struct_fixture::types::Pair;
use compact_rust_cross_circuit_oracle_fixture::ledger_contract as cross_circuit_contract;
use compact_rust_hmt_default_oracle_fixture::ledger_contract as historic_merkle_default_contract;
use compact_rust_hmt_insert_oracle_fixture::ledger_contract as historic_merkle_contract;
use compact_rust_merkle_tree_oracle_fixture::ledger_contract as merkle_contract;
use compact_rust_nested_stateful_ternary_fixture::ledger_contract as nested_stateful_ternary_contract;
use compact_rust_nested_witness_call_oracle_fixture::ledger_contract as expression_contract;
use compact_rust_set_boolean_fixture::ledger_contract as set_contract;
use compact_rust_set_size_oracle_fixture::ledger_contract as set_size_contract;
use compact_rust_ternary_cond_oracle_fixture::ledger_contract as conditional_counter_contract;
use compact_rust_tiny_oracle_fixture::ledger_contract as tiny_contract;
use compact_rust_wide_uint_oracle_fixture::ledger_contract as wide_contract;
use compact_rust_witness_cell_write_fixture::ledger_contract as witness_contract;
use midnight_base_crypto::data_provider::{FetchMode, MidnightDataProvider, OutputMode};
use midnight_base_crypto::signatures::Signature;
use midnight_base_crypto::time::Timestamp;
use midnight_compact_runtime::Field;
use midnight_compact_runtime::context::{ConstructorContext, WitnessContext};
use midnight_compact_runtime::fab::AlignedValue;
use midnight_compact_runtime::ledger::{
    DefaultDB, StateValue, historic_merkle_tree_view_at_path, list_view_at_path, map_view_at_path,
    merkle_tree_view_at_path, read_cell, read_cell_at_path, read_counter, set_view_at_path,
};
use midnight_compact_runtime::recording::RecordedCircuitResult;
use midnight_compact_runtime::transaction::{
    CallSpec, Observation, ObservedCallError, ObservedContractState, prepare_call,
};
use midnight_compact_runtime::{BoundedUint, FixedBytes, FixedVector, WideUint};
use midnight_ledger::construct::{ContractCallExt, ContractCallPrototype};
use midnight_ledger::semantics::{TransactionContext, TransactionResult};
use midnight_ledger::structure::{
    ContractDeploy, INITIAL_PARAMETERS, Intent, LedgerState, ProofMarker, ProofPreimageMarker,
    ProofPreimageVersioned, Transaction,
};
use midnight_ledger::verify::WellFormedStrictness;
use midnight_onchain_runtime::context::BlockContext;
use midnight_onchain_state::state::{
    ContractMaintenanceAuthority, ContractOperation, ContractState, EntryPointBuf,
};
use midnight_onchain_vm::ops::Op;
use midnight_serialize::{tagged_deserialize, tagged_serialize};
use midnight_storage::storage::HashMap;
use midnight_transient_crypto::commitment::{PedersenRandomness, PureGeneratorPedersen};
use midnight_transient_crypto::curve::Fr;
use midnight_transient_crypto::fab::AlignedValueExt;
use midnight_transient_crypto::hash::transient_commit;
use midnight_transient_crypto::proofs::{
    KeyLocation, PARAMS_VERIFIER, ProofPreimage, ProvingKeyMaterial, Resolver, VerifierKey,
};
use midnight_zkir::{IrSource, LocalProvingProvider};
use rand::{SeedableRng, rngs::StdRng};
use rand_chacha::ChaCha20Rng;

struct ArtifactResolver {
    root: PathBuf,
    circuit: &'static str,
}

struct HandoffConfig {
    network_id: String,
    ttl: Timestamp,
    validation_time: Timestamp,
}

impl HandoffConfig {
    fn offline() -> Self {
        Self {
            network_id: "local-test".into(),
            ttl: Timestamp::from_secs(0),
            validation_time: Timestamp::from_secs(0),
        }
    }

    fn from_env() -> Result<Self, Box<dyn Error>> {
        match (
            env::var("COMPACT_RUST_HANDOFF_NETWORK_ID"),
            env::var("COMPACT_RUST_HANDOFF_TTL_SECS"),
        ) {
            (Err(env::VarError::NotPresent), Err(env::VarError::NotPresent)) => Ok(Self::offline()),
            (Ok(network_id), Ok(ttl)) => {
                if network_id.is_empty() || network_id.trim() != network_id {
                    return Err(
                        "handoff network ID must be nonempty and have no surrounding whitespace"
                            .into(),
                    );
                }
                let ttl: u64 = ttl.parse()?;
                let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
                if ttl <= now + 60 {
                    return Err("handoff TTL must be at least 60 seconds in the future".into());
                }
                if ttl > now + 3_600 {
                    return Err("handoff TTL exceeds ledger-8's one-hour global TTL".into());
                }
                Ok(Self {
                    network_id,
                    ttl: Timestamp::from_secs(ttl),
                    validation_time: Timestamp::from_secs(now),
                })
            }
            _ => Err(
                "set both COMPACT_RUST_HANDOFF_NETWORK_ID and COMPACT_RUST_HANDOFF_TTL_SECS".into(),
            ),
        }
    }
}

impl Resolver for ArtifactResolver {
    async fn resolve_key(&self, location: KeyLocation) -> io::Result<Option<ProvingKeyMaterial>> {
        if location.0.as_ref() != self.circuit {
            return Ok(None);
        }
        Ok(Some(ProvingKeyMaterial {
            prover_key: fs::read(self.root.join(format!("keys/{}.prover", self.circuit)))?,
            verifier_key: fs::read(self.root.join(format!("keys/{}.verifier", self.circuit)))?,
            ir_source: fs::read(self.root.join(format!("zkir/{}.bzkir", self.circuit)))?,
        }))
    }
}

fn prove_counter(
    root: &Path,
    call: &ContractCallPrototype<DefaultDB>,
) -> Result<(), Box<dyn Error>> {
    let ir: IrSource = tagged_deserialize(&mut BufReader::new(File::open(
        root.join("zkir/increment.bzkir"),
    )?))?;
    let expected_statement = [0x70u64, 1, 1, 0, 0x0e, 1, 0xa1]
        .into_iter()
        .map(Fr::from)
        .collect::<Vec<_>>();
    let mut value_fields = Vec::new();
    call.input.value_only_field_repr(&mut value_fields);
    call.output.value_only_field_repr(&mut value_fields);
    let commitment = transient_commit(&value_fields[..], call.communication_commitment_rand);
    let preimage =
        match <ProofPreimage as ContractCallExt<DefaultDB>>::construct_proof(call, commitment) {
            ProofPreimageVersioned::V2(preimage) => preimage,
            _ => return Err("ledger produced an unsupported proof preimage version".into()),
        };
    if preimage.public_transcript_inputs != expected_statement {
        return Err(format!(
            "generated counter statement differs from fixture ZKIR: {:?}",
            preimage.public_transcript_inputs
        )
        .into());
    }
    if !preimage.inputs.is_empty()
        || !preimage.private_transcript.is_empty()
        || !preimage.public_transcript_outputs.is_empty()
    {
        return Err("counter proof unexpectedly requires private, input, or output fields".into());
    }
    let skips = preimage.check(&ir)?;
    if skips != [None, None, None] {
        return Err(format!("unexpected counter public input skips: {skips:?}").into());
    }

    let params = MidnightDataProvider::new(FetchMode::OnDemand, OutputMode::Log, vec![])?;
    let (proof, proof_skips) = futures_executor::block_on(preimage.prove::<IrSource>(
        ChaCha20Rng::from_seed([42; 32]),
        &params,
        &ArtifactResolver {
            root: root.to_owned(),
            circuit: "increment",
        },
    ))?;
    if proof_skips != skips {
        return Err("proof and preimage checks disagree about skipped public inputs".into());
    }

    let verifier: VerifierKey = tagged_deserialize(&mut BufReader::new(File::open(
        root.join("keys/increment.verifier"),
    )?))?;
    let mut public_inputs = vec![
        preimage.binding_input,
        preimage
            .communications_commitment
            .expect("counter fixture supplies a communications commitment")
            .0,
    ];
    public_inputs.extend(preimage.public_transcript_inputs.iter().copied());
    verifier.verify(&PARAMS_VERIFIER, &proof, public_inputs.iter().copied())?;
    public_inputs[0] = Fr::from(1u64);
    if verifier
        .verify(&PARAMS_VERIFIER, &proof, public_inputs.iter().copied())
        .is_ok()
    {
        return Err("counter proof accepted a different binding input".into());
    }
    println!(
        "counter proof verified ({} bytes); changed binding rejected",
        proof.0.len()
    );
    Ok(())
}

fn check_generated_trace<Private, Output: Into<AlignedValue>, Input: Into<AlignedValue>>(
    root: &Path,
    circuit: &'static str,
    recorded: RecordedCircuitResult<Private, Output>,
    input: Input,
) -> Result<ContractCallPrototype<DefaultDB>, Box<dyn Error>> {
    let verifier: VerifierKey = tagged_deserialize(&mut BufReader::new(File::open(
        root.join(format!("keys/{circuit}.verifier")),
    )?))?;
    let call = prepare_call(
        recorded,
        CallSpec::new(circuit, verifier, input, Fr::from(0u64)),
    )?;
    println!("generated {circuit} trace replayed and partitioned");
    Ok(call)
}

fn check_observed_call_parity(
    root: &Path,
    circuit: &'static str,
    deploy: &ContractDeploy<DefaultDB>,
    manual: &ContractCallPrototype<DefaultDB>,
    observed: &ContractCallPrototype<DefaultDB>,
) -> Result<(), Box<dyn Error>> {
    // The prototype Debug representation includes the complete public and
    // private transcripts, effects, gas, input/output and key location.
    if format!("{manual:?}") != format!("{observed:?}") {
        return Err(format!("{circuit} observed call differs from manual adapter").into());
    }
    let prove_and_apply = |call: ContractCallPrototype<DefaultDB>| -> Result<(Vec<u8>, ContractState<DefaultDB>), Box<dyn Error>> {
        let mut intent_rng = StdRng::seed_from_u64(0x0041_4452_3433);
        let intent: Intent<Signature, ProofPreimageMarker, PedersenRandomness, DefaultDB> =
            Intent::empty(&mut intent_rng, Timestamp::from_secs(0)).add_call::<ProofPreimage>(call);
        let transaction =
            Transaction::from_intents("local-test", HashMap::new().insert(1_u16, intent));
        let mut preproof_bytes = Vec::new();
        tagged_serialize(&transaction, &mut preproof_bytes)?;
        let resolver = ArtifactResolver {
            root: root.to_owned(),
            circuit,
        };
        let params = MidnightDataProvider::new(FetchMode::OnDemand, OutputMode::Log, vec![])?;
        let provider = LocalProvingProvider {
            rng: StdRng::seed_from_u64(0x0050_524f_5645),
            resolver: &resolver,
            params: &params,
        };
        let proven = futures_executor::block_on(
            transaction.prove(provider, &INITIAL_PARAMETERS.cost_model.runtime_cost_model),
        )?;
        let sealed = proven.seal(StdRng::seed_from_u64(0x5345_414c));
        let mut ledger = LedgerState::<DefaultDB>::new("local-test");
        ledger.contract = ledger
            .contract
            .insert(deploy.address(), deploy.initial_state.clone());
        let mut strictness = WellFormedStrictness::default();
        strictness.enforce_balancing = false;
        let validation_time = Timestamp::from_secs(0);
        let verified = sealed.well_formed(&ledger, strictness, validation_time)?;
        let context = TransactionContext {
            ref_state: ledger.clone(),
            block_context: BlockContext {
                tblock: validation_time,
                last_block_time: validation_time,
                ..BlockContext::default()
            },
            whitelist: None,
        };
        let (updated, outcome) = ledger.apply(&verified, &context);
        if !matches!(outcome, TransactionResult::Success(_)) {
            return Err(format!("{circuit} parity call application failed: {outcome:?}").into());
        }
        let state = updated
            .contract
            .get(&deploy.address())
            .ok_or("parity call removed the contract")?
            .clone();
        Ok((preproof_bytes, state))
    };
    let manual_result = prove_and_apply(manual.clone())?;
    let observed_result = prove_and_apply(observed.clone())?;
    if manual_result.0 != observed_result.0 {
        return Err(format!("{circuit} observed call changed pre-proof transaction bytes").into());
    }
    if manual_result.1 != observed_result.1 {
        return Err(format!("{circuit} observed call changed applied contract state").into());
    }
    // midnight-proofs 0.7.0 blinds quotient limbs with OsRng, so proof and
    // sealed bytes from two independent valid runs need not match exactly.
    println!(
        "{circuit} observed call matches manual prototype, {} pre-proof bytes and independently applied state",
        observed_result.0.len()
    );
    Ok(())
}

fn make_deploy(
    root: &Path,
    circuit: &'static str,
    state: StateValue<DefaultDB>,
    rng: &mut StdRng,
) -> Result<ContractDeploy<DefaultDB>, Box<dyn Error>> {
    let verifier: VerifierKey = tagged_deserialize(&mut BufReader::new(File::open(
        root.join(format!("keys/{circuit}.verifier")),
    )?))?;
    let operations = HashMap::new().insert(
        EntryPointBuf(circuit.as_bytes().to_vec()),
        ContractOperation::new(Some(verifier)),
    );
    let contract: ContractState<DefaultDB> =
        ContractState::new(state, operations, ContractMaintenanceAuthority::default());
    Ok(ContractDeploy::new(rng, contract))
}

fn write_sealed_handoff(
    path: &Path,
    transaction: &Transaction<Signature, ProofMarker, PureGeneratorPedersen, DefaultDB>,
    label: &str,
) -> Result<(), Box<dyn Error>> {
    let mut bytes = Vec::new();
    tagged_serialize(transaction, &mut bytes)?;
    let roundtrip: Transaction<Signature, ProofMarker, PureGeneratorPedersen, DefaultDB> =
        tagged_deserialize(&mut bytes.as_slice())?;
    let mut roundtrip_bytes = Vec::new();
    tagged_serialize(&roundtrip, &mut roundtrip_bytes)?;
    if bytes != roundtrip_bytes {
        return Err(format!("sealed {label} changed across Rust serialization").into());
    }
    fs::write(path, bytes)?;
    println!(
        "sealed {label} ledger-v8 transaction written to {}",
        path.display()
    );
    Ok(())
}

#[expect(
    clippy::too_many_arguments,
    reason = "proof handoff requires independent transaction and state inputs"
)]
fn check_transaction_with_handoff<F>(
    root: &Path,
    circuit: &'static str,
    deploy: ContractDeploy<DefaultDB>,
    call: ContractCallPrototype<DefaultDB>,
    rng: &mut StdRng,
    handoff_path: Option<&Path>,
    deploy_handoff_path: Option<&Path>,
    handoff: &HandoffConfig,
    check_state: F,
) -> Result<(), Box<dyn Error>>
where
    F: FnOnce(&ContractState<DefaultDB>) -> Result<(), Box<dyn Error>>,
{
    let address = deploy.address();
    let deploy_intent: Intent<Signature, ProofPreimageMarker, PedersenRandomness, DefaultDB> =
        Intent::empty(rng, handoff.ttl).add_deploy(deploy.clone());
    let deploy_tx = Transaction::from_intents(
        handoff.network_id.as_str(),
        HashMap::new().insert(1_u16, deploy_intent),
    );
    let empty_ledger = LedgerState::<DefaultDB>::new(handoff.network_id.as_str());
    let mut strictness = WellFormedStrictness::default();
    strictness.enforce_balancing = false;
    let sealed_deploy = if deploy_handoff_path.is_some() {
        let resolver = ArtifactResolver {
            root: root.to_owned(),
            circuit,
        };
        let params = MidnightDataProvider::new(FetchMode::OnDemand, OutputMode::Log, vec![])?;
        let provider = LocalProvingProvider {
            rng: StdRng::seed_from_u64(0x4445504c4f59),
            resolver: &resolver,
            params: &params,
        };
        let proven_deploy = futures_executor::block_on(
            deploy_tx.prove(provider, &INITIAL_PARAMETERS.cost_model.runtime_cost_model),
        )?;
        let sealed_deploy = proven_deploy.seal(StdRng::seed_from_u64(0x4445504c4f59));
        if sealed_deploy
            .deploys()
            .map(|(_, action)| action.address())
            .collect::<Vec<_>>()
            != vec![address]
        {
            return Err("sealed deployment address changed".into());
        }
        Some(sealed_deploy)
    } else {
        None
    };
    let verified_deploy = match &sealed_deploy {
        Some(transaction) => {
            transaction.well_formed(&empty_ledger, strictness, handoff.validation_time)?
        }
        None => deploy_tx.well_formed(&empty_ledger, strictness, handoff.validation_time)?,
    };
    let deploy_context = TransactionContext {
        ref_state: empty_ledger.clone(),
        block_context: BlockContext {
            tblock: handoff.validation_time,
            last_block_time: handoff.validation_time,
            ..BlockContext::default()
        },
        whitelist: None,
    };
    let (ledger, deploy_outcome) = empty_ledger.apply(&verified_deploy, &deploy_context);
    if !matches!(deploy_outcome, TransactionResult::Success(_)) {
        return Err(format!("{circuit} deployment application failed: {deploy_outcome:?}").into());
    }
    if !ledger.contract.contains_key(&address) {
        return Err(format!("{circuit} deployed contract is absent at {address:?}").into());
    }
    if let (Some(path), Some(transaction)) = (deploy_handoff_path, sealed_deploy.as_ref()) {
        write_sealed_handoff(path, transaction, "deployment")?;
    }
    let call_intent: Intent<Signature, ProofPreimageMarker, PedersenRandomness, DefaultDB> =
        Intent::empty(rng, handoff.ttl).add_call::<ProofPreimage>(call);
    let call_tx = Transaction::from_intents(
        handoff.network_id.as_str(),
        HashMap::new().insert(1_u16, call_intent),
    );
    call_tx.well_formed(&ledger, strictness, handoff.validation_time)?;
    let resolver = ArtifactResolver {
        root: root.to_owned(),
        circuit,
    };
    let params = MidnightDataProvider::new(FetchMode::OnDemand, OutputMode::Log, vec![])?;
    let provider = LocalProvingProvider {
        rng: StdRng::seed_from_u64(0x43414c4c),
        resolver: &resolver,
        params: &params,
    };
    let proven = futures_executor::block_on(
        call_tx.prove(provider, &INITIAL_PARAMETERS.cost_model.runtime_cost_model),
    )?;
    let verified = proven.well_formed(&ledger, strictness, handoff.validation_time)?;
    if let Some(path) = handoff_path {
        let sealed = proven.seal(StdRng::seed_from_u64(0x57414c4c4554));
        sealed.well_formed(&ledger, strictness, handoff.validation_time)?;
        if sealed
            .calls()
            .map(|(_, action)| action.address)
            .collect::<Vec<_>>()
            != vec![address]
        {
            return Err("sealed call address changed".into());
        }
        write_sealed_handoff(path, &sealed, "call")?;
    }
    let context = TransactionContext {
        ref_state: ledger.clone(),
        block_context: BlockContext {
            tblock: handoff.validation_time,
            last_block_time: handoff.validation_time,
            ..BlockContext::default()
        },
        whitelist: None,
    };
    let (updated, outcome) = ledger.apply(&verified, &context);
    if !matches!(outcome, TransactionResult::Success(_)) {
        return Err(format!("{circuit} call application failed: {outcome:?}").into());
    }
    let contract = updated
        .contract
        .get(&address)
        .ok_or("deployed contract disappeared")?;
    check_state(contract)?;
    println!("{circuit} deployment and proven call validated and applied at address {address:?}");
    Ok(())
}

fn check_transaction<F>(
    root: &Path,
    circuit: &'static str,
    deploy: ContractDeploy<DefaultDB>,
    call: ContractCallPrototype<DefaultDB>,
    rng: &mut StdRng,
    check_state: F,
) -> Result<(), Box<dyn Error>>
where
    F: FnOnce(&ContractState<DefaultDB>) -> Result<(), Box<dyn Error>>,
{
    check_transaction_with_handoff(
        root,
        circuit,
        deploy,
        call,
        rng,
        None,
        None,
        &HandoffConfig::offline(),
        check_state,
    )
}

struct Secret;

impl witness_contract::Witnesses<u64> for Secret {
    fn secret(
        &self,
        context: WitnessContext<'_, u64, witness_contract::LedgerView<'_>>,
        seed: Field,
    ) -> (u64, Field) {
        let private = *context.private_state;
        let cell = context.ledger.cell().expect("witness Cell read");
        assert_eq!(cell, Field::from(if private == 7 { 0_u64 } else { 9_u64 }));
        (private + 1, seed + Field::from(private))
    }
}

struct NestedSecret;

type Uint248 = WideUint<{ (1_u128 << 120) - 1 }, { u128::MAX }>;

struct WideSecret;

impl wide_contract::Witnesses<u64> for WideSecret {
    fn nextWide(
        &self,
        context: WitnessContext<'_, u64, wide_contract::LedgerView<'_>>,
    ) -> (u64, Uint248) {
        (
            *context.private_state + 1,
            Uint248::from_le_bytes(&[0xff; 31]).expect("Uint<248> maximum is valid"),
        )
    }
}

struct TinySecret;

impl tiny_contract::Witnesses<()> for TinySecret {
    fn private_secret_key(
        &self,
        _context: WitnessContext<'_, (), tiny_contract::LedgerView<'_>>,
    ) -> ((), midnight_compact_runtime::FixedBytes<32>) {
        ((), midnight_compact_runtime::FixedBytes::new([7; 32]))
    }
}

impl expression_contract::Witnesses<u64> for NestedSecret {
    fn secret(
        &self,
        context: WitnessContext<'_, u64, expression_contract::LedgerView<'_>>,
    ) -> (u64, Field) {
        (*context.private_state + 1, Field::from(7_u64))
    }

    fn ordered(
        &self,
        context: WitnessContext<'_, u64, expression_contract::LedgerView<'_>>,
    ) -> (u64, Field) {
        let private = *context.private_state;
        (private + 1, Field::from(private))
    }
}

fn check_composite_cell_proof(root: &Path) -> Result<(), Box<dyn Error>> {
    let mut rng = StdRng::seed_from_u64(0x434f_4d50_4345_4c4c);
    let value = Pair {
        amount: Field::from(42_u64),
        active: true,
    };
    let initial = composite_cell_contract::initial_state(ConstructorContext::new(()))?;
    let deploy = make_deploy(
        root,
        "set_record",
        initial.ledger_state.get_ref().clone(),
        &mut rng,
    )?;
    let context = initial.into_circuit_context(deploy.address());
    let recorded = composite_cell_contract::recorded::set_record(context, value.clone())?;
    let call = check_generated_trace(root, "set_record", recorded, value.clone())?;
    check_transaction(root, "set_record", deploy, call, &mut rng, |state| {
        let StateValue::Array(fields) = state.data.get_ref() else {
            return Err("composite Cell state is not an array".into());
        };
        if read_cell::<Pair, _>(fields.get(0).ok_or("composite Cell missing")?)? != value {
            return Err("proven composite Cell write differs from expected value".into());
        }
        Ok(())
    })?;

    let initial = composite_cell_contract::initial_state(ConstructorContext::new(()))?;
    let deploy = make_deploy(
        root,
        "read_record",
        initial.ledger_state.get_ref().clone(),
        &mut rng,
    )?;
    let context = initial.into_circuit_context(deploy.address());
    let recorded = composite_cell_contract::recorded::read_record(context)?;
    if recorded.execution.result != Pair::default() {
        return Err("fresh composite Cell read differs from default".into());
    }
    let call = check_generated_trace(root, "read_record", recorded, ())?;
    check_transaction(root, "read_record", deploy, call, &mut rng, |state| {
        let StateValue::Array(fields) = state.data.get_ref() else {
            return Err("composite Cell read state is not an array".into());
        };
        if read_cell::<Pair, _>(fields.get(0).ok_or("composite Cell missing")?)? != Pair::default()
        {
            return Err("proven composite Cell read changed the state".into());
        }
        Ok(())
    })
}

fn check_boolean_observation_proof(root: &Path) -> Result<(), Box<dyn Error>> {
    let mut rng = StdRng::seed_from_u64(0x424f_4f4c_4f42_5301);
    for (circuit, flag_index) in [("check_set_empty", 0), ("check_map_empty", 1)] {
        let initial = set_size_contract::initial_state(ConstructorContext::new(()))?;
        let deploy = make_deploy(
            root,
            circuit,
            initial.ledger_state.get_ref().clone(),
            &mut rng,
        )?;
        let context = initial.into_circuit_context(deploy.address());
        let recorded = match circuit {
            "check_set_empty" => set_size_contract::recorded::check_set_empty(context)?,
            "check_map_empty" => set_size_contract::recorded::check_map_empty(context)?,
            _ => unreachable!(),
        };
        let call = check_generated_trace(root, circuit, recorded, ())?;
        check_transaction(root, circuit, deploy, call, &mut rng, |state| {
            let StateValue::Array(fields) = state.data.get_ref() else {
                return Err("Boolean observation state is not an array".into());
            };
            if !read_cell::<bool, _>(fields.get(flag_index).ok_or("flag Cell missing")?)? {
                return Err(format!("{circuit} proof did not store true").into());
            }
            Ok(())
        })?;
    }
    Ok(())
}

fn check_conditional_counter_proof(root: &Path) -> Result<(), Box<dyn Error>> {
    let mut rng = StdRng::seed_from_u64(0x434f_4e44_434e_5452);
    for (circuit, condition, flag, expected_counter) in [
        ("streamWrite", false, false, 2),
        ("streamWrite", true, false, 1),
        ("streamConstAnnotated", false, false, 2),
        ("streamConstAnnotated", false, true, 1),
        ("streamIncrement", false, false, 4),
        ("streamIncrement", false, true, 3),
    ] {
        let initial = conditional_counter_contract::initial_state(
            ConstructorContext::new(()),
            true,
            true,
            Field::from(111_u64),
        )?;
        let seed = initial.into_circuit_context(Default::default());
        let seed = if flag {
            seed.write_cell(0_u8, true)?.context
        } else {
            seed
        };
        let deploy = make_deploy(root, circuit, seed.query.state.get_ref().clone(), &mut rng)?;
        let context = midnight_compact_runtime::context::CircuitContext::from_contract_state(
            (),
            deploy.address(),
            &deploy.initial_state,
        );
        let call = if circuit == "streamWrite" {
            let recorded = conditional_counter_contract::recorded::streamWrite(
                context,
                condition,
                Field::from(777_u64),
            )?;
            check_generated_trace(root, circuit, recorded, (condition, Field::from(777_u64)))?
        } else if circuit == "streamConstAnnotated" {
            let recorded = conditional_counter_contract::recorded::streamConstAnnotated(context)?;
            check_generated_trace(root, circuit, recorded, ())?
        } else {
            let recorded = conditional_counter_contract::recorded::streamIncrement(context)?;
            check_generated_trace(root, circuit, recorded, ())?
        };
        check_transaction(root, circuit, deploy, call, &mut rng, |state| {
            let StateValue::Array(fields) = state.data.get_ref() else {
                return Err("conditional Counter state is not an array".into());
            };
            if read_cell::<bool, _>(fields.get(0).ok_or("flag Cell missing")?)? != flag {
                return Err(format!("{circuit} proof used the wrong ledger flag").into());
            }
            if circuit == "streamIncrement" {
                let wide = read_cell::<
                    midnight_compact_runtime::BoundedUint<18446744073709551615>,
                    _,
                >(fields.get(2).ok_or("Uint64 Cell missing")?)?;
                if wide.value() != if flag { 10 } else { 20 } {
                    return Err("streamIncrement proof stored the wrong Uint64 Cell".into());
                }
            } else {
                let expected_field: u64 = if circuit == "streamWrite" { 777 } else { 1 };
                if read_cell::<Field, _>(fields.get(1).ok_or("Field Cell missing")?)?
                    != Field::from(expected_field)
                {
                    return Err(format!("{circuit} proof stored the wrong Field").into());
                }
            }
            if read_counter(fields.get(4).ok_or("Counter missing")?)? != expected_counter {
                return Err(format!("{circuit} proof used the wrong Counter amount").into());
            }
            Ok(())
        })?;
        println!("{circuit} ledger flag={flag} counter={expected_counter} proof applied");
    }
    Ok(())
}

fn check_unsigned_sequential_proof(root: &Path) -> Result<(), Box<dyn Error>> {
    let circuit = "reset_and_set";
    let mut rng = StdRng::seed_from_u64(0x0052_4553_4554_5345);
    let initial = cross_circuit_contract::initial_state(ConstructorContext::new(()))?;
    let deploy = make_deploy(
        root,
        circuit,
        initial.ledger_state.get_ref().clone(),
        &mut rng,
    )?;
    let value = BoundedUint::<{ u64::MAX as u128 }>::new(7)?;
    let recorded = cross_circuit_contract::recorded::reset_and_set(
        initial.into_circuit_context(deploy.address()),
        value,
    )?;
    if !matches!(
        recorded.public.verify_ops(),
        [
            Op::Push { storage: false, .. },
            Op::Push { storage: true, .. },
            Op::Ins {
                cached: false,
                n: 1
            },
            Op::Push { storage: false, .. },
            Op::Push { storage: true, .. },
            Op::Ins {
                cached: false,
                n: 1
            },
        ]
    ) {
        return Err("reset_and_set did not record two ordered Cell writes".into());
    }
    let call = check_generated_trace(root, circuit, recorded, value)?;
    check_transaction(root, circuit, deploy, call, &mut rng, |state| {
        let written =
            read_cell_at_path::<BoundedUint<{ u64::MAX as u128 }>, _>(state.data.get_ref(), &[0])?;
        if written.value() != 7 {
            return Err("reset_and_set proof did not persist the second unsigned write".into());
        }
        Ok(())
    })?;
    println!("reset_and_set(7) ordered writes proved and applied");
    Ok(())
}

fn check_conditional_field_proof(root: &Path) -> Result<(), Box<dyn Error>> {
    let mut rng = StdRng::seed_from_u64(0x434f_4e44_4649_454c);
    for (condition, expected_field) in [(false, 0_u64), (true, 777_u64)] {
        let circuit = "walkerWrite";
        let initial = conditional_counter_contract::initial_state(
            ConstructorContext::new(()),
            true,
            true,
            Field::from(111_u64),
        )?;
        let deploy = make_deploy(
            root,
            circuit,
            initial.ledger_state.get_ref().clone(),
            &mut rng,
        )?;
        let context = initial.into_circuit_context(deploy.address());
        let input = (condition, Field::from(777_u64));
        let recorded =
            conditional_counter_contract::recorded::walkerWrite(context, input.0, input.1)?;
        let manual = check_generated_trace(root, circuit, recorded, input)?;
        let observed = ObservedContractState::new(
            deploy.address(),
            deploy.initial_state.clone(),
            Observation {
                transaction_hash: [0; 32],
                block_hash: [0; 32],
                block_height: 0,
            },
        );
        let verifier: VerifierKey = tagged_deserialize(&mut BufReader::new(File::open(
            root.join("keys/walkerWrite.verifier"),
        )?))?;
        let call = conditional_counter_contract::Contract::default()
            .recording
            .walkerWrite_call(&observed, (), input.0, input.1)?
            .prepare(verifier, Fr::from(0u64))?;
        if format!("{manual:?}") != format!("{call:?}") {
            return Err("walkerWrite typed observed call differs from the manual prototype".into());
        }
        check_transaction(root, circuit, deploy, call, &mut rng, |state| {
            let StateValue::Array(fields) = state.data.get_ref() else {
                return Err("conditional Field state is not an array".into());
            };
            let field = read_cell::<Field, _>(fields.get(1).ok_or("Field Cell missing")?)?;
            if field != Field::from(expected_field) {
                return Err("conditional Field proof stored the wrong branch value".into());
            }
            if read_counter(fields.get(4).ok_or("Counter missing")?)? != 0 {
                return Err("conditional Field proof changed the Counter".into());
            }
            Ok(())
        })?;
    }
    Ok(())
}

fn check_conditional_assert_eq_proof(root: &Path) -> Result<(), Box<dyn Error>> {
    let circuit = "streamAssertEq";
    let mut rng = StdRng::seed_from_u64(0x0099_4153_5345_5254);
    for flag in [false, true] {
        let initial = conditional_counter_contract::initial_state(
            ConstructorContext::new(()),
            true,
            true,
            Field::from(111_u64),
        )?;
        let seed = initial.into_circuit_context(Default::default());
        let seed = if flag {
            seed.write_cell(0_u8, true)?.context
        } else {
            seed
        };
        let deploy = make_deploy(root, circuit, seed.query.state.get_ref().clone(), &mut rng)?;
        let observed = ObservedContractState::new(
            deploy.address(),
            deploy.initial_state.clone(),
            Observation {
                transaction_hash: [0; 32],
                block_hash: [0; 32],
                block_height: 0,
            },
        );
        let recorded =
            conditional_counter_contract::recorded::streamAssertEq(observed.circuit_context(()))?;
        let expected_state = recorded.execution.context.query.state.get_ref().clone();
        let manual = check_generated_trace(root, circuit, recorded, ())?;
        let verifier: VerifierKey = tagged_deserialize(&mut BufReader::new(File::open(
            root.join("keys/streamAssertEq.verifier"),
        )?))?;
        let call = conditional_counter_contract::Contract::default()
            .recording
            .streamAssertEq_call(&observed, ())?
            .prepare(verifier, Fr::from(0_u64))?;
        if format!("{manual:?}") != format!("{call:?}") {
            return Err("streamAssertEq typed observed call differs from manual prototype".into());
        }
        check_transaction(root, circuit, deploy, call, &mut rng, |state| {
            let data = state.data.get_ref();
            if data != &expected_state {
                return Err("streamAssertEq proof changed unexpected ledger state".into());
            }
            if read_cell_at_path::<bool, _>(data, &[0])? != flag {
                return Err("streamAssertEq proof changed the flag".into());
            }
            if read_cell_at_path::<Field, _>(data, &[1])? != Field::from(1_u64) {
                return Err("streamAssertEq proof stored the wrong Field".into());
            }
            Ok(())
        })?;
    }
    println!("streamAssertEq both branches proved and applied through ledger-8");
    Ok(())
}

fn check_closed_unsigned_ternary_comparison_proof(root: &Path) -> Result<(), Box<dyn Error>> {
    let mut rng = StdRng::seed_from_u64(0x0113_434f_4d50_4152);
    for (circuit, condition, expected_field) in [
        ("walkerCompareEq", Some(true), 1_u64),
        ("walkerCompareEq", Some(false), 0_u64),
        ("streamCompareEq", None, 0_u64),
    ] {
        let initial = conditional_counter_contract::initial_state(
            ConstructorContext::new(()),
            true,
            true,
            Field::from(111_u64),
        )?;
        let deploy = make_deploy(
            root,
            circuit,
            initial.ledger_state.get_ref().clone(),
            &mut rng,
        )?;
        let observed = ObservedContractState::new(
            deploy.address(),
            deploy.initial_state.clone(),
            Observation {
                transaction_hash: [0; 32],
                block_hash: [0; 32],
                block_height: 0,
            },
        );
        let verifier: VerifierKey = tagged_deserialize(&mut BufReader::new(File::open(
            root.join(format!("keys/{circuit}.verifier")),
        )?))?;
        let generated = conditional_counter_contract::Contract::default();
        let (manual, typed, expected_state) = if let Some(condition) = condition {
            let x = BoundedUint::<255>::new(1)?;
            let recorded = conditional_counter_contract::recorded::walkerCompareEq(
                observed.circuit_context(()),
                condition,
                x,
            )?;
            let expected_state = recorded.execution.context.query.state.get_ref().clone();
            let manual = check_generated_trace(root, circuit, recorded, (condition, x))?;
            let typed = generated
                .recording
                .walkerCompareEq_call(&observed, (), condition, x)?
                .prepare(verifier, Fr::from(0_u64))?;
            (manual, typed, expected_state)
        } else {
            let recorded = conditional_counter_contract::recorded::streamCompareEq(
                observed.circuit_context(()),
            )?;
            let expected_state = recorded.execution.context.query.state.get_ref().clone();
            let manual = check_generated_trace(root, circuit, recorded, ())?;
            let typed = generated
                .recording
                .streamCompareEq_call(&observed, ())?
                .prepare(verifier, Fr::from(0_u64))?;
            (manual, typed, expected_state)
        };
        if format!("{manual:?}") != format!("{typed:?}") {
            return Err(format!("{circuit} typed observed call differs from manual trace").into());
        }
        check_transaction(root, circuit, deploy, typed, &mut rng, |state| {
            let data = state.data.get_ref();
            if data != &expected_state {
                return Err(format!("{circuit} proof changed unexpected ledger state").into());
            }
            if read_cell_at_path::<Field, _>(data, &[1])? != Field::from(expected_field) {
                return Err(format!("{circuit} proof stored the wrong comparison value").into());
            }
            Ok(())
        })?;
    }
    println!("closed unsigned ternary comparisons proved and applied through ledger-8");
    Ok(())
}

fn check_nested_uint64_cell_counter_proof(root: &Path) -> Result<(), Box<dyn Error>> {
    let mut rng = StdRng::seed_from_u64(0x0132_4e45_5354_4544);
    let circuit = "run";
    for choice in [true, false] {
        let initial = nested_stateful_ternary_contract::initial_state(ConstructorContext::new(()))?;
        let seed = initial.into_circuit_context(Default::default());
        let deploy = make_deploy(root, circuit, seed.query.state.get_ref().clone(), &mut rng)?;
        let observed = ObservedContractState::new(
            deploy.address(),
            deploy.initial_state.clone(),
            Observation {
                transaction_hash: [0; 32],
                block_hash: [0; 32],
                block_height: 0,
            },
        );
        let verifier: VerifierKey = tagged_deserialize(&mut BufReader::new(File::open(
            root.join(format!("keys/{circuit}.verifier")),
        )?))?;
        let recorded =
            nested_stateful_ternary_contract::recorded::run(observed.circuit_context(()), choice)?;
        let expected_state = recorded.execution.context.query.state.get_ref().clone();
        let manual = check_generated_trace(root, circuit, recorded, choice)?;
        let typed = nested_stateful_ternary_contract::Contract::default()
            .recording
            .run_call(&observed, (), choice)?
            .prepare(verifier, Fr::from(0_u64))?;
        if format!("{manual:?}") != format!("{typed:?}") {
            return Err(format!("{circuit} typed observed call differs from manual trace").into());
        }
        let expected = BoundedUint::<18446744073709551615>::new(if choice { 1 } else { 4 })?;
        check_transaction(root, circuit, deploy, typed, &mut rng, |state| {
            let data = state.data.get_ref();
            if data != &expected_state {
                return Err("nested Uint proof changed unexpected ledger state".into());
            }
            if read_cell_at_path::<BoundedUint<18446744073709551615>, _>(data, &[0])? != expected {
                return Err("nested Uint proof stored the wrong branch".into());
            }
            let StateValue::Array(fields) = data else {
                return Err("nested Uint state is not an array".into());
            };
            if read_counter(fields.get(1).ok_or("Counter missing")?)? != 1 {
                return Err("nested Uint proof changed the Counter".into());
            }
            Ok(())
        })?;
    }
    println!("nested Uint<4> to Uint<64> Cell and Counter proved and applied through ledger-8");
    Ok(())
}

fn check_nested_uint4_proof(root: &Path) -> Result<(), Box<dyn Error>> {
    let mut rng = StdRng::seed_from_u64(0x0121_4e45_5354_4544);
    for (circuit, args, flag, expected_field) in [
        ("walkerNestedIf", Some((true, true)), false, 1_u64),
        ("walkerNestedIf", Some((true, false)), false, 2_u64),
        ("walkerNestedIf", Some((false, true)), false, 3_u64),
        ("walkerNestedIf", Some((false, false)), false, 4_u64),
        ("streamNestedIf", None, false, 4_u64),
        ("streamNestedIf", None, true, 1_u64),
    ] {
        let initial = conditional_counter_contract::initial_state(
            ConstructorContext::new(()),
            true,
            true,
            Field::from(111_u64),
        )?;
        let seed = initial.into_circuit_context(Default::default());
        let seed = if flag {
            seed.write_cell(0_u8, true)?.context
        } else {
            seed
        };
        let deploy = make_deploy(root, circuit, seed.query.state.get_ref().clone(), &mut rng)?;
        let observed = ObservedContractState::new(
            deploy.address(),
            deploy.initial_state.clone(),
            Observation {
                transaction_hash: [0; 32],
                block_hash: [0; 32],
                block_height: 0,
            },
        );
        let verifier: VerifierKey = tagged_deserialize(&mut BufReader::new(File::open(
            root.join(format!("keys/{circuit}.verifier")),
        )?))?;
        let generated = conditional_counter_contract::Contract::default();
        let (manual, typed, expected_state) = if let Some((c, d)) = args {
            let recorded = conditional_counter_contract::recorded::walkerNestedIf(
                observed.circuit_context(()),
                c,
                d,
            )?;
            let expected_state = recorded.execution.context.query.state.get_ref().clone();
            let manual = check_generated_trace(root, circuit, recorded, (c, d))?;
            let typed = generated
                .recording
                .walkerNestedIf_call(&observed, (), c, d)?
                .prepare(verifier, Fr::from(0_u64))?;
            (manual, typed, expected_state)
        } else {
            let recorded = conditional_counter_contract::recorded::streamNestedIf(
                observed.circuit_context(()),
            )?;
            let expected_state = recorded.execution.context.query.state.get_ref().clone();
            let manual = check_generated_trace(root, circuit, recorded, ())?;
            let typed = generated
                .recording
                .streamNestedIf_call(&observed, ())?
                .prepare(verifier, Fr::from(0_u64))?;
            (manual, typed, expected_state)
        };
        if format!("{manual:?}") != format!("{typed:?}") {
            return Err(format!("{circuit} typed observed call differs from manual trace").into());
        }
        check_transaction(root, circuit, deploy, typed, &mut rng, |state| {
            let data = state.data.get_ref();
            if data != &expected_state {
                return Err(format!("{circuit} proof changed unexpected ledger state").into());
            }
            if read_cell_at_path::<bool, _>(data, &[0])? != flag {
                return Err(format!("{circuit} proof changed the flag").into());
            }
            if read_cell_at_path::<Field, _>(data, &[1])? != Field::from(expected_field) {
                return Err(format!("{circuit} proof stored the wrong nested branch").into());
            }
            let expected_count = if args.is_some() { 0 } else { 1 };
            let StateValue::Array(fields) = data else {
                return Err("nested Uint state is not an array".into());
            };
            if read_counter(fields.get(4).ok_or("Counter missing")?)? != expected_count {
                return Err(format!("{circuit} proof stored the wrong Counter").into());
            }
            Ok(())
        })?;
    }
    println!("nested Uint<4> conditional branches proved and applied through ledger-8");
    Ok(())
}

fn check_conditional_field_vector_proof(root: &Path) -> Result<(), Box<dyn Error>> {
    let mut rng = StdRng::seed_from_u64(0x0131_5645_4354_4f52);
    let circuit = "streamVectorElement";
    for flag in [false, true] {
        let initial = conditional_counter_contract::initial_state(
            ConstructorContext::new(()),
            true,
            true,
            Field::from(111_u64),
        )?;
        let seed = initial.into_circuit_context(Default::default());
        let seed = if flag {
            seed.write_cell(0_u8, true)?.context
        } else {
            seed
        };
        let deploy = make_deploy(root, circuit, seed.query.state.get_ref().clone(), &mut rng)?;
        let observed = ObservedContractState::new(
            deploy.address(),
            deploy.initial_state.clone(),
            Observation {
                transaction_hash: [0; 32],
                block_hash: [0; 32],
                block_height: 0,
            },
        );
        let verifier: VerifierKey = tagged_deserialize(&mut BufReader::new(File::open(
            root.join(format!("keys/{circuit}.verifier")),
        )?))?;
        let recorded = conditional_counter_contract::recorded::streamVectorElement(
            observed.circuit_context(()),
        )?;
        let expected_state = recorded.execution.context.query.state.get_ref().clone();
        let manual = check_generated_trace(root, circuit, recorded, ())?;
        let typed = conditional_counter_contract::Contract::default()
            .recording
            .streamVectorElement_call(&observed, ())?
            .prepare(verifier, Fr::from(0_u64))?;
        if format!("{manual:?}") != format!("{typed:?}") {
            return Err(format!("{circuit} typed observed call differs from manual trace").into());
        }
        let expected_vector = FixedVector::new([
            Field::from(if flag { 1_u64 } else { 2_u64 }),
            Field::from(if flag { 3_u64 } else { 4_u64 }),
        ]);
        check_transaction(root, circuit, deploy, typed, &mut rng, |state| {
            let data = state.data.get_ref();
            if data != &expected_state {
                return Err(format!("{circuit} proof changed unexpected ledger state").into());
            }
            if read_cell_at_path::<bool, _>(data, &[0])? != flag {
                return Err(format!("{circuit} proof changed the flag").into());
            }
            if read_cell_at_path::<FixedVector<Field, 2>, _>(data, &[3])? != expected_vector {
                return Err(format!("{circuit} proof stored the wrong Field vector").into());
            }
            let StateValue::Array(fields) = data else {
                return Err("conditional vector state is not an array".into());
            };
            if read_counter(fields.get(4).ok_or("Counter missing")?)? != 0 {
                return Err(format!("{circuit} proof changed the Counter").into());
            }
            Ok(())
        })?;
    }
    println!("closed conditional Field vectors proved and applied through ledger-8");
    Ok(())
}

fn check_annotated_uint8_proof(root: &Path) -> Result<(), Box<dyn Error>> {
    let mut rng = StdRng::seed_from_u64(0x0127_5549_4e54_3038);
    for condition in [true, false] {
        let circuit = "walkerConstAnnotated";
        let initial = conditional_counter_contract::initial_state(
            ConstructorContext::new(()),
            true,
            true,
            Field::from(111_u64),
        )?;
        let seed = initial.into_circuit_context(Default::default());
        let deploy = make_deploy(root, circuit, seed.query.state.get_ref().clone(), &mut rng)?;
        let observed = ObservedContractState::new(
            deploy.address(),
            deploy.initial_state.clone(),
            Observation {
                transaction_hash: [0; 32],
                block_hash: [0; 32],
                block_height: 0,
            },
        );
        let verifier: VerifierKey = tagged_deserialize(&mut BufReader::new(File::open(
            root.join(format!("keys/{circuit}.verifier")),
        )?))?;
        let recorded = conditional_counter_contract::recorded::walkerConstAnnotated(
            observed.circuit_context(()),
            condition,
        )?;
        let expected_state = recorded.execution.context.query.state.get_ref().clone();
        let manual = check_generated_trace(root, circuit, recorded, condition)?;
        let typed = conditional_counter_contract::Contract::default()
            .recording
            .walkerConstAnnotated_call(&observed, (), condition)?
            .prepare(verifier, Fr::from(0_u64))?;
        if format!("{manual:?}") != format!("{typed:?}") {
            return Err(format!("{circuit} typed observed call differs from manual trace").into());
        }
        let expected_field = Field::from(if condition { 1_u64 } else { 2_u64 });
        check_transaction(root, circuit, deploy, typed, &mut rng, |state| {
            let data = state.data.get_ref();
            if data != &expected_state {
                return Err(format!("{circuit} proof changed unexpected ledger state").into());
            }
            if read_cell_at_path::<bool, _>(data, &[0])? {
                return Err(format!("{circuit} proof changed the flag").into());
            }
            if read_cell_at_path::<Field, _>(data, &[1])? != expected_field {
                return Err(format!("{circuit} proof stored the wrong Uint branch").into());
            }
            let StateValue::Array(fields) = data else {
                return Err("annotated Uint state is not an array".into());
            };
            if read_counter(fields.get(4).ok_or("Counter missing")?)? != 0 {
                return Err(format!("{circuit} proof changed the Counter").into());
            }
            Ok(())
        })?;
    }
    println!("annotated Uint<8> conditional branches proved and applied through ledger-8");
    Ok(())
}

fn check_closed_curve_argument_proof(root: &Path) -> Result<(), Box<dyn Error>> {
    let mut rng = StdRng::seed_from_u64(0x0126_4355_5256_4541);
    for (circuit, condition, flag) in [
        ("walkerNativeArg", Some(true), false),
        ("walkerNativeArg", Some(false), false),
        ("streamNativeArg", None, false),
        ("streamNativeArg", None, true),
    ] {
        let initial = conditional_counter_contract::initial_state(
            ConstructorContext::new(()),
            true,
            true,
            Field::from(111_u64),
        )?;
        let seed = initial.into_circuit_context(Default::default());
        let seed = if flag {
            seed.write_cell(0_u8, true)?.context
        } else {
            seed
        };
        let deploy = make_deploy(root, circuit, seed.query.state.get_ref().clone(), &mut rng)?;
        let observed = ObservedContractState::new(
            deploy.address(),
            deploy.initial_state.clone(),
            Observation {
                transaction_hash: [0; 32],
                block_hash: [0; 32],
                block_height: 0,
            },
        );
        let verifier: VerifierKey = tagged_deserialize(&mut BufReader::new(File::open(
            root.join(format!("keys/{circuit}.verifier")),
        )?))?;
        let generated = conditional_counter_contract::Contract::default();
        let (manual, typed, expected_state) = if let Some(condition) = condition {
            let recorded = conditional_counter_contract::recorded::walkerNativeArg(
                observed.circuit_context(()),
                condition,
            )?;
            let expected_state = recorded.execution.context.query.state.get_ref().clone();
            let manual = check_generated_trace(root, circuit, recorded, condition)?;
            let typed = generated
                .recording
                .walkerNativeArg_call(&observed, (), condition)?
                .prepare(verifier, Fr::from(0_u64))?;
            (manual, typed, expected_state)
        } else {
            let recorded = conditional_counter_contract::recorded::streamNativeArg(
                observed.circuit_context(()),
            )?;
            let expected_state = recorded.execution.context.query.state.get_ref().clone();
            let manual = check_generated_trace(root, circuit, recorded, ())?;
            let typed = generated
                .recording
                .streamNativeArg_call(&observed, ())?
                .prepare(verifier, Fr::from(0_u64))?;
            (manual, typed, expected_state)
        };
        if format!("{manual:?}") != format!("{typed:?}") {
            return Err(format!("{circuit} typed observed call differs from manual trace").into());
        }
        let chosen = if condition.unwrap_or(flag) {
            1_u64
        } else {
            2_u64
        };
        let expected_x = midnight_compact_runtime::jubjub_point_x(
            midnight_compact_runtime::hash_to_curve(Field::from(chosen)),
        );
        check_transaction(root, circuit, deploy, typed, &mut rng, |state| {
            let data = state.data.get_ref();
            if data != &expected_state {
                return Err(format!("{circuit} proof changed unexpected ledger state").into());
            }
            if read_cell_at_path::<bool, _>(data, &[0])? != flag {
                return Err(format!("{circuit} proof changed the flag").into());
            }
            if read_cell_at_path::<Field, _>(data, &[1])? != expected_x {
                return Err(format!("{circuit} proof stored the wrong curve X coordinate").into());
            }
            let StateValue::Array(fields) = data else {
                return Err("closed curve argument state is not an array".into());
            };
            if read_counter(fields.get(4).ok_or("Counter missing")?)? != 0 {
                return Err(format!("{circuit} proof changed the Counter").into());
            }
            Ok(())
        })?;
    }
    println!("closed conditional curve arguments proved and applied through ledger-8");
    Ok(())
}

fn check_closed_ternary_struct_member_proof(root: &Path) -> Result<(), Box<dyn Error>> {
    let mut rng = StdRng::seed_from_u64(0x0115_5354_5255_4354);
    for (circuit, condition, expected_field) in [
        ("walkerStructMember", Some(true), 1_u64),
        ("walkerStructMember", Some(false), 2_u64),
        ("streamStructMember", None, 2_u64),
    ] {
        let initial = conditional_counter_contract::initial_state(
            ConstructorContext::new(()),
            true,
            true,
            Field::from(111_u64),
        )?;
        let deploy = make_deploy(
            root,
            circuit,
            initial.ledger_state.get_ref().clone(),
            &mut rng,
        )?;
        let observed = ObservedContractState::new(
            deploy.address(),
            deploy.initial_state.clone(),
            Observation {
                transaction_hash: [0; 32],
                block_hash: [0; 32],
                block_height: 0,
            },
        );
        let verifier: VerifierKey = tagged_deserialize(&mut BufReader::new(File::open(
            root.join(format!("keys/{circuit}.verifier")),
        )?))?;
        let generated = conditional_counter_contract::Contract::default();
        let (manual, typed, expected_state) = if let Some(condition) = condition {
            let recorded = conditional_counter_contract::recorded::walkerStructMember(
                observed.circuit_context(()),
                condition,
            )?;
            let expected_state = recorded.execution.context.query.state.get_ref().clone();
            let manual = check_generated_trace(root, circuit, recorded, condition)?;
            let typed = generated
                .recording
                .walkerStructMember_call(&observed, (), condition)?
                .prepare(verifier, Fr::from(0_u64))?;
            (manual, typed, expected_state)
        } else {
            let recorded = conditional_counter_contract::recorded::streamStructMember(
                observed.circuit_context(()),
            )?;
            let expected_state = recorded.execution.context.query.state.get_ref().clone();
            let manual = check_generated_trace(root, circuit, recorded, ())?;
            let typed = generated
                .recording
                .streamStructMember_call(&observed, ())?
                .prepare(verifier, Fr::from(0_u64))?;
            (manual, typed, expected_state)
        };
        if format!("{manual:?}") != format!("{typed:?}") {
            return Err(format!("{circuit} typed observed call differs from manual trace").into());
        }
        check_transaction(root, circuit, deploy, typed, &mut rng, |state| {
            let data = state.data.get_ref();
            if data != &expected_state {
                return Err(format!("{circuit} proof changed unexpected ledger state").into());
            }
            if read_cell_at_path::<Field, _>(data, &[1])? != Field::from(expected_field) {
                return Err(format!("{circuit} proof stored the wrong struct member").into());
            }
            Ok(())
        })?;
    }
    println!("closed ternary struct members proved and applied through ledger-8");
    Ok(())
}

fn check_conditional_set_proof(root: &Path) -> Result<(), Box<dyn Error>> {
    let circuit = "choose";
    let mut rng = StdRng::seed_from_u64(0x0103_4348_4f4f_5345);
    for (name, value, insert, seed_true) in [
        ("insertTrue", true, true, false),
        ("removeTrue", true, false, true),
        ("insertFalse", false, true, false),
    ] {
        let initial = set_contract::initial_state(ConstructorContext::new(()))?;
        let state = if seed_true {
            set_contract::add(initial.into_circuit_context(Default::default()), true)?
                .context
                .query
                .state
                .get_ref()
                .clone()
        } else {
            initial.ledger_state.get_ref().clone()
        };
        let deploy = make_deploy(root, circuit, state, &mut rng)?;
        let observed = ObservedContractState::new(
            deploy.address(),
            deploy.initial_state.clone(),
            Observation {
                transaction_hash: [0; 32],
                block_hash: [0; 32],
                block_height: 0,
            },
        );
        let recorded = set_contract::recorded::choose(observed.circuit_context(()), value, insert)?;
        if recorded.execution.result != insert {
            return Err(format!("{name}: wrong recorded membership result").into());
        }
        let expected_state = recorded.execution.context.query.state.get_ref().clone();
        let manual = check_generated_trace(root, circuit, recorded, (value, insert))?;
        let verifier: VerifierKey = tagged_deserialize(&mut BufReader::new(File::open(
            root.join("keys/choose.verifier"),
        )?))?;
        let call = set_contract::Contract::default()
            .recording
            .choose_call(&observed, (), value, insert)?
            .prepare(verifier, Fr::from(0_u64))?;
        if format!("{manual:?}") != format!("{call:?}") {
            return Err(
                format!("{name}: typed observed call differs from manual prototype").into(),
            );
        }
        check_transaction(root, circuit, deploy, call, &mut rng, |state| {
            let data = state.data.get_ref();
            if data != &expected_state {
                return Err(format!("{name}: proof changed unexpected ledger state").into());
            }
            if set_view_at_path::<bool, _>(data, &[0])?.member(value) != insert {
                return Err(format!("{name}: proven Set membership differs from branch").into());
            }
            Ok(())
        })?;
    }
    println!("choose three branches proved and applied through ledger-8");
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    // Ledger proof construction can exceed the macOS main-thread stack.
    // Give every selector the same explicit budget, including direct paths.
    let proof = std::thread::Builder::new()
        .name("compact-proof-smoke".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(|| run().map_err(|error| error.to_string()))?;
    match proof.join() {
        Ok(Ok(())) => Ok(()),
        Ok(Err(error)) => Err(error.into()),
        Err(_) => Err("proof-smoke dispatcher panicked".into()),
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let mut arguments = env::args_os().skip(1);
    let first = arguments.next();
    if let Some(result) = direct_dispatch::try_run(first.as_deref(), &mut arguments) {
        return result;
    }
    positional_suite::run(positional_inputs::PositionalInputs::parse(
        first,
        &mut arguments,
    )?)
}
