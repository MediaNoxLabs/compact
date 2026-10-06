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
//! Original constructor data is deployed, then three source transitions are proved.
//! The stored constructor id stays zero; the ledger deployment address is distinct.
use super::*;
use compact_rust_did_adoption_fixture::{
    ledger_contract as c, ledger_slots as slots, runtime as r, types,
};
use serde_json::{Value, json};
#[path = "../../../tests-rust-backend/did-adoption/support/alias_calls.rs"]
mod alias_calls;
#[path = "../../../tests-rust-backend/did-adoption/support/codec.rs"]
mod codec;
#[path = "../../../tests-rust-backend/did-adoption/support/lifecycle_calls.rs"]
mod lifecycle_calls;
#[path = "../../../tests-rust-backend/did-adoption/support/lifecycle_witness.rs"]
mod lifecycle_witness;
#[path = "../../../tests-rust-backend/did-adoption/support/point_calls.rs"]
mod point_calls;
use lifecycle_witness::Witness;

#[derive(Clone, Copy)]
enum Lifecycle {
    Points,
    Aliases,
}
pub(super) fn run(root: &Path) -> Result<(), Box<dyn Error>> {
    run_lifecycle(root, Lifecycle::Points)
}
pub(super) fn run_aliases(root: &Path) -> Result<(), Box<dyn Error>> {
    run_lifecycle(root, Lifecycle::Aliases)
}
fn run_lifecycle(root: &Path, lifecycle: Lifecycle) -> Result<(), Box<dyn Error>> {
    let (capture, scenario_id, calls, operations): (&str, &str, &[&str], &[&str]) = match lifecycle
    {
        Lifecycle::Points => (
            include_str!("../../../tests-rust-backend/did-adoption/oracle/lifecycle.json"),
            "authorization-lifecycle",
            &["rotate", "recover", "deactivate"],
            &["rotateControllerKey", "recoverControllerKey", "deactivate"],
        ),
        Lifecycle::Aliases => (
            include_str!("../../../tests-rust-backend/did-adoption/oracle/alias-lifecycle.json"),
            "alias-recording",
            &["insert-unicode", "remove-unicode"],
            &[
                "rotateControllerKey",
                "recoverControllerKey",
                "deactivate",
                "setAlsoKnownAs",
            ],
        ),
    };
    let capture: Value = serde_json::from_str(capture)?;
    let scenario = capture["scenarios"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["id"] == scenario_id)
        .unwrap();
    let mut rng = StdRng::seed_from_u64(0x0259);
    let initial = c::initial_state(
        r::context::ConstructorContext::new(0u64),
        &Witness::new(json!({})),
    )?;
    let mut private = initial.private_state;
    let constructor_data = initial.ledger_state.get_ref().clone();
    let stored_id = slots::id.inspect(&constructor_data)?;
    if stored_id.bytes != r::FixedBytes::new([0; 32]) {
        return Err("original constructor id changed".into());
    }
    let operation_names = operations;
    let mut operations = HashMap::new();
    for name in operation_names {
        let vk: VerifierKey = tagged_deserialize(&mut BufReader::new(File::open(
            root.join(format!("keys/{name}.verifier")),
        )?))?;
        operations = operations.insert(
            EntryPointBuf(name.as_bytes().to_vec()),
            ContractOperation::new(Some(vk)),
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
    if r::ledger::contract_address_bytes(&address) == stored_id.bytes {
        return Err("deployment test did not distinguish stored id and address".into());
    }
    let mut state = super::qualified_coin_funding::fee_funded_state(&mut rng)?;
    let async_runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    // Separate upstream Dust funding, never inserted or rewritten DID contract data.
    async_runtime.block_on(state.give_fee_token(&mut rng, 4));
    let intent: Intent<Signature, ProofPreimageMarker, PedersenRandomness, DefaultDB> =
        Intent::empty(&mut rng, Timestamp::from_secs(state.time.to_secs() + 3600))
            .add_deploy(deploy);
    let tx = Transaction::from_intents("local-test", HashMap::new().insert(1u16, intent));
    let resolver = super::qualified_coin_funding::fee_resolver(root, "deactivate")?;
    let params = MidnightDataProvider::new(FetchMode::OnDemand, OutputMode::Log, vec![])?;
    let provider = LocalProvingProvider {
        rng: StdRng::seed_from_u64(0x259d),
        resolver: &resolver,
        params: &params,
    };
    let proven = futures_executor::block_on(
        tx.prove(provider, &INITIAL_PARAMETERS.cost_model.runtime_cost_model),
    )?;
    let sealed = proven.seal(StdRng::seed_from_u64(0x259e));
    let balanced = async_runtime.block_on(state.balance_tx(rng.clone(), sealed, &resolver))?;
    let result = state.apply(&balanced, WellFormedStrictness::default())?;
    if !matches!(result, TransactionResult::Success(_)) {
        return Err(format!("strict DID deployment failed: {result:?}").into());
    }
    if state
        .ledger
        .contract
        .get(&address)
        .ok_or("deployed DID absent")?
        .data
        .get_ref()
        != &constructor_data
    {
        return Err("deployment changed constructor data".into());
    }
    println!(
        "DID original constructor data: actual default-strict Dust-funded deployment applied; stored zero id retained, distinct deployed address"
    );
    for (number, &id) in calls.iter().enumerate() {
        let row = scenario["steps"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["id"] == id)
            .unwrap();
        let name = match id {
            "rotate" => "rotateControllerKey",
            "recover" => "recoverControllerKey",
            "deactivate" => "deactivate",
            _ => "setAlsoKnownAs",
        };
        let prior = state
            .ledger
            .contract
            .get(&address)
            .ok_or("DID missing before call")?;
        let expected_before: ContractState<DefaultDB> =
            tagged_deserialize(&mut hex::decode(row["before"].as_str().unwrap())?.as_slice())?;
        if prior.data != expected_before.data || private != row["privateBefore"].as_u64().unwrap() {
            return Err("applied prestate differs from original TS lifecycle".into());
        }
        // Offline test observation metadata; ledger data is fetched only after apply.
        let observed = ObservedContractState::new(
            address,
            prior.clone(),
            Observation {
                transaction_hash: [0; 32],
                block_hash: [0; 32],
                block_height: number as u64 + 1,
            },
        );
        let native = lifecycle_calls::invoke(
            observed.circuit_context(private),
            &Witness::new(row["options"].clone()),
            row,
        )?;
        let record = match lifecycle {
            Lifecycle::Points => point_calls::invoke_recorded,
            Lifecycle::Aliases => alias_calls::invoke_recorded,
        };
        let recorded = record(
            observed.circuit_context(private),
            &Witness::new(row["options"].clone()),
            row,
        )?;
        if native.context.query.state != recorded.execution.context.query.state
            || native.gas_cost != recorded.execution.gas_cost
            || native.private_transcript_outputs != recorded.execution.private_transcript_outputs
        {
            return Err("DID native/recorded mismatch".into());
        }
        let expected = native.context.query.state.get_ref().clone();
        let signature = types::SchnorrSignature {
            announcement: codec::point("2"),
            response: codec::field_hex(row["responseHex"].as_str().unwrap()),
        };
        let version = codec::version(row["version"].as_str().unwrap());
        let input = if name == "setAlsoKnownAs" {
            AlignedValue::from((
                codec::string(&row["args"]["value"]),
                codec::set(&row["args"]["mutation"]),
                signature.clone(),
                version,
            ))
        } else if id == "deactivate" {
            AlignedValue::from((signature.clone(), version))
        } else {
            AlignedValue::from((
                codec::point(row["args"]["key"].as_str().unwrap()),
                signature.clone(),
                version,
            ))
        };
        let manual = check_generated_trace(root, name, recorded, input)?;
        let verifier: VerifierKey = tagged_deserialize(&mut BufReader::new(File::open(
            root.join(format!("keys/{name}.verifier")),
        )?))?;
        let facade = c::Contract::from(Witness::new(row["options"].clone()));
        let call = match id {
            "rotate" => facade.recording().rotateControllerKey_call(
                &observed,
                private,
                codec::point(row["args"]["key"].as_str().unwrap()),
                signature,
                version,
            )?,
            "recover" => facade.recording().recoverControllerKey_call(
                &observed,
                private,
                codec::point(row["args"]["key"].as_str().unwrap()),
                signature,
                version,
            )?,
            "deactivate" => facade
                .recording()
                .deactivate_call(&observed, private, signature, version)?,
            _ => facade.recording().setAlsoKnownAs_call(
                &observed,
                private,
                codec::string(&row["args"]["value"]),
                codec::set(&row["args"]["mutation"]),
                signature,
                version,
            )?,
        };
        let prepared = call.prepare(verifier.clone(), Fr::from(0u64))?;
        if format!("{manual:?}") != format!("{prepared:?}") {
            return Err("typed call preparation differs from manual FAB binding".into());
        }
        super::kernel_shielded_effects::prove_and_verify_call(root, name, &prepared, &verifier)?;
        let intent: Intent<Signature, ProofPreimageMarker, PedersenRandomness, DefaultDB> =
            Intent::empty(&mut rng, Timestamp::from_secs(state.time.to_secs() + 3600))
                .add_call::<ProofPreimage>(prepared);
        let tx = Transaction::from_intents("local-test", HashMap::new().insert(1u16, intent));
        let resolver = super::qualified_coin_funding::fee_resolver(root, name)?;
        let provider = LocalProvingProvider {
            rng: StdRng::seed_from_u64(0x2590 + number as u64),
            resolver: &resolver,
            params: &params,
        };
        let proven = futures_executor::block_on(
            tx.prove(provider, &INITIAL_PARAMETERS.cost_model.runtime_cost_model),
        )?;
        let sealed = proven.seal(StdRng::seed_from_u64(0x259a + number as u64));
        let balanced = async_runtime.block_on(state.balance_tx(rng.clone(), sealed, &resolver))?;
        let accepted_at = state.time;
        let result = state.apply(&balanced, WellFormedStrictness::default())?;
        if !matches!(result, TransactionResult::Success(_)) {
            return Err(format!("DID {name} strict application failed: {result:?}").into());
        }
        let applied = state
            .ledger
            .contract
            .get(&address)
            .ok_or("DID absent after call")?;
        let expected_ts: ContractState<DefaultDB> =
            tagged_deserialize(&mut hex::decode(row["after"].as_str().unwrap())?.as_slice())?;
        if applied.data.get_ref() != &expected
            || applied.data != expected_ts.data
            || slots::id.inspect(applied.data.get_ref())? != stored_id
        {
            return Err(
                "applied DID transition differs from TS/native or rewrote stored id".into(),
            );
        }
        private = native.context.private_state;
        let unchanged = state.ledger.clone();
        let verified =
            balanced.well_formed(&state.ledger, WellFormedStrictness::default(), accepted_at)?;
        let replay_context = TransactionContext {
            ref_state: state.ledger.clone(),
            block_context: BlockContext {
                tblock: accepted_at,
                last_block_time: accepted_at,
                ..BlockContext::default()
            },
            whitelist: None,
        };
        let (replayed, replay) = state.ledger.apply(&verified, &replay_context);
        if !matches!(
            replay,
            TransactionResult::Failure(
                midnight_ledger::error::TransactionInvalid::ReplayProtectionViolation(
                    midnight_ledger::error::TransactionApplicationError::IntentAlreadyExists
                )
            )
        ) {
            return Err(format!("expected exact replay-protection refusal, got {replay:?}").into());
        }
        if replayed != unchanged {
            return Err("rejected replay changed ledger state".into());
        }
        println!(
            "original DID {name}: nonempty proof verified, changed input rejected, strict separate-Dust ledger application, same-time replay refusal: {replay:?}"
        );
        if state.ledger != unchanged {
            return Err("replay validation mutated ledger".into());
        }
    }
    let final_state = state
        .ledger
        .contract
        .get(&address)
        .ok_or("DID final state absent")?;
    let data = final_state.data.get_ref();
    match lifecycle {
        Lifecycle::Points => {
            if slots::active.inspect(data)?
                || !slots::deactivated.inspect(data)?
                || slots::version.inspect(data)? != 3
                || slots::operationCount.inspect(data)? != 3
            {
                return Err("final DID Point lifecycle fields differ".into());
            }
            println!(
                "original DID deploy -> rotate -> recover -> deactivate: strict sequential acceptance; constructor execution not proved, stored-id semantics unchanged"
            );
        }
        Lifecycle::Aliases => {
            if !slots::active.inspect(data)?
                || slots::deactivated.inspect(data)?
                || slots::version.inspect(data)? != 2
                || slots::operationCount.inspect(data)? != 2
                || !slots::alsoKnownAs.inspect(data)?.is_empty()
            {
                return Err("final DID alias cycle fields differ".into());
            }
            println!(
                "original DID deploy -> setAlsoKnownAs insert -> remove: strict sequential acceptance; constructor execution not proved, stored-id semantics unchanged"
            );
        }
    }
    Ok(())
}
