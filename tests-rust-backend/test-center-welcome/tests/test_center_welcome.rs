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

use std::cell::{Cell, RefCell};

use compact_rust_test_center_welcome_fixture::ledger_contract::{
    LedgerView, PublicStateView, Witnesses, add_organizer, add_participant, check_in,
    initial_state, recorded,
};
use compact_rust_test_center_welcome_fixture::types::{Maybe, MaybeCompact1};
use midnight_compact_runtime as runtime;
use midnight_onchain_state::state::{
    ContractMaintenanceAuthority, ContractOperation, ContractState, EntryPointBuf,
};
use midnight_serialize::tagged_serialize;
use midnight_storage::storage::HashMap;
use runtime::context::{ConstructorContext, WitnessContext};
use runtime::ledger::ContractAddress;
use runtime::ledger::{DefaultDB, StateValue};
use runtime::{FixedBytes, FixedVector, OpaqueString};

#[derive(Clone, Copy, Default)]
enum SecretMode {
    #[default]
    Zero,
    Missing,
    Other,
}

#[derive(Default)]
struct OrganizerWitness(
    RefCell<Vec<u64>>,
    RefCell<Vec<(u64, String)>>,
    Cell<SecretMode>,
);

impl Witnesses<u64> for OrganizerWitness {
    fn local_sk(&self, context: WitnessContext<'_, u64, LedgerView<'_>>) -> (u64, MaybeCompact1) {
        self.0.borrow_mut().push(*context.private_state);
        (
            *context.private_state,
            MaybeCompact1 {
                is_some: !matches!(self.2.get(), SecretMode::Missing),
                value: FixedBytes::new(
                    [if matches!(self.2.get(), SecretMode::Other) {
                        1
                    } else {
                        0
                    }; 32],
                ),
            },
        )
    }

    fn set_local_id(
        &self,
        context: WitnessContext<'_, u64, LedgerView<'_>>,
        participant: OpaqueString,
    ) -> (u64, ()) {
        self.1
            .borrow_mut()
            .push((*context.private_state, participant.0));
        (*context.private_state + 1, ())
    }
}

fn participants(present: bool) -> FixedVector<Maybe, 5000> {
    FixedVector::new(std::array::from_fn(|index| Maybe {
        is_some: present && index == 7,
        value: OpaqueString::from(if present && index == 7 { "alice" } else { "" }),
    }))
}

fn vm_shape(actual: serde_json::Value) -> serde_json::Value {
    serde_json::Value::Array(actual.as_array().unwrap().iter().map(|operation| {
        if let Some(kind) = operation.as_str() { return serde_json::json!({"kind":kind}); }
        if let Some(idx) = operation.get("idx") {
            serde_json::json!({"kind":"idx", "cached":idx["cached"], "pushPath":idx["pushPath"], "pathLength":idx["path"].as_array().unwrap().len()})
        } else if let Some(push) = operation.get("push") {
            serde_json::json!({"kind":"push", "storage":push["storage"]})
        } else if let Some(ins) = operation.get("ins") {
            serde_json::json!({"kind":"ins", "cached":ins["cached"], "n":ins["n"]})
        } else if let Some(dup) = operation.get("dup") {
            serde_json::json!({"kind":"dup", "n":dup["n"]})
        } else if let Some(popeq) = operation.get("popeq") {
            serde_json::json!({"kind":"popeq", "cached":popeq["cached"], "resultAtoms":popeq["result"]["value"]})
        } else { panic!("unexpected VM operation: {operation}") }
    }).collect())
}

#[test]
fn original_welcome_check_in_matches_typescript_and_replays() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/test-center-welcome.json"
    ))
    .unwrap();
    let success = &oracle["cases"][1]["checkIn"];
    let native_witness = OrganizerWitness::default();
    let native_initial = initial_state(
        ConstructorContext::new(7_u64),
        &native_witness,
        participants(true),
    )
    .unwrap();
    assert_eq!(
        state_hex(native_initial.ledger_state.get_ref().clone()),
        success["initialStateHex"]
    );
    let native = check_in(
        native_initial.into_circuit_context(ContractAddress::default()),
        &native_witness,
        OpaqueString::from("alice"),
    )
    .unwrap();
    let recorded_witness = OrganizerWitness::default();
    let recorded_initial = initial_state(
        ConstructorContext::new(7_u64),
        &recorded_witness,
        participants(true),
    )
    .unwrap();
    let recorded = recorded::check_in(
        recorded_initial.into_circuit_context(ContractAddress::default()),
        &recorded_witness,
        OpaqueString::from("alice"),
    )
    .unwrap();
    let _: () = native.result;
    let _: () = recorded.execution.result;
    assert_eq!(success["result"], serde_json::json!([]));
    for (execution, witness) in [
        (&native, &native_witness),
        (&recorded.execution, &recorded_witness),
    ] {
        assert_eq!(
            state_hex(execution.context.query.state.get_ref().clone()),
            success["afterStateHex"]
        );
        assert_eq!(
            execution.context.private_state,
            success["privateState"].as_u64().unwrap()
        );
        assert_eq!(*witness.1.borrow(), vec![(7, "alice".to_owned())]);
        let cost = serde_json::to_value(execution.gas_cost).unwrap();
        for dimension in ["readTime", "computeTime", "bytesWritten", "bytesDeleted"] {
            let oracle_total: u64 = success["queries"]
                .as_array()
                .unwrap()
                .iter()
                .map(|query| {
                    query["gasCost"][dimension]
                        .as_str()
                        .unwrap()
                        .parse::<u64>()
                        .unwrap()
                })
                .sum();
            assert_eq!(
                cost[dimension].as_u64().unwrap(),
                oracle_total,
                "{dimension}"
            );
        }
        assert_eq!(execution.private_transcript_outputs.len(), 1);
        let output = &execution.private_transcript_outputs[0];
        let atoms = output
            .value
            .0
            .iter()
            .map(|atom| &atom.0)
            .collect::<Vec<_>>();
        assert_eq!(
            serde_json::to_value(atoms).unwrap(),
            success["privateTranscriptOutputs"][0]["valueAtoms"]
        );
        assert_eq!(
            serde_json::to_value(&output.alignment).unwrap(),
            success["privateTranscriptOutputs"][0]["alignment"]
        );
    }
    assert_eq!(native.gas_cost, recorded.execution.gas_cost);
    assert_eq!(
        native.context.query.effects,
        recorded.execution.context.query.effects
    );
    assert_eq!(
        native.private_transcript_outputs,
        recorded.execution.private_transcript_outputs
    );
    assert_eq!(
        vm_shape(serde_json::to_value(recorded.public.verify_ops()).unwrap()),
        success["publicTranscriptShape"]
    );
    let replay = recorded
        .public
        .initial()
        .query(
            recorded.public.verify_ops(),
            None,
            &recorded.execution.context.cost_model,
        )
        .unwrap();
    assert_eq!(
        replay.context.state.get_ref(),
        recorded.execution.context.query.state.get_ref()
    );
    assert_eq!(
        replay.context.effects,
        recorded.execution.context.query.effects
    );

    let failure = &oracle["cases"][0]["checkIn"];
    assert_eq!(failure["compactError"], true);
    assert_eq!(failure["initialStateHex"], failure["afterStateHex"]);
    assert!(failure["witnessCalls"].as_array().unwrap().is_empty());
    for recorded_mode in [false, true] {
        let witness = OrganizerWitness::default();
        let initial = initial_state(
            ConstructorContext::new(7_u64),
            &witness,
            participants(false),
        )
        .unwrap();
        assert_eq!(
            state_hex(initial.ledger_state.get_ref().clone()),
            failure["initialStateHex"]
        );
        let context = initial.into_circuit_context(ContractAddress::default());
        let error = if recorded_mode {
            recorded::check_in(context, &witness, OpaqueString::from("bob"))
                .err()
                .unwrap()
        } else {
            check_in(context, &witness, OpaqueString::from("bob"))
                .err()
                .unwrap()
        };
        assert!(matches!(error, runtime::CompactError::AssertionFailed(_)));
        assert_eq!(error.to_string(), failure["error"]);
        assert!(witness.1.borrow().is_empty());
    }
}

#[test]
fn original_welcome_organizer_gate_matches_typescript_success_and_failures() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/test-center-welcome.json"
    ))
    .unwrap();
    for (case_name, participant) in [("participantSuccess", true), ("organizerSuccess", false)] {
        let expected = &oracle["organizerCalls"][case_name];
        assert_eq!(expected["success"], true);
        let native_witness = OrganizerWitness::default();
        let native_initial = initial_state(
            ConstructorContext::new(7_u64),
            &native_witness,
            participants(true),
        )
        .unwrap();
        assert_eq!(
            state_hex(native_initial.ledger_state.get_ref().clone()),
            expected["initialStateHex"]
        );
        let native_context = native_initial.into_circuit_context(ContractAddress::default());
        let native = if participant {
            add_participant(native_context, &native_witness, OpaqueString::from("bob")).unwrap()
        } else {
            add_organizer(native_context, &native_witness, FixedBytes::new([7; 32])).unwrap()
        };
        let recorded_witness = OrganizerWitness::default();
        let recorded_initial = initial_state(
            ConstructorContext::new(7_u64),
            &recorded_witness,
            participants(true),
        )
        .unwrap();
        let recorded_context = recorded_initial.into_circuit_context(ContractAddress::default());
        let recorded = if participant {
            recorded::add_participant(
                recorded_context,
                &recorded_witness,
                OpaqueString::from("bob"),
            )
            .unwrap()
        } else {
            recorded::add_organizer(
                recorded_context,
                &recorded_witness,
                FixedBytes::new([7; 32]),
            )
            .unwrap()
        };
        let _: () = native.result;
        let _: () = recorded.execution.result;
        assert_eq!(expected["result"], serde_json::json!([]));
        for (execution, witness) in [
            (&native, &native_witness),
            (&recorded.execution, &recorded_witness),
        ] {
            assert_eq!(
                state_hex(execution.context.query.state.get_ref().clone()),
                expected["afterStateHex"]
            );
            assert_eq!(
                execution.context.private_state,
                expected["privateState"].as_u64().unwrap()
            );
            assert_eq!(
                serde_json::to_value(&witness.0.borrow()[1..]).unwrap(),
                expected["witnessCalls"]
            );
            assert!(witness.1.borrow().is_empty());
            let actual_cost = serde_json::to_value(execution.gas_cost).unwrap();
            for dimension in ["readTime", "computeTime", "bytesWritten", "bytesDeleted"] {
                let expected_total: u64 = expected["queries"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|query| {
                        query["gasCost"][dimension]
                            .as_str()
                            .unwrap()
                            .parse::<u64>()
                            .unwrap()
                    })
                    .sum();
                assert_eq!(
                    actual_cost[dimension].as_u64().unwrap(),
                    expected_total,
                    "{case_name} {dimension}"
                );
            }
            assert_eq!(execution.private_transcript_outputs.len(), 1);
            let output = &execution.private_transcript_outputs[0];
            let atoms = output
                .value
                .0
                .iter()
                .map(|atom| &atom.0)
                .collect::<Vec<_>>();
            assert_eq!(
                serde_json::to_value(atoms).unwrap(),
                expected["privateTranscriptOutputs"][0]["valueAtoms"]
            );
            assert_eq!(
                serde_json::to_value(&output.alignment).unwrap(),
                expected["privateTranscriptOutputs"][0]["alignment"]
            );
        }
        assert_eq!(native.gas_cost, recorded.execution.gas_cost);
        assert_eq!(
            native.context.query.effects,
            recorded.execution.context.query.effects
        );
        assert_eq!(
            native.private_transcript_outputs,
            recorded.execution.private_transcript_outputs
        );
        assert_eq!(
            vm_shape(serde_json::to_value(recorded.public.verify_ops()).unwrap()),
            expected["publicTranscriptShape"]
        );
        let replay = recorded
            .public
            .initial()
            .query(
                recorded.public.verify_ops(),
                None,
                &recorded.execution.context.cost_model,
            )
            .unwrap();
        assert_eq!(
            replay.context.state.get_ref(),
            recorded.execution.context.query.state.get_ref()
        );
        assert_eq!(
            replay.context.effects,
            recorded.execution.context.query.effects
        );
    }

    for (case_name, participant, mode) in [
        ("participantMissingKey", true, SecretMode::Missing),
        ("participantNotOrganizer", true, SecretMode::Other),
        ("organizerMissingKey", false, SecretMode::Missing),
        ("organizerNotOrganizer", false, SecretMode::Other),
    ] {
        let expected = &oracle["organizerCalls"][case_name];
        assert_eq!(expected["success"], false);
        assert_eq!(expected["compactError"], true);
        assert_eq!(expected["initialStateHex"], expected["afterStateHex"]);
        assert_eq!(
            expected["queries"].as_array().unwrap().len(),
            if matches!(mode, SecretMode::Missing) {
                0
            } else {
                1
            }
        );
        for recorded_mode in [false, true] {
            let witness = OrganizerWitness::default();
            let initial =
                initial_state(ConstructorContext::new(7_u64), &witness, participants(true))
                    .unwrap();
            assert_eq!(
                state_hex(initial.ledger_state.get_ref().clone()),
                expected["initialStateHex"]
            );
            witness.2.set(mode);
            let context = initial.into_circuit_context(ContractAddress::default());
            let error = match (recorded_mode, participant) {
                (false, true) => add_participant(context, &witness, OpaqueString::from("bob"))
                    .err()
                    .unwrap(),
                (true, true) => {
                    recorded::add_participant(context, &witness, OpaqueString::from("bob"))
                        .err()
                        .unwrap()
                }
                (false, false) => add_organizer(context, &witness, FixedBytes::new([7; 32]))
                    .err()
                    .unwrap(),
                (true, false) => {
                    recorded::add_organizer(context, &witness, FixedBytes::new([7; 32]))
                        .err()
                        .unwrap()
                }
            };
            assert!(matches!(error, runtime::CompactError::AssertionFailed(_)));
            assert_eq!(error.to_string(), expected["error"]);
            assert_eq!(
                serde_json::to_value(&witness.0.borrow()[1..]).unwrap(),
                expected["witnessCalls"]
            );
            assert!(witness.1.borrow().is_empty());
        }
    }
}

fn state_hex(state: StateValue<DefaultDB>) -> String {
    let mut operations: HashMap<EntryPointBuf, ContractOperation, DefaultDB> = HashMap::new();
    for name in ["add_participant", "add_organizer", "check_in"] {
        operations = operations.insert(
            EntryPointBuf(name.as_bytes().to_vec()),
            ContractOperation::new(None),
        );
    }
    let state = ContractState::new(state, operations, ContractMaintenanceAuthority::default());
    let mut bytes = Vec::new();
    tagged_serialize(&state, &mut bytes).unwrap();
    hex::encode(bytes)
}

#[test]
fn original_welcome_constructor_matches_typescript_empty_and_one_participant() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/test-center-welcome.json"
    ))
    .unwrap();
    assert_eq!(
        oracle["source"],
        "test-center/test-contracts/welcome.compact"
    );
    for case in oracle["cases"].as_array().unwrap() {
        let present = case["present"].as_bool().unwrap();
        let participants = participants(present);
        let witness = OrganizerWitness::default();
        let initial =
            initial_state(ConstructorContext::new(7_u64), &witness, participants).unwrap();
        assert_eq!(
            serde_json::to_value(witness.0.borrow().as_slice()).unwrap(),
            case["witnessCalls"]
        );
        assert_eq!(
            initial.private_state,
            case["privateState"].as_u64().unwrap()
        );
        assert_eq!(
            state_hex(initial.ledger_state.get_ref().clone()),
            case["stateHex"]
        );
        let view = PublicStateView::from(&initial);
        assert_eq!(
            view.eligible_participants()
                .unwrap()
                .member(OpaqueString::from("alice")),
            case["eligibleAlice"].as_bool().unwrap()
        );
        assert_eq!(
            view.eligible_participants()
                .unwrap()
                .size()
                .unwrap()
                .value(),
            case["eligibleSize"]
                .as_str()
                .unwrap()
                .parse::<u128>()
                .unwrap()
        );
        assert_eq!(
            view.organizer_pks().unwrap().size().unwrap().value(),
            case["organizerSize"]
                .as_str()
                .unwrap()
                .parse::<u128>()
                .unwrap()
        );
        // The checked TS fixture also keeps ordered VM tags and all four gas
        // dimensions. ConstructorResult does not expose these to Rust callers,
        // so this test asserts the observable state/private boundary only.
        assert_eq!(
            case["constructorQueries"].as_array().unwrap().len(),
            if present { 5 } else { 4 }
        );
    }
}
