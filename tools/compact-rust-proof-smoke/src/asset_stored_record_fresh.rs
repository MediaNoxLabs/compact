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

//! Prove an asset record freshness read from a state seeded with one nonempty note.

use super::*;
use compact_rust_asset_registry_oracle_fixture::ledger_contract as asset;
use compact_rust_asset_registry_oracle_fixture::ledger_slots as slots;
use compact_rust_asset_registry_oracle_fixture::types::{
    AssetClass, AssetRecord, FreshnessPolicy, Provenance, RecordMutation,
};
use midnight_compact_runtime as runtime;
use runtime::recording::RecordingFrame;

const RETAINED_IR: &str =
    include_str!("../../compact-rust-backend/tests/asset-registry-schema20-ir.json");

pub(super) fn run(root: &Path) -> Result<(), Box<dyn Error>> {
    counter_scenario::validate_ir(
        &std::fs::read(root.join("contract/compact-rust-ir.json"))?,
        RETAINED_IR,
    )?;
    const CIRCUIT: &str = "assertStoredRecordFresh";
    let witnesses = super::asset_writable::AssetWitness;
    let initial = asset::initial_state(ConstructorContext::new(7_u64), &witnesses)?;
    if initial.private_state != 10 {
        return Err("asset constructor private state differs from TypeScript".into());
    }
    let key = runtime::OpaqueString::from("asset-1");
    let record = AssetRecord {
        code: FixedBytes::new([0; 32]),
        note: runtime::OpaqueString::from("nonempty note"),
        provenance: Provenance {
            facility: FixedBytes::new([4; 32]),
            registeredAt: BoundedUint::new(100)?,
        },
        kind: AssetClass::Instrument,
        quantity: BoundedUint::new(5)?,
    };
    // Supply constructor and native setRecord state, then prove the read-only
    // guard. This scenario does not prove deployment or the seed transition.
    let seeded = asset::setRecord(
        initial.into_circuit_context(runtime::ledger::ContractAddress::default()),
        &witnesses,
        key.clone(),
        record.clone(),
        RecordMutation::Insert,
    )?;
    if seeded.context.private_state != 11 {
        return Err("asset record insertion did not advance private state".into());
    }
    let seeded_state = seeded.context.query.state.get_ref().clone();
    let mut rng = StdRng::seed_from_u64(0x0097_4153_4752_414e);
    let deploy = make_deploy(root, CIRCUIT, seeded_state.clone(), &mut rng)?;
    let observed = ObservedContractState::new(
        deploy.address(),
        deploy.initial_state.clone(),
        Observation {
            transaction_hash: [0; 32],
            block_hash: [0; 32],
            block_height: 0,
        },
    );
    let policy = FreshnessPolicy {
        enforceMaxAge: true,
        maxAge: BoundedUint::new(50)?,
    };
    let now = BoundedUint::new(120)?;
    let native = asset::assertStoredRecordFresh(
        observed.circuit_context(11_u64),
        key.clone(),
        policy.clone(),
        now,
    )?;
    let recorded = asset::recorded::assertStoredRecordFresh(
        observed.circuit_context(11_u64),
        key.clone(),
        policy.clone(),
        now,
    )?;
    let generated = asset::Contract::from(witnesses);
    let call = generated.recording().assertStoredRecordFresh_call(
        &observed,
        11_u64,
        key.clone(),
        policy.clone(),
        now,
    )?;
    // Independent source-ordered public trace and explicit freshness predicate.
    let (frame, present) = slots::records.record_member(
        RecordingFrame::new(observed.circuit_context(11_u64)),
        key.clone(),
    )?;
    if !present {
        return Err("manual freshness record missing".into());
    }
    let (frame, stored) = slots::records.record_lookup(frame, key.clone())?;
    if stored != record
        || now.value() < stored.provenance.registeredAt.value()
        || (policy.enforceMaxAge
            && now.value() - stored.provenance.registeredAt.value() > policy.maxAge.value())
    {
        return Err("manual freshness predicate or composite record differs".into());
    }
    let manual = frame.finish(());
    if native.context.private_state != 11 || !native.private_transcript_outputs.is_empty() {
        return Err("freshness native private state/output differs".into());
    }
    let expected_replay = manual.public.initial().query(
        manual.public.verify_ops(),
        None,
        &manual.execution.context.cost_model,
    )?;
    for candidate in [&recorded, call.recorded(), &manual] {
        if candidate.execution.context.query.state != native.context.query.state
            || candidate.execution.context.query.effects != native.context.query.effects
            || candidate.execution.gas_cost != native.gas_cost
            || candidate.execution.context.private_state != 11
            || !candidate.execution.private_transcript_outputs.is_empty()
            || candidate.public.verify_ops() != manual.public.verify_ops()
        {
            return Err("freshness native/recorded/observed/manual execution differs".into());
        }
        let replay = candidate.public.initial().query(
            candidate.public.verify_ops(),
            None,
            &candidate.execution.context.cost_model,
        )?;
        if replay.context.state != native.context.query.state
            || replay.context.effects != native.context.query.effects
            || replay.gas_cost != expected_replay.gas_cost
        {
            return Err("freshness verifier replay differs".into());
        }
    }
    let expected_state = native.context.query.state.get_ref().clone();
    if expected_state != seeded_state
        || slots::records
            .inspect(&expected_state)?
            .lookup(key.clone())?
            != record
    {
        return Err("freshness read changed seeded state or composite record".into());
    }
    let verifier: VerifierKey = tagged_deserialize(&mut BufReader::new(File::open(
        root.join("keys/assertStoredRecordFresh.verifier"),
    )?))?;
    let prepared = call.prepare(verifier.clone(), Fr::from(0_u64))?;
    let expected_input = AlignedValue::from((key.clone(), policy.clone(), now));
    if prepared.input != expected_input || prepared.output != AlignedValue::from(()) {
        return Err("freshness original argument order/Unit output binding differs".into());
    }
    let direct = check_generated_trace(root, CIRCUIT, recorded, (key, policy, now))?;
    if format!("{direct:?}") != format!("{prepared:?}") {
        return Err("freshness observed/direct preparation differs".into());
    }
    kernel_shielded_effects::prove_and_verify_call(root, CIRCUIT, &prepared, &verifier)?;
    opaque_public::apply_funded(root, CIRCUIT, prepared, deploy, &expected_state, rng, 594)?;
    println!(
        "asset stored freshness: source-bound native/recorded/observed/manual/replay, verified proof and changed-binding refusal, default-strict funded ledger-8 application; constructor and native record seed supplied, deployment and seed not proved"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn freshness_source_changes_refuse_before_keys() {
        counter_scenario::validate_ir(RETAINED_IR.as_bytes(), RETAINED_IR).unwrap();
        let mut changed: serde_json::Value = serde_json::from_str(RETAINED_IR).unwrap();
        changed["independent_stored_freshness_guard"] = true.into();
        assert!(
            counter_scenario::validate_ir(&serde_json::to_vec(&changed).unwrap(), RETAINED_IR)
                .is_err()
        );
    }
    #[test]
    fn freshness_absent_source_refuses_before_keys_or_state() {
        let root = std::env::temp_dir().join(format!(
            "compact-stored-freshness-absent-{}-{}",
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
