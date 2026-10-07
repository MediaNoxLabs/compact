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

use midnight_base_crypto::{
    data_provider::{FetchMode, MidnightDataProvider, OutputMode},
    signatures::Signature,
    time::Timestamp,
};
use midnight_compact_runtime as r;
use midnight_ledger::{
    construct::{ContractCallExt, ContractCallPrototype},
    dust::{DUST_EXPECTED_FILES, DustResolver},
    prove::Resolver as LedgerResolver,
    semantics::{TransactionContext, TransactionResult},
    structure::{
        ContractDeploy, INITIAL_PARAMETERS, Intent, ProofPreimageMarker, ProofPreimageVersioned,
        Transaction,
    },
    test_utilities::TestState,
    verify::WellFormedStrictness,
};
use midnight_onchain_runtime::context::BlockContext;
use midnight_onchain_state::state::{
    ContractMaintenanceAuthority, ContractOperation, ContractState, EntryPointBuf,
};
use midnight_serialize::{tagged_deserialize, tagged_serialize};
use midnight_storage::storage::HashMap;
use midnight_transient_crypto::{
    commitment::PedersenRandomness,
    curve::Fr,
    fab::AlignedValueExt,
    hash::transient_commit,
    proofs::{
        KeyLocation, PARAMS_VERIFIER, ProofPreimage, ProverKey, ProvingKeyMaterial, Resolver,
        VerifierKey, Zkir,
    },
};
use midnight_zkir::{IrSource, LocalProvingProvider};
use midnight_zswap::prove::ZswapResolver;
use r::transaction::{CallSpec, Observation, ObservedContractState, prepare_call};
use rand::{SeedableRng, rngs::StdRng};
use rand_chacha::ChaCha20Rng;
use std::{
    error::Error,
    fs::{self, File},
    io::{self, BufReader},
    path::{Path, PathBuf},
};

#[path = "did_primitive_reducers/cases.rs"]
mod cases;
struct ArtifactResolver {
    root: PathBuf,
    operations: &'static [&'static str],
}
impl Resolver for ArtifactResolver {
    async fn resolve_key(&self, location: KeyLocation) -> io::Result<Option<ProvingKeyMaterial>> {
        if !self.operations.contains(&location.0.as_ref()) {
            return Ok(None);
        }
        Ok(Some(ProvingKeyMaterial {
            prover_key: fs::read(self.root.join(format!("keys/{}.prover", location.0)))?,
            verifier_key: fs::read(self.root.join(format!("keys/{}.verifier", location.0)))?,
            ir_source: fs::read(self.root.join(format!("zkir/{}.bzkir", location.0)))?,
        }))
    }
}
fn fee_resolver(
    root: &Path,
    operations: &'static [&'static str],
) -> Result<LedgerResolver, Box<dyn Error>> {
    let zswap = ZswapResolver(MidnightDataProvider::new(
        FetchMode::Synchronous,
        OutputMode::Log,
        midnight_zswap::ZSWAP_EXPECTED_FILES.to_owned(),
    )?);
    let dust = DustResolver(MidnightDataProvider::new(
        FetchMode::Synchronous,
        OutputMode::Log,
        DUST_EXPECTED_FILES.to_owned(),
    )?);
    let root = root.to_owned();
    Ok(LedgerResolver::new(
        zswap,
        dust,
        Box::new(move |location| {
            let resolver = ArtifactResolver {
                root: root.clone(),
                operations,
            };
            Box::pin(async move { resolver.resolve_key(location).await })
        }),
    ))
}
fn direct_proof(
    root: &Path,
    operation: &str,
    call: &ContractCallPrototype<r::ledger::DefaultDB>,
    verifier: &VerifierKey,
) -> Result<usize, Box<dyn Error>> {
    let mut fields = Vec::new();
    call.input.value_only_field_repr(&mut fields);
    call.output.value_only_field_repr(&mut fields);
    let commitment = transient_commit(&fields, call.communication_commitment_rand);
    let preimage = match <ProofPreimage as ContractCallExt<r::ledger::DefaultDB>>::construct_proof(
        call, commitment,
    ) {
        ProofPreimageVersioned::V2(preimage) => preimage,
        _ => return Err("unexpected preimage version".into()),
    };
    let ir: IrSource = tagged_deserialize(&mut BufReader::new(File::open(
        root.join(format!("zkir/{operation}.bzkir")),
    )?))?;
    let pk: ProverKey<IrSource> = tagged_deserialize(&mut BufReader::new(File::open(
        root.join(format!("keys/{operation}.prover")),
    )?))?;
    let skips = preimage.check(&ir)?;
    let params = MidnightDataProvider::new(FetchMode::OnDemand, OutputMode::Log, vec![])?;
    let (proof, mut inputs, proved_skips) = futures_executor::block_on(ir.prove(
        ChaCha20Rng::from_seed([0x88; 32]),
        &params,
        pk,
        &preimage,
    ))?;
    if skips != proved_skips {
        return Err("proof skip mismatch".into());
    }
    verifier.verify(&PARAMS_VERIFIER, &proof, inputs.iter().copied())?;
    inputs[0] = if inputs[0] == Fr::from(0u64) {
        Fr::from(1u64)
    } else {
        Fr::from(0u64)
    };
    if verifier
        .verify(&PARAMS_VERIFIER, &proof, inputs.iter().copied())
        .is_ok()
    {
        return Err("changed binding accepted".into());
    }
    Ok(proof.0.len())
}
pub(crate) fn run(root: &Path, kind: &str) -> Result<(), Box<dyn Error>> {
    let fixture = cases::fixture(kind)?;
    let mut rng = StdRng::seed_from_u64(0x294);
    let constructor_data = (fixture.initial)()?.ledger_state.get_ref().clone();
    let mut operations = HashMap::new();
    for name in fixture.operations {
        let verifier: VerifierKey = tagged_deserialize(&mut BufReader::new(File::open(
            root.join(format!("keys/{name}.verifier")),
        )?))?;
        operations = operations.insert(
            EntryPointBuf(name.as_bytes().to_vec()),
            ContractOperation::new(Some(verifier)),
        );
    }
    let deploy = ContractDeploy::new(
        &mut rng,
        ContractState::new(
            constructor_data.clone(),
            operations,
            ContractMaintenanceAuthority::default(),
        ),
    );
    let address = deploy.address();
    let mut state = TestState::<r::ledger::DefaultDB>::new(&mut rng);
    let async_runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    async_runtime.block_on(state.give_fee_token(&mut rng, 4));
    let resolver = fee_resolver(root, fixture.operations)?;
    let params = MidnightDataProvider::new(FetchMode::OnDemand, OutputMode::Log, vec![])?;
    let intent: Intent<Signature, ProofPreimageMarker, PedersenRandomness, r::ledger::DefaultDB> =
        Intent::empty(&mut rng, Timestamp::from_secs(state.time.to_secs() + 3600))
            .add_deploy(deploy);
    let tx = Transaction::from_intents("local-test", HashMap::new().insert(1u16, intent));
    let provider = LocalProvingProvider {
        rng: StdRng::seed_from_u64(0x295),
        resolver: &resolver,
        params: &params,
    };
    let proven = futures_executor::block_on(
        tx.prove(provider, &INITIAL_PARAMETERS.cost_model.runtime_cost_model),
    )?;
    let balanced = async_runtime.block_on(state.balance_tx(
        rng.clone(),
        proven.seal(StdRng::seed_from_u64(0x296)),
        &resolver,
    ))?;
    let applied = state.apply(&balanced, WellFormedStrictness::default())?;
    if !matches!(applied, TransactionResult::Success(_)) {
        return Err(format!("deployment failed: {applied:?}").into());
    }
    if state
        .ledger
        .contract
        .get(&address)
        .ok_or("missing deployment")?
        .data
        .get_ref()
        != &constructor_data
    {
        return Err("constructor data mismatch".into());
    }
    let mut results = vec![];
    for (index, selected) in fixture.cases.iter().enumerate() {
        let case = selected.id;
        let operation = selected.operation;
        let verifier: VerifierKey = tagged_deserialize(&mut BufReader::new(File::open(
            root.join(format!("keys/{operation}.verifier")),
        )?))?;
        let prior = state
            .ledger
            .contract
            .get(&address)
            .ok_or("missing contract")?;
        let observed = ObservedContractState::new(
            address,
            prior.clone(),
            Observation {
                transaction_hash: [0; 32],
                block_hash: [0; 32],
                block_height: 1,
            },
        );
        let (recorded, input) = (fixture.call)(selected, &observed)?;
        let expected = recorded.execution.context.query.state.clone();
        let prepared = prepare_call(
            recorded,
            CallSpec::new(operation, verifier.clone(), input, Fr::from(0u64)),
        )?;
        let proof_bytes = direct_proof(root, operation, &prepared, &verifier)?;
        let intent: Intent<
            Signature,
            ProofPreimageMarker,
            PedersenRandomness,
            r::ledger::DefaultDB,
        > = Intent::empty(&mut rng, Timestamp::from_secs(state.time.to_secs() + 3600))
            .add_call::<ProofPreimage>(prepared);
        let tx = Transaction::from_intents("local-test", HashMap::new().insert(1u16, intent));
        let provider = LocalProvingProvider {
            rng: StdRng::seed_from_u64(0x297 + index as u64),
            resolver: &resolver,
            params: &params,
        };
        let proven = futures_executor::block_on(
            tx.prove(provider, &INITIAL_PARAMETERS.cost_model.runtime_cost_model),
        )?;
        let balanced = async_runtime.block_on(state.balance_tx(
            rng.clone(),
            proven.seal(StdRng::seed_from_u64(0x298 + index as u64)),
            &resolver,
        ))?;
        let time = state.time;
        let result = state.apply(&balanced, WellFormedStrictness::default())?;
        if !matches!(result, TransactionResult::Success(_)) {
            return Err(format!("strict call failed: {result:?}").into());
        }
        let after = state
            .ledger
            .contract
            .get(&address)
            .ok_or("contract disappeared")?;
        if after.data != expected {
            return Err("applied state does not match recorded state".into());
        }
        let saved = state.ledger.clone();
        let verified =
            balanced.well_formed(&state.ledger, WellFormedStrictness::default(), time)?;
        let context = TransactionContext {
            ref_state: state.ledger.clone(),
            block_context: BlockContext {
                tblock: time,
                last_block_time: time,
                ..BlockContext::default()
            },
            whitelist: None,
        };
        let (replayed, replay) = state.ledger.apply(&verified, &context);
        if !matches!(
            replay,
            TransactionResult::Failure(
                midnight_ledger::error::TransactionInvalid::ReplayProtectionViolation(
                    midnight_ledger::error::TransactionApplicationError::IntentAlreadyExists
                )
            )
        ) {
            return Err(format!("replay outcome: {replay:?}").into());
        }
        if replayed != saved {
            return Err("replay mutated state".into());
        }
        let mut bytes = vec![];
        tagged_serialize(&after, &mut bytes)?;
        fs::write(root.join(format!("{case}-state.bin")), &bytes)?;
        results.push(serde_json::json!({"case":case,"operation":operation,"proof_bytes":proof_bytes,"changed_binding_rejected":true,"applied":true,"recorded_state_matches":true,"replay_refusal":"IntentAlreadyExists","replay_unchanged":true,"state_bytes":bytes.len()}));
    }
    let result = serde_json::json!({"format":"compact-did-primitive-reducer-proof/v1","kind":kind,"strictness":"default","constructor_data_deployed":true,"constructor_execution_proved":false,"status":"passed","calls":results});
    fs::write(
        root.join("proof-result.json"),
        serde_json::to_vec_pretty(&result)?,
    )?;
    println!("{result}");
    Ok(())
}
