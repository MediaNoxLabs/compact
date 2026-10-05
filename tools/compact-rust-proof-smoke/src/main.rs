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
mod adt_set_vector;
mod asset_custody_grant;
mod asset_freshness;
mod asset_grant_effective;
mod asset_record_write;
mod asset_removal;
mod asset_stored_record_fresh;
mod asset_watch_write;
mod asset_writable;
mod bboard;
mod boolean_pair_assert;
mod closed_pure_field;
mod counter_less_than;
mod election_commit;
#[path = "../../../tests-rust-backend/election-oracle/support/commit.rs"]
mod election_membership_support;
mod election_reveal;
mod election_topic;
mod field_pair_hash;
mod field_to_bytes32;
mod guarded_recording;
mod impure_field_helper;
mod inline_type_scope;
mod let_return;
mod literal_bytes_field;
mod merkle_hash;
mod merkle_indexed;
mod merkle_reset;
mod merkle_root;
mod merkle_verify;
mod mixed_width_recording;
mod opaque_string_map;
mod opaque_string_set;
mod pair_hash_cell;
mod persistent_commit;
mod pure_assert_call;
mod pure_field_arguments;
mod schnorr_attestation;
mod stateful_pair_hash;
mod stateful_pure_return;
mod struct_constructor_cell;
mod test_center_counter;
mod test_center_welcome;
mod unsigned_recording;
mod unused_field_reads;
mod vector_map;
mod witness_assert;
mod witness_vector_action;
mod witness_vector_let;
mod zerocash_mint;
mod zerocash_spend;

use std::env;
use std::error::Error;
use std::ffi::OsStr;
use std::fs::{self, File};
use std::io::{self, BufReader};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use compact_rust_cell_boolean_fixture::ledger_contract as cell_contract;
use compact_rust_cell_read_fixture::ledger_contract as cell_read_contract;
use compact_rust_cell_struct_fixture::ledger_contract as composite_cell_contract;
use compact_rust_cell_struct_fixture::types::Pair;
use compact_rust_chunked_cell_fixture::ledger_contract as chunked_cell_contract;
use compact_rust_chunked_list_fixture::ledger_contract as chunked_list_contract;
use compact_rust_chunked_map_fixture::ledger_contract as chunked_map_contract;
use compact_rust_chunked_set_observed_fixture::ledger_contract as chunked_set_contract;
use compact_rust_constructor_list_actions_fixture::ledger_contract as constructor_list_contract;
use compact_rust_constructor_map_actions_fixture::ledger_contract as constructor_map_contract;
use compact_rust_counter_fixture::ledger_contract as counter_contract;
use compact_rust_counter_parameter_fixture::ledger_contract as counter_parameter_contract;
use compact_rust_hmt_default_oracle_fixture::ledger_contract as historic_merkle_default_contract;
use compact_rust_hmt_insert_oracle_fixture::ledger_contract as historic_merkle_contract;
use compact_rust_list_field_fixture::ledger_contract as list_contract;
use compact_rust_map_boolean_field_fixture::ledger_contract as map_contract;
use compact_rust_merkle_tree_oracle_fixture::ledger_contract as merkle_contract;
use compact_rust_nested_map_shape_fixture::ledger_contract as nested_map_shape_contract;
use compact_rust_nested_stateful_ternary_fixture::ledger_contract as nested_stateful_ternary_contract;
use compact_rust_nested_witness_call_oracle_fixture::ledger_contract as expression_contract;
use compact_rust_observed_composite_keys_fixture::ledger_contract as composite_key_contract;
use compact_rust_observed_composite_keys_fixture::types::CompositeKey;
use compact_rust_recorded_enum_cell_fixture::ledger_contract as enum_cell_contract;
use compact_rust_recorded_enum_cell_fixture::types::Choice;
use compact_rust_set_boolean_fixture::ledger_contract as set_contract;
use compact_rust_set_oracle_fixture::ledger_contract as set_oracle_contract;
use compact_rust_set_size_oracle_fixture::ledger_contract as set_size_contract;
use compact_rust_stateful_circuit_call_fixture::ledger_contract as nested_contract;
use compact_rust_ternary_cond_oracle_fixture::ledger_contract as conditional_counter_contract;
use compact_rust_tiny_oracle_fixture::ledger_contract as tiny_contract;
use compact_rust_uints_oracle_fixture::ledger_contract as uints_contract;
use compact_rust_vector_key_adt_fixture::ledger_contract as vector_key_contract;
use compact_rust_wide_uint_oracle_fixture::ledger_contract as wide_contract;
use compact_rust_witness_cell_write_fixture::ledger_contract as witness_contract;
use compact_rust_witness_list_shapes_fixture::ledger_contract as list_shapes_contract;
use compact_rust_witness_list_shapes_fixture::types::{Choice as ListChoice, Packet};
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
    CallSpec, Observation, ObservedCallError, ObservedContractState, RecordedCall,
    decode_verifier_key, prepare_call,
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
    for (circuit, condition, expected_counter) in [
        ("streamWrite", false, 2),
        ("streamWrite", true, 1),
        ("streamConstAnnotated", false, 2),
        ("streamIncrement", false, 4),
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
        let context = initial.into_circuit_context(deploy.address());
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
            if circuit == "streamIncrement" {
                let wide = read_cell::<
                    midnight_compact_runtime::BoundedUint<18446744073709551615>,
                    _,
                >(fields.get(2).ok_or("Uint64 Cell missing")?)?;
                if wide.value() != 20 {
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
    }
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
    if first.as_deref() == Some(OsStr::new("--mixed-width-recording")) {
        let root = arguments
            .next()
            .ok_or("usage: compact-rust-proof-smoke --mixed-width-recording <proof-output>")?;
        if arguments.next().is_some() {
            return Err(
                "usage: compact-rust-proof-smoke --mixed-width-recording <proof-output>".into(),
            );
        }
        let proof = std::thread::Builder::new()
            .name("mixed-width-recording-proof".into())
            .stack_size(64 * 1024 * 1024)
            .spawn(move || {
                mixed_width_recording::run(Path::new(&root)).map_err(|e| e.to_string())
            })?;
        return match proof.join() {
            Ok(Ok(())) => Ok(()),
            Ok(Err(error)) => Err(error.into()),
            Err(_) => Err("mixed-width recording proof thread panicked".into()),
        };
    }
    if first.as_deref() == Some(OsStr::new("--wide-unsigned-field-cast")) {
        let root = arguments
            .next()
            .ok_or("usage: compact-rust-proof-smoke --wide-unsigned-field-cast <proof-output>")?;
        if arguments.next().is_some() {
            return Err(
                "usage: compact-rust-proof-smoke --wide-unsigned-field-cast <proof-output>".into(),
            );
        }
        return unsigned_recording::wide_cast(Path::new(&root));
    }
    if first.as_deref() == Some(OsStr::new("--closed-pure-unsigned-call")) {
        let root = arguments
            .next()
            .ok_or("usage: compact-rust-proof-smoke --closed-pure-unsigned-call <proof-output>")?;
        if arguments.next().is_some() {
            return Err(
                "usage: compact-rust-proof-smoke --closed-pure-unsigned-call <proof-output>".into(),
            );
        }
        return unsigned_recording::widening_call(Path::new(&root));
    }
    if first.as_deref() == Some(OsStr::new("--test-center-counter")) {
        let root = arguments
            .next()
            .ok_or("usage: compact-rust-proof-smoke --test-center-counter <proof-output>")?;
        if arguments.next().is_some() {
            return Err(
                "usage: compact-rust-proof-smoke --test-center-counter <proof-output>".into(),
            );
        }
        return test_center_counter::run(Path::new(&root));
    }
    if first.as_deref() == Some(OsStr::new("--test-center-welcome")) {
        let root = arguments
            .next()
            .ok_or("usage: compact-rust-proof-smoke --test-center-welcome <proof-output>")?;
        if arguments.next().is_some() {
            return Err(
                "usage: compact-rust-proof-smoke --test-center-welcome <proof-output>".into(),
            );
        }
        return test_center_welcome::run(Path::new(&root));
    }
    if first.as_deref() == Some(OsStr::new("--opaque-string-set")) {
        let root = arguments
            .next()
            .ok_or("usage: compact-rust-proof-smoke --opaque-string-set <proof-output>")?;
        if arguments.next().is_some() {
            return Err(
                "usage: compact-rust-proof-smoke --opaque-string-set <proof-output>".into(),
            );
        }
        return opaque_string_set::run(Path::new(&root));
    }
    if first.as_deref() == Some(OsStr::new("--opaque-string-map")) {
        let root = arguments
            .next()
            .ok_or("usage: compact-rust-proof-smoke --opaque-string-map <proof-output>")?;
        if arguments.next().is_some() {
            return Err(
                "usage: compact-rust-proof-smoke --opaque-string-map <proof-output>".into(),
            );
        }
        return opaque_string_map::run(Path::new(&root));
    }
    if first.as_deref() == Some(OsStr::new("--conditional-set")) {
        let root = arguments
            .next()
            .ok_or("usage: compact-rust-proof-smoke --conditional-set <proof-output>")?;
        if arguments.next().is_some() {
            return Err("usage: compact-rust-proof-smoke --conditional-set <proof-output>".into());
        }
        return check_conditional_set_proof(Path::new(&root));
    }
    if first.as_deref() == Some(OsStr::new("--adt-set-enum")) {
        let root = arguments
            .next()
            .ok_or("usage: compact-rust-proof-smoke --adt-set-enum <proof-output>")?;
        if arguments.next().is_some() {
            return Err("usage: compact-rust-proof-smoke --adt-set-enum <proof-output>".into());
        }
        return adt_set_enum::run(Path::new(&root));
    }
    if first.as_deref() == Some(OsStr::new("--guarded-recording")) {
        let root = arguments
            .next()
            .ok_or("usage: compact-rust-proof-smoke --guarded-recording <proof-output>")?;
        if arguments.next().is_some() {
            return Err(
                "usage: compact-rust-proof-smoke --guarded-recording <proof-output>".into(),
            );
        }
        return guarded_recording::run(Path::new(&root));
    }
    if first.as_deref() == Some(OsStr::new("--asset-freshness")) {
        let root = arguments
            .next()
            .ok_or("usage: compact-rust-proof-smoke --asset-freshness <proof-output>")?;
        if arguments.next().is_some() {
            return Err("usage: compact-rust-proof-smoke --asset-freshness <proof-output>".into());
        }
        return asset_freshness::run(Path::new(&root));
    }
    if first.as_deref() == Some(OsStr::new("--asset-writable")) {
        let root = arguments
            .next()
            .ok_or("usage: compact-rust-proof-smoke --asset-writable <proof-output>")?;
        if arguments.next().is_some() {
            return Err("usage: compact-rust-proof-smoke --asset-writable <proof-output>".into());
        }
        return asset_writable::run(Path::new(&root));
    }
    if first.as_deref() == Some(OsStr::new("--asset-custody-grant")) {
        let root = arguments
            .next()
            .ok_or("usage: compact-rust-proof-smoke --asset-custody-grant <proof-output>")?;
        if arguments.next().is_some() {
            return Err(
                "usage: compact-rust-proof-smoke --asset-custody-grant <proof-output>".into(),
            );
        }
        return asset_custody_grant::run(Path::new(&root));
    }
    if first.as_deref() == Some(OsStr::new("--asset-record-write")) {
        let root = arguments
            .next()
            .ok_or("usage: compact-rust-proof-smoke --asset-record-write <proof-output>")?;
        if arguments.next().is_some() {
            return Err(
                "usage: compact-rust-proof-smoke --asset-record-write <proof-output>".into(),
            );
        }
        return asset_record_write::run(Path::new(&root));
    }
    if first.as_deref() == Some(OsStr::new("--schnorr-attestation")) {
        let root = arguments
            .next()
            .ok_or("usage: compact-rust-proof-smoke --schnorr-attestation <proof-output>")?;
        if arguments.next().is_some() {
            return Err(
                "usage: compact-rust-proof-smoke --schnorr-attestation <proof-output>".into(),
            );
        }
        return schnorr_attestation::run(Path::new(&root));
    }
    if first.as_deref() == Some(OsStr::new("--asset-watch-write")) {
        let root = arguments
            .next()
            .ok_or("usage: compact-rust-proof-smoke --asset-watch-write <proof-output>")?;
        if arguments.next().is_some() {
            return Err(
                "usage: compact-rust-proof-smoke --asset-watch-write <proof-output>".into(),
            );
        }
        return asset_watch_write::run(Path::new(&root));
    }
    if first.as_deref() == Some(OsStr::new("--asset-removal")) {
        let root = arguments
            .next()
            .ok_or("usage: compact-rust-proof-smoke --asset-removal <proof-output>")?;
        if arguments.next().is_some() {
            return Err("usage: compact-rust-proof-smoke --asset-removal <proof-output>".into());
        }
        return asset_removal::run(Path::new(&root));
    }
    if first.as_deref() == Some(OsStr::new("--asset-grant-effective")) {
        let root = arguments
            .next()
            .ok_or("usage: compact-rust-proof-smoke --asset-grant-effective <proof-output>")?;
        if arguments.next().is_some() {
            return Err(
                "usage: compact-rust-proof-smoke --asset-grant-effective <proof-output>".into(),
            );
        }
        return asset_grant_effective::run(Path::new(&root));
    }
    if first.as_deref() == Some(OsStr::new("--asset-stored-record-fresh")) {
        let root = arguments
            .next()
            .ok_or("usage: compact-rust-proof-smoke --asset-stored-record-fresh <proof-output>")?;
        if arguments.next().is_some() {
            return Err(
                "usage: compact-rust-proof-smoke --asset-stored-record-fresh <proof-output>".into(),
            );
        }
        return asset_stored_record_fresh::run(Path::new(&root));
    }
    if first.as_deref() == Some(OsStr::new("--adt-set-vector")) {
        let root = arguments
            .next()
            .ok_or("usage: compact-rust-proof-smoke --adt-set-vector <proof-output>")?;
        if arguments.next().is_some() {
            return Err("usage: compact-rust-proof-smoke --adt-set-vector <proof-output>".into());
        }
        return adt_set_vector::run(Path::new(&root));
    }
    if first.as_deref() == Some(OsStr::new("--adt-list-vector-field-4")) {
        let root = arguments
            .next()
            .ok_or("usage: compact-rust-proof-smoke --adt-list-vector-field-4 <proof-output>")?;
        if arguments.next().is_some() {
            return Err(
                "usage: compact-rust-proof-smoke --adt-list-vector-field-4 <proof-output>".into(),
            );
        }
        return adt_list_vector_field_4::run(Path::new(&root));
    }
    if first.as_deref() == Some(OsStr::new("--adt-list-bytes")) {
        let root = arguments
            .next()
            .ok_or("usage: compact-rust-proof-smoke --adt-list-bytes <proof-output>")?;
        if arguments.next().is_some() {
            return Err("usage: compact-rust-proof-smoke --adt-list-bytes <proof-output>".into());
        }
        return adt_list_bytes::run(Path::new(&root));
    }
    if first.as_deref() == Some(OsStr::new("--adt-list-enum")) {
        let root = arguments
            .next()
            .ok_or("usage: compact-rust-proof-smoke --adt-list-enum <proof-output>")?;
        if arguments.next().is_some() {
            return Err("usage: compact-rust-proof-smoke --adt-list-enum <proof-output>".into());
        }
        return adt_list_enum::run(Path::new(&root));
    }
    if first.as_deref() == Some(OsStr::new("--adt-list-field")) {
        let root = arguments
            .next()
            .ok_or("usage: compact-rust-proof-smoke --adt-list-field <proof-output>")?;
        if arguments.next().is_some() {
            return Err("usage: compact-rust-proof-smoke --adt-list-field <proof-output>".into());
        }
        return adt_list_field::run(Path::new(&root));
    }
    if first.as_deref() == Some(OsStr::new("--closed-pure-field")) {
        let root = arguments
            .next()
            .ok_or("usage: compact-rust-proof-smoke --closed-pure-field <proof-output>")?;
        if arguments.next().is_some() {
            return Err(
                "usage: compact-rust-proof-smoke --closed-pure-field <proof-output>".into(),
            );
        }
        return closed_pure_field::run(Path::new(&root));
    }
    if first.as_deref() == Some(OsStr::new("--struct-constructor-cell")) {
        let root = arguments
            .next()
            .ok_or("usage: compact-rust-proof-smoke --struct-constructor-cell <proof-output>")?;
        if arguments.next().is_some() {
            return Err(
                "usage: compact-rust-proof-smoke --struct-constructor-cell <proof-output>".into(),
            );
        }
        return struct_constructor_cell::run(Path::new(&root));
    }
    if first.as_deref() == Some(OsStr::new("--pure-assert-call")) {
        let pure_root = arguments.next().ok_or(
            "usage: compact-rust-proof-smoke --pure-assert-call <pure-call-proof> <assert-proof>",
        )?;
        let assert_root = arguments.next().ok_or(
            "usage: compact-rust-proof-smoke --pure-assert-call <pure-call-proof> <assert-proof>",
        )?;
        if arguments.next().is_some() {
            return Err(
                "usage: compact-rust-proof-smoke --pure-assert-call <pure-call-proof> <assert-proof>"
                    .into(),
            );
        }
        return pure_assert_call::run(Path::new(&pure_root), Path::new(&assert_root));
    }
    if first.as_deref() == Some(OsStr::new("--field-pair-hash")) {
        let root = arguments
            .next()
            .ok_or("usage: compact-rust-proof-smoke --field-pair-hash <proof-output>")?;
        if arguments.next().is_some() {
            return Err("usage: compact-rust-proof-smoke --field-pair-hash <proof-output>".into());
        }
        let proof = std::thread::Builder::new()
            .name("field-pair-hash-proof".into())
            .stack_size(64 * 1024 * 1024)
            .spawn(move || field_pair_hash::run(Path::new(&root)).map_err(|e| e.to_string()))?;
        return match proof.join() {
            Ok(Ok(())) => Ok(()),
            Ok(Err(error)) => Err(error.into()),
            Err(_) => Err("field pair hash proof thread panicked".into()),
        };
    }
    if first.as_deref() == Some(OsStr::new("--impure-field-helper")) {
        let root = arguments
            .next()
            .ok_or("usage: compact-rust-proof-smoke --impure-field-helper <proof-output>")?;
        if arguments.next().is_some() {
            return Err(
                "usage: compact-rust-proof-smoke --impure-field-helper <proof-output>".into(),
            );
        }
        let proof = std::thread::Builder::new()
            .name("impure-field-helper-proof".into())
            .stack_size(64 * 1024 * 1024)
            .spawn(move || impure_field_helper::run(Path::new(&root)).map_err(|e| e.to_string()))?;
        return match proof.join() {
            Ok(Ok(())) => Ok(()),
            Ok(Err(error)) => Err(error.into()),
            Err(_) => Err("impure Field helper proof thread panicked".into()),
        };
    }
    if first.as_deref() == Some(OsStr::new("--unused-field-reads")) {
        let seven = arguments.next().ok_or("expected seven proof output")?;
        let eight_a = arguments.next().ok_or("expected eight-a proof output")?;
        let eight_b = arguments.next().ok_or("expected eight-b proof output")?;
        if arguments.next().is_some() {
            return Err("unexpected unused-field-reads argument".into());
        }
        return unused_field_reads::run(
            Path::new(&seven),
            Path::new(&eight_a),
            Path::new(&eight_b),
        );
    }
    if first.as_deref() == Some(OsStr::new("--let-return")) {
        let root = arguments
            .next()
            .ok_or("usage: compact-rust-proof-smoke --let-return <proof-output>")?;
        if arguments.next().is_some() {
            return Err("usage: compact-rust-proof-smoke --let-return <proof-output>".into());
        }
        let proof = std::thread::Builder::new()
            .name("let-return-proof".into())
            .stack_size(64 * 1024 * 1024)
            .spawn(move || let_return::run(Path::new(&root)).map_err(|e| e.to_string()))?;
        return match proof.join() {
            Ok(Ok(())) => Ok(()),
            Ok(Err(error)) => Err(error.into()),
            Err(_) => Err("root-Let proof thread panicked".into()),
        };
    }
    if first.as_deref() == Some(OsStr::new("--field-to-bytes32")) {
        let root = arguments
            .next()
            .ok_or("usage: compact-rust-proof-smoke --field-to-bytes32 <proof-output>")?;
        if arguments.next().is_some() {
            return Err("usage: compact-rust-proof-smoke --field-to-bytes32 <proof-output>".into());
        }
        let proof = std::thread::Builder::new()
            .name("field-to-bytes32-proof".into())
            .stack_size(64 * 1024 * 1024)
            .spawn(move || field_to_bytes32::run(Path::new(&root)).map_err(|e| e.to_string()))?;
        return match proof.join() {
            Ok(Ok(())) => Ok(()),
            Ok(Err(error)) => Err(error.into()),
            Err(_) => Err("Field-to-Bytes32 proof thread panicked".into()),
        };
    }
    if first.as_deref() == Some(OsStr::new("--literal-bytes-field")) {
        let root = arguments
            .next()
            .ok_or("usage: compact-rust-proof-smoke --literal-bytes-field <proof-output>")?;
        if arguments.next().is_some() {
            return Err(
                "usage: compact-rust-proof-smoke --literal-bytes-field <proof-output>".into(),
            );
        }
        let proof = std::thread::Builder::new()
            .name("literal-bytes-field-proof".into())
            .stack_size(64 * 1024 * 1024)
            .spawn(move || literal_bytes_field::run(Path::new(&root)).map_err(|e| e.to_string()))?;
        return match proof.join() {
            Ok(Ok(())) => Ok(()),
            Ok(Err(error)) => Err(error.into()),
            Err(_) => Err("literal Bytes-to-Field proof thread panicked".into()),
        };
    }
    if first.as_deref() == Some(OsStr::new("--counter-less-than")) {
        let root = arguments
            .next()
            .ok_or("usage: compact-rust-proof-smoke --counter-less-than <proof-output>")?;
        if arguments.next().is_some() {
            return Err(
                "usage: compact-rust-proof-smoke --counter-less-than <proof-output>".into(),
            );
        }
        let proof = std::thread::Builder::new()
            .name("counter-less-than-proof".into())
            .stack_size(64 * 1024 * 1024)
            .spawn(move || counter_less_than::run(Path::new(&root)).map_err(|e| e.to_string()))?;
        return match proof.join() {
            Ok(Ok(())) => Ok(()),
            Ok(Err(error)) => Err(error.into()),
            Err(_) => Err("Counter lessThan proof thread panicked".into()),
        };
    }
    if first.as_deref() == Some(OsStr::new("--pair-hash-cell")) {
        let root = arguments
            .next()
            .ok_or("usage: compact-rust-proof-smoke --pair-hash-cell <proof-output>")?;
        if arguments.next().is_some() {
            return Err("usage: compact-rust-proof-smoke --pair-hash-cell <proof-output>".into());
        }
        let proof = std::thread::Builder::new()
            .name("pair-hash-cell-proof".into())
            .stack_size(64 * 1024 * 1024)
            .spawn(move || pair_hash_cell::run(Path::new(&root)).map_err(|e| e.to_string()))?;
        return match proof.join() {
            Ok(Ok(())) => Ok(()),
            Ok(Err(error)) => Err(error.into()),
            Err(_) => Err("pair hash Cell proof thread panicked".into()),
        };
    }
    if first.as_deref() == Some(OsStr::new("--stateful-pair-hash")) {
        let root = arguments
            .next()
            .ok_or("usage: compact-rust-proof-smoke --stateful-pair-hash <proof-output>")?;
        if arguments.next().is_some() {
            return Err(
                "usage: compact-rust-proof-smoke --stateful-pair-hash <proof-output>".into(),
            );
        }
        let proof = std::thread::Builder::new()
            .name("stateful-pair-hash-proof".into())
            .stack_size(64 * 1024 * 1024)
            .spawn(move || stateful_pair_hash::run(Path::new(&root)).map_err(|e| e.to_string()))?;
        return match proof.join() {
            Ok(Ok(())) => Ok(()),
            Ok(Err(error)) => Err(error.into()),
            Err(_) => Err("stateful pair hash proof thread panicked".into()),
        };
    }
    if first.as_deref() == Some(OsStr::new("--boolean-pair-assert")) {
        let root = arguments
            .next()
            .ok_or("usage: compact-rust-proof-smoke --boolean-pair-assert <proof-output>")?;
        if arguments.next().is_some() {
            return Err(
                "usage: compact-rust-proof-smoke --boolean-pair-assert <proof-output>".into(),
            );
        }
        let proof = std::thread::Builder::new()
            .name("boolean-pair-assert-proof".into())
            .stack_size(64 * 1024 * 1024)
            .spawn(move || boolean_pair_assert::run(Path::new(&root)).map_err(|e| e.to_string()))?;
        return match proof.join() {
            Ok(Ok(())) => Ok(()),
            Ok(Err(error)) => Err(error.into()),
            Err(_) => Err("Boolean pair assertion proof thread panicked".into()),
        };
    }
    if first.as_deref() == Some(OsStr::new("--pure-field-arguments")) {
        let internal = arguments.next().ok_or(
            "usage: compact-rust-proof-smoke --pure-field-arguments <internal-proof> <ternary-proof>",
        )?;
        let ternary = arguments.next().ok_or(
            "usage: compact-rust-proof-smoke --pure-field-arguments <internal-proof> <ternary-proof>",
        )?;
        if arguments.next().is_some() {
            return Err("usage: compact-rust-proof-smoke --pure-field-arguments <internal-proof> <ternary-proof>".into());
        }
        // Ledger proof composition exceeds macOS's default main-thread stack.
        // Keep the larger stack scoped to this proof smoke, rather than raising
        // the stack limit for every target-gate process.
        let proof = std::thread::Builder::new()
            .name("pure-field-arguments-proof".into())
            .stack_size(64 * 1024 * 1024)
            .spawn(move || {
                pure_field_arguments::run(Path::new(&internal), Path::new(&ternary))
                    .map_err(|error| error.to_string())
            })?;
        return match proof.join() {
            Ok(Ok(())) => Ok(()),
            Ok(Err(error)) => Err(error.into()),
            Err(_) => Err("pure Field argument proof thread panicked".into()),
        };
    }
    if first.as_deref() == Some(OsStr::new("--stateful-pure-return")) {
        let root = arguments
            .next()
            .ok_or("usage: compact-rust-proof-smoke --stateful-pure-return <proof-output>")?;
        if arguments.next().is_some() {
            return Err(
                "usage: compact-rust-proof-smoke --stateful-pure-return <proof-output>".into(),
            );
        }
        let proof = std::thread::Builder::new()
            .name("stateful-pure-return-proof".into())
            .stack_size(64 * 1024 * 1024)
            .spawn(move || {
                stateful_pure_return::run(Path::new(&root)).map_err(|e| e.to_string())
            })?;
        return match proof.join() {
            Ok(Ok(())) => Ok(()),
            Ok(Err(error)) => Err(error.into()),
            Err(_) => Err("stateful pure return proof thread panicked".into()),
        };
    }
    if first.as_deref() == Some(OsStr::new("--persistent-commit")) {
        let root = arguments
            .next()
            .ok_or("usage: compact-rust-proof-smoke --persistent-commit <proof-output>")?;
        if arguments.next().is_some() {
            return Err(
                "usage: compact-rust-proof-smoke --persistent-commit <proof-output>".into(),
            );
        }
        return persistent_commit::run(Path::new(&root));
    }
    if first.as_deref() == Some(OsStr::new("--assert-witness")) {
        let root = arguments
            .next()
            .ok_or("usage: compact-rust-proof-smoke --assert-witness <proof-output>")?;
        if arguments.next().is_some() {
            return Err("usage: compact-rust-proof-smoke --assert-witness <proof-output>".into());
        }
        return witness_assert::run(Path::new(&root));
    }
    if first.as_deref() == Some(OsStr::new("--witness-vector-let")) {
        let root = arguments
            .next()
            .ok_or("usage: compact-rust-proof-smoke --witness-vector-let <proof-output>")?;
        if arguments.next().is_some() {
            return Err(
                "usage: compact-rust-proof-smoke --witness-vector-let <proof-output>".into(),
            );
        }
        return witness_vector_let::run(Path::new(&root));
    }
    if first.as_deref() == Some(OsStr::new("--witness-vector-bare")) {
        let root = arguments
            .next()
            .ok_or("usage: compact-rust-proof-smoke --witness-vector-bare <proof-output>")?;
        if arguments.next().is_some() {
            return Err(
                "usage: compact-rust-proof-smoke --witness-vector-bare <proof-output>".into(),
            );
        }
        return witness_vector_let::run_bare(Path::new(&root));
    }
    if first.as_deref() == Some(OsStr::new("--witness-vector-discard")) {
        let root = arguments
            .next()
            .ok_or("usage: compact-rust-proof-smoke --witness-vector-discard <proof-output>")?;
        if arguments.next().is_some() {
            return Err(
                "usage: compact-rust-proof-smoke --witness-vector-discard <proof-output>".into(),
            );
        }
        return witness_vector_action::run(Path::new(&root));
    }
    if first.as_deref() == Some(OsStr::new("--composite-cell")) {
        let root = arguments
            .next()
            .ok_or("usage: compact-rust-proof-smoke --composite-cell <proof-output>")?;
        if arguments.next().is_some() {
            return Err("usage: compact-rust-proof-smoke --composite-cell <proof-output>".into());
        }
        return check_composite_cell_proof(Path::new(&root));
    }
    if first.as_deref() == Some(OsStr::new("--boolean-observation")) {
        let root = arguments
            .next()
            .ok_or("usage: compact-rust-proof-smoke --boolean-observation <proof-output>")?;
        if arguments.next().is_some() {
            return Err(
                "usage: compact-rust-proof-smoke --boolean-observation <proof-output>".into(),
            );
        }
        return check_boolean_observation_proof(Path::new(&root));
    }
    if first.as_deref() == Some(OsStr::new("--conditional-counter")) {
        let root = arguments
            .next()
            .ok_or("usage: compact-rust-proof-smoke --conditional-counter <proof-output>")?;
        if arguments.next().is_some() {
            return Err(
                "usage: compact-rust-proof-smoke --conditional-counter <proof-output>".into(),
            );
        }
        return check_conditional_counter_proof(Path::new(&root));
    }
    if first.as_deref() == Some(OsStr::new("--conditional-field")) {
        let root = arguments
            .next()
            .ok_or("usage: compact-rust-proof-smoke --conditional-field <proof-output>")?;
        if arguments.next().is_some() {
            return Err(
                "usage: compact-rust-proof-smoke --conditional-field <proof-output>".into(),
            );
        }
        return check_conditional_field_proof(Path::new(&root));
    }
    if first.as_deref() == Some(OsStr::new("--conditional-assert-eq")) {
        let root = arguments
            .next()
            .ok_or("usage: compact-rust-proof-smoke --conditional-assert-eq <proof-output>")?;
        if arguments.next().is_some() {
            return Err(
                "usage: compact-rust-proof-smoke --conditional-assert-eq <proof-output>".into(),
            );
        }
        return check_conditional_assert_eq_proof(Path::new(&root));
    }
    if first.as_deref() == Some(OsStr::new("--conditional-compare-eq")) {
        let root = arguments
            .next()
            .ok_or("usage: compact-rust-proof-smoke --conditional-compare-eq <proof-output>")?;
        if arguments.next().is_some() {
            return Err(
                "usage: compact-rust-proof-smoke --conditional-compare-eq <proof-output>".into(),
            );
        }
        return check_closed_unsigned_ternary_comparison_proof(Path::new(&root));
    }
    if first.as_deref() == Some(OsStr::new("--nested-uint64-cell-counter")) {
        let root = arguments
            .next()
            .ok_or("usage: compact-rust-proof-smoke --nested-uint64-cell-counter <proof-output>")?;
        if arguments.next().is_some() {
            return Err(
                "usage: compact-rust-proof-smoke --nested-uint64-cell-counter <proof-output>"
                    .into(),
            );
        }
        let root = PathBuf::from(root);
        return match std::thread::Builder::new()
            .stack_size(64 * 1024 * 1024)
            .spawn(move || {
                check_nested_uint64_cell_counter_proof(&root).map_err(|error| error.to_string())
            })?
            .join()
        {
            Ok(Ok(())) => Ok(()),
            Ok(Err(error)) => Err(error.into()),
            Err(_) => Err("nested Uint<64> Cell/Counter proof thread panicked".into()),
        };
    }
    if first.as_deref() == Some(OsStr::new("--nested-uint4")) {
        let root = arguments
            .next()
            .ok_or("usage: compact-rust-proof-smoke --nested-uint4 <proof-output>")?;
        if arguments.next().is_some() {
            return Err("usage: compact-rust-proof-smoke --nested-uint4 <proof-output>".into());
        }
        // The ledger/ZKIR proof composition needs more than the default
        // macOS main-thread stack. Give this focused mode its own bound.
        let root = PathBuf::from(root);
        return match std::thread::Builder::new()
            .stack_size(64 * 1024 * 1024)
            .spawn(move || check_nested_uint4_proof(&root).map_err(|error| error.to_string()))?
            .join()
        {
            Ok(Ok(())) => Ok(()),
            Ok(Err(error)) => Err(error.into()),
            Err(_) => Err("nested Uint<4> proof thread panicked".into()),
        };
    }
    if first.as_deref() == Some(OsStr::new("--conditional-field-vector")) {
        let root = arguments
            .next()
            .ok_or("usage: compact-rust-proof-smoke --conditional-field-vector <proof-output>")?;
        if arguments.next().is_some() {
            return Err(
                "usage: compact-rust-proof-smoke --conditional-field-vector <proof-output>".into(),
            );
        }
        let root = PathBuf::from(root);
        return match std::thread::Builder::new()
            .stack_size(64 * 1024 * 1024)
            .spawn(move || {
                check_conditional_field_vector_proof(&root).map_err(|error| error.to_string())
            })?
            .join()
        {
            Ok(Ok(())) => Ok(()),
            Ok(Err(error)) => Err(error.into()),
            Err(_) => Err("conditional Field vector proof thread panicked".into()),
        };
    }
    if first.as_deref() == Some(OsStr::new("--annotated-uint8")) {
        let root = arguments
            .next()
            .ok_or("usage: compact-rust-proof-smoke --annotated-uint8 <proof-output>")?;
        if arguments.next().is_some() {
            return Err("usage: compact-rust-proof-smoke --annotated-uint8 <proof-output>".into());
        }
        let root = PathBuf::from(root);
        return match std::thread::Builder::new()
            .stack_size(64 * 1024 * 1024)
            .spawn(move || check_annotated_uint8_proof(&root).map_err(|error| error.to_string()))?
            .join()
        {
            Ok(Ok(())) => Ok(()),
            Ok(Err(error)) => Err(error.into()),
            Err(_) => Err("annotated Uint<8> proof thread panicked".into()),
        };
    }
    if first.as_deref() == Some(OsStr::new("--native-curve-arg")) {
        let root = arguments
            .next()
            .ok_or("usage: compact-rust-proof-smoke --native-curve-arg <proof-output>")?;
        if arguments.next().is_some() {
            return Err("usage: compact-rust-proof-smoke --native-curve-arg <proof-output>".into());
        }
        let root = PathBuf::from(root);
        return match std::thread::Builder::new()
            .stack_size(64 * 1024 * 1024)
            .spawn(move || {
                check_closed_curve_argument_proof(&root).map_err(|error| error.to_string())
            })?
            .join()
        {
            Ok(Ok(())) => Ok(()),
            Ok(Err(error)) => Err(error.into()),
            Err(_) => Err("closed curve argument proof thread panicked".into()),
        };
    }
    if first.as_deref() == Some(OsStr::new("--conditional-struct-member")) {
        let root = arguments
            .next()
            .ok_or("usage: compact-rust-proof-smoke --conditional-struct-member <proof-output>")?;
        if arguments.next().is_some() {
            return Err(
                "usage: compact-rust-proof-smoke --conditional-struct-member <proof-output>".into(),
            );
        }
        return check_closed_ternary_struct_member_proof(Path::new(&root));
    }
    if first.as_deref() == Some(OsStr::new("--merkle-verify")) {
        let root = arguments
            .next()
            .ok_or("usage: compact-rust-proof-smoke --merkle-verify <compiled-contract-output>")?;
        if arguments.next().is_some() {
            return Err(
                "usage: compact-rust-proof-smoke --merkle-verify <compiled-contract-output>".into(),
            );
        }
        return merkle_verify::run(Path::new(&root));
    }
    if first.as_deref() == Some(OsStr::new("--merkle-reset")) {
        let root = arguments
            .next()
            .ok_or("usage: compact-rust-proof-smoke --merkle-reset <proof-output>")?;
        if arguments.next().is_some() {
            return Err("usage: compact-rust-proof-smoke --merkle-reset <proof-output>".into());
        }
        return merkle_reset::run(Path::new(&root));
    }
    if first.as_deref() == Some(OsStr::new("--vector-map")) {
        let identity = arguments
            .next()
            .ok_or("usage: --vector-map <identity-output> <arithmetic-constructor-output>")?;
        let arithmetic = arguments
            .next()
            .ok_or("usage: --vector-map <identity-output> <arithmetic-constructor-output>")?;
        if arguments.next().is_some() {
            return Err(
                "usage: --vector-map <identity-output> <arithmetic-constructor-output>".into(),
            );
        }
        let proof = std::thread::Builder::new()
            .name("vector-map-proof".into())
            .stack_size(64 * 1024 * 1024)
            .spawn(move || {
                vector_map::run(Path::new(&identity), Path::new(&arithmetic))
                    .map_err(|e| e.to_string())
            })?;
        return match proof.join() {
            Ok(Ok(())) => Ok(()),
            Ok(Err(error)) => Err(error.into()),
            Err(_) => Err("vector map proof thread panicked".into()),
        };
    }
    if first.as_deref() == Some(OsStr::new("--merkle-root")) {
        let root = arguments
            .next()
            .ok_or("usage: compact-rust-proof-smoke --merkle-root <proof-output>")?;
        if arguments.next().is_some() {
            return Err("usage: compact-rust-proof-smoke --merkle-root <proof-output>".into());
        }
        return merkle_root::run(Path::new(&root));
    }
    if first.as_deref() == Some(OsStr::new("--zerocash-spend")) {
        let root = arguments
            .next()
            .ok_or("usage: --zerocash-spend <proof-output>")?;
        if arguments.next().is_some() {
            return Err("usage: --zerocash-spend <proof-output>".into());
        }
        return zerocash_spend::run(Path::new(&root));
    }
    if first.as_deref() == Some(OsStr::new("--bboard")) {
        let root = arguments.next().ok_or("usage: --bboard <proof-output>")?;
        if arguments.next().is_some() {
            return Err("usage: --bboard <proof-output>".into());
        }
        return bboard::run(Path::new(&root));
    }
    if first.as_deref() == Some(OsStr::new("--election-reveal")) {
        let root = arguments
            .next()
            .ok_or("usage: --election-reveal <proof-output>")?;
        if arguments.next().is_some() {
            return Err("usage: --election-reveal <proof-output>".into());
        }
        return election_reveal::run(Path::new(&root));
    }
    if first.as_deref() == Some(OsStr::new("--election-commit")) {
        let root = arguments
            .next()
            .ok_or("usage: --election-commit <proof-output>")?;
        if arguments.next().is_some() {
            return Err("usage: --election-commit <proof-output>".into());
        }
        return election_commit::run(Path::new(&root));
    }
    if first.as_deref() == Some(OsStr::new("--election-add-voter")) {
        let root = arguments
            .next()
            .ok_or("usage: --election-add-voter <proof-output>")?;
        if arguments.next().is_some() {
            return Err("usage: --election-add-voter <proof-output>".into());
        }
        return election_topic::add_voter(Path::new(&root));
    }
    if first.as_deref() == Some(OsStr::new("--election-advance")) {
        let root = arguments
            .next()
            .ok_or("usage: --election-advance <proof-output>")?;
        if arguments.next().is_some() {
            return Err("usage: --election-advance <proof-output>".into());
        }
        return election_topic::advance(Path::new(&root));
    }
    if first.as_deref() == Some(OsStr::new("--zerocash-mint")) {
        let root = arguments
            .next()
            .ok_or("usage: --zerocash-mint <proof-output>")?;
        if arguments.next().is_some() {
            return Err("usage: --zerocash-mint <proof-output>".into());
        }
        return zerocash_mint::run(Path::new(&root));
    }
    if first.as_deref() == Some(OsStr::new("--election-topic")) {
        let root = arguments
            .next()
            .ok_or("usage: --election-topic <proof-output>")?;
        if arguments.next().is_some() {
            return Err("usage: --election-topic <proof-output>".into());
        }
        return election_topic::run(Path::new(&root));
    }
    if first.as_deref() == Some(OsStr::new("--inline-type-scope")) {
        let root = arguments
            .next()
            .ok_or("usage: compact-rust-proof-smoke --inline-type-scope <proof-output>")?;
        if arguments.next().is_some() {
            return Err(
                "usage: compact-rust-proof-smoke --inline-type-scope <proof-output>".into(),
            );
        }
        let proof = std::thread::Builder::new()
            .name("inline-type-scope-proof".into())
            .stack_size(64 * 1024 * 1024)
            .spawn(move || inline_type_scope::run(Path::new(&root)).map_err(|e| e.to_string()))?;
        return match proof.join() {
            Ok(Ok(())) => Ok(()),
            Ok(Err(error)) => Err(error.into()),
            Err(_) => Err("inline type scope proof thread panicked".into()),
        };
    }
    if first.as_deref() == Some(OsStr::new("--merkle-hash")) {
        let root = arguments
            .next()
            .ok_or("usage: compact-rust-proof-smoke --merkle-hash <proof-output>")?;
        if arguments.next().is_some() {
            return Err("usage: compact-rust-proof-smoke --merkle-hash <proof-output>".into());
        }
        let proof = std::thread::Builder::new()
            .name("merkle-hash-proof".into())
            .stack_size(64 * 1024 * 1024)
            .spawn(move || merkle_hash::run(Path::new(&root)).map_err(|e| e.to_string()))?;
        return match proof.join() {
            Ok(Ok(())) => Ok(()),
            Ok(Err(error)) => Err(error.into()),
            Err(_) => Err("Merkle hash proof thread panicked".into()),
        };
    }
    if first.as_deref() == Some(OsStr::new("--historic-merkle-hash")) {
        let root = arguments
            .next()
            .ok_or("usage: compact-rust-proof-smoke --historic-merkle-hash <proof-output>")?;
        if arguments.next().is_some() {
            return Err(
                "usage: compact-rust-proof-smoke --historic-merkle-hash <proof-output>".into(),
            );
        }
        let proof = std::thread::Builder::new()
            .name("historic-merkle-hash-proof".into())
            .stack_size(64 * 1024 * 1024)
            .spawn(move || {
                merkle_hash::run_historic(Path::new(&root)).map_err(|e| e.to_string())
            })?;
        return match proof.join() {
            Ok(Ok(())) => Ok(()),
            Ok(Err(error)) => Err(error.into()),
            Err(_) => Err("historic Merkle hash proof thread panicked".into()),
        };
    }
    if first.as_deref() == Some(OsStr::new("--historic-merkle-indexed-hash")) {
        let root = arguments.next().ok_or(
            "usage: compact-rust-proof-smoke --historic-merkle-indexed-hash <proof-output>",
        )?;
        if arguments.next().is_some() {
            return Err(
                "usage: compact-rust-proof-smoke --historic-merkle-indexed-hash <proof-output>"
                    .into(),
            );
        }
        let proof = std::thread::Builder::new()
            .name("historic-merkle-indexed-hash-proof".into())
            .stack_size(64 * 1024 * 1024)
            .spawn(move || {
                merkle_hash::run_historic_indexed(Path::new(&root)).map_err(|e| e.to_string())
            })?;
        return match proof.join() {
            Ok(Ok(())) => Ok(()),
            Ok(Err(error)) => Err(error.into()),
            Err(_) => Err("historic Merkle indexed hash proof thread panicked".into()),
        };
    }
    if first.as_deref() == Some(OsStr::new("--historic-merkle-reset-history")) {
        let root = arguments.next().ok_or(
            "usage: compact-rust-proof-smoke --historic-merkle-reset-history <proof-output>",
        )?;
        if arguments.next().is_some() {
            return Err(
                "usage: compact-rust-proof-smoke --historic-merkle-reset-history <proof-output>"
                    .into(),
            );
        }
        let proof = std::thread::Builder::new()
            .name("historic-merkle-reset-history-proof".into())
            .stack_size(64 * 1024 * 1024)
            .spawn(move || {
                merkle_hash::run_historic_reset_history(Path::new(&root)).map_err(|e| e.to_string())
            })?;
        return match proof.join() {
            Ok(Ok(())) => Ok(()),
            Ok(Err(error)) => Err(error.into()),
            Err(_) => Err("historic Merkle reset-history proof thread panicked".into()),
        };
    }
    if first.as_deref() == Some(OsStr::new("--historic-merkle-reset-tree")) {
        let root = arguments
            .next()
            .ok_or("usage: compact-rust-proof-smoke --historic-merkle-reset-tree <proof-output>")?;
        if arguments.next().is_some() {
            return Err(
                "usage: compact-rust-proof-smoke --historic-merkle-reset-tree <proof-output>"
                    .into(),
            );
        }
        let proof = std::thread::Builder::new()
            .name("historic-merkle-reset-tree-proof".into())
            .stack_size(64 * 1024 * 1024)
            .spawn(move || {
                merkle_hash::run_historic_reset_tree(Path::new(&root)).map_err(|e| e.to_string())
            })?;
        return match proof.join() {
            Ok(Ok(())) => Ok(()),
            Ok(Err(error)) => Err(error.into()),
            Err(_) => Err("historic Merkle tree reset proof thread panicked".into()),
        };
    }
    if first.as_deref() == Some(OsStr::new("--historic-merkle-known")) {
        let root = arguments
            .next()
            .ok_or("usage: compact-rust-proof-smoke --historic-merkle-known <proof-output>")?;
        if arguments.next().is_some() {
            return Err(
                "usage: compact-rust-proof-smoke --historic-merkle-known <proof-output>".into(),
            );
        }
        let proof = std::thread::Builder::new()
            .name("historic-merkle-known-proof".into())
            .stack_size(64 * 1024 * 1024)
            .spawn(move || {
                merkle_hash::run_historic_known(Path::new(&root)).map_err(|e| e.to_string())
            })?;
        return match proof.join() {
            Ok(Ok(())) => Ok(()),
            Ok(Err(error)) => Err(error.into()),
            Err(_) => Err("historic Merkle known proof thread panicked".into()),
        };
    }
    if first.as_deref() == Some(OsStr::new("--merkle-indexed")) {
        let root = arguments
            .next()
            .ok_or("usage: compact-rust-proof-smoke --merkle-indexed <proof-output>")?;
        if arguments.next().is_some() {
            return Err("usage: compact-rust-proof-smoke --merkle-indexed <proof-output>".into());
        }
        let proof = std::thread::Builder::new()
            .name("merkle-indexed-proof".into())
            .stack_size(64 * 1024 * 1024)
            .spawn(move || merkle_indexed::run(Path::new(&root)).map_err(|e| e.to_string()))?;
        return match proof.join() {
            Ok(Ok(())) => Ok(()),
            Ok(Err(error)) => Err(error.into()),
            Err(_) => Err("indexed Merkle proof thread panicked".into()),
        };
    }
    if first.as_deref() == Some(OsStr::new("--historic-merkle-indexed")) {
        let insert_root = arguments.next().ok_or(
            "usage: compact-rust-proof-smoke --historic-merkle-indexed <insert-output> <default-output>",
        )?;
        let default_root = arguments.next().ok_or(
            "usage: compact-rust-proof-smoke --historic-merkle-indexed <insert-output> <default-output>",
        )?;
        if arguments.next().is_some() {
            return Err("usage: compact-rust-proof-smoke --historic-merkle-indexed <insert-output> <default-output>".into());
        }
        let proof = std::thread::Builder::new()
            .name("historic-merkle-indexed-proof".into())
            .stack_size(64 * 1024 * 1024)
            .spawn(move || {
                merkle_indexed::run_historic(Path::new(&insert_root), Path::new(&default_root))
                    .map_err(|e| e.to_string())
            })?;
        return match proof.join() {
            Ok(Ok(())) => Ok(()),
            Ok(Err(error)) => Err(error.into()),
            Err(_) => Err("historic indexed Merkle proof thread panicked".into()),
        };
    }
    let counter_root = first.ok_or(
        "usage: compact-rust-proof-smoke <counter-output> <cell-output> <cell-read-output>",
    )?;
    let cell_root = arguments.next().ok_or(
        "usage: compact-rust-proof-smoke <counter-output> <cell-output> <cell-read-output>",
    )?;
    let cell_read_root = arguments.next().ok_or(
        "usage: compact-rust-proof-smoke <counter-output> <cell-output> <cell-read-output>",
    )?;
    let witness_root = arguments.next();
    let nested_root = arguments.next();
    let expression_root = arguments.next();
    let set_root = arguments.next();
    let set_oracle_root = arguments.next();
    let map_root = arguments.next();
    let constructor_map_root = arguments.next();
    let list_root = arguments.next();
    let constructor_list_root = arguments.next();
    let enum_cell_root = arguments.next();
    let tiny_root = arguments.next();
    let nested_map_shape_root = arguments.next();
    let list_shapes_root = arguments.next();
    let merkle_root = arguments.next();
    let historic_merkle_root = arguments.next();
    let vector_key_root = arguments.next();
    let counter_parameter_root = arguments.next();
    let composite_key_root = arguments.next();
    let chunked_set_root = arguments.next();
    let chunked_list_root = arguments.next();
    let chunked_map_root = arguments.next();
    let chunked_cell_root = arguments.next();
    let uints_root = arguments.next();
    let wide_root = arguments.next();
    if arguments.next().is_some() {
        return Err(
            "usage: compact-rust-proof-smoke <counter-output> <cell-output> <cell-read-output> [witness-output] [nested-output] [nested-witness-output] [set-output] [set-oracle-output] [map-output] [constructor-map-output] [list-output] [constructor-list-output] [enum-cell-output] [tiny-output] [nested-map-shape-output] [list-shapes-output] [merkle-output] [historic-merkle-output] [vector-key-output] [counter-parameter-output] [composite-key-output] [chunked-set-output] [chunked-list-output] [chunked-map-output] [chunked-cell-output] [uints-output] [wide-uint-output]"
                .into(),
        );
    }
    let counter_root = Path::new(&counter_root);
    let cell_root = Path::new(&cell_root);
    let cell_read_root = Path::new(&cell_read_root);
    let handoff_config = HandoffConfig::from_env()?;
    let mut rng = StdRng::seed_from_u64(0x434f4d50414354);
    let counter_initial = counter_contract::initial_state(ConstructorContext::new(()))?;
    let counter_deploy = make_deploy(
        counter_root,
        "increment",
        counter_initial.ledger_state.get_ref().clone(),
        &mut rng,
    )?;
    let counter_context = counter_initial.into_circuit_context(counter_deploy.address());
    let counter_recorded = counter_contract::Contract::default()
        .recording
        .increment(counter_context)?;
    let counter_manual = check_generated_trace(counter_root, "increment", counter_recorded, ())?;
    let counter_observed = ObservedContractState::new(
        counter_deploy.address(),
        counter_deploy.initial_state.clone(),
        Observation {
            transaction_hash: [0; 32],
            block_hash: [0; 32],
            block_height: 0,
        },
    );
    let counter_verifier: VerifierKey = tagged_deserialize(&mut BufReader::new(File::open(
        counter_root.join("keys/increment.verifier"),
    )?))?;
    let mut state_bytes = Vec::new();
    tagged_serialize(counter_observed.contract(), &mut state_bytes)?;
    let decoded = ObservedContractState::decode(
        counter_observed.address(),
        &state_bytes,
        counter_observed.observation(),
    )?;
    if decoded.contract() != counter_observed.contract() {
        return Err("observed state decoder changed ledger state".into());
    }
    state_bytes.push(0);
    if ObservedContractState::decode(
        counter_observed.address(),
        &state_bytes,
        counter_observed.observation(),
    )
    .is_ok()
    {
        return Err("observed state decoder accepted trailing bytes".into());
    }
    let missing_operation = ObservedContractState::new(
        counter_observed.address(),
        ContractState::new(
            counter_deploy.initial_state.data.get_ref().clone(),
            HashMap::new(),
            ContractMaintenanceAuthority::default(),
        ),
        counter_observed.observation(),
    );
    let missing_call = counter_contract::Contract::default()
        .recording
        .increment_call(&missing_operation, ())?;
    if !matches!(
        missing_call.prepare(counter_verifier.clone(), Fr::from(0u64)),
        Err(ObservedCallError::MissingOperation(_))
    ) {
        return Err("observed call accepted a missing operation".into());
    }
    let wrong_verifier: VerifierKey = tagged_deserialize(&mut BufReader::new(File::open(
        cell_root.join("keys/set_flag.verifier"),
    )?))?;
    let wrong_key_call = counter_contract::Contract::default()
        .recording
        .increment_call(&counter_observed, ())?;
    if !matches!(
        wrong_key_call.prepare(wrong_verifier, Fr::from(0u64)),
        Err(ObservedCallError::VerifierMismatch(_))
    ) {
        return Err("observed call accepted a different verifier".into());
    }
    let mut bad_address = counter_observed.address();
    bad_address.0.0[0] ^= 1;
    let wrong_address = ObservedContractState::new(
        bad_address,
        counter_deploy.initial_state.clone(),
        counter_observed.observation(),
    );
    let recorded_at_address = counter_contract::Contract::default()
        .recording
        .increment(counter_observed.circuit_context(()))?;
    if !matches!(
        RecordedCall::new(&wrong_address, recorded_at_address, "increment", ())
            .prepare(counter_verifier.clone(), Fr::from(0u64)),
        Err(ObservedCallError::AddressMismatch)
    ) {
        return Err("observed call accepted a different address".into());
    }
    let recorded_for_stale = counter_contract::Contract::default()
        .recording
        .increment(counter_observed.circuit_context(()))?;
    let stale_state = ContractState::new(
        recorded_for_stale
            .execution
            .context
            .query
            .state
            .get_ref()
            .clone(),
        counter_deploy.initial_state.operations.clone(),
        ContractMaintenanceAuthority::default(),
    );
    let stale_observed = ObservedContractState::new(
        counter_observed.address(),
        stale_state,
        counter_observed.observation(),
    );
    if !matches!(
        RecordedCall::new(&stale_observed, recorded_for_stale, "increment", ())
            .prepare(counter_verifier.clone(), Fr::from(0u64)),
        Err(ObservedCallError::StateMismatch)
    ) {
        return Err("observed call accepted a stale state".into());
    }
    let counter_call = counter_contract::Contract::default()
        .recording
        .increment_call(&counter_observed, ())?
        .prepare(counter_verifier, Fr::from(0u64))?;
    check_observed_call_parity(
        counter_root,
        "increment",
        &counter_deploy,
        &counter_manual,
        &counter_call,
    )?;
    prove_counter(counter_root, &counter_call)?;
    let counter_handoff = env::var_os("COMPACT_RUST_WALLET_HANDOFF").map(PathBuf::from);
    let counter_deploy_handoff = env::var_os("COMPACT_RUST_DEPLOY_HANDOFF").map(PathBuf::from);
    check_transaction_with_handoff(
        counter_root,
        "increment",
        counter_deploy,
        counter_call,
        &mut rng,
        counter_handoff.as_deref(),
        counter_deploy_handoff.as_deref(),
        &handoff_config,
        |contract| {
            let StateValue::Array(fields) = contract.data.get_ref() else {
                return Err("counter contract state is not an array".into());
            };
            if read_counter(fields.get(0).ok_or("counter field missing")?)? != 1 {
                return Err("proven counter call did not increment ledger state".into());
            }
            Ok(())
        },
    )?;

    if let Some(root) = counter_parameter_root.as_ref() {
        let root = Path::new(root);
        let initial = counter_parameter_contract::initial_state(ConstructorContext::new(()))?;
        let deploy = make_deploy(
            root,
            "increment_by",
            initial.ledger_state.get_ref().clone(),
            &mut rng,
        )?;
        let amount = BoundedUint::<65535>::new(3)?;
        let recorded = counter_parameter_contract::Contract::default()
            .recording
            .increment_by(initial.into_circuit_context(deploy.address()), amount)?;
        let manual = check_generated_trace(root, "increment_by", recorded, amount)?;
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
            root.join("keys/increment_by.verifier"),
        )?))?;
        let call = counter_parameter_contract::Contract::default()
            .recording
            .increment_by_call(&observed, (), amount)?
            .prepare(verifier, Fr::from(0u64))?;
        check_observed_call_parity(root, "increment_by", &deploy, &manual, &call)?;
        check_transaction(root, "increment_by", deploy, call, &mut rng, |contract| {
            let StateValue::Array(fields) = contract.data.get_ref() else {
                return Err("counter-parameter state is not an array".into());
            };
            if read_counter(fields.get(0).ok_or("counter-parameter field missing")?)? != 3 {
                return Err("proven typed counter call did not add the parameter".into());
            }
            Ok(())
        })?;

        let initial = counter_parameter_contract::initial_state(ConstructorContext::new(()))?;
        let seeded = counter_parameter_contract::increment_by(
            initial.into_circuit_context(Default::default()),
            BoundedUint::<65535>::new(3)?,
        )?;
        let reset_deploy = make_deploy(
            root,
            "reset_round",
            seeded.context.query.state.get_ref().clone(),
            &mut rng,
        )?;
        let reset_context = midnight_compact_runtime::context::CircuitContext::from_contract_state(
            (),
            reset_deploy.address(),
            &reset_deploy.initial_state,
        );
        let recorded = counter_parameter_contract::Contract::default()
            .recording
            .reset_round(reset_context)?;
        let manual = check_generated_trace(root, "reset_round", recorded, ())?;
        let observed = ObservedContractState::new(
            reset_deploy.address(),
            reset_deploy.initial_state.clone(),
            Observation {
                transaction_hash: [0; 32],
                block_hash: [0; 32],
                block_height: 0,
            },
        );
        let verifier: VerifierKey = tagged_deserialize(&mut BufReader::new(File::open(
            root.join("keys/reset_round.verifier"),
        )?))?;
        let call = counter_parameter_contract::Contract::default()
            .recording
            .reset_round_call(&observed, ())?
            .prepare(verifier, Fr::from(0u64))?;
        check_observed_call_parity(root, "reset_round", &reset_deploy, &manual, &call)?;
        check_transaction(
            root,
            "reset_round",
            reset_deploy,
            call,
            &mut rng,
            |contract| {
                let StateValue::Array(fields) = contract.data.get_ref() else {
                    return Err("reset-round state is not an array".into());
                };
                if read_counter(fields.get(0).ok_or("reset-round field missing")?)? != 0 {
                    return Err("proven typed Counter reset did not clear the seeded value".into());
                }
                Ok(())
            },
        )?;
    }

    let cell_initial = cell_contract::initial_state(ConstructorContext::new(()))?;
    let cell_deploy = make_deploy(
        cell_root,
        "set_flag",
        cell_initial.ledger_state.get_ref().clone(),
        &mut rng,
    )?;
    let cell_context = cell_initial.into_circuit_context(cell_deploy.address());
    let cell_recorded = cell_contract::Contract::default()
        .recording
        .set_flag(cell_context)?;
    let cell_call = check_generated_trace(cell_root, "set_flag", cell_recorded, ())?;
    check_transaction(
        cell_root,
        "set_flag",
        cell_deploy,
        cell_call,
        &mut rng,
        |contract| {
            let StateValue::Array(fields) = contract.data.get_ref() else {
                return Err("cell contract state is not an array".into());
            };
            if !read_cell::<bool, _>(fields.get(0).ok_or("cell field missing")?)? {
                return Err("proven Cell call did not set the ledger flag".into());
            }
            Ok(())
        },
    )?;

    let cell_read_initial = cell_read_contract::initial_state(ConstructorContext::new(()))?;
    let cell_read_deploy = make_deploy(
        cell_read_root,
        "read_flag",
        cell_read_initial.ledger_state.get_ref().clone(),
        &mut rng,
    )?;
    let cell_read_context = cell_read_initial.into_circuit_context(cell_read_deploy.address());
    let cell_read_recorded = cell_read_contract::Contract::default()
        .recording
        .read_flag(cell_read_context)?;
    if cell_read_recorded.execution.result {
        return Err("new Cell contract unexpectedly read true".into());
    }
    let cell_read_call =
        check_generated_trace(cell_read_root, "read_flag", cell_read_recorded, ())?;
    check_transaction(
        cell_read_root,
        "read_flag",
        cell_read_deploy,
        cell_read_call,
        &mut rng,
        |contract| {
            let StateValue::Array(fields) = contract.data.get_ref() else {
                return Err("Cell read contract state is not an array".into());
            };
            if read_cell::<bool, _>(fields.get(0).ok_or("cell read field missing")?)? {
                return Err("proven Cell read unexpectedly changed ledger state".into());
            }
            Ok(())
        },
    )?;

    if let Some(witness_root) = witness_root {
        let witness_root = Path::new(&witness_root);
        let witness_initial = witness_contract::initial_state(ConstructorContext::new(7_u64))?;
        let witness_deploy = make_deploy(
            witness_root,
            "write_twice",
            witness_initial.ledger_state.get_ref().clone(),
            &mut rng,
        )?;
        let context = witness_initial.into_circuit_context(witness_deploy.address());
        let seed = Field::from(2_u64);
        let recorded = witness_contract::Contract::from(Secret)
            .recording()
            .write_twice(context, seed)?;
        if recorded.execution.private_transcript_outputs.len() != 2 {
            return Err("witnessed call did not record two private values".into());
        }
        let witness_call = check_generated_trace(witness_root, "write_twice", recorded, seed)?;
        check_transaction(
            witness_root,
            "write_twice",
            witness_deploy,
            witness_call,
            &mut rng,
            |contract| {
                let StateValue::Array(fields) = contract.data.get_ref() else {
                    return Err("witnessed contract state is not an array".into());
                };
                if read_cell::<Field, _>(fields.get(0).ok_or("witness Cell missing")?)?
                    != Field::from(10_u64)
                {
                    return Err("proven witness call did not write the final Cell value".into());
                }
                Ok(())
            },
        )?;

        let offset_initial = witness_contract::initial_state(ConstructorContext::new(7_u64))?;
        let offset_deploy = make_deploy(
            witness_root,
            "write_offset",
            offset_initial.ledger_state.get_ref().clone(),
            &mut rng,
        )?;
        let offset_context = offset_initial.into_circuit_context(offset_deploy.address());
        let offset = Field::from(5_u64);
        let offset_contract = witness_contract::Contract::from(Secret);
        let offset_recorded =
            offset_contract
                .recording()
                .write_offset(offset_context, seed, offset)?;
        if offset_recorded.execution.context.private_state != 8
            || offset_recorded.execution.private_transcript_outputs
                != [AlignedValue::from(Field::from(9_u64))]
        {
            return Err("witnessed offset private output differs from TypeScript oracle".into());
        }
        if !matches!(
            offset_recorded.public.verify_ops(),
            [
                Op::Push { storage: false, .. },
                Op::Push { storage: true, .. },
                Op::Ins {
                    cached: false,
                    n: 1
                },
            ]
        ) {
            return Err("witnessed offset VM shape differs from TypeScript oracle".into());
        }
        let manual_offset = check_generated_trace(
            witness_root,
            "write_offset",
            offset_recorded,
            (seed, offset),
        )?;
        let observed_offset = ObservedContractState::new(
            offset_deploy.address(),
            offset_deploy.initial_state.clone(),
            Observation {
                transaction_hash: [0; 32],
                block_hash: [0; 32],
                block_height: 0,
            },
        );
        let verifier =
            decode_verifier_key(&fs::read(witness_root.join("keys/write_offset.verifier"))?)?;
        let typed_offset = offset_contract
            .recording()
            .write_offset_call(&observed_offset, 7_u64, seed, offset)?
            .prepare(verifier, Fr::from(0_u64))?;
        check_observed_call_parity(
            witness_root,
            "write_offset",
            &offset_deploy,
            &manual_offset,
            &typed_offset,
        )?;
        check_transaction(
            witness_root,
            "write_offset",
            offset_deploy,
            typed_offset,
            &mut rng,
            |contract| {
                let StateValue::Array(fields) = contract.data.get_ref() else {
                    return Err("witnessed offset state is not an array".into());
                };
                if read_cell::<Field, _>(fields.get(0).ok_or("witnessed offset Cell missing")?)?
                    != Field::from(14_u64)
                {
                    return Err("proven witnessed offset call wrote the wrong Cell value".into());
                }
                Ok(())
            },
        )?;

        let nested_initial = witness_contract::initial_state(ConstructorContext::new(7_u64))?;
        let nested_deploy = make_deploy(
            witness_root,
            "write_nested_twice",
            nested_initial.ledger_state.get_ref().clone(),
            &mut rng,
        )?;
        let context = nested_initial.into_circuit_context(nested_deploy.address());
        let recorded = witness_contract::Contract::from(Secret)
            .recording()
            .write_nested_twice(context, seed)?;
        if recorded.execution.private_transcript_outputs.len() != 2 {
            return Err("nested witness call lost a private value".into());
        }
        let nested_call =
            check_generated_trace(witness_root, "write_nested_twice", recorded, seed)?;
        check_transaction(
            witness_root,
            "write_nested_twice",
            nested_deploy,
            nested_call,
            &mut rng,
            |contract| {
                let StateValue::Array(fields) = contract.data.get_ref() else {
                    return Err("nested witnessed state is not an array".into());
                };
                if read_cell::<Field, _>(fields.get(0).ok_or("nested witness Cell missing")?)?
                    != Field::from(10_u64)
                {
                    return Err("proven nested witness call did not write both values".into());
                }
                Ok(())
            },
        )?;
    }
    if let Some(nested_root) = nested_root {
        let nested_root = Path::new(&nested_root);
        let nested_initial = nested_contract::initial_state(ConstructorContext::new(()))?;
        let nested_deploy = make_deploy(
            nested_root,
            "bump_twice",
            nested_initial.ledger_state.get_ref().clone(),
            &mut rng,
        )?;
        let context = nested_initial.into_circuit_context(nested_deploy.address());
        let recorded = nested_contract::Contract::default()
            .recording
            .bump_twice(context)?;
        let nested_call = check_generated_trace(nested_root, "bump_twice", recorded, ())?;
        check_transaction(
            nested_root,
            "bump_twice",
            nested_deploy,
            nested_call,
            &mut rng,
            |contract| {
                let StateValue::Array(fields) = contract.data.get_ref() else {
                    return Err("nested Counter state is not an array".into());
                };
                if read_counter(fields.get(0).ok_or("nested Counter missing")?)? != 2 {
                    return Err("proven nested call did not increment twice".into());
                }
                Ok(())
            },
        )?;

        let parameterized_initial = nested_contract::initial_state(ConstructorContext::new(()))?;
        let parameterized_deploy = make_deploy(
            nested_root,
            "add_twice",
            parameterized_initial.ledger_state.get_ref().clone(),
            &mut rng,
        )?;
        let context = parameterized_initial.into_circuit_context(parameterized_deploy.address());
        let amount = BoundedUint::<65535>::new(3)?;
        let recorded = nested_contract::Contract::default()
            .recording
            .add_twice(context, amount)?;
        let parameterized_manual =
            check_generated_trace(nested_root, "add_twice", recorded, amount)?;
        let parameterized_observed = ObservedContractState::new(
            parameterized_deploy.address(),
            parameterized_deploy.initial_state.clone(),
            Observation {
                transaction_hash: [0; 32],
                block_hash: [0; 32],
                block_height: 0,
            },
        );
        let parameterized_verifier: VerifierKey = tagged_deserialize(&mut BufReader::new(
            File::open(nested_root.join("keys/add_twice.verifier"))?,
        ))?;
        let parameterized_call = nested_contract::Contract::default()
            .recording
            .add_twice_call(&parameterized_observed, (), amount)?
            .prepare(parameterized_verifier, Fr::from(0u64))?;
        check_observed_call_parity(
            nested_root,
            "add_twice",
            &parameterized_deploy,
            &parameterized_manual,
            &parameterized_call,
        )?;
        check_transaction(
            nested_root,
            "add_twice",
            parameterized_deploy,
            parameterized_call,
            &mut rng,
            |contract| {
                let StateValue::Array(fields) = contract.data.get_ref() else {
                    return Err("parameterized Counter state is not an array".into());
                };
                if read_counter(fields.get(0).ok_or("parameterized Counter missing")?)? != 6 {
                    return Err("proven parameterized call did not increment twice".into());
                }
                Ok(())
            },
        )?;
    }
    if let Some(expression_root) = expression_root {
        let expression_root = Path::new(&expression_root);
        for circuit in ["outer", "outerValue", "outerValue2", "outerValueExpr"] {
            let initial = expression_contract::initial_state(ConstructorContext::new(7_u64))?;
            let deploy = make_deploy(
                expression_root,
                circuit,
                initial.ledger_state.get_ref().clone(),
                &mut rng,
            )?;
            let context = initial.into_circuit_context(deploy.address());
            let contract = expression_contract::Contract::from(NestedSecret);
            let recorded = match circuit {
                "outer" => contract.recording().outer(context)?,
                "outerValue" => contract.recording().outerValue(context)?,
                "outerValue2" => contract.recording().outerValue2(context)?,
                "outerValueExpr" => contract.recording().outerValueExpr(context)?,
                _ => unreachable!(),
            };
            let expected_outputs = match circuit {
                "outerValue2" => 3,
                "outerValueExpr" => 2,
                _ => 1,
            };
            if recorded.execution.private_transcript_outputs.len() != expected_outputs {
                return Err(
                    format!("{circuit} did not record {expected_outputs} private values").into(),
                );
            }
            let call = check_generated_trace(expression_root, circuit, recorded, ())?;
            check_transaction(
                expression_root,
                circuit,
                deploy,
                call,
                &mut rng,
                |contract| {
                    let StateValue::Array(fields) = contract.data.get_ref() else {
                        return Err("nested expression state is not an array".into());
                    };
                    let expected = match circuit {
                        "outerValue2" => 24_u64,
                        "outerValueExpr" => 15_u64,
                        _ => 7_u64,
                    };
                    if read_cell::<Field, _>(fields.get(0).ok_or("Field Cell missing")?)?
                        != Field::from(expected)
                    {
                        return Err(format!(
                            "proven nested expression did not write Field {expected}"
                        )
                        .into());
                    }
                    Ok(())
                },
            )?;
        }
    }
    if let Some(set_root) = set_root {
        let set_root = Path::new(&set_root);
        for circuit in [
            "add",
            "contains",
            "remove",
            "seen_size",
            "seen_is_empty",
            "add_field",
            "contains_field",
            "reset_fields",
        ] {
            let initial = set_contract::initial_state(ConstructorContext::new(()))?;
            let deploy = make_deploy(
                set_root,
                circuit,
                initial.ledger_state.get_ref().clone(),
                &mut rng,
            )?;
            let context = initial.into_circuit_context(deploy.address());
            let contract = set_contract::Contract::default();
            let call = match circuit {
                "add" => check_generated_trace(
                    set_root,
                    circuit,
                    contract.recording.add(context, true)?,
                    true,
                )?,
                "contains" => {
                    let recorded = contract.recording.contains(context, true)?;
                    if recorded.execution.result {
                        return Err("empty Set unexpectedly contains true".into());
                    }
                    check_generated_trace(set_root, circuit, recorded, true)?
                }
                "remove" => check_generated_trace(
                    set_root,
                    circuit,
                    contract.recording.remove(context, true)?,
                    true,
                )?,
                "seen_size" => {
                    let recorded = contract.recording.seen_size(context)?;
                    if recorded.execution.result.value() != 0 {
                        return Err("empty Set unexpectedly has nonzero size".into());
                    }
                    check_generated_trace(set_root, circuit, recorded, ())?
                }
                "seen_is_empty" => {
                    let recorded = contract.recording.seen_is_empty(context)?;
                    if !recorded.execution.result {
                        return Err("new Set unexpectedly nonempty".into());
                    }
                    check_generated_trace(set_root, circuit, recorded, ())?
                }
                "add_field" => {
                    let key = Field::from(7_u64);
                    check_generated_trace(
                        set_root,
                        circuit,
                        contract.recording.add_field(context, key)?,
                        key,
                    )?
                }
                "contains_field" => {
                    let key = Field::from(7_u64);
                    let recorded = contract.recording.contains_field(context, key)?;
                    if recorded.execution.result {
                        return Err("empty Field Set unexpectedly contains seven".into());
                    }
                    check_generated_trace(set_root, circuit, recorded, key)?
                }
                "reset_fields" => check_generated_trace(
                    set_root,
                    circuit,
                    contract.recording.reset_fields(context)?,
                    (),
                )?,
                _ => unreachable!(),
            };
            check_transaction(set_root, circuit, deploy, call, &mut rng, |contract| {
                let StateValue::Array(fields) = contract.data.get_ref() else {
                    return Err("Set contract state is not an array".into());
                };
                let StateValue::Map(set) = fields.get(0).ok_or("Set field missing")? else {
                    return Err("Set field is not a map".into());
                };
                if set.size() != usize::from(circuit == "add") {
                    return Err("proven Set call produced the wrong size".into());
                }
                let StateValue::Map(fields_set) = fields.get(1).ok_or("Field Set missing")? else {
                    return Err("Field Set is not a map".into());
                };
                if fields_set.size() != usize::from(circuit == "add_field") {
                    return Err("proven Field Set call produced the wrong size".into());
                }
                Ok(())
            })?;
        }
    }
    if let Some(set_oracle_root) = set_oracle_root {
        let set_oracle_root = Path::new(&set_oracle_root);
        let initial = set_oracle_contract::initial_state(ConstructorContext::new(()))?;
        let deploy = make_deploy(
            set_oracle_root,
            "check",
            initial.ledger_state.get_ref().clone(),
            &mut rng,
        )?;
        let context = initial.into_circuit_context(deploy.address());
        let key = Field::from(7_u64);
        let recorded = set_oracle_contract::Contract::default()
            .recording
            .check(context, key)?;
        let call = check_generated_trace(set_oracle_root, "check", recorded, key)?;
        check_transaction(
            set_oracle_root,
            "check",
            deploy,
            call,
            &mut rng,
            |contract| {
                let StateValue::Array(fields) = contract.data.get_ref() else {
                    return Err("Set oracle state is not an array".into());
                };
                if read_cell::<bool, _>(fields.get(0).ok_or("Set oracle flag missing")?)? {
                    return Err("proven Set member query unexpectedly found an element".into());
                }
                Ok(())
            },
        )?;
    }
    if let Some(map_root) = map_root {
        let map_root = Path::new(&map_root);
        for circuit in [
            "put",
            "put_pair",
            "put_default",
            "has",
            "remove_key",
            "table_size",
            "table_is_empty",
            "reset_table",
        ] {
            let initial = map_contract::initial_state(ConstructorContext::new(()))?;
            let deploy = make_deploy(
                map_root,
                circuit,
                initial.ledger_state.get_ref().clone(),
                &mut rng,
            )?;
            let context = initial.into_circuit_context(deploy.address());
            let contract = map_contract::Contract::default();
            let call = match circuit {
                "put" => {
                    let recorded = contract.recording.put(context, true, Field::from(42_u64))?;
                    // Independently captured in map-boolean-put-input.json.
                    if !matches!(
                        recorded.public.verify_ops(),
                        [
                            Op::Idx { cached: false, push_path: true, path },
                            Op::Push { storage: false, .. },
                            Op::Push { storage: true, .. },
                            Op::Ins { cached: false, n: 1 },
                            Op::Ins { cached: true, n: 1 },
                        ] if path.len() == 1
                    ) {
                        return Err(
                            "Map put VM operation shape differs from TypeScript oracle".into()
                        );
                    }
                    let manual = check_generated_trace(
                        map_root,
                        circuit,
                        recorded,
                        (true, Field::from(42_u64)),
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
                    let verifier =
                        decode_verifier_key(&fs::read(map_root.join("keys/put.verifier"))?)?;
                    let typed = contract
                        .recording
                        .put_call(&observed, (), true, Field::from(42_u64))?
                        .prepare(verifier, Fr::from(0u64))?;
                    check_observed_call_parity(map_root, "put", &deploy, &manual, &typed)?;
                    typed
                }
                "put_pair" => {
                    let recorded =
                        contract
                            .recording
                            .put_pair(context, true, false, Field::from(42_u64))?;
                    // Two ordered Map insertions captured independently in
                    // map-three-parameter-input.json.
                    if !matches!(
                        recorded.public.verify_ops(),
                        [
                            Op::Idx { cached: false, push_path: true, path: first },
                            Op::Push { storage: false, .. },
                            Op::Push { storage: true, .. },
                            Op::Ins { cached: false, n: 1 },
                            Op::Ins { cached: true, n: 1 },
                            Op::Idx { cached: false, push_path: true, path: second },
                            Op::Push { storage: false, .. },
                            Op::Push { storage: true, .. },
                            Op::Ins { cached: false, n: 1 },
                            Op::Ins { cached: true, n: 1 },
                        ] if first.len() == 1 && second.len() == 1
                    ) {
                        return Err(
                            "Map put_pair VM operation shape differs from TypeScript oracle".into(),
                        );
                    }
                    let manual = check_generated_trace(
                        map_root,
                        circuit,
                        recorded,
                        (true, false, Field::from(42_u64)),
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
                    let verifier =
                        decode_verifier_key(&fs::read(map_root.join("keys/put_pair.verifier"))?)?;
                    let typed = contract
                        .recording
                        .put_pair_call(&observed, (), true, false, Field::from(42_u64))?
                        .prepare(verifier, Fr::from(0_u64))?;
                    check_observed_call_parity(map_root, "put_pair", &deploy, &manual, &typed)?;
                    typed
                }
                "put_default" => check_generated_trace(
                    map_root,
                    circuit,
                    contract.recording.put_default(context, true)?,
                    true,
                )?,
                "has" => {
                    let recorded = contract.recording.has(context, true)?;
                    if recorded.execution.result {
                        return Err("empty Map unexpectedly contains true".into());
                    }
                    check_generated_trace(map_root, circuit, recorded, true)?
                }
                "remove_key" => check_generated_trace(
                    map_root,
                    circuit,
                    contract.recording.remove_key(context, true)?,
                    true,
                )?,
                "table_size" => {
                    let recorded = contract.recording.table_size(context)?;
                    if recorded.execution.result.value() != 0 {
                        return Err("empty Map unexpectedly has nonzero size".into());
                    }
                    check_generated_trace(map_root, circuit, recorded, ())?
                }
                "table_is_empty" => {
                    let recorded = contract.recording.table_is_empty(context)?;
                    if !recorded.execution.result {
                        return Err("new Map unexpectedly nonempty".into());
                    }
                    check_generated_trace(map_root, circuit, recorded, ())?
                }
                "reset_table" => check_generated_trace(
                    map_root,
                    circuit,
                    contract.recording.reset_table(context)?,
                    (),
                )?,
                _ => unreachable!(),
            };
            check_transaction(map_root, circuit, deploy, call, &mut rng, |contract| {
                let StateValue::Array(fields) = contract.data.get_ref() else {
                    return Err("Map contract state is not an array".into());
                };
                let StateValue::Map(table) = fields.get(0).ok_or("Map table missing")? else {
                    return Err("Map table field is not a map".into());
                };
                let expected = match circuit {
                    "put_pair" => 2,
                    "put" | "put_default" => 1,
                    _ => 0,
                };
                if table.size() != expected {
                    return Err("proven Map call produced the wrong size".into());
                }
                Ok(())
            })?;
        }
    }
    if let Some(constructor_map_root) = constructor_map_root {
        let constructor_map_root = Path::new(&constructor_map_root);
        for circuit in [
            "table_size",
            "history_size",
            "get_true",
            "get_false_history",
        ] {
            let initial = constructor_map_contract::initial_state(ConstructorContext::new(()))?;
            let deploy = make_deploy(
                constructor_map_root,
                circuit,
                initial.ledger_state.get_ref().clone(),
                &mut rng,
            )?;
            let context = initial.into_circuit_context(deploy.address());
            let contract = constructor_map_contract::Contract::default();
            let call = match circuit {
                "table_size" => {
                    let recorded = contract.recording.table_size(context)?;
                    if recorded.execution.result.value() != 1 {
                        return Err("constructor Map table size differs".into());
                    }
                    check_generated_trace(constructor_map_root, circuit, recorded, ())?
                }
                "history_size" => {
                    let recorded = contract.recording.history_size(context)?;
                    if recorded.execution.result.value() != 1 {
                        return Err("constructor Map history size differs".into());
                    }
                    check_generated_trace(constructor_map_root, circuit, recorded, ())?
                }
                "get_true" => {
                    let recorded = contract.recording.get_true(context)?;
                    if recorded.execution.result != Field::from(1_u64) {
                        return Err("constructor Map lookup of true differs".into());
                    }
                    check_generated_trace(constructor_map_root, circuit, recorded, ())?
                }
                "get_false_history" => {
                    let recorded = contract.recording.get_false_history(context)?;
                    if recorded.execution.result != Field::from(0_u64) {
                        return Err("constructor Map default lookup differs".into());
                    }
                    check_generated_trace(constructor_map_root, circuit, recorded, ())?
                }
                _ => unreachable!(),
            };
            check_transaction(
                constructor_map_root,
                circuit,
                deploy,
                call,
                &mut rng,
                |contract| {
                    let StateValue::Array(fields) = contract.data.get_ref() else {
                        return Err("constructor Map state is not an array".into());
                    };
                    for index in 0..2 {
                        let StateValue::Map(table) =
                            fields.get(index).ok_or("constructor Map field missing")?
                        else {
                            return Err("constructor Map field is not a map".into());
                        };
                        if table.size() != 1 {
                            return Err("constructor Map read changed state".into());
                        }
                    }
                    Ok(())
                },
            )?;
        }
    }
    if let Some(list_root) = list_root {
        let list_root = Path::new(&list_root);
        for circuit in [
            "item_count",
            "items_empty",
            "first_item",
            "prepend",
            "clear_items",
        ] {
            let initial = list_contract::initial_state(ConstructorContext::new(()))?;
            let deploy = make_deploy(
                list_root,
                circuit,
                initial.ledger_state.get_ref().clone(),
                &mut rng,
            )?;
            let context = initial.into_circuit_context(deploy.address());
            let contract = list_contract::Contract::default();
            let call = match circuit {
                "item_count" => {
                    let recorded = contract.recording.item_count(context)?;
                    if recorded.execution.result.value() != 0 {
                        return Err("empty List unexpectedly has nonzero length".into());
                    }
                    check_generated_trace(list_root, circuit, recorded, ())?
                }
                "items_empty" => {
                    let recorded = contract.recording.items_empty(context)?;
                    if !recorded.execution.result {
                        return Err("new List unexpectedly nonempty".into());
                    }
                    check_generated_trace(list_root, circuit, recorded, ())?
                }
                "first_item" => {
                    let recorded = contract.recording.first_item(context)?;
                    if recorded.execution.result.is_some
                        || recorded.execution.result.value != Field::from(0_u64)
                    {
                        return Err("empty List head differs from Maybe default".into());
                    }
                    check_generated_trace(list_root, circuit, recorded, ())?
                }
                "prepend" => {
                    let value = Field::from(7_u64);
                    check_generated_trace(
                        list_root,
                        circuit,
                        contract.recording.prepend(context, value)?,
                        value,
                    )?
                }
                "clear_items" => check_generated_trace(
                    list_root,
                    circuit,
                    contract.recording.clear_items(context)?,
                    (),
                )?,
                _ => unreachable!(),
            };
            check_transaction(list_root, circuit, deploy, call, &mut rng, |contract| {
                let StateValue::Array(fields) = contract.data.get_ref() else {
                    return Err("List contract state is not an array".into());
                };
                let StateValue::Array(items) = fields.get(0).ok_or("List field missing")? else {
                    return Err("List field is not an array".into());
                };
                let length = read_cell::<u64, _>(items.get(2).ok_or("List length missing")?)?;
                if length != u64::from(circuit == "prepend") {
                    return Err("proven List call produced the wrong length".into());
                }
                Ok(())
            })?;
        }
    }
    if let Some(constructor_list_root) = constructor_list_root {
        let constructor_list_root = Path::new(&constructor_list_root);
        for circuit in [
            "item_count",
            "history_count",
            "first_item",
            "drop_first",
            "clear_items",
        ] {
            let initial = constructor_list_contract::initial_state(ConstructorContext::new(()))?;
            let deploy = make_deploy(
                constructor_list_root,
                circuit,
                initial.ledger_state.get_ref().clone(),
                &mut rng,
            )?;
            let context = initial.into_circuit_context(deploy.address());
            let contract = constructor_list_contract::Contract::default();
            let call = match circuit {
                "item_count" => {
                    let recorded = contract.recording.item_count(context)?;
                    if recorded.execution.result.value() != 1 {
                        return Err("constructor List item count differs".into());
                    }
                    check_generated_trace(constructor_list_root, circuit, recorded, ())?
                }
                "history_count" => {
                    let recorded = contract.recording.history_count(context)?;
                    if recorded.execution.result.value() != 1 {
                        return Err("constructor List history count differs".into());
                    }
                    check_generated_trace(constructor_list_root, circuit, recorded, ())?
                }
                "first_item" => {
                    let recorded = contract.recording.first_item(context)?;
                    if !recorded.execution.result.is_some
                        || recorded.execution.result.value != Field::from(1_u64)
                    {
                        return Err("constructor List head differs".into());
                    }
                    check_generated_trace(constructor_list_root, circuit, recorded, ())?
                }
                "drop_first" => check_generated_trace(
                    constructor_list_root,
                    circuit,
                    contract.recording.drop_first(context)?,
                    (),
                )?,
                "clear_items" => check_generated_trace(
                    constructor_list_root,
                    circuit,
                    contract.recording.clear_items(context)?,
                    (),
                )?,
                _ => unreachable!(),
            };
            check_transaction(
                constructor_list_root,
                circuit,
                deploy,
                call,
                &mut rng,
                |contract| {
                    let StateValue::Array(fields) = contract.data.get_ref() else {
                        return Err("constructor List state is not an array".into());
                    };
                    for index in 0..2 {
                        let StateValue::Array(items) =
                            fields.get(index).ok_or("List field missing")?
                        else {
                            return Err("constructor List field is not an array".into());
                        };
                        let length =
                            read_cell::<u64, _>(items.get(2).ok_or("List length missing")?)?;
                        let expected = u64::from(
                            index != 0 || !matches!(circuit, "drop_first" | "clear_items"),
                        );
                        if length != expected {
                            return Err("proven constructor List call produced wrong length".into());
                        }
                    }
                    Ok(())
                },
            )?;
        }
    }
    if let Some(list_shapes_root) = list_shapes_root.as_ref().map(Path::new) {
        let initial = list_shapes_contract::initial_state(ConstructorContext::new(()))?;
        let deploy = make_deploy(
            list_shapes_root,
            "first_choice",
            initial.ledger_state.get_ref().clone(),
            &mut rng,
        )?;
        let context = initial.into_circuit_context(deploy.address());
        let recorded = list_shapes_contract::Contract::default()
            .recording
            .first_choice(context)?;
        if recorded.execution.result.is_some || recorded.execution.result.value != ListChoice::yes {
            return Err("empty Choice List head did not return the Compact default".into());
        }
        let call = check_generated_trace(list_shapes_root, "first_choice", recorded, ())?;
        check_transaction(
            list_shapes_root,
            "first_choice",
            deploy,
            call,
            &mut rng,
            |contract| {
                let StateValue::Array(fields) = contract.data.get_ref() else {
                    return Err("List shape contract state is not an array".into());
                };
                let StateValue::Array(items) = fields.get(3).ok_or("Choice List field missing")?
                else {
                    return Err("Choice List field is not an array".into());
                };
                if read_cell::<u64, _>(items.get(2).ok_or("Choice List length missing")?)? != 0 {
                    return Err("proven Choice head read changed the List".into());
                }
                Ok(())
            },
        )?;
        let initial = list_shapes_contract::initial_state(ConstructorContext::new(()))?;
        let deploy = make_deploy(
            list_shapes_root,
            "first_packet",
            initial.ledger_state.get_ref().clone(),
            &mut rng,
        )?;
        let context = initial.into_circuit_context(deploy.address());
        let recorded = list_shapes_contract::Contract::default()
            .recording
            .first_packet(context)?;
        if recorded.execution.result.is_some || recorded.execution.result.value != Packet::default()
        {
            return Err("empty packet List head did not return the Compact default".into());
        }
        let call = check_generated_trace(list_shapes_root, "first_packet", recorded, ())?;
        check_transaction(
            list_shapes_root,
            "first_packet",
            deploy,
            call,
            &mut rng,
            |contract| {
                let StateValue::Array(fields) = contract.data.get_ref() else {
                    return Err("List shape contract state is not an array".into());
                };
                let StateValue::Array(items) = fields.get(4).ok_or("packet List field missing")?
                else {
                    return Err("packet List field is not an array".into());
                };
                if read_cell::<u64, _>(items.get(2).ok_or("packet List length missing")?)? != 0 {
                    return Err("proven packet head read changed the List".into());
                }
                Ok(())
            },
        )?;
        for (circuit, index) in [
            ("push_flag", 0),
            ("push_count", 1),
            ("push_tag", 2),
            ("push_choice", 3),
            ("push_packet", 4),
        ] {
            let initial = list_shapes_contract::initial_state(ConstructorContext::new(()))?;
            let deploy = make_deploy(
                list_shapes_root,
                circuit,
                initial.ledger_state.get_ref().clone(),
                &mut rng,
            )?;
            let context = initial.into_circuit_context(deploy.address());
            let contract = list_shapes_contract::Contract::default();
            let call = match circuit {
                "push_flag" => check_generated_trace(
                    list_shapes_root,
                    circuit,
                    contract.recording.push_flag(context, true)?,
                    true,
                )?,
                "push_count" => {
                    let value = BoundedUint::<65535>::new(42)?;
                    check_generated_trace(
                        list_shapes_root,
                        circuit,
                        contract.recording.push_count(context, value)?,
                        value,
                    )?
                }
                "push_tag" => {
                    let value = FixedBytes::new([1, 2, 3]);
                    check_generated_trace(
                        list_shapes_root,
                        circuit,
                        contract.recording.push_tag(context, value)?,
                        value,
                    )?
                }
                "push_choice" => check_generated_trace(
                    list_shapes_root,
                    circuit,
                    contract.recording.push_choice(context, ListChoice::no)?,
                    ListChoice::no,
                )?,
                "push_packet" => {
                    let value = Packet {
                        tag: FixedBytes::new([4, 5, 6]),
                        count: BoundedUint::<65535>::new(7)?,
                    };
                    check_generated_trace(
                        list_shapes_root,
                        circuit,
                        contract.recording.push_packet(context, value.clone())?,
                        value,
                    )?
                }
                _ => unreachable!(),
            };
            check_transaction(
                list_shapes_root,
                circuit,
                deploy,
                call,
                &mut rng,
                |contract| {
                    let StateValue::Array(fields) = contract.data.get_ref() else {
                        return Err("List shape contract state is not an array".into());
                    };
                    let StateValue::Array(items) = fields.get(index).ok_or("List field missing")?
                    else {
                        return Err("List field is not an array".into());
                    };
                    if read_cell::<u64, _>(items.get(2).ok_or("List length missing")?)? != 1 {
                        return Err("proven typed List call produced wrong length".into());
                    }
                    let head = items.get(0).ok_or("List head missing")?;
                    let correct = match circuit {
                        "push_flag" => read_cell::<bool, _>(head)?,
                        "push_count" => {
                            read_cell::<BoundedUint<65535>, _>(head)?
                                == BoundedUint::<65535>::new(42)?
                        }
                        "push_tag" => {
                            read_cell::<FixedBytes<3>, _>(head)? == FixedBytes::new([1, 2, 3])
                        }
                        "push_choice" => read_cell::<ListChoice, _>(head)? == ListChoice::no,
                        "push_packet" => {
                            read_cell::<Packet, _>(head)?
                                == Packet {
                                    tag: FixedBytes::new([4, 5, 6]),
                                    count: BoundedUint::<65535>::new(7)?,
                                }
                        }
                        _ => unreachable!(),
                    };
                    if !correct {
                        return Err("proven typed List call wrote wrong head".into());
                    }
                    Ok(())
                },
            )?;
        }
    }
    if let Some(enum_cell_root) = enum_cell_root.as_ref().map(Path::new) {
        let initial = enum_cell_contract::initial_state(ConstructorContext::new(()))?;
        let deploy = make_deploy(
            enum_cell_root,
            "choose",
            initial.ledger_state.get_ref().clone(),
            &mut rng,
        )?;
        let context = initial.into_circuit_context(deploy.address());
        let recorded = enum_cell_contract::Contract::default()
            .recording
            .choose(context, Choice::no)?;
        let call = check_generated_trace(enum_cell_root, "choose", recorded, Choice::no)?;
        check_transaction(
            enum_cell_root,
            "choose",
            deploy,
            call,
            &mut rng,
            |contract| {
                let StateValue::Array(fields) = contract.data.get_ref() else {
                    return Err("enum Cell state is not an array".into());
                };
                if read_cell::<Choice, _>(fields.get(0).ok_or("enum Cell missing")?)? != Choice::no
                {
                    return Err("proven enum Cell call did not write Choice::no".into());
                }
                Ok(())
            },
        )?;
    }
    if let Some(tiny_root) = tiny_root.as_ref().map(Path::new) {
        let witness = TinySecret;
        let initial = tiny_contract::initial_state(
            ConstructorContext::new(()),
            &witness,
            Field::from(42u64),
        )?;
        let deploy = make_deploy(
            tiny_root,
            "clear",
            initial.ledger_state.get_ref().clone(),
            &mut rng,
        )?;
        let context = initial.into_circuit_context(deploy.address());
        let recorded = tiny_contract::Contract::from(TinySecret)
            .recording()
            .clear(context)?;
        let call = check_generated_trace(tiny_root, "clear", recorded, ())?;
        check_transaction(tiny_root, "clear", deploy, call, &mut rng, |contract| {
            let StateValue::Array(fields) = contract.data.get_ref() else {
                return Err("tiny state is not an array".into());
            };
            if read_cell::<compact_rust_tiny_oracle_fixture::types::STATE, _>(
                fields.get(2).ok_or("tiny state Cell missing")?,
            )? != compact_rust_tiny_oracle_fixture::types::STATE::unset
            {
                return Err("proven tiny clear did not unset the state".into());
            }
            Ok(())
        })?;

        let initial = tiny_contract::initial_state(
            ConstructorContext::new(()),
            &witness,
            Field::from(42u64),
        )?;
        let cleared = tiny_contract::clear(
            initial
                .into_circuit_context(midnight_compact_runtime::ledger::ContractAddress::default()),
            &witness,
        )?;
        let deploy = make_deploy(
            tiny_root,
            "set",
            cleared.context.query.state.get_ref().clone(),
            &mut rng,
        )?;
        let context = cleared
            .context
            .into_constructor_result()
            .into_circuit_context(deploy.address());
        let recorded = tiny_contract::Contract::from(TinySecret)
            .recording()
            .set(context, Field::from(99u64))?;
        let call = check_generated_trace(tiny_root, "set", recorded, Field::from(99u64))?;
        check_transaction(tiny_root, "set", deploy, call, &mut rng, |contract| {
            let StateValue::Array(fields) = contract.data.get_ref() else {
                return Err("tiny state is not an array".into());
            };
            if read_cell::<Field, _>(fields.get(1).ok_or("tiny value Cell missing")?)?
                != Field::from(99u64)
            {
                return Err("proven tiny set did not write 99".into());
            }
            Ok(())
        })?;

        let initial = tiny_contract::initial_state(
            ConstructorContext::new(()),
            &witness,
            Field::from(42u64),
        )?;
        let deploy = make_deploy(
            tiny_root,
            "get",
            initial.ledger_state.get_ref().clone(),
            &mut rng,
        )?;
        let context = initial.into_circuit_context(deploy.address());
        let recorded = tiny_contract::Contract::default().recording.get(context)?;
        let call = check_generated_trace(tiny_root, "get", recorded, ())?;
        check_transaction(tiny_root, "get", deploy, call, &mut rng, |contract| {
            let StateValue::Array(fields) = contract.data.get_ref() else {
                return Err("tiny state is not an array".into());
            };
            if read_cell::<Field, _>(fields.get(1).ok_or("tiny value Cell missing")?)?
                != Field::from(42u64)
            {
                return Err("proven tiny get changed the value".into());
            }
            Ok(())
        })?;

        let initial = tiny_contract::initial_state(
            ConstructorContext::new(()),
            &witness,
            Field::from(42u64),
        )?;
        let cleared = tiny_contract::clear(
            initial
                .into_circuit_context(midnight_compact_runtime::ledger::ContractAddress::default()),
            &witness,
        )?;
        let deploy = make_deploy(
            tiny_root,
            "get",
            cleared.context.query.state.get_ref().clone(),
            &mut rng,
        )?;
        let context = cleared
            .context
            .into_constructor_result()
            .into_circuit_context(deploy.address());
        let recorded = tiny_contract::Contract::default().recording.get(context)?;
        if recorded.execution.result.is_some {
            return Err("tiny get returned Some for an unset value".into());
        }
        let call = check_generated_trace(tiny_root, "get", recorded, ())?;
        check_transaction(tiny_root, "get", deploy, call, &mut rng, |contract| {
            let StateValue::Array(fields) = contract.data.get_ref() else {
                return Err("tiny state is not an array".into());
            };
            if read_cell::<compact_rust_tiny_oracle_fixture::types::STATE, _>(
                fields.get(2).ok_or("tiny state Cell missing")?,
            )? != compact_rust_tiny_oracle_fixture::types::STATE::unset
            {
                return Err("proven tiny absent get changed the state".into());
            }
            Ok(())
        })?;
    }
    if let Some(shape_root) = nested_map_shape_root.as_ref().map(Path::new) {
        let circuit = "check_nested_empty";
        let initial = nested_map_shape_contract::initial_state(ConstructorContext::new(()))?;
        let deploy = make_deploy(
            shape_root,
            circuit,
            initial.ledger_state.get_ref().clone(),
            &mut rng,
        )?;
        let context = initial.into_circuit_context(deploy.address());
        let recorded = nested_map_shape_contract::Contract::default()
            .recording
            .check_nested_empty(context)?;
        if !recorded.execution.result {
            return Err("new nested Map unexpectedly nonempty".into());
        }
        let call = check_generated_trace(shape_root, circuit, recorded, ())?;
        check_transaction(shape_root, circuit, deploy, call, &mut rng, |contract| {
            let StateValue::Array(fields) = contract.data.get_ref() else {
                return Err("nested Map state is not an array".into());
            };
            let StateValue::Map(map) = fields.get(0).ok_or("nested Map field missing")? else {
                return Err("nested Map field has wrong ledger shape".into());
            };
            if map.size() != 0 {
                return Err("proven nested Map shape read changed the Map".into());
            }
            Ok(())
        })?;
    }
    if let Some(merkle_root) = merkle_root.as_ref().map(Path::new) {
        let circuit = "append";
        let initial = merkle_contract::initial_state(ConstructorContext::new(()))?;
        let deploy = make_deploy(
            merkle_root,
            circuit,
            initial.ledger_state.get_ref().clone(),
            &mut rng,
        )?;
        let context = initial.into_circuit_context(deploy.address());
        let value = BoundedUint::<255>::new(7)?;
        let recorded = merkle_contract::Contract::default()
            .recording
            .append(context, value)?;
        let call = check_generated_trace(merkle_root, circuit, recorded, value)?;
        check_transaction(merkle_root, circuit, deploy, call, &mut rng, |contract| {
            let tree = merkle_tree_view_at_path(contract.data.get_ref(), &[0])?;
            let generated = merkle_contract::PublicStateView::from(contract).t()?;
            if generated.root() != tree.root()
                || generated.first_free()? != tree.first_free()?
                || generated.find_path_for_leaf(value).is_some()
                    != tree.find_path_for_leaf(value).is_some()
            {
                return Err("generated proven Merkle view differs from raw view".into());
            }
            if tree.first_free()?.value() != 1 {
                return Err("proven Merkle append did not advance the leaf index".into());
            }
            if tree.find_path_for_leaf(value).is_none() {
                return Err("proven Merkle append did not store the typed leaf".into());
            }
            Ok(())
        })?;

        let circuit = "full";
        let initial = merkle_contract::initial_state(ConstructorContext::new(()))?;
        let deploy = make_deploy(
            merkle_root,
            circuit,
            initial.ledger_state.get_ref().clone(),
            &mut rng,
        )?;
        let context = initial.into_circuit_context(deploy.address());
        let recorded = merkle_contract::Contract::default()
            .recording
            .full(context)?;
        if recorded.execution.result {
            return Err("new plain Merkle tree unexpectedly full".into());
        }
        let call = check_generated_trace(merkle_root, circuit, recorded, ())?;
        check_transaction(merkle_root, circuit, deploy, call, &mut rng, |contract| {
            let tree = merkle_tree_view_at_path(contract.data.get_ref(), &[0])?;
            if tree.first_free()?.value() != 0 {
                return Err("proven plain Merkle fullness read changed the tree".into());
            }
            Ok(())
        })?;
    }
    if let Some(historic_root) = historic_merkle_root.as_ref().map(Path::new) {
        let circuit = "append";
        let initial = historic_merkle_contract::initial_state(ConstructorContext::new(()))?;
        let deploy = make_deploy(
            historic_root,
            circuit,
            initial.ledger_state.get_ref().clone(),
            &mut rng,
        )?;
        let context = initial.into_circuit_context(deploy.address());
        let value = BoundedUint::<255>::new(7)?;
        let recorded = historic_merkle_contract::Contract::default()
            .recording
            .append(context, value)?;
        let call = check_generated_trace(historic_root, circuit, recorded, value)?;
        check_transaction(historic_root, circuit, deploy, call, &mut rng, |contract| {
            let tree = historic_merkle_tree_view_at_path(contract.data.get_ref(), &[0])?;
            let generated = historic_merkle_contract::PublicStateView::from(contract).t()?;
            if generated.root() != tree.root()
                || generated.first_free()? != tree.first_free()?
                || generated.find_path_for_leaf(value).is_some()
                    != tree.find_path_for_leaf(value).is_some()
                || generated.history()? != tree.history()?
            {
                return Err("generated proven historic Merkle view differs from raw view".into());
            }
            if tree.first_free()?.value() != 1 {
                return Err("proven historic append did not advance the leaf index".into());
            }
            if tree.find_path_for_leaf(value).is_none() {
                return Err("proven historic append did not store the typed leaf".into());
            }
            if tree.history()?.len() != 2 {
                return Err("proven historic append did not retain both roots".into());
            }
            Ok(())
        })?;

        let circuit = "full";
        let initial = historic_merkle_contract::initial_state(ConstructorContext::new(()))?;
        let deploy = make_deploy(
            historic_root,
            circuit,
            initial.ledger_state.get_ref().clone(),
            &mut rng,
        )?;
        let context = initial.into_circuit_context(deploy.address());
        let recorded = historic_merkle_contract::Contract::default()
            .recording
            .full(context)?;
        if recorded.execution.result {
            return Err("new historic Merkle tree unexpectedly full".into());
        }
        let call = check_generated_trace(historic_root, circuit, recorded, ())?;
        check_transaction(historic_root, circuit, deploy, call, &mut rng, |contract| {
            let tree = historic_merkle_tree_view_at_path(contract.data.get_ref(), &[0])?;
            if tree.first_free()?.value() != 0 || tree.history()?.len() != 1 {
                return Err("proven historic Merkle fullness read changed the tree".into());
            }
            Ok(())
        })?;
    }
    if let Some(vector_root) = vector_key_root.as_ref().map(Path::new) {
        let circuit = "setInsert";
        let initial = vector_key_contract::initial_state(ConstructorContext::new(()))?;
        let deploy = make_deploy(
            vector_root,
            circuit,
            initial.ledger_state.get_ref().clone(),
            &mut rng,
        )?;
        let context = initial.into_circuit_context(deploy.address());
        let recorded = vector_key_contract::Contract::default()
            .recording
            .setInsert(context)?;
        let call = check_generated_trace(vector_root, circuit, recorded, ())?;
        check_transaction(vector_root, circuit, deploy, call, &mut rng, |contract| {
            let StateValue::Array(fields) = contract.data.get_ref() else {
                return Err("vector-key contract state is not an array".into());
            };
            let StateValue::Map(set) = fields.get(0).ok_or("vector-key Set missing")? else {
                return Err("vector-key Set has the wrong ledger shape".into());
            };
            let key = FixedVector::new([Field::from(0u64), Field::from(1u64)]);
            let view = set_view_at_path::<FixedVector<Field, 2>, _>(contract.data.get_ref(), &[0])?;
            if set.size() != 1 || !view.member(key) {
                return Err("proven vector-key Set insert lost the declared key or size".into());
            }
            Ok(())
        })?;
    }
    if let Some(composite_root) = composite_key_root.as_ref().map(Path::new) {
        let contract = composite_key_contract::Contract::default();
        let vector = FixedVector::new([Field::from(3_u64), Field::from(5_u64)]);
        let pair = (Field::from(42_u64), true);
        let key = CompositeKey {
            vector: vector.clone(),
            pair,
        };
        for (circuit, path) in [
            ("insert_vector", 0_u8),
            ("insert_tuple", 1_u8),
            ("insert_struct", 2_u8),
            ("roundtrip_tuple", 1_u8),
            ("roundtrip_struct", 2_u8),
            ("record_twelve", 3_u8),
        ] {
            let initial = composite_key_contract::initial_state(ConstructorContext::new(()))?;
            let deploy = make_deploy(
                composite_root,
                circuit,
                initial.ledger_state.get_ref().clone(),
                &mut rng,
            )?;
            let context = initial.into_circuit_context(deploy.address());
            let observed = ObservedContractState::new(
                deploy.address(),
                deploy.initial_state.clone(),
                Observation {
                    transaction_hash: [0; 32],
                    block_hash: [0; 32],
                    block_height: 0,
                },
            );
            let verifier = decode_verifier_key(&fs::read(
                composite_root.join(format!("keys/{circuit}.verifier")),
            )?)?;
            let (manual, typed) = match circuit {
                "insert_vector" => {
                    let recorded = contract.recording.insert_vector(context, vector.clone())?;
                    let manual =
                        check_generated_trace(composite_root, circuit, recorded, vector.clone())?;
                    let typed = contract
                        .recording
                        .insert_vector_call(&observed, (), vector.clone())?
                        .prepare(verifier, Fr::from(0_u64))?;
                    (manual, typed)
                }
                "insert_tuple" => {
                    let recorded = contract.recording.insert_tuple(context, pair)?;
                    let manual = check_generated_trace(composite_root, circuit, recorded, pair)?;
                    let typed = contract
                        .recording
                        .insert_tuple_call(&observed, (), pair)?
                        .prepare(verifier, Fr::from(0_u64))?;
                    (manual, typed)
                }
                "insert_struct" => {
                    let recorded = contract.recording.insert_struct(context, key.clone())?;
                    let manual =
                        check_generated_trace(composite_root, circuit, recorded, key.clone())?;
                    let typed = contract
                        .recording
                        .insert_struct_call(&observed, (), key.clone())?
                        .prepare(verifier, Fr::from(0_u64))?;
                    (manual, typed)
                }
                "roundtrip_tuple" => {
                    let recorded = contract.recording.roundtrip_tuple(context, pair)?;
                    if recorded.execution.result {
                        return Err("tuple roundtrip unexpectedly found a removed key".into());
                    }
                    let manual = check_generated_trace(composite_root, circuit, recorded, pair)?;
                    let typed = contract
                        .recording
                        .roundtrip_tuple_call(&observed, (), pair)?
                        .prepare(verifier, Fr::from(0_u64))?;
                    (manual, typed)
                }
                "roundtrip_struct" => {
                    let recorded = contract.recording.roundtrip_struct(context, key.clone())?;
                    if recorded.execution.result {
                        return Err("struct roundtrip unexpectedly found a removed key".into());
                    }
                    let manual =
                        check_generated_trace(composite_root, circuit, recorded, key.clone())?;
                    let typed = contract
                        .recording
                        .roundtrip_struct_call(&observed, (), key.clone())?
                        .prepare(verifier, Fr::from(0_u64))?;
                    (manual, typed)
                }
                "record_twelve" => {
                    let recorded = contract.recording.record_twelve(
                        context,
                        1_u64.into(),
                        2_u64.into(),
                        3_u64.into(),
                        4_u64.into(),
                        5_u64.into(),
                        6_u64.into(),
                        7_u64.into(),
                        8_u64.into(),
                        9_u64.into(),
                        10_u64.into(),
                        11_u64.into(),
                        12_u64.into(),
                    )?;
                    let input = AlignedValue::concat(
                        &(1_u64..=12)
                            .map(|value| AlignedValue::from(Field::from(value)))
                            .collect::<Vec<_>>(),
                    );
                    let manual = check_generated_trace(composite_root, circuit, recorded, input)?;
                    let typed = contract
                        .recording
                        .record_twelve_call(
                            &observed,
                            (),
                            1_u64.into(),
                            2_u64.into(),
                            3_u64.into(),
                            4_u64.into(),
                            5_u64.into(),
                            6_u64.into(),
                            7_u64.into(),
                            8_u64.into(),
                            9_u64.into(),
                            10_u64.into(),
                            11_u64.into(),
                            12_u64.into(),
                        )?
                        .prepare(verifier, Fr::from(0_u64))?;
                    (manual, typed)
                }
                _ => unreachable!(),
            };
            check_observed_call_parity(composite_root, circuit, &deploy, &manual, &typed)?;
            check_transaction(composite_root, circuit, deploy, typed, &mut rng, |state| {
                let StateValue::Array(fields) = state.data.get_ref() else {
                    return Err("composite-key contract state is not an array".into());
                };
                let StateValue::Map(set) = fields
                    .get(path as usize)
                    .ok_or("composite-key Set missing")?
                else {
                    return Err("composite-key Set has the wrong ledger shape".into());
                };
                let present = match circuit {
                    "insert_vector" => {
                        set_view_at_path::<FixedVector<Field, 2>, _>(state.data.get_ref(), &[path])?
                            .member(vector.clone())
                    }
                    "insert_tuple" | "roundtrip_tuple" => {
                        set_view_at_path::<(Field, bool), _>(state.data.get_ref(), &[path])?
                            .member(pair)
                    }
                    "insert_struct" | "roundtrip_struct" => {
                        set_view_at_path::<CompositeKey, _>(state.data.get_ref(), &[path])?
                            .member(key.clone())
                    }
                    "record_twelve" => set_view_at_path::<Field, _>(state.data.get_ref(), &[path])?
                        .member(Field::from(78_u64)),
                    _ => unreachable!(),
                };
                let should_exist = !circuit.starts_with("roundtrip_");
                if set.size() != usize::from(should_exist) || present != should_exist {
                    return Err("proven composite-key Set differs from expected state".into());
                }
                Ok(())
            })?;
        }
    }
    if let Some(chunked_root) = chunked_set_root.as_ref().map(Path::new) {
        let contract = chunked_set_contract::Contract::default();
        let key = (Field::from(42_u64), true);
        for circuit in ["insert_key", "roundtrip_key", "key_count", "empty"] {
            let initial = chunked_set_contract::initial_state(ConstructorContext::new(()))?;
            let deploy = make_deploy(
                chunked_root,
                circuit,
                initial.ledger_state.get_ref().clone(),
                &mut rng,
            )?;
            let context = initial.into_circuit_context(deploy.address());
            let observed = ObservedContractState::new(
                deploy.address(),
                deploy.initial_state.clone(),
                Observation {
                    transaction_hash: [0; 32],
                    block_hash: [0; 32],
                    block_height: 0,
                },
            );
            let verifier = decode_verifier_key(&fs::read(
                chunked_root.join(format!("keys/{circuit}.verifier")),
            )?)?;
            let (manual, typed) = match circuit {
                "insert_key" => {
                    let recorded = contract.recording.insert_key(context, key)?;
                    let manual = check_generated_trace(chunked_root, circuit, recorded, key)?;
                    let typed = contract
                        .recording
                        .insert_key_call(&observed, (), key)?
                        .prepare(verifier, Fr::from(0_u64))?;
                    (manual, typed)
                }
                "roundtrip_key" => {
                    let recorded = contract.recording.roundtrip_key(context, key)?;
                    if recorded.execution.result {
                        return Err("chunked Set roundtrip found a removed key".into());
                    }
                    let manual = check_generated_trace(chunked_root, circuit, recorded, key)?;
                    let typed = contract
                        .recording
                        .roundtrip_key_call(&observed, (), key)?
                        .prepare(verifier, Fr::from(0_u64))?;
                    (manual, typed)
                }
                "key_count" => {
                    let recorded = contract.recording.key_count(context)?;
                    if recorded.execution.result.value() != 0 {
                        return Err("fresh chunked Set count is not zero".into());
                    }
                    let manual = check_generated_trace(chunked_root, circuit, recorded, ())?;
                    let typed = contract
                        .recording
                        .key_count_call(&observed, ())?
                        .prepare(verifier, Fr::from(0_u64))?;
                    (manual, typed)
                }
                "empty" => {
                    let recorded = contract.recording.empty(context)?;
                    if !recorded.execution.result {
                        return Err("fresh chunked Set is not empty".into());
                    }
                    let manual = check_generated_trace(chunked_root, circuit, recorded, ())?;
                    let typed = contract
                        .recording
                        .empty_call(&observed, ())?
                        .prepare(verifier, Fr::from(0_u64))?;
                    (manual, typed)
                }
                _ => unreachable!(),
            };
            check_observed_call_parity(chunked_root, circuit, &deploy, &manual, &typed)?;
            check_transaction(chunked_root, circuit, deploy, typed, &mut rng, |state| {
                let view = set_view_at_path::<(Field, bool), _>(state.data.get_ref(), &[1, 14])?;
                let generated = chunked_set_contract::PublicStateView::from(state).keySet()?;
                let should_exist = circuit == "insert_key";
                if generated.size()? != view.size()? || generated.member(key) != view.member(key) {
                    return Err("generated chunked Set view differs from raw applied state".into());
                }
                if view.size()?.value() != u128::from(should_exist)
                    || view.member(key) != should_exist
                {
                    return Err("proven chunked Set differs from expected state".into());
                }
                Ok(())
            })?;
        }
    }
    if let Some(chunked_root) = chunked_list_root.as_ref().map(Path::new) {
        let contract = chunked_list_contract::Contract::default();
        for circuit in [
            "item_count",
            "items_empty",
            "first_item",
            "prepend",
            "drop_first",
            "clear_items",
        ] {
            let initial = chunked_list_contract::initial_state(ConstructorContext::new(()))?;
            let deploy = make_deploy(
                chunked_root,
                circuit,
                initial.ledger_state.get_ref().clone(),
                &mut rng,
            )?;
            let context = initial.into_circuit_context(deploy.address());
            let observed = ObservedContractState::new(
                deploy.address(),
                deploy.initial_state.clone(),
                Observation {
                    transaction_hash: [0; 32],
                    block_hash: [0; 32],
                    block_height: 0,
                },
            );
            let verifier = decode_verifier_key(&fs::read(
                chunked_root.join(format!("keys/{circuit}.verifier")),
            )?)?;
            let (manual, typed) = match circuit {
                "item_count" => {
                    let recorded = contract.recording.item_count(context)?;
                    if recorded.execution.result.value() != 1 {
                        return Err("chunked List constructor did not insert its item".into());
                    }
                    let manual = check_generated_trace(chunked_root, circuit, recorded, ())?;
                    let typed = contract
                        .recording
                        .item_count_call(&observed, ())?
                        .prepare(verifier, Fr::from(0_u64))?;
                    (manual, typed)
                }
                "items_empty" => {
                    let recorded = contract.recording.items_empty(context)?;
                    if recorded.execution.result {
                        return Err("constructed chunked List is empty".into());
                    }
                    let manual = check_generated_trace(chunked_root, circuit, recorded, ())?;
                    let typed = contract
                        .recording
                        .items_empty_call(&observed, ())?
                        .prepare(verifier, Fr::from(0_u64))?;
                    (manual, typed)
                }
                "first_item" => {
                    let recorded = contract.recording.first_item(context)?;
                    if !recorded.execution.result.is_some
                        || recorded.execution.result.value != Field::from(1_u64)
                    {
                        return Err("chunked List head differs from constructor".into());
                    }
                    let manual = check_generated_trace(chunked_root, circuit, recorded, ())?;
                    let typed = contract
                        .recording
                        .first_item_call(&observed, ())?
                        .prepare(verifier, Fr::from(0_u64))?;
                    (manual, typed)
                }
                "prepend" => {
                    let value = Field::from(7_u64);
                    let recorded = contract.recording.prepend(context, value)?;
                    let manual = check_generated_trace(chunked_root, circuit, recorded, value)?;
                    let typed = contract
                        .recording
                        .prepend_call(&observed, (), value)?
                        .prepare(verifier, Fr::from(0_u64))?;
                    (manual, typed)
                }
                "drop_first" => {
                    let recorded = contract.recording.drop_first(context)?;
                    let manual = check_generated_trace(chunked_root, circuit, recorded, ())?;
                    let typed = contract
                        .recording
                        .drop_first_call(&observed, ())?
                        .prepare(verifier, Fr::from(0_u64))?;
                    (manual, typed)
                }
                "clear_items" => {
                    let recorded = contract.recording.clear_items(context)?;
                    let manual = check_generated_trace(chunked_root, circuit, recorded, ())?;
                    let typed = contract
                        .recording
                        .clear_items_call(&observed, ())?
                        .prepare(verifier, Fr::from(0_u64))?;
                    (manual, typed)
                }
                _ => unreachable!(),
            };
            check_observed_call_parity(chunked_root, circuit, &deploy, &manual, &typed)?;
            check_transaction(chunked_root, circuit, deploy, typed, &mut rng, |state| {
                let view = list_view_at_path::<Field, _>(state.data.get_ref(), &[1, 14])?;
                let generated = chunked_list_contract::PublicStateView::from(state).items()?;
                if generated.head()? != view.head()?
                    || generated.length()? != view.length()?
                    || generated.is_empty() != view.is_empty()
                {
                    return Err("generated proven chunked List view differs from raw view".into());
                }
                let expected_length = match circuit {
                    "prepend" => 2,
                    "drop_first" | "clear_items" => 0,
                    _ => 1,
                };
                if view.length()?.value() != expected_length {
                    return Err("proven chunked List length differs".into());
                }
                let expected_head = match circuit {
                    "prepend" => Some(Field::from(7_u64)),
                    "drop_first" | "clear_items" => None,
                    _ => Some(Field::from(1_u64)),
                };
                if view.head()? != expected_head {
                    return Err("proven chunked List head differs".into());
                }
                Ok(())
            })?;
        }
    }
    if let Some(chunked_root) = chunked_map_root.as_ref().map(Path::new) {
        let contract = chunked_map_contract::Contract::default();
        for circuit in [
            "put",
            "put_pair",
            "put_default",
            "has",
            "get",
            "remove_key",
            "table_size",
            "table_is_empty",
            "reset_table",
        ] {
            let initial = chunked_map_contract::initial_state(ConstructorContext::new(()))?;
            let deploy = make_deploy(
                chunked_root,
                circuit,
                initial.ledger_state.get_ref().clone(),
                &mut rng,
            )?;
            let context = initial.into_circuit_context(deploy.address());
            let observed = ObservedContractState::new(
                deploy.address(),
                deploy.initial_state.clone(),
                Observation {
                    transaction_hash: [0; 32],
                    block_hash: [0; 32],
                    block_height: 0,
                },
            );
            let verifier = decode_verifier_key(&fs::read(
                chunked_root.join(format!("keys/{circuit}.verifier")),
            )?)?;
            let (manual, typed) = match circuit {
                "put" => {
                    let recorded = contract.recording.put(context, false, Field::from(7_u64))?;
                    let manual = check_generated_trace(
                        chunked_root,
                        circuit,
                        recorded,
                        (false, Field::from(7_u64)),
                    )?;
                    let typed = contract
                        .recording
                        .put_call(&observed, (), false, Field::from(7_u64))?
                        .prepare(verifier, Fr::from(0_u64))?;
                    (manual, typed)
                }
                "put_pair" => {
                    let recorded =
                        contract
                            .recording
                            .put_pair(context, true, false, Field::from(42_u64))?;
                    let manual = check_generated_trace(
                        chunked_root,
                        circuit,
                        recorded,
                        (true, false, Field::from(42_u64)),
                    )?;
                    let typed = contract
                        .recording
                        .put_pair_call(&observed, (), true, false, Field::from(42_u64))?
                        .prepare(verifier, Fr::from(0_u64))?;
                    (manual, typed)
                }
                "put_default" => {
                    let recorded = contract.recording.put_default(context, false)?;
                    let manual = check_generated_trace(chunked_root, circuit, recorded, false)?;
                    let typed = contract
                        .recording
                        .put_default_call(&observed, (), false)?
                        .prepare(verifier, Fr::from(0_u64))?;
                    (manual, typed)
                }
                "has" => {
                    let recorded = contract.recording.has(context, true)?;
                    if !recorded.execution.result {
                        return Err("seeded chunked Map key is absent".into());
                    }
                    let manual = check_generated_trace(chunked_root, circuit, recorded, true)?;
                    let typed = contract
                        .recording
                        .has_call(&observed, (), true)?
                        .prepare(verifier, Fr::from(0_u64))?;
                    (manual, typed)
                }
                "get" => {
                    let recorded = contract.recording.get(context, true)?;
                    if recorded.execution.result != Field::from(1_u64) {
                        return Err("seeded chunked Map value differs".into());
                    }
                    let manual = check_generated_trace(chunked_root, circuit, recorded, true)?;
                    let typed = contract
                        .recording
                        .get_call(&observed, (), true)?
                        .prepare(verifier, Fr::from(0_u64))?;
                    (manual, typed)
                }
                "remove_key" => {
                    let recorded = contract.recording.remove_key(context, true)?;
                    let manual = check_generated_trace(chunked_root, circuit, recorded, true)?;
                    let typed = contract
                        .recording
                        .remove_key_call(&observed, (), true)?
                        .prepare(verifier, Fr::from(0_u64))?;
                    (manual, typed)
                }
                "table_size" => {
                    let recorded = contract.recording.table_size(context)?;
                    if recorded.execution.result.value() != 1 {
                        return Err("seeded chunked Map size differs".into());
                    }
                    let manual = check_generated_trace(chunked_root, circuit, recorded, ())?;
                    let typed = contract
                        .recording
                        .table_size_call(&observed, ())?
                        .prepare(verifier, Fr::from(0_u64))?;
                    (manual, typed)
                }
                "table_is_empty" => {
                    let recorded = contract.recording.table_is_empty(context)?;
                    if recorded.execution.result {
                        return Err("seeded chunked Map is empty".into());
                    }
                    let manual = check_generated_trace(chunked_root, circuit, recorded, ())?;
                    let typed = contract
                        .recording
                        .table_is_empty_call(&observed, ())?
                        .prepare(verifier, Fr::from(0_u64))?;
                    (manual, typed)
                }
                "reset_table" => {
                    let recorded = contract.recording.reset_table(context)?;
                    let manual = check_generated_trace(chunked_root, circuit, recorded, ())?;
                    let typed = contract
                        .recording
                        .reset_table_call(&observed, ())?
                        .prepare(verifier, Fr::from(0_u64))?;
                    (manual, typed)
                }
                _ => unreachable!(),
            };
            check_observed_call_parity(chunked_root, circuit, &deploy, &manual, &typed)?;
            check_transaction(chunked_root, circuit, deploy, typed, &mut rng, |state| {
                let view = map_view_at_path::<bool, Field, _>(state.data.get_ref(), &[1, 14])?;
                let generated = chunked_map_contract::PublicStateView::from(state).table()?;
                if generated.size()? != view.size()?
                    || generated.member(true) != view.member(true)
                    || generated.is_empty() != view.is_empty()
                {
                    return Err("generated chunked Map view differs from raw applied state".into());
                }
                if view.member(true) && generated.lookup(true)? != view.lookup(true)? {
                    return Err("generated chunked Map value differs from raw applied state".into());
                }
                let expected_size = match circuit {
                    "put" | "put_pair" | "put_default" => 2,
                    "remove_key" | "reset_table" => 0,
                    _ => 1,
                };
                if view.size()?.value() != expected_size {
                    return Err("proven chunked Map size differs".into());
                }
                let expected_true = match circuit {
                    "put_pair" => Some(Field::from(42_u64)),
                    "remove_key" | "reset_table" => None,
                    _ => Some(Field::from(1_u64)),
                };
                if view.member(true) != expected_true.is_some() {
                    return Err("proven chunked Map membership differs".into());
                }
                if let Some(value) = expected_true
                    && view.lookup(true)? != value
                {
                    return Err("proven chunked Map value differs".into());
                }
                Ok(())
            })?;
        }
    }
    if let Some(chunked_root) = chunked_cell_root.as_ref().map(Path::new) {
        let contract = chunked_cell_contract::Contract::default();
        for circuit in [
            "set_active",
            "get_active",
            "assert_active",
            "set_amount",
            "get_amount",
            "add_amount",
            "active_equals",
            "plus_amount",
            "subtract_amount",
            "multiply_amount",
        ] {
            let initial = chunked_cell_contract::initial_state(ConstructorContext::new(()))?;
            let deploy = make_deploy(
                chunked_root,
                circuit,
                initial.ledger_state.get_ref().clone(),
                &mut rng,
            )?;
            let context = initial.into_circuit_context(deploy.address());
            let observed = ObservedContractState::new(
                deploy.address(),
                deploy.initial_state.clone(),
                Observation {
                    transaction_hash: [0; 32],
                    block_hash: [0; 32],
                    block_height: 0,
                },
            );
            let verifier = decode_verifier_key(&fs::read(
                chunked_root.join(format!("keys/{circuit}.verifier")),
            )?)?;
            let (manual, typed) = match circuit {
                "set_active" => {
                    let recorded = contract.recording.set_active(context, false)?;
                    let manual = check_generated_trace(chunked_root, circuit, recorded, false)?;
                    let typed = contract
                        .recording
                        .set_active_call(&observed, (), false)?
                        .prepare(verifier, Fr::from(0_u64))?;
                    (manual, typed)
                }
                "get_active" => {
                    let recorded = contract.recording.get_active(context)?;
                    if !recorded.execution.result {
                        return Err("seeded chunked Cell Boolean differs".into());
                    }
                    let manual = check_generated_trace(chunked_root, circuit, recorded, ())?;
                    let typed = contract
                        .recording
                        .get_active_call(&observed, ())?
                        .prepare(verifier, Fr::from(0_u64))?;
                    (manual, typed)
                }
                "assert_active" => {
                    let recorded = contract.recording.assert_active(context, true)?;
                    let manual = check_generated_trace(chunked_root, circuit, recorded, true)?;
                    let typed = contract
                        .recording
                        .assert_active_call(&observed, (), true)?
                        .prepare(verifier, Fr::from(0_u64))?;
                    (manual, typed)
                }
                "set_amount" => {
                    let recorded = contract
                        .recording
                        .set_amount(context, Field::from(11_u64))?;
                    let manual = check_generated_trace(
                        chunked_root,
                        circuit,
                        recorded,
                        Field::from(11_u64),
                    )?;
                    let typed = contract
                        .recording
                        .set_amount_call(&observed, (), Field::from(11_u64))?
                        .prepare(verifier, Fr::from(0_u64))?;
                    (manual, typed)
                }
                "get_amount" => {
                    let recorded = contract.recording.get_amount(context)?;
                    if recorded.execution.result != Field::from(3_u64) {
                        return Err("seeded chunked Cell Field differs".into());
                    }
                    let manual = check_generated_trace(chunked_root, circuit, recorded, ())?;
                    let typed = contract
                        .recording
                        .get_amount_call(&observed, ())?
                        .prepare(verifier, Fr::from(0_u64))?;
                    (manual, typed)
                }
                "add_amount" => {
                    let recorded = contract.recording.add_amount(context, Field::from(7_u64))?;
                    let manual =
                        check_generated_trace(chunked_root, circuit, recorded, Field::from(7_u64))?;
                    let typed = contract
                        .recording
                        .add_amount_call(&observed, (), Field::from(7_u64))?
                        .prepare(verifier, Fr::from(0_u64))?;
                    (manual, typed)
                }
                "active_equals" => {
                    let recorded = contract.recording.active_equals(context, true)?;
                    if !recorded.execution.result {
                        return Err("seeded chunked Cell equality differs".into());
                    }
                    let manual = check_generated_trace(chunked_root, circuit, recorded, true)?;
                    let typed = contract
                        .recording
                        .active_equals_call(&observed, (), true)?
                        .prepare(verifier, Fr::from(0_u64))?;
                    (manual, typed)
                }
                "plus_amount" => {
                    let recorded = contract
                        .recording
                        .plus_amount(context, Field::from(7_u64))?;
                    if recorded.execution.result != Field::from(10_u64) {
                        return Err("seeded chunked Cell sum differs".into());
                    }
                    let manual =
                        check_generated_trace(chunked_root, circuit, recorded, Field::from(7_u64))?;
                    let typed = contract
                        .recording
                        .plus_amount_call(&observed, (), Field::from(7_u64))?
                        .prepare(verifier, Fr::from(0_u64))?;
                    (manual, typed)
                }
                "subtract_amount" => {
                    let recorded = contract
                        .recording
                        .subtract_amount(context, Field::from(2_u64))?;
                    let manual =
                        check_generated_trace(chunked_root, circuit, recorded, Field::from(2_u64))?;
                    let typed = contract
                        .recording
                        .subtract_amount_call(&observed, (), Field::from(2_u64))?
                        .prepare(verifier, Fr::from(0_u64))?;
                    (manual, typed)
                }
                "multiply_amount" => {
                    let recorded = contract
                        .recording
                        .multiply_amount(context, Field::from(7_u64))?;
                    let manual =
                        check_generated_trace(chunked_root, circuit, recorded, Field::from(7_u64))?;
                    let typed = contract
                        .recording
                        .multiply_amount_call(&observed, (), Field::from(7_u64))?
                        .prepare(verifier, Fr::from(0_u64))?;
                    (manual, typed)
                }
                _ => unreachable!(),
            };
            check_observed_call_parity(chunked_root, circuit, &deploy, &manual, &typed)?;
            check_transaction(chunked_root, circuit, deploy, typed, &mut rng, |state| {
                let actual_active = read_cell_at_path::<bool, _>(state.data.get_ref(), &[1, 14])?;
                let actual_amount = read_cell_at_path::<Field, _>(state.data.get_ref(), &[1, 13])?;
                let expected_active = circuit != "set_active";
                let expected_amount = match circuit {
                    "set_amount" => Field::from(11_u64),
                    "add_amount" => Field::from(10_u64),
                    "subtract_amount" => Field::from(1_u64),
                    "multiply_amount" => Field::from(21_u64),
                    _ => Field::from(3_u64),
                };
                if actual_active != expected_active || actual_amount != expected_amount {
                    return Err("proven chunked Cell state differs".into());
                }
                Ok(())
            })?;
        }
    }
    if let Some(root) = uints_root.as_ref().map(Path::new) {
        let initial = uints_contract::initial_state(ConstructorContext::new(()))?;
        let deploy = make_deploy(
            root,
            "set_byte",
            initial.ledger_state.get_ref().clone(),
            &mut rng,
        )?;
        let amount = BoundedUint::<255>::new(255)?;
        let contract = uints_contract::Contract::default();
        let recorded = contract
            .recording
            .set_byte(initial.into_circuit_context(deploy.address()), amount)?;
        let manual = check_generated_trace(root, "set_byte", recorded, amount)?;
        let observed = ObservedContractState::new(
            deploy.address(),
            deploy.initial_state.clone(),
            Observation {
                transaction_hash: [0; 32],
                block_hash: [0; 32],
                block_height: 0,
            },
        );
        let verifier = decode_verifier_key(&fs::read(root.join("keys/set_byte.verifier"))?)?;
        let typed = contract
            .recording
            .set_byte_call(&observed, (), amount)?
            .prepare(verifier, Fr::from(0_u64))?;
        check_observed_call_parity(root, "set_byte", &deploy, &manual, &typed)?;
        check_transaction(root, "set_byte", deploy, typed, &mut rng, |state| {
            let byte = read_cell_at_path::<BoundedUint<255>, _>(state.data.get_ref(), &[0])?;
            if byte.value() != 255 {
                return Err("proven Uint<8> Cell write did not persist 255".into());
            }
            Ok(())
        })?;
    }
    if let Some(root) = wide_root.as_ref().map(Path::new) {
        let contract = wide_contract::Contract::from(WideSecret);
        let maximum = Uint248::from_le_bytes(&[0xff; 31])?;
        let initial = wide_contract::initial_state(ConstructorContext::new(7_u64))?;
        let deploy = make_deploy(
            root,
            "writeWide",
            initial.ledger_state.get_ref().clone(),
            &mut rng,
        )?;
        let recorded = contract
            .recording()
            .writeWide(initial.into_circuit_context(deploy.address()))?;
        if recorded.execution.context.private_state != 8_u64 {
            return Err("recorded Uint<248> witness did not advance private state".into());
        }
        let manual = check_generated_trace(root, "writeWide", recorded, ())?;
        let observed = ObservedContractState::new(
            deploy.address(),
            deploy.initial_state.clone(),
            Observation {
                transaction_hash: [0; 32],
                block_hash: [0; 32],
                block_height: 0,
            },
        );
        let verifier = decode_verifier_key(&fs::read(root.join("keys/writeWide.verifier"))?)?;
        let typed = contract
            .recording()
            .writeWide_call(&observed, 7_u64)?
            .prepare(verifier, Fr::from(0_u64))?;
        check_observed_call_parity(root, "writeWide", &deploy, &manual, &typed)?;
        check_transaction(root, "writeWide", deploy, typed, &mut rng, |state| {
            let wide = read_cell_at_path::<Uint248, _>(state.data.get_ref(), &[0])?;
            if wide != maximum {
                return Err("proven Uint<248> witness write did not persist maximum".into());
            }
            Ok(())
        })?;

        // Prove a read against a seeded deployment whose Cell contains the
        // same witness value. The write and read use separate deployments;
        // this check does not imply a chained ledger transaction history.
        let initial = wide_contract::initial_state(ConstructorContext::new(7_u64))?;
        let seeded = wide_contract::writeWide(
            initial.into_circuit_context(Default::default()),
            &WideSecret,
        )?;
        let seed_state = seeded.context.query.state.get_ref().clone();
        let deploy = make_deploy(root, "readWide", seed_state.clone(), &mut rng)?;
        let context = midnight_compact_runtime::context::CircuitContext::from_contract_state(
            7_u64,
            deploy.address(),
            &deploy.initial_state,
        );
        let recorded = contract.recording().readWide(context)?;
        if recorded.execution.result != maximum {
            return Err("recorded Uint<248> Cell read differed from seeded value".into());
        }
        let manual = check_generated_trace(root, "readWide", recorded, ())?;
        let observed = ObservedContractState::new(
            deploy.address(),
            deploy.initial_state.clone(),
            Observation {
                transaction_hash: [0; 32],
                block_hash: [0; 32],
                block_height: 0,
            },
        );
        let verifier = decode_verifier_key(&fs::read(root.join("keys/readWide.verifier"))?)?;
        let typed = contract
            .recording()
            .readWide_call(&observed, 7_u64)?
            .prepare(verifier, Fr::from(0_u64))?;
        check_observed_call_parity(root, "readWide", &deploy, &manual, &typed)?;
        check_transaction(root, "readWide", deploy, typed, &mut rng, |state| {
            if state.data.get_ref() != &seed_state {
                return Err("proven Uint<248> Cell read changed ledger state".into());
            }
            Ok(())
        })?;
    }
    Ok(())
}
