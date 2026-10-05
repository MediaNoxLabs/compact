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

use super::*;
use compact_rust_test_center_welcome_fixture::ledger_contract as contract;
use compact_rust_test_center_welcome_fixture::types::Maybe;
use midnight_compact_runtime::{CompactError, OpaqueString};

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

    for circuit in ["add_participant", "add_organizer"] {
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
            circuit,
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
        let (recorded, input) = if circuit == "add_participant" {
            (
                generated
                    .recording()
                    .add_participant(observed.circuit_context(7_u64), OpaqueString::from("bob"))?,
                AlignedValue::from(OpaqueString::from("bob")),
            )
        } else {
            (
                generated
                    .recording()
                    .add_organizer(observed.circuit_context(7_u64), FixedBytes::new([7; 32]))?,
                AlignedValue::from(FixedBytes::new([7; 32])),
            )
        };
        if recorded.execution.context.private_state != 7
            || recorded.execution.private_transcript_outputs.len() != 1
            || recorded.public.verify_ops().len() != 10
        {
            return Err(format!("{circuit} changed ordered public/private effects").into());
        }
        let expected_state = recorded.execution.context.query.state.get_ref().clone();
        let manual = check_generated_trace(root, circuit, recorded, input)?;
        let verifier: VerifierKey = tagged_deserialize(&mut BufReader::new(File::open(
            root.join(format!("keys/{circuit}.verifier")),
        )?))?;
        let typed = if circuit == "add_participant" {
            generated
                .recording()
                .add_participant_call(&observed, 7_u64, OpaqueString::from("bob"))?
                .prepare(verifier, Fr::from(0_u64))?
        } else {
            generated
                .recording()
                .add_organizer_call(&observed, 7_u64, FixedBytes::new([7; 32]))?
                .prepare(verifier, Fr::from(0_u64))?
        };
        if format!("{manual:?}") != format!("{typed:?}") {
            return Err(
                format!("{circuit} typed observed call differs from direct recording").into(),
            );
        }
        check_transaction(root, circuit, deploy, typed, &mut rng, |state| {
            let data = state.data.get_ref();
            if data != &expected_state {
                return Err(format!("proven {circuit} changed unexpected ledger state").into());
            }
            if circuit == "add_participant" {
                if !set_view_at_path::<OpaqueString, _>(data, &[1])?
                    .member(OpaqueString::from("bob"))
                {
                    return Err("proven add_participant did not insert bob".into());
                }
            } else if !set_view_at_path::<FixedBytes<32>, _>(data, &[0])?
                .member(FixedBytes::new([7; 32]))
            {
                return Err("proven add_organizer did not insert the key".into());
            }
            Ok(())
        })?;
        println!("original Welcome {circuit} proof verified and applied through ledger-8");
    }
    Ok(())
}
