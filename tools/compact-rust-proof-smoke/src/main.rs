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

//! Prove emitted Counter, Cell, Set, Map, List, and enum Cell artifacts in offline ledger-8 transactions.
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
use compact_rust_constructor_list_actions_fixture::ledger_contract as constructor_list_contract;
use compact_rust_constructor_map_actions_fixture::ledger_contract as constructor_map_contract;
use compact_rust_counter_fixture::ledger_contract as counter_contract;
use compact_rust_list_field_fixture::ledger_contract as list_contract;
use compact_rust_map_boolean_field_fixture::ledger_contract as map_contract;
use compact_rust_nested_witness_call_oracle_fixture::ledger_contract as expression_contract;
use compact_rust_recorded_enum_cell_fixture::ledger_contract as enum_cell_contract;
use compact_rust_recorded_enum_cell_fixture::types::Choice;
use compact_rust_set_boolean_fixture::ledger_contract as set_contract;
use compact_rust_set_oracle_fixture::ledger_contract as set_oracle_contract;
use compact_rust_stateful_circuit_call_fixture::ledger_contract as nested_contract;
use compact_rust_witness_cell_write_fixture::ledger_contract as witness_contract;
use midnight_base_crypto::data_provider::{FetchMode, MidnightDataProvider, OutputMode};
use midnight_base_crypto::time::Timestamp;
use midnight_compact_runtime::BoundedUint;
use midnight_compact_runtime::Field;
use midnight_compact_runtime::context::{ConstructorContext, WitnessContext};
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

impl expression_contract::Witnesses<u64> for NestedSecret {
    fn secret(
        &self,
        context: WitnessContext<'_, u64, expression_contract::LedgerView<'_>>,
    ) -> (u64, Field) {
        (*context.private_state + 1, Field::from(7_u64))
    }
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
    if arguments.next().is_some() {
        return Err(
            "usage: compact-rust-proof-smoke <counter-output> <cell-output> <cell-read-output> [witness-output] [nested-output] [nested-witness-output] [set-output] [set-oracle-output] [map-output] [constructor-map-output] [list-output] [constructor-list-output] [enum-cell-output]"
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
    let counter_call = check_generated_trace(counter_root, "increment", counter_recorded, ())?;
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
        let parameterized_call = check_generated_trace(nested_root, "add_twice", recorded, amount)?;
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
        for circuit in ["outer", "outerValue"] {
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
                _ => unreachable!(),
            };
            if recorded.execution.private_transcript_outputs.len() != 1 {
                return Err(format!("{circuit} did not record one private value").into());
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
                    if read_cell::<Field, _>(fields.get(0).ok_or("Field Cell missing")?)?
                        != Field::from(7_u64)
                    {
                        return Err("proven nested expression did not write Field 7".into());
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
                "put" => check_generated_trace(
                    map_root,
                    circuit,
                    contract.recording.put(context, true, Field::from(42_u64))?,
                    (true, Field::from(42_u64)),
                )?,
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
                let expected = usize::from(matches!(circuit, "put" | "put_default"));
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
    Ok(())
}
