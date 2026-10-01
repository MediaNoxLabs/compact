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

//! Prove emitted Counter and Cell artifacts in offline ledger-8 transactions.
//!
//! The proof uses the ledger-derived statement from a generated counter VM
//! program and checks it against the fixture's known ZKIR encoding. The call
//! commitment uses the value-field encoding from ledger-8's Intent::add_call.
//! Deployment combines the generated constructor state with the emitted key.

use std::env;
use std::error::Error;
use std::fs::{self, File};
use std::io::{self, BufReader};
use std::path::{Path, PathBuf};

use compact_rust_cell_boolean_fixture::ledger_contract as cell_contract;
use compact_rust_cell_read_fixture::ledger_contract as cell_read_contract;
use compact_rust_counter_fixture::ledger_contract as counter_contract;
use midnight_base_crypto::data_provider::{FetchMode, MidnightDataProvider, OutputMode};
use midnight_base_crypto::time::Timestamp;
use midnight_compact_runtime::context::ConstructorContext;
use midnight_compact_runtime::fab::AlignedValue;
use midnight_compact_runtime::ledger::{DefaultDB, StateValue, read_cell, read_counter};
use midnight_compact_runtime::recording::RecordedCircuitResult;
use midnight_compact_runtime::transaction::{CallSpec, prepare_call};
use midnight_ledger::construct::{ContractCallExt, ContractCallPrototype};
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
    circuit: &'static str,
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

fn check_generated_trace<Output: Into<AlignedValue>>(
    root: &Path,
    circuit: &'static str,
    recorded: RecordedCircuitResult<(), Output>,
) -> Result<ContractCallPrototype<DefaultDB>, Box<dyn Error>> {
    let verifier: VerifierKey = tagged_deserialize(&mut BufReader::new(File::open(
        root.join(format!("keys/{circuit}.verifier")),
    )?))?;
    let call = prepare_call(
        recorded,
        CallSpec::new(circuit, verifier, (), Fr::from(0u64)),
    )?;
    println!("generated {circuit} trace replayed and partitioned");
    Ok(call)
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
    let verified = proven.well_formed(&ledger, strictness, Timestamp::from_secs(0))?;
    let context = TransactionContext {
        ref_state: ledger.clone(),
        block_context: BlockContext::default(),
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
    check_state(&contract)?;
    println!("{circuit} deployment and proven call validated and applied at address {address:?}");
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut arguments = env::args_os().skip(1);
    let counter_root = arguments.next().ok_or(
        "usage: compact-rust-proof-smoke <counter-output> <cell-output> <cell-read-output>",
    )?;
    let cell_root = arguments.next().ok_or(
        "usage: compact-rust-proof-smoke <counter-output> <cell-output> <cell-read-output>",
    )?;
    let cell_read_root = arguments.next().ok_or(
        "usage: compact-rust-proof-smoke <counter-output> <cell-output> <cell-read-output>",
    )?;
    if arguments.next().is_some() {
        return Err(
            "usage: compact-rust-proof-smoke <counter-output> <cell-output> <cell-read-output>"
                .into(),
        );
    }
    let counter_root = Path::new(&counter_root);
    let cell_root = Path::new(&cell_root);
    let cell_read_root = Path::new(&cell_read_root);
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
    let counter_call = check_generated_trace(counter_root, "increment", counter_recorded)?;
    prove_counter(counter_root, &counter_call)?;
    check_transaction(
        counter_root,
        "increment",
        counter_deploy,
        counter_call,
        &mut rng,
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
    let cell_call = check_generated_trace(cell_root, "set_flag", cell_recorded)?;
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
    let cell_read_call = check_generated_trace(cell_read_root, "read_flag", cell_read_recorded)?;
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
    )
}
