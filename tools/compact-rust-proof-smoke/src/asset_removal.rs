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

//! Source-bound removal from supplied constructor and native Insert state.

use super::*;
use asset_record_write::manual::TimestampWitness;
use compact_rust_asset_registry_oracle_fixture::ledger_contract as asset;
use compact_rust_asset_registry_oracle_fixture::ledger_slots as slots;
use compact_rust_asset_registry_oracle_fixture::types::{AssetClass, AssetRecord, RecordMutation};
use midnight_compact_runtime as runtime;

mod manual;

const RETAINED_IR: &str =
    include_str!("../../compact-rust-backend/tests/asset-registry-schema20-ir.json");

pub(super) fn run(root: &Path) -> Result<(), Box<dyn Error>> {
    counter_scenario::validate_ir(
        &std::fs::read(root.join("contract/compact-rust-ir.json"))?,
        RETAINED_IR,
    )?;
    const CIRCUIT: &str = "removeRecord";
    let witnesses = super::asset_writable::AssetWitness;
    let initial = asset::initial_state(ConstructorContext::new(7_u64), &witnesses)?;
    if initial.private_state != 10 {
        return Err("asset constructor private state differs from TypeScript".into());
    }
    let key = runtime::OpaqueString::from("asset-1");
    let record = AssetRecord {
        kind: AssetClass::Instrument,
        ..Default::default()
    };
    // Constructor and native Insert are supplied setup. Neither deployment nor
    // the seed transition is proved by this selected removal scenario.
    let seeded = asset::setRecord(
        initial.into_circuit_context(runtime::ledger::ContractAddress::default()),
        &witnesses,
        key.clone(),
        record,
        RecordMutation::Insert,
    )?;
    if seeded.context.private_state != 11 {
        return Err("asset insertion did not advance private state".into());
    }
    let mut rng = StdRng::seed_from_u64(0x0097_4153_5245_4d4f);
    let deploy = make_deploy(
        root,
        CIRCUIT,
        seeded.context.query.state.get_ref().clone(),
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
    let native_witness = TimestampWitness::default();
    let recorded_witness = TimestampWitness::default();
    let observed_witness = TimestampWitness::default();
    let manual_witness = TimestampWitness::default();
    let native = asset::removeRecord(
        observed.circuit_context(11_u64),
        &native_witness,
        key.clone(),
    )?;
    let recorded = asset::recorded::removeRecord(
        observed.circuit_context(11_u64),
        &recorded_witness,
        key.clone(),
    )?;
    let call = asset::Contract::from(observed_witness.clone())
        .recording()
        .removeRecord_call(&observed, 11_u64, key.clone())?;
    let manual = manual::remove(
        observed.circuit_context(11_u64),
        key.clone(),
        &manual_witness,
    )?;
    // This existing comparison checks private11→12, one timestamp callback per
    // lane, its exact FAB, public operations/state/effects/execution gas and
    // verifier replay against separately measured manual replay gas.
    asset_record_write::manual::compare(
        &native,
        &recorded,
        call.recorded(),
        &manual,
        11,
        &[
            &native_witness,
            &recorded_witness,
            &observed_witness,
            &manual_witness,
        ],
    )?;
    let expected_state = native.context.query.state.get_ref().clone();
    if slots::records.inspect(&expected_state)?.member(key.clone()) {
        return Err("asset removal left record in Map".into());
    }
    if !slots::retiredKeys
        .inspect(&expected_state)?
        .member(key.clone())
    {
        return Err("asset removal omitted retired key".into());
    }
    // The Compact source counts inserted records and does not decrement this
    // Counter on removal. Both write metadata Counters advance exactly once.
    if slots::recordCount.inspect(&expected_state)? != 1
        || slots::revision.inspect(&expected_state)? != 2
        || slots::writeCount.inspect(&expected_state)? != 2
    {
        return Err("asset removal stored wrong Counter metadata".into());
    }
    let input = AlignedValue::from(key);
    let direct = check_generated_trace(root, CIRCUIT, recorded, input.clone())?;
    let verifier: VerifierKey = tagged_deserialize(&mut BufReader::new(File::open(
        root.join("keys/removeRecord.verifier"),
    )?))?;
    let prepared = call.prepare(verifier.clone(), Fr::from(0_u64))?;
    if prepared.input != input || prepared.output != AlignedValue::from(()) {
        return Err("asset removal declared input/Unit output differs".into());
    }
    if format!("{direct:?}") != format!("{prepared:?}") {
        return Err("asset removal observed/direct preparation differs".into());
    }
    kernel_shielded_effects::prove_and_verify_call(root, CIRCUIT, &prepared, &verifier)?;
    opaque_public::apply_funded(root, CIRCUIT, prepared, deploy, &expected_state, rng, 601)?;
    println!(
        "asset removal: source-bound native/recorded/observed/manual/replay, verified proof and changed-binding refusal, default-strict funded ledger-8 application; constructor and native Insert supplied, deployment and seed not proved"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn altered_whole_asset_source_refuses_before_keys() {
        counter_scenario::validate_ir(RETAINED_IR.as_bytes(), RETAINED_IR).unwrap();
        let mut changed: serde_json::Value = serde_json::from_str(RETAINED_IR).unwrap();
        changed["independent_removal_source_control"] = true.into();
        assert!(
            counter_scenario::validate_ir(&serde_json::to_vec(&changed).unwrap(), RETAINED_IR)
                .is_err()
        );
    }

    #[test]
    fn absent_asset_source_refuses_before_keys_or_state() {
        let root = std::env::temp_dir().join(format!(
            "compact-removal-proof-absent-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        assert!(!root.exists());
        assert_eq!(
            run(&root)
                .unwrap_err()
                .downcast_ref::<io::Error>()
                .unwrap()
                .kind(),
            io::ErrorKind::NotFound
        );
        assert!(!root.exists());
    }
}
