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
// Licensed under the Apache License, Version 2.0 (the "License"); you may not use
// this file except in compliance with the License. You may obtain a copy of the
// License at http://www.apache.org/licenses/LICENSE-2.0

//! Prove the original Welcome source's recorded check-in and organizer paths.

mod check_in;

pub(super) use check_in::run as run_check_in;

use super::*;
use compact_rust_test_center_welcome_fixture::ledger_contract as contract;
use compact_rust_test_center_welcome_fixture::types::Maybe;
use midnight_compact_runtime::{CompactError, OpaqueString};
use std::{cell::RefCell, rc::Rc};

const RETAINED_IR: &str = include_str!("../../compact-rust-backend/tests/welcome-schema20-ir.json");

struct CheckInWitness;

impl contract::Witnesses<u64> for CheckInWitness {
    fn local_sk(
        &self,
        context: WitnessContext<'_, u64, contract::LedgerView<'_>>,
    ) -> (
        u64,
        compact_rust_test_center_welcome_fixture::types::MaybeCompact1,
    ) {
        (
            *context.private_state,
            compact_rust_test_center_welcome_fixture::types::MaybeCompact1 {
                is_some: true,
                value: FixedBytes::new([0; 32]),
            },
        )
    }

    fn set_local_id(
        &self,
        context: WitnessContext<'_, u64, contract::LedgerView<'_>>,
        participant: OpaqueString,
    ) -> (u64, ()) {
        assert_eq!(participant.0, "alice");
        (*context.private_state + 1, ())
    }
}

pub(super) fn run(root: &Path) -> Result<(), Box<dyn Error>> {
    let mut rng = StdRng::seed_from_u64(0x0120_5745_4c43_4f4d);
    let participants = FixedVector::new(std::array::from_fn(|index| Maybe {
        is_some: index == 7,
        value: OpaqueString::from(if index == 7 { "alice" } else { "" }),
    }));
    let initial = contract::initial_state(
        ConstructorContext::new(7_u64),
        &CheckInWitness,
        participants,
    )?;
    let deploy = make_deploy(
        root,
        "check_in",
        initial.ledger_state.get_ref().clone(),
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
    let generated = contract::Contract::from(CheckInWitness);
    let failure = generated
        .recording()
        .check_in_call(&observed, 7_u64, OpaqueString::from("bob"))
        .err()
        .ok_or("ineligible Welcome check-in unexpectedly produced an observed call")?;
    if !matches!(failure, CompactError::AssertionFailed(_)) {
        return Err(format!("ineligible check-in failed incorrectly: {failure}").into());
    }
    let recorded = generated
        .recording()
        .check_in(observed.circuit_context(7_u64), OpaqueString::from("alice"))?;
    if recorded.execution.context.private_state != 8
        || recorded.execution.private_transcript_outputs.len() != 1
        || recorded.public.verify_ops().len() != 10
    {
        return Err("Welcome check-in recording changed its public/private effects".into());
    }
    let expected_state = recorded.execution.context.query.state.get_ref().clone();
    let manual = check_generated_trace(root, "check_in", recorded, OpaqueString::from("alice"))?;
    let verifier: VerifierKey = tagged_deserialize(&mut BufReader::new(File::open(
        root.join("keys/check_in.verifier"),
    )?))?;
    let typed = generated
        .recording()
        .check_in_call(&observed, 7_u64, OpaqueString::from("alice"))?
        .prepare(verifier, Fr::from(0_u64))?;
    if format!("{manual:?}") != format!("{typed:?}") {
        return Err("Welcome typed observed call differs from direct recording".into());
    }
    check_transaction(root, "check_in", deploy, typed, &mut rng, |state| {
        let data = state.data.get_ref();
        if data != &expected_state {
            return Err("proven Welcome check-in changed unexpected ledger state".into());
        }
        if !set_view_at_path::<OpaqueString, _>(data, &[2])?.member(OpaqueString::from("alice")) {
            return Err("proven Welcome check-in did not insert alice".into());
        }
        Ok(())
    })?;
    println!("original Welcome check_in proof verified and applied through ledger-8");

    organizer_calls(root, &mut rng)
}

/// Prove only the organizer roots; no check-in key material is required.
pub(super) fn run_organizers(root: &Path) -> Result<(), Box<dyn Error>> {
    let mut rng = StdRng::seed_from_u64(0x0590_5745_4c43_4f4d);
    organizer_calls(root, &mut rng)
}

#[derive(Clone, Default)]
struct OrganizerWitness(Rc<RefCell<Vec<u64>>>);
impl contract::Witnesses<u64> for OrganizerWitness {
    fn local_sk(
        &self,
        context: WitnessContext<'_, u64, contract::LedgerView<'_>>,
    ) -> (
        u64,
        compact_rust_test_center_welcome_fixture::types::MaybeCompact1,
    ) {
        self.0.borrow_mut().push(*context.private_state);
        (
            *context.private_state,
            compact_rust_test_center_welcome_fixture::types::MaybeCompact1 {
                is_some: true,
                value: FixedBytes::new([0; 32]),
            },
        )
    }
    fn set_local_id(
        &self,
        _: WitnessContext<'_, u64, contract::LedgerView<'_>>,
        _: OpaqueString,
    ) -> (u64, ()) {
        panic!("organizer calls must not set a local participant id")
    }
}

fn organizer_calls(root: &Path, rng: &mut StdRng) -> Result<(), Box<dyn Error>> {
    counter_scenario::validate_ir(
        &std::fs::read(root.join("contract/compact-rust-ir.json"))?,
        RETAINED_IR,
    )?;
    for circuit in ["add_participant", "add_organizer"] {
        let participants = FixedVector::new(std::array::from_fn(|index| Maybe {
            is_some: index == 7,
            value: OpaqueString::from(if index == 7 { "alice" } else { "" }),
        }));
        // Constructor state is supplied, not proved as a deployment transaction.
        let initial = contract::initial_state(
            ConstructorContext::new(7_u64),
            &CheckInWitness,
            participants,
        )?;
        let deploy = make_deploy(root, circuit, initial.ledger_state.get_ref().clone(), rng)?;
        let observed = ObservedContractState::new(
            deploy.address(),
            deploy.initial_state.clone(),
            Observation {
                transaction_hash: [0; 32],
                block_hash: [0; 32],
                block_height: 0,
            },
        );
        let native_witness = OrganizerWitness::default();
        let recorded_witness = OrganizerWitness::default();
        let observed_witness = OrganizerWitness::default();
        let generated = contract::Contract::from(observed_witness.clone());
        let (native, recorded, call, input) = if circuit == "add_participant" {
            (
                contract::add_participant(
                    observed.circuit_context(7_u64),
                    &native_witness,
                    OpaqueString::from("bob"),
                )?,
                contract::recorded::add_participant(
                    observed.circuit_context(7_u64),
                    &recorded_witness,
                    OpaqueString::from("bob"),
                )?,
                generated.recording().add_participant_call(
                    &observed,
                    7_u64,
                    OpaqueString::from("bob"),
                )?,
                AlignedValue::from(OpaqueString::from("bob")),
            )
        } else {
            (
                contract::add_organizer(
                    observed.circuit_context(7_u64),
                    &native_witness,
                    FixedBytes::new([7; 32]),
                )?,
                contract::recorded::add_organizer(
                    observed.circuit_context(7_u64),
                    &recorded_witness,
                    FixedBytes::new([7; 32]),
                )?,
                generated.recording().add_organizer_call(
                    &observed,
                    7_u64,
                    FixedBytes::new([7; 32]),
                )?,
                AlignedValue::from(FixedBytes::new([7; 32])),
            )
        };
        for witness in [&native_witness, &recorded_witness, &observed_witness] {
            if *witness.0.borrow() != [7] {
                return Err(format!("{circuit} changed the local_sk callback prefix").into());
            }
        }
        let expected_private = vec![AlignedValue::from(
            compact_rust_test_center_welcome_fixture::types::MaybeCompact1 {
                is_some: true,
                value: FixedBytes::new([0; 32]),
            },
        )];
        if native.context.private_state != 7
            || native.private_transcript_outputs != expected_private
        {
            return Err(format!("{circuit} changed native private output").into());
        }
        for candidate in [&recorded, call.recorded()] {
            if candidate.execution.context.query.state != native.context.query.state
                || candidate.execution.context.query.effects != native.context.query.effects
                || candidate.execution.gas_cost != native.gas_cost
                || candidate.execution.context.private_state != 7
                || candidate.execution.private_transcript_outputs != expected_private
                || candidate.public.verify_ops() != recorded.public.verify_ops()
                || candidate.public.verify_ops().len() != 10
            {
                return Err(format!("{circuit} native/recorded/observed effects differ").into());
            }
        }
        let expected_state = native.context.query.state.get_ref().clone();
        if circuit == "add_participant" {
            if !set_view_at_path::<OpaqueString, _>(&expected_state, &[1])?
                .member(OpaqueString::from("bob"))
            {
                return Err("add_participant did not insert bob".into());
            }
        } else if !set_view_at_path::<FixedBytes<32>, _>(&expected_state, &[0])?
            .member(FixedBytes::new([7; 32]))
        {
            return Err("add_organizer did not insert the key".into());
        }
        let manual = check_generated_trace(root, circuit, recorded, input.clone())?;
        let verifier: VerifierKey = tagged_deserialize(&mut BufReader::new(File::open(
            root.join(format!("keys/{circuit}.verifier")),
        )?))?;
        let typed = call.prepare(verifier.clone(), Fr::from(0_u64))?;
        if typed.input != input || typed.output != AlignedValue::from(()) {
            return Err(format!("{circuit} changed original input/Unit output bindings").into());
        }
        if format!("{manual:?}") != format!("{typed:?}") {
            return Err(
                format!("{circuit} typed observed call differs from direct recording").into(),
            );
        }
        kernel_shielded_effects::prove_and_verify_call(root, circuit, &typed, &verifier)?;
        opaque_public::apply_funded(
            root,
            circuit,
            typed,
            deploy,
            &expected_state,
            rng.clone(),
            590,
        )?;
        println!(
            "original Welcome {circuit}: verified proof, changed-binding refusal and default-strict funded ledger-8 application; constructor state supplied, deployment not proved"
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn organizer_source_changes_refuse_before_keys() {
        counter_scenario::validate_ir(RETAINED_IR.as_bytes(), RETAINED_IR).unwrap();
        let mut changed: serde_json::Value = serde_json::from_str(RETAINED_IR).unwrap();
        changed["independent_organizer_guard"] = true.into();
        assert!(
            counter_scenario::validate_ir(&serde_json::to_vec(&changed).unwrap(), RETAINED_IR)
                .is_err()
        );
    }
    #[test]
    fn organizer_absent_source_refuses_before_keys_or_state() {
        let root = std::env::temp_dir().join(format!(
            "compact-welcome-organizer-absent-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        assert!(!root.exists());
        assert_eq!(
            run_organizers(&root)
                .unwrap_err()
                .downcast_ref::<io::Error>()
                .unwrap()
                .kind(),
            io::ErrorKind::NotFound
        );
        assert!(!root.exists());
    }
}
