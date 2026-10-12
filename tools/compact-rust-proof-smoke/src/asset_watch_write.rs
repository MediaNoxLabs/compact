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

//! Prove typed asset watch Add and Drop with a Unicode opaque key.

use super::*;
use compact_rust_asset_registry_oracle_fixture::ledger_contract as asset;
use compact_rust_asset_registry_oracle_fixture::types::{ListMutation, RecordMutation};
use midnight_compact_runtime as runtime;
use std::{cell::RefCell, rc::Rc};

const RETAINED_IR: &str =
    include_str!("../../compact-rust-backend/tests/asset-registry-schema20-ir.json");

#[derive(Clone, Default)]
struct WatchWitness(Rc<RefCell<Vec<u64>>>);
impl asset::Witnesses<u64> for WatchWitness {
    fn localOperatorKey(
        &self,
        _: WitnessContext<'_, u64, asset::LedgerView<'_>>,
    ) -> (u64, runtime::JubjubPoint) {
        panic!("setWatch must not request an operator key")
    }
    fn localAuditorKey(
        &self,
        _: WitnessContext<'_, u64, asset::LedgerView<'_>>,
    ) -> (u64, runtime::JubjubPoint) {
        panic!("setWatch must not request an auditor key")
    }
    fn currentTimestamp(
        &self,
        context: WitnessContext<'_, u64, asset::LedgerView<'_>>,
    ) -> (u64, BoundedUint<{ u64::MAX as u128 }>) {
        self.0.borrow_mut().push(*context.private_state);
        asset::Witnesses::currentTimestamp(&super::asset_writable::AssetWitness, context)
    }
}

pub(super) fn run(root: &Path) -> Result<(), Box<dyn Error>> {
    counter_scenario::validate_ir(
        &std::fs::read(root.join("contract/compact-rust-ir.json"))?,
        RETAINED_IR,
    )?;
    const CIRCUIT: &str = "setWatch";
    let key = runtime::OpaqueString::from("record-α-1");
    for drop in [false, true] {
        let witnesses = super::asset_writable::AssetWitness;
        let initial = asset::initial_state(ConstructorContext::new(7_u64), &witnesses)?;
        if initial.private_state != 10 {
            return Err("asset watch constructor private state differs from TypeScript".into());
        }
        let context = asset::setRecord(
            initial.into_circuit_context(runtime::ledger::ContractAddress::default()),
            &witnesses,
            key.clone(),
            super::asset_record_write::record(false)?,
            RecordMutation::Insert,
        )?
        .context;
        let context = if drop {
            asset::setWatch(context, &witnesses, key.clone(), ListMutation::Add)?.context
        } else {
            context
        };
        let private_state = context.private_state;
        if private_state != if drop { 12 } else { 11 } {
            return Err("asset watch prestate has wrong private witness count".into());
        }
        let mutation = if drop {
            ListMutation::Drop
        } else {
            ListMutation::Add
        };
        let mut rng = StdRng::seed_from_u64(if drop { 0x0161_5002 } else { 0x0161_5001 });
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
        let native_witness = WatchWitness::default();
        let recorded_witness = WatchWitness::default();
        let observed_witness = WatchWitness::default();
        let generated = asset::Contract::from(observed_witness.clone());
        // Invalid mutation is an early guard: no timestamp callback in any lane.
        let native_error = asset::setWatch(
            observed.circuit_context(private_state),
            &native_witness,
            key.clone(),
            ListMutation::Unspecified,
        )
        .err()
        .ok_or("invalid watch mutation succeeded natively")?;
        let recorded_error = asset::recorded::setWatch(
            observed.circuit_context(private_state),
            &recorded_witness,
            key.clone(),
            ListMutation::Unspecified,
        )
        .err()
        .ok_or("invalid watch mutation recorded")?;
        let observed_error = generated
            .recording()
            .setWatch_call(
                &observed,
                private_state,
                key.clone(),
                ListMutation::Unspecified,
            )
            .err()
            .ok_or("invalid watch mutation observed")?;
        if native_error
            != runtime::CompactError::AssertionFailed("watch mutation must be Add or Drop".into())
            || recorded_error != native_error
            || observed_error != native_error
            || !native_witness.0.borrow().is_empty()
            || !recorded_witness.0.borrow().is_empty()
            || !observed_witness.0.borrow().is_empty()
        {
            return Err("watch invalid-mutation guard/error prefix differs".into());
        }
        let native = asset::setWatch(
            observed.circuit_context(private_state),
            &native_witness,
            key.clone(),
            mutation,
        )?;
        let recorded = asset::recorded::setWatch(
            observed.circuit_context(private_state),
            &recorded_witness,
            key.clone(),
            mutation,
        )?;
        let call =
            generated
                .recording()
                .setWatch_call(&observed, private_state, key.clone(), mutation)?;
        let expected_private = vec![AlignedValue::from(
            BoundedUint::<{ u64::MAX as u128 }>::new(1_700_000_000)?,
        )];
        if native.context.private_state != private_state + 1
            || native.private_transcript_outputs != expected_private
        {
            return Err("watch native private timestamp differs".into());
        }
        for witness in [&native_witness, &recorded_witness, &observed_witness] {
            if *witness.0.borrow() != [private_state] {
                return Err("watch timestamp callback order differs".into());
            }
        }
        for candidate in [&recorded, call.recorded()] {
            if native.context.query.state != candidate.execution.context.query.state
                || native.context.query.effects != candidate.execution.context.query.effects
                || native.gas_cost != candidate.execution.gas_cost
                || candidate.execution.context.private_state != private_state + 1
                || candidate.execution.private_transcript_outputs != expected_private
                || candidate.public.verify_ops() != recorded.public.verify_ops()
            {
                return Err(format!(
                    "asset watch drop={drop} native/recorded/observed execution differs"
                )
                .into());
            }
        }
        let expected_state = native.context.query.state.get_ref().clone();
        let input = AlignedValue::concat(&[
            AlignedValue::from(key.clone()),
            AlignedValue::from(mutation),
        ]);
        let manual = check_generated_trace(root, CIRCUIT, recorded, input.clone())?;
        let verifier: VerifierKey = tagged_deserialize(&mut BufReader::new(File::open(
            root.join("keys/setWatch.verifier"),
        )?))?;
        let typed = call.prepare(verifier.clone(), Fr::from(0_u64))?;
        if typed.input != input || typed.output != AlignedValue::from(()) {
            return Err("watch declared input/Unit output binding differs".into());
        }
        if format!("{manual:?}") != format!("{typed:?}") {
            return Err(
                format!("asset watch drop={drop} typed call differs from recording").into(),
            );
        }
        {
            let data = &expected_state;
            if set_view_at_path::<runtime::OpaqueString, _>(data, &[1, 13])?.member(key.clone())
                == drop
            {
                return Err("proven asset watch stored wrong membership".into());
            }
            let StateValue::Array(fields) = data else {
                return Err("asset watch root is not an array".into());
            };
            let Some(StateValue::Array(metadata)) = fields.get(1) else {
                return Err("asset watch metadata is not an array".into());
            };
            if read_counter(metadata.get(5).ok_or("missing recordCount")?)? != 1 {
                return Err("proven asset watch changed record count".into());
            }
            let expected_writes = if drop { 3 } else { 2 };
            for index in [8, 9] {
                if read_counter(metadata.get(index).ok_or("missing watch write counter")?)?
                    != expected_writes
                {
                    return Err("proven asset watch stored wrong write count".into());
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
            592 + u64::from(drop),
        )?;
        println!(
            "typed asset watch drop={drop}: verified proof, changed-binding refusal and default-strict funded ledger-8 application; constructor and seed state supplied, deployment and seed calls not proved"
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn watch_source_changes_refuse_before_keys() {
        counter_scenario::validate_ir(RETAINED_IR.as_bytes(), RETAINED_IR).unwrap();
        let mut changed: serde_json::Value = serde_json::from_str(RETAINED_IR).unwrap();
        changed["independent_watch_guard"] = true.into();
        assert!(
            counter_scenario::validate_ir(&serde_json::to_vec(&changed).unwrap(), RETAINED_IR)
                .is_err()
        );
    }
    #[test]
    fn watch_absent_source_refuses_before_keys_or_state() {
        let root = std::env::temp_dir().join(format!(
            "compact-watch-proof-absent-{}-{}",
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
