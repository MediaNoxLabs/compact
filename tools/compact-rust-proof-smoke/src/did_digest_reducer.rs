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

use compact_rust_did_digest_read_reducer_fixture::{
    ledger_contract as c, ledger_slots, runtime as r,
};
use midnight_base_crypto::{
    data_provider::{FetchMode, MidnightDataProvider, OutputMode},
    signatures::Signature,
    time::Timestamp,
};
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
use r::{
    context::{ConstructorContext, WitnessContext},
    fab::AlignedValue,
    transaction::{CallSpec, Observation, ObservedContractState, prepare_call},
};
use rand::{SeedableRng, rngs::StdRng};
use rand_chacha::ChaCha20Rng;
use std::{
    error::Error,
    fs::{self, File},
    io::{self, BufReader},
    path::{Path, PathBuf},
};

struct Witness;
impl c::TryWitnesses<u64> for Witness {
    fn admitted(
        &self,
        context: WitnessContext<'_, u64, c::LedgerView<'_>>,
    ) -> Result<(u64, bool), r::CompactError> {
        Ok((*context.private_state + 1, true))
    }
}
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
fn fee_resolver(root: &Path, circuit: &'static str) -> Result<LedgerResolver, Box<dyn Error>> {
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
                circuit,
            };
            Box::pin(async move { resolver.resolve_key(location).await })
        }),
    ))
}
fn direct_proof(
    root: &Path,
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
        root.join("zkir/verify.bzkir"),
    )?))?;
    let pk: ProverKey<IrSource> = tagged_deserialize(&mut BufReader::new(File::open(
        root.join("keys/verify.prover"),
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
pub(super) fn run(root: &Path) -> Result<(), Box<dyn Error>> {
    let mut rng = StdRng::seed_from_u64(0x288);
    let point = r::ec_mul_generator(r::Field::from(5u64))?;
    let id = r::OpaqueString("reducer-renamed".to_owned());
    let initial = c::initial_state(ConstructorContext::new(0u64), id.clone(), point)?;
    let constructor_data = initial.ledger_state.get_ref().clone();
    if !ledger_slots::active.inspect(&constructor_data)?
        || ledger_slots::methods.inspect(&constructor_data)?.is_empty()
    {
        return Err("constructor did not seed active declared Map".into());
    }
    let verifier: VerifierKey = tagged_deserialize(&mut BufReader::new(File::open(
        root.join("keys/verify.verifier"),
    )?))?;
    let operations = HashMap::new().insert(
        EntryPointBuf(b"verify".to_vec()),
        ContractOperation::new(Some(verifier.clone())),
    );
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
    let intent: Intent<Signature, ProofPreimageMarker, PedersenRandomness, r::ledger::DefaultDB> =
        Intent::empty(&mut rng, Timestamp::from_secs(state.time.to_secs() + 3600))
            .add_deploy(deploy);
    let tx = Transaction::from_intents("local-test", HashMap::new().insert(1u16, intent));
    let resolver = fee_resolver(root, "verify")?;
    let params = MidnightDataProvider::new(FetchMode::OnDemand, OutputMode::Log, vec![])?;
    let provider = LocalProvingProvider {
        rng: StdRng::seed_from_u64(0x289),
        resolver: &resolver,
        params: &params,
    };
    let proven = futures_executor::block_on(
        tx.prove(provider, &INITIAL_PARAMETERS.cost_model.runtime_cost_model),
    )?;
    let balanced = async_runtime.block_on(state.balance_tx(
        rng.clone(),
        proven.seal(StdRng::seed_from_u64(0x28a)),
        &resolver,
    ))?;
    let applied = state.apply(&balanced, WellFormedStrictness::default())?;
    if !matches!(applied, TransactionResult::Success(_)) {
        return Err(format!("deployment failed: {applied:?}").into());
    }
    let prior = state
        .ledger
        .contract
        .get(&address)
        .ok_or("deployed contract absent")?;
    if prior.data.get_ref() != &constructor_data {
        return Err("deployment changed constructor data".into());
    }
    let observed = ObservedContractState::new(
        address,
        prior.clone(),
        Observation {
            transaction_hash: [0; 32],
            block_hash: [0; 32],
            block_height: 1,
        },
    );
    let witness = Witness;
    let native = c::verify(observed.circuit_context(0u64), &witness, id.clone(), point)?;
    let recorded =
        c::recorded::verify(observed.circuit_context(0u64), &witness, id.clone(), point)?;
    if native.context.query.state != recorded.execution.context.query.state
        || native.gas_cost != recorded.execution.gas_cost
        || native.private_transcript_outputs != recorded.execution.private_transcript_outputs
    {
        return Err("native/recorded mismatch".into());
    }
    let input = AlignedValue::from((id.clone(), point));
    let manual = prepare_call(
        recorded,
        CallSpec::new("verify", verifier.clone(), input, Fr::from(0u64)),
    )?;
    let facade = c::Contract::from(Witness);
    let prepared = facade
        .recording()
        .verify_call(&observed, 0u64, id, point)?
        .prepare(verifier.clone(), Fr::from(0u64))?;
    if format!("{manual:?}") != format!("{prepared:?}") {
        return Err("observed/manual call mismatch".into());
    }
    let proof_bytes = direct_proof(root, &prepared, &verifier)?;
    let intent: Intent<Signature, ProofPreimageMarker, PedersenRandomness, r::ledger::DefaultDB> =
        Intent::empty(&mut rng, Timestamp::from_secs(state.time.to_secs() + 3600))
            .add_call::<ProofPreimage>(prepared);
    let tx = Transaction::from_intents("local-test", HashMap::new().insert(1u16, intent));
    let resolver = fee_resolver(root, "verify")?;
    let provider = LocalProvingProvider {
        rng: StdRng::seed_from_u64(0x28b),
        resolver: &resolver,
        params: &params,
    };
    let proven = futures_executor::block_on(
        tx.prove(provider, &INITIAL_PARAMETERS.cost_model.runtime_cost_model),
    )?;
    let balanced = async_runtime.block_on(state.balance_tx(
        rng.clone(),
        proven.seal(StdRng::seed_from_u64(0x28c)),
        &resolver,
    ))?;
    let time = state.time;
    let result = state.apply(&balanced, WellFormedStrictness::default())?;
    if !matches!(result, TransactionResult::Success(_)) {
        return Err(format!("strict reducer read failed: {result:?}").into());
    }
    let after = state
        .ledger
        .contract
        .get(&address)
        .ok_or("contract absent after read")?;
    if after.data.get_ref() != &constructor_data {
        return Err("read-only reducer changed contract data".into());
    }
    let saved = state.ledger.clone();
    let verified = balanced.well_formed(&state.ledger, WellFormedStrictness::default(), time)?;
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
        return Err(format!("unexpected replay outcome: {replay:?}").into());
    }
    if replayed != saved {
        return Err("replay mutated ledger".into());
    }
    let bytes = {
        let mut b = Vec::new();
        tagged_serialize(&after, &mut b)?;
        b
    };
    fs::write(root.join("reducer-final-state.bin"), &bytes)?;
    let receipt = serde_json::json!({"status":"passed","strictness":"default","constructor_execution_proved":false,"deployed_constructor_data":true,"proof_bytes":proof_bytes,"changed_binding_rejected":true,"applied":true,"read_only_contract_data":true,"replay_refusal":"IntentAlreadyExists","final_state_bytes":bytes.len()});
    fs::write(
        root.join("reducer-proof-result.json"),
        serde_json::to_vec_pretty(&receipt)?,
    )?;
    println!("reducer strict proof passed: {receipt}");
    Ok(())
}
