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

//! Record, prove and export a Counter call from an indexer-observed ledger-8 ContractState.
//! The input bytes are public state, not an authenticated full ledger snapshot.

use std::env;
use std::error::Error;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use compact_rust_counter_fixture::ledger_contract as counter_contract;
use futures_executor::block_on;
use midnight_base_crypto::data_provider::{FetchMode, MidnightDataProvider, OutputMode};
use midnight_base_crypto::signatures::Signature;
use midnight_base_crypto::time::Timestamp;
use midnight_compact_runtime::context::CircuitContext;
use midnight_compact_runtime::ledger::{
    ContractAddress, ContractState, DefaultDB, StateValue, read_counter,
};
use midnight_compact_runtime::transaction::{
    Observation, ObservedContractState, decode_verifier_key,
};
use midnight_ledger::semantics::{TransactionContext, TransactionResult};
use midnight_ledger::structure::{
    INITIAL_PARAMETERS, Intent, LedgerState, ProofMarker, ProofPreimageMarker, Transaction,
};
use midnight_ledger::verify::WellFormedStrictness;
use midnight_onchain_runtime::context::BlockContext;
use midnight_serialize::{tagged_deserialize, tagged_serialize};
use midnight_storage::storage::HashMap;
use midnight_transient_crypto::commitment::{PedersenRandomness, PureGeneratorPedersen};
use midnight_transient_crypto::curve::Fr;
use midnight_transient_crypto::proofs::{KeyLocation, ProofPreimage, ProvingKeyMaterial, Resolver};
use midnight_zkir::LocalProvingProvider;
use rand::{SeedableRng, rngs::StdRng};

const USAGE: &str = "usage: record_from_confirmed <state.bin> <deploy.bin> [<artifacts> <call-output.bin> <network-id> <ttl-secs> <expected-address-hex> <observed-tx-hash> <observed-block-hash> <observed-block-height>]";

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

fn counter_value(state: &StateValue<DefaultDB>) -> Result<u64, Box<dyn Error>> {
    let StateValue::Array(fields) = state else {
        return Err("counter contract state is not an array".into());
    };
    Ok(read_counter(fields.get(0).ok_or("Counter field missing")?)?)
}

fn address_hex(address: ContractAddress) -> String {
    address
        .0
        .0
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn parse_hash(label: &str, value: &str) -> Result<[u8; 32], Box<dyn Error>> {
    let raw = value.strip_prefix("0x").unwrap_or(value);
    if raw.len() != 64 || !raw.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(format!("{label} must be 32 bytes of hex").into());
    }
    let mut bytes = [0u8; 32];
    for (index, byte) in bytes.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&raw[index * 2..index * 2 + 2], 16)?;
    }
    Ok(bytes)
}

fn prove_and_export(
    observed: ObservedContractState,
    artifacts: &Path,
    output: &Path,
    network_id: &str,
    ttl_secs: u64,
    expected_address: &str,
) -> Result<(), Box<dyn Error>> {
    let address = observed.address();
    if network_id.is_empty() || network_id.trim() != network_id {
        return Err("network ID must be nonempty with no surrounding whitespace".into());
    }
    if address_hex(address) != expected_address.to_ascii_lowercase() {
        return Err("indexed address does not match deployment transaction".into());
    }
    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    if ttl_secs <= now + 60 || ttl_secs > now + 3_600 {
        return Err("call TTL must be 1–60 minutes in the future".into());
    }
    let verifier = decode_verifier_key(&fs::read(artifacts.join("keys/increment.verifier"))?)?;
    let before = counter_value(observed.contract().data.get_ref())?;
    let recorded_call = counter_contract::Contract::default()
        .recording
        .increment_call(&observed, ())?;
    let after = counter_value(
        recorded_call
            .recorded()
            .execution
            .context
            .query
            .state
            .get_ref(),
    )?;
    if after != before.checked_add(1).ok_or("Counter overflow")? {
        return Err("recorded increment did not advance confirmed state".into());
    }
    let call = recorded_call.prepare(verifier, Fr::from(0_u64))?;
    let mut rng = StdRng::seed_from_u64(0x5345_434f_4e44);
    let intent: Intent<Signature, ProofPreimageMarker, PedersenRandomness, DefaultDB> =
        Intent::empty(&mut rng, Timestamp::from_secs(ttl_secs)).add_call::<ProofPreimage>(call);
    let transaction = Transaction::from_intents(network_id, HashMap::new().insert(1_u16, intent));
    let resolver = ArtifactResolver {
        root: artifacts.to_owned(),
    };
    let params = MidnightDataProvider::new(FetchMode::OnDemand, OutputMode::Log, vec![])?;
    let provider = LocalProvingProvider {
        rng: StdRng::seed_from_u64(0x0050_524f_5645),
        resolver: &resolver,
        params: &params,
    };
    let proven =
        block_on(transaction.prove(provider, &INITIAL_PARAMETERS.cost_model.runtime_cost_model))?;
    let sealed = proven.seal(StdRng::seed_from_u64(0x5345_414c));
    if sealed
        .calls()
        .map(|(_, call)| call.address)
        .collect::<Vec<_>>()
        != vec![address]
    {
        return Err("sealed call address differs from confirmed deployment".into());
    }

    // This local projection validates proof and state transition without claiming
    // to reconstruct unrelated live balances, fees or replay protection.
    let mut projected_ledger = LedgerState::<DefaultDB>::new(network_id);
    projected_ledger.contract = projected_ledger
        .contract
        .insert(address, observed.contract().clone());
    let mut strictness = WellFormedStrictness::default();
    strictness.enforce_balancing = false;
    let validation_time = Timestamp::from_secs(now);
    let verified = sealed.well_formed(&projected_ledger, strictness, validation_time)?;
    let context = TransactionContext {
        ref_state: projected_ledger.clone(),
        block_context: BlockContext {
            tblock: validation_time,
            last_block_time: validation_time,
            ..BlockContext::default()
        },
        whitelist: None,
    };
    let (updated, outcome) = projected_ledger.apply(&verified, &context);
    if !matches!(outcome, TransactionResult::Success(_)) {
        return Err(format!("confirmed-state call application failed: {outcome:?}").into());
    }
    let next = updated
        .contract
        .get(&address)
        .ok_or("contract disappeared")?;
    if counter_value(next.data.get_ref())? != after {
        return Err("proven call did not produce recorded Counter state".into());
    }
    // A call recorded against round = 1 must not validate against round = 2.
    // This is a local state-precondition check, not a network replay guarantee.
    if let Ok(replayed) = sealed.well_formed(&updated, strictness, validation_time) {
        let replay_context = TransactionContext {
            ref_state: updated.clone(),
            ..context
        };
        let (_, replay_outcome) = updated.apply(&replayed, &replay_context);
        if matches!(replay_outcome, TransactionResult::Success(_)) {
            return Err("stale confirmed-state call was accepted by the local projection".into());
        }
    }
    let mut bytes = Vec::new();
    tagged_serialize(&sealed, &mut bytes)?;
    fs::write(output, bytes)?;
    println!(
        "proved confirmed-state Counter call: {before} -> {after}, {}",
        output.display()
    );
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut arguments = env::args().skip(1);
    let state_path = arguments.next().ok_or(USAGE)?;
    let deploy_path = arguments.next().ok_or(USAGE)?;
    let rest = arguments.collect::<Vec<_>>();
    if !rest.is_empty() && rest.len() != 8 {
        return Err(USAGE.into());
    }
    let state_bytes = fs::read(state_path)?;
    let deploy_bytes = fs::read(deploy_path)?;
    let deploy: Transaction<Signature, ProofMarker, PureGeneratorPedersen, DefaultDB> =
        tagged_deserialize(&mut deploy_bytes.as_slice())?;
    let address = deploy
        .deploys()
        .next()
        .ok_or("expected one deployment")?
        .1
        .address();
    if rest.is_empty() {
        let contract: ContractState<DefaultDB> = tagged_deserialize(&mut state_bytes.as_slice())?;
        let before = counter_value(contract.data.get_ref())?;
        let context = CircuitContext::from_contract_state((), address, &contract);
        let recorded = counter_contract::Contract::default()
            .recording
            .increment(context)?;
        let after = counter_value(recorded.execution.context.query.state.get_ref())?;
        if after != before.checked_add(1).ok_or("Counter overflow")? {
            return Err("recorded increment did not advance confirmed state".into());
        }
        println!("recorded indexed Counter transition: {before} -> {after}");
    } else {
        let observation = Observation {
            transaction_hash: parse_hash("observed transaction hash", &rest[5])?,
            block_hash: parse_hash("observed block hash", &rest[6])?,
            block_height: rest[7].parse()?,
        };
        let observed = ObservedContractState::decode(address, &state_bytes, observation)?;
        prove_and_export(
            observed,
            Path::new(&rest[0]),
            Path::new(&rest[1]),
            &rest[2],
            rest[3].parse()?,
            &rest[4],
        )?;
    }
    Ok(())
}
