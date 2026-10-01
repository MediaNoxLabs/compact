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
//! The proof uses the fixture's deterministic ZKIR statement and checks
//! artifact compatibility. Separately, the generated counter call's recorded
//! VM program is replayed and partitioned into a ledger PreTranscript. Binding
//! those partitioned transcript inputs to a call proof remains an integration
//! gap. Deployment combines the generated constructor state with the emitted
//! verifier key.

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
use midnight_compact_runtime::ledger::{ContractAddress, DefaultDB};
use midnight_ledger::construct::{PreTranscript, partition_transcripts};
use midnight_ledger::structure::{
    ContractDeploy, INITIAL_PARAMETERS, Intent, LedgerState, ProofPreimageMarker, Transaction,
};
use midnight_ledger::verify::WellFormedStrictness;
use midnight_onchain_state::state::{
    ContractMaintenanceAuthority, ContractOperation, ContractState, EntryPointBuf,
};
use midnight_serialize::tagged_deserialize;
use midnight_storage::storage::HashMap;
use midnight_transient_crypto::commitment::PedersenRandomness;
use midnight_transient_crypto::curve::Fr;
use midnight_transient_crypto::hash::transient_commit;
use midnight_transient_crypto::proofs::{
    KeyLocation, PARAMS_VERIFIER, ProofPreimage, ProvingKeyMaterial, Resolver, VerifierKey,
};
use midnight_zkir::IrSource;
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

fn prove_counter(root: &Path) -> Result<(), Box<dyn Error>> {
    let ir: IrSource = tagged_deserialize(&mut BufReader::new(File::open(
        root.join("zkir/increment.bzkir"),
    )?))?;
    let statement = [0x70u64, 1, 1, 0, 0x0e, 1, 0xa1]
        .into_iter()
        .map(Fr::from)
        .collect::<Vec<_>>();
    let opening = Fr::from(0u64);
    let preimage = ProofPreimage {
        inputs: vec![],
        private_transcript: vec![],
        public_transcript_inputs: statement,
        public_transcript_outputs: vec![],
        binding_input: Fr::from(0u64),
        communications_commitment: Some((transient_commit(&[] as &[Fr], opening), opening)),
        key_location: KeyLocation(Cow::Borrowed("increment")),
    };
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

fn check_generated_counter_trace() -> Result<(), Box<dyn Error>> {
    let constructor = initial_state(ConstructorContext::new(()))?;
    let context = constructor.into_circuit_context(ContractAddress::default());
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
    println!("generated counter trace replayed and partitioned");
    Ok(())
}

fn check_counter_deploy(root: &Path) -> Result<(), Box<dyn Error>> {
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
    let mut rng = StdRng::seed_from_u64(0x434f4d50414354);
    let deploy = ContractDeploy::new(&mut rng, contract);
    let address = deploy.address();
    let intent: Intent<(), ProofPreimageMarker, PedersenRandomness, DefaultDB> =
        Intent::empty(&mut rng, Timestamp::from_secs(0)).add_deploy(deploy);
    let transaction = Transaction::from_intents("local-test", HashMap::new().insert(1_u16, intent));
    let ledger = LedgerState::<DefaultDB>::new("local-test");
    let mut strictness = WellFormedStrictness::default();
    strictness.enforce_balancing = false;
    transaction.well_formed(&ledger, strictness, Timestamp::from_secs(0))?;
    println!("counter deployment validated at address {address:?}");
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let root = env::args_os()
        .nth(1)
        .ok_or("usage: compact-rust-proof-smoke <compiler-output-directory>")?;
    let root = Path::new(&root);
    check_generated_counter_trace()?;
    prove_counter(root)?;
    check_counter_deploy(root)
}
