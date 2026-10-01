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

//! Prove emitted counter artifacts and validate an offline ledger-8 deployment.
//!
//! The proof uses the ledger-derived statement from a generated counter VM
//! program and checks it against the fixture's known ZKIR encoding. The call
//! commitment uses the value-field encoding from ledger-8's Intent::add_call.
//! Deployment combines the generated constructor state with the emitted key.

use std::borrow::Cow;
use std::env;
use std::error::Error;
use std::fs::{self, File};
use std::io::{self, BufReader};
use std::path::{Path, PathBuf};

use compact_rust_counter_fixture::ledger_contract::{initial_state, recorded};
use midnight_base_crypto::data_provider::{FetchMode, MidnightDataProvider, OutputMode};
use midnight_base_crypto::time::Timestamp;
use midnight_compact_runtime::context::ConstructorContext;
use midnight_compact_runtime::ledger::{ContractAddress, DefaultDB, StateValue, read_counter};
use midnight_ledger::construct::{
    ContractCallExt, ContractCallPrototype, PreTranscript, partition_transcripts,
};
use midnight_ledger::semantics::{TransactionContext, TransactionResult};
use midnight_ledger::structure::{
    ContractDeploy, INITIAL_PARAMETERS, Intent, LedgerState, ProofPreimageMarker,
    ProofPreimageVersioned, Transaction,
};
use midnight_ledger::verify::WellFormedStrictness;
use midnight_onchain_runtime::context::BlockContext;
use midnight_onchain_state::state::{
    ContractMaintenanceAuthority, ContractOperation, ContractState, EntryPointBuf,
};
use midnight_serialize::tagged_deserialize;
use midnight_storage::storage::HashMap;
use midnight_transient_crypto::commitment::PedersenRandomness;
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
}

impl Resolver for ArtifactResolver {
    async fn resolve_key(&self, location: KeyLocation) -> io::Result<Option<ProvingKeyMaterial>> {
        if location.0.as_ref() != "increment" {
            return Ok(None);
        }
        Ok(Some(ProvingKeyMaterial {
            prover_key: fs::read(self.root.join("keys/increment.prover"))?,
            verifier_key: fs::read(self.root.join("keys/increment.verifier"))?,
            ir_source: fs::read(self.root.join("zkir/increment.bzkir"))?,
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

fn check_generated_counter_trace(
    root: &Path,
    address: ContractAddress,
) -> Result<ContractCallPrototype<DefaultDB>, Box<dyn Error>> {
    let constructor = initial_state(ConstructorContext::new(()))?;
    let context = constructor.into_circuit_context(address);
    let recorded = recorded::increment(context)?;
    let replay = recorded.public.initial().query(
        recorded.public.verify_ops(),
        None,
        &recorded.execution.context.cost_model,
    )?;
    if replay.context.effects != recorded.execution.context.query.effects {
        return Err("generated counter replay effects differ from native execution".into());
    }
    let (context, program) = recorded.public.into_parts();
    let transcripts = partition_transcripts(
        &[PreTranscript {
            context,
            program,
            comm_comm: None,
        }],
        &INITIAL_PARAMETERS,
    )?;
    let (guaranteed, fallible) = transcripts
        .first()
        .ok_or("counter trace did not partition")?;
    let transcript = guaranteed
        .as_ref()
        .or(fallible.as_ref())
        .ok_or("counter trace has no partitioned transcript")?;
    if transcript.effects != replay.context.effects {
        return Err("partitioned counter effects differ from replay".into());
    }
    let verifier: VerifierKey = tagged_deserialize(&mut BufReader::new(File::open(
        root.join("keys/increment.verifier"),
    )?))?;
    println!("generated counter trace replayed and partitioned");
    Ok(ContractCallPrototype {
        address,
        entry_point: EntryPointBuf(b"increment".to_vec()),
        op: ContractOperation::new(Some(verifier)),
        guaranteed_public_transcript: guaranteed.clone(),
        fallible_public_transcript: fallible.clone(),
        private_transcript_outputs: recorded.execution.private_transcript_outputs,
        input: ().into(),
        output: ().into(),
        communication_commitment_rand: Fr::from(0u64),
        key_location: KeyLocation(Cow::Borrowed("increment")),
    })
}

fn make_counter_deploy(
    root: &Path,
    rng: &mut StdRng,
) -> Result<ContractDeploy<DefaultDB>, Box<dyn Error>> {
    let verifier: VerifierKey = tagged_deserialize(&mut BufReader::new(File::open(
        root.join("keys/increment.verifier"),
    )?))?;
    let initial = initial_state(ConstructorContext::new(()))?;
    let operations = HashMap::new().insert(
        EntryPointBuf(b"increment".to_vec()),
        ContractOperation::new(Some(verifier)),
    );
    let contract: ContractState<DefaultDB> = ContractState::new(
        initial.ledger_state.get_ref().clone(),
        operations,
        ContractMaintenanceAuthority::default(),
    );
    Ok(ContractDeploy::new(rng, contract))
}

fn check_counter_transaction(
    root: &Path,
    deploy: ContractDeploy<DefaultDB>,
    call: ContractCallPrototype<DefaultDB>,
    rng: &mut StdRng,
) -> Result<(), Box<dyn Error>> {
    let address = deploy.address();
    let deploy_intent: Intent<(), ProofPreimageMarker, PedersenRandomness, DefaultDB> =
        Intent::empty(rng, Timestamp::from_secs(0)).add_deploy(deploy.clone());
    let deploy_tx =
        Transaction::from_intents("local-test", HashMap::new().insert(1_u16, deploy_intent));
    let mut ledger = LedgerState::<DefaultDB>::new("local-test");
    let mut strictness = WellFormedStrictness::default();
    strictness.enforce_balancing = false;
    deploy_tx.well_formed(&ledger, strictness, Timestamp::from_secs(0))?;

    // Model the state after the separately validated deployment. The call is
    // checked against a ledger that contains its target contract.
    ledger.contract = ledger.contract.insert(address, deploy.initial_state);
    let call_intent: Intent<(), ProofPreimageMarker, PedersenRandomness, DefaultDB> =
        Intent::empty(rng, Timestamp::from_secs(0)).add_call::<ProofPreimage>(call);
    let call_tx =
        Transaction::from_intents("local-test", HashMap::new().insert(1_u16, call_intent));
    call_tx.well_formed(&ledger, strictness, Timestamp::from_secs(0))?;
    let resolver = ArtifactResolver {
        root: root.to_owned(),
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
    let verified = proven.well_formed(&ledger, strictness, Timestamp::from_secs(0))?;
    let context = TransactionContext {
        ref_state: ledger.clone(),
        block_context: BlockContext::default(),
        whitelist: None,
    };
    let (updated, outcome) = ledger.apply(&verified, &context);
    if !matches!(outcome, TransactionResult::Success(_)) {
        return Err(format!("counter call application failed: {outcome:?}").into());
    }
    let contract = updated
        .contract
        .get(&address)
        .ok_or("deployed contract disappeared")?;
    let StateValue::Array(fields) = contract.data.get_ref() else {
        return Err("counter contract state is not an array".into());
    };
    if read_counter(fields.get(0).ok_or("counter field missing")?)? != 1 {
        return Err("proven counter call did not increment ledger state".into());
    }
    println!("counter deployment and proven call validated and applied at address {address:?}");
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let root = env::args_os()
        .nth(1)
        .ok_or("usage: compact-rust-proof-smoke <compiler-output-directory>")?;
    let root = Path::new(&root);
    let mut rng = StdRng::seed_from_u64(0x434f4d50414354);
    let deploy = make_counter_deploy(root, &mut rng)?;
    let call = check_generated_counter_trace(root, deploy.address())?;
    prove_counter(root, &call)?;
    check_counter_transaction(root, deploy, call, &mut rng)
}
