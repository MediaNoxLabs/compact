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
//! Original DID deactivate: seeded prior state, strict call and separate Dust.
use super::did_deactivate_support::Witness;
use super::*;
use compact_rust_did_adoption_fixture::{ledger_contract as c, ledger_slots as slots, types};
use midnight_compact_runtime as runtime;
use serde_json::Value;
use std::cell::RefCell;
pub(super) fn run(root: &Path) -> Result<(), Box<dyn Error>> {
    let capture: Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/unit-composition.json"
    ))?;
    let row = capture["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["id"] == "did/valid")
        .unwrap();
    let prior: ContractState<DefaultDB> =
        tagged_deserialize(&mut hex::decode(row["before"].as_str().unwrap())?.as_slice())?;
    let signature = types::SchnorrSignature {
        announcement: runtime::ec_mul_generator(Field::from(2u64))?,
        response: Field::from_le_bytes(&hex::decode(row["responseHex"].as_str().unwrap())?)
            .ok_or("invalid fixture response")?,
    };
    let witness = || Witness {
        options: row["options"].clone(),
        calls: RefCell::default(),
    };
    let mut rng = StdRng::seed_from_u64(0x0251);
    let deploy = make_deploy(root, "deactivate", prior.data.get_ref().clone(), &mut rng)?;
    let address = deploy.address();
    let mut state = super::qualified_coin_funding::fee_funded_state(&mut rng)?;
    state.ledger.contract = state
        .ledger
        .contract
        .insert(address, deploy.initial_state.clone());
    let observed = ObservedContractState::new(
        address,
        deploy.initial_state,
        Observation {
            transaction_hash: [0; 32],
            block_hash: [0; 32],
            block_height: 0,
        },
    );
    let native = c::deactivate(
        observed.circuit_context(7u64),
        &witness(),
        signature.clone(),
        runtime::BoundedUint::new(0)?,
    )?;
    let recorded = c::recorded::deactivate(
        observed.circuit_context(7u64),
        &witness(),
        signature.clone(),
        runtime::BoundedUint::new(0)?,
    )?;
    if native.context.query.state != recorded.execution.context.query.state
        || native.gas_cost != recorded.execution.gas_cost
        || native.private_transcript_outputs != recorded.execution.private_transcript_outputs
    {
        return Err("DID native/recorded mismatch".into());
    }
    let expected = native.context.query.state.get_ref().clone();
    let input = AlignedValue::from((
        signature.clone(),
        runtime::BoundedUint::<18446744073709551615>::new(0)?,
    ));
    let manual = check_generated_trace(root, "deactivate", recorded, input)?;
    let verifier: VerifierKey = tagged_deserialize(&mut BufReader::new(File::open(
        root.join("keys/deactivate.verifier"),
    )?))?;
    let generated = c::Contract::from(witness());
    let prepared = generated
        .recording()
        .deactivate_call(&observed, 7u64, signature, runtime::BoundedUint::new(0)?)?
        .prepare(verifier.clone(), Fr::from(0u64))?;
    if format!("{manual:?}") != format!("{prepared:?}") {
        return Err("DID typed/manual preparation mismatch".into());
    }
    super::kernel_shielded_effects::prove_and_verify_call(
        root,
        "deactivate",
        &prepared,
        &verifier,
    )?;
    let intent: Intent<Signature, ProofPreimageMarker, PedersenRandomness, DefaultDB> =
        Intent::empty(&mut rng, Timestamp::from_secs(state.time.to_secs() + 3600))
            .add_call::<ProofPreimage>(prepared);
    let tx = Transaction::from_intents("local-test", HashMap::new().insert(1u16, intent));
    let resolver = super::qualified_coin_funding::fee_resolver(root, "deactivate")?;
    let params = MidnightDataProvider::new(FetchMode::OnDemand, OutputMode::Log, vec![])?;
    let provider = LocalProvingProvider {
        rng: StdRng::seed_from_u64(0x02515052),
        resolver: &resolver,
        params: &params,
    };
    let proven = futures_executor::block_on(
        tx.prove(provider, &INITIAL_PARAMETERS.cost_model.runtime_cost_model),
    )?;
    let sealed = proven.seal(StdRng::seed_from_u64(0x02515345));
    let sealed = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(state.balance_tx(rng, sealed, &resolver))?;
    let verified =
        sealed.well_formed(&state.ledger, WellFormedStrictness::default(), state.time)?;
    let context = TransactionContext {
        ref_state: state.ledger.clone(),
        block_context: BlockContext {
            tblock: state.time,
            last_block_time: state.time,
            ..BlockContext::default()
        },
        whitelist: None,
    };
    let (updated, result) = state.ledger.apply(&verified, &context);
    if !matches!(result, TransactionResult::Success(_)) {
        return Err(format!("DID apply failed: {result:?}").into());
    }
    let actual = updated.contract.get(&address).ok_or("DID missing")?;
    if actual.data.get_ref() != &expected
        || slots::active.inspect(actual.data.get_ref())?
        || !slots::deactivated.inspect(actual.data.get_ref())?
    {
        return Err("DID final state differs".into());
    }
    let second = c::recorded::deactivate(
        runtime::context::CircuitContext::from_contract_state(9u64, address, actual),
        &witness(),
        types::SchnorrSignature {
            announcement: runtime::ec_mul_generator(Field::from(2u64))?,
            response: Field::from_le_bytes(&hex::decode(row["responseHex"].as_str().unwrap())?)
                .ok_or("invalid fixture response")?,
        },
        runtime::BoundedUint::new(0)?,
    );
    if !matches!(second,Err(runtime::CompactError::AssertionFailed(ref message)) if message=="Controller authorization version is stale")
    {
        return Err("DID repeat did not reject stale version first".into());
    }
    println!(
        "original DID deactivate: proof verified, changed binding rejected, default-strict separate Dust ledger apply; exact final state and stale repeat rejection; constructor-derived seeded prior state, not a full DID lifecycle"
    );
    Ok(())
}
