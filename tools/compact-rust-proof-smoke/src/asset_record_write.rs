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

//! Prove typed asset-record Insert and Update with a Unicode opaque note.

use super::*;
use compact_rust_asset_registry_oracle_fixture::ledger_contract as asset;
use compact_rust_asset_registry_oracle_fixture::types::{
    AssetClass, AssetRecord, Provenance, RecordMutation,
};
use midnight_compact_runtime as runtime;

const RETAINED_IR: &str =
    include_str!("../../compact-rust-backend/tests/asset-registry-schema20-ir.json");

pub(super) mod manual;

pub(super) fn record(revised: bool) -> Result<AssetRecord, Box<dyn Error>> {
    Ok(AssetRecord {
        code: FixedBytes::new([if revised { 3 } else { 0 }; 32]),
        note: runtime::OpaqueString::from(if revised {
            "更新済み café"
        } else {
            "検査資料 🔒"
        }),
        provenance: Provenance {
            facility: FixedBytes::new([if revised { 6 } else { 4 }; 32]),
            registeredAt: BoundedUint::new(if revised { 101 } else { 91 })?,
        },
        kind: if revised {
            AssetClass::Container
        } else {
            AssetClass::Instrument
        },
        quantity: BoundedUint::new(if revised { 9 } else { 5 })?,
    })
}

pub(super) fn run(root: &Path) -> Result<(), Box<dyn Error>> {
    counter_scenario::validate_ir(
        &std::fs::read(root.join("contract/compact-rust-ir.json"))?,
        RETAINED_IR,
    )?;
    // Constructor and optional native Insert seed are supplied; neither is proved.
    const CIRCUIT: &str = "setRecord";
    let key = runtime::OpaqueString::from("record-α-1");
    for update in [false, true] {
        let witnesses = super::asset_writable::AssetWitness;
        let initial = asset::initial_state(ConstructorContext::new(7_u64), &witnesses)?;
        if initial.private_state != 10 {
            return Err("asset record constructor private state differs from TypeScript".into());
        }
        let context = initial.into_circuit_context(runtime::ledger::ContractAddress::default());
        let context = if update {
            asset::setRecord(
                context,
                &witnesses,
                key.clone(),
                record(false)?,
                RecordMutation::Insert,
            )?
            .context
        } else {
            context
        };
        let private_state = context.private_state;
        if private_state != if update { 11 } else { 10 } {
            return Err("asset record prestate has wrong private witness count".into());
        }
        let value = record(update)?;
        let mutation = if update {
            RecordMutation::Update
        } else {
            RecordMutation::Insert
        };
        let mut rng = StdRng::seed_from_u64(if update { 0x0157_5002 } else { 0x0157_5001 });
        let deploy = make_deploy(
            root,
            CIRCUIT,
            context.query.state.get_ref().clone(),
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
        let native_witness = manual::TimestampWitness::default();
        let recorded_witness = manual::TimestampWitness::default();
        let observed_witness = manual::TimestampWitness::default();
        let manual_witness = manual::TimestampWitness::default();
        let generated = asset::Contract::from(observed_witness.clone());
        let native = asset::setRecord(
            observed.circuit_context(private_state),
            &native_witness,
            key.clone(),
            value.clone(),
            mutation,
        )?;
        let recorded = asset::recorded::setRecord(
            observed.circuit_context(private_state),
            &recorded_witness,
            key.clone(),
            value.clone(),
            mutation,
        )?;
        let call = generated.recording().setRecord_call(
            &observed,
            private_state,
            key.clone(),
            value.clone(),
            mutation,
        )?;
        let manual_execution = manual::write(
            observed.circuit_context(private_state),
            key.clone(),
            manual::StoredValue::Record(value.clone()),
            update,
            &manual_witness,
        )?;
        manual::compare(
            &native,
            &recorded,
            call.recorded(),
            &manual_execution,
            private_state,
            &[
                &native_witness,
                &recorded_witness,
                &observed_witness,
                &manual_witness,
            ],
        )?;
        let expected_state = native.context.query.state.get_ref().clone();
        let input = AlignedValue::concat(&[
            AlignedValue::from(key.clone()),
            AlignedValue::from(value.clone()),
            AlignedValue::from(mutation),
        ]);
        let manual = check_generated_trace(root, CIRCUIT, recorded, input.clone())?;
        let verifier: VerifierKey = tagged_deserialize(&mut BufReader::new(File::open(
            root.join("keys/setRecord.verifier"),
        )?))?;
        let typed = call.prepare(verifier.clone(), Fr::from(0_u64))?;
        if typed.input != input || typed.output != AlignedValue::from(()) {
            return Err(
                "Map write declared key/value/mutation order or Unit output differs".into(),
            );
        }
        if format!("{manual:?}") != format!("{typed:?}") {
            return Err(
                format!("asset record update={update} typed call differs from recording").into(),
            );
        }
        {
            let data = &expected_state;
            let stored = map_view_at_path::<runtime::OpaqueString, AssetRecord, _>(data, &[1, 10])?
                .lookup(key.clone())?;
            if stored != value {
                return Err("proven asset record stored wrong typed value".into());
            }
            let StateValue::Array(fields) = data else {
                return Err("asset record root is not an array".into());
            };
            let Some(StateValue::Array(metadata)) = fields.get(1) else {
                return Err("asset record metadata is not an array".into());
            };
            let expected_count = if update { 2 } else { 1 };
            if read_counter(metadata.get(5).ok_or("missing recordCount")?)? != 1 {
                return Err("proven asset record stored wrong record count".into());
            }
            for index in [8, 9] {
                if read_counter(metadata.get(index).ok_or("missing record write counter")?)?
                    != expected_count
                {
                    return Err("proven asset record stored wrong write count".into());
                }
            }
        }
        kernel_shielded_effects::prove_and_verify_call(root, CIRCUIT, &typed, &verifier)?;
        opaque_public::apply_funded(
            root,
            CIRCUIT,
            typed,
            deploy,
            &expected_state,
            rng,
            596 + u64::from(update),
        )?;
        println!(
            "typed asset record update={update}: source-bound native/recorded/observed/manual/replay, verified proof and changed-binding refusal, default-strict funded ledger-8 application; constructor and Update seed supplied, deployment and seed not proved"
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn map_write_source_changes_refuse_before_keys() {
        counter_scenario::validate_ir(RETAINED_IR.as_bytes(), RETAINED_IR).unwrap();
        let mut changed: serde_json::Value = serde_json::from_str(RETAINED_IR).unwrap();
        changed["independent_map_write_guard"] = true.into();
        assert!(
            counter_scenario::validate_ir(&serde_json::to_vec(&changed).unwrap(), RETAINED_IR)
                .is_err()
        );
    }
    #[test]
    fn map_write_absent_source_refuses_before_keys_or_state() {
        let root = std::env::temp_dir().join(format!(
            "compact-record-write-absent-{}-{}",
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
