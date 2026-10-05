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

#[path = "../support.rs"]
mod support;
use compact_rust_test_center_bboard_fixture::ledger_contract as contract;
use midnight_compact_runtime as runtime;
use runtime::OpaqueString;
use runtime::context::CircuitContext;
use support::*;

fn state_hex(context: &CircuitContext<u64>) -> String {
    let mut bytes = Vec::new();
    midnight_serialize::tagged_serialize(&state(context.query.state.get_ref().clone()), &mut bytes)
        .unwrap();
    hex::encode(bytes)
}
fn check<T: PartialEq + std::fmt::Debug>(
    native: runtime::context::CircuitResult<u64, T>,
    recorded: runtime::recording::RecordedCircuitResult<u64, T>,
    expected: &serde_json::Value,
) -> CircuitContext<u64> {
    assert_eq!(native.result, recorded.execution.result);
    assert_eq!(native.gas_cost, recorded.execution.gas_cost);
    assert_eq!(
        native.private_transcript_outputs,
        recorded.execution.private_transcript_outputs
    );
    assert_eq!(
        native.context.private_state,
        recorded.execution.context.private_state
    );
    assert_eq!(
        native.context.query.state.get_ref(),
        recorded.execution.context.query.state.get_ref()
    );
    assert_eq!(
        native.context.query.effects,
        recorded.execution.context.query.effects
    );
    assert_eq!(state_hex(&recorded.execution.context), expected["stateHex"]);
    assert_eq!(
        recorded.execution.context.private_state,
        expected["privateState"].as_u64().unwrap()
    );
    let gas = serde_json::to_value(recorded.execution.gas_cost).unwrap();
    for dim in ["readTime", "computeTime", "bytesWritten", "bytesDeleted"] {
        let sum: u64 = expected["queries"]
            .as_array()
            .unwrap()
            .iter()
            .map(|q| q["gasCost"][dim].as_str().unwrap().parse::<u64>().unwrap())
            .sum();
        assert_eq!(gas[dim], sum);
    }
    assert_eq!(recorded.execution.private_transcript_outputs.len(), 1);
    let output = &recorded.execution.private_transcript_outputs[0];
    let atoms: Vec<_> = output.value.0.iter().map(|atom| &atom.0).collect();
    assert_eq!(
        serde_json::to_value(atoms).unwrap(),
        expected["privateTranscriptOutputs"][0]["value"]
    );
    assert_eq!(
        serde_json::to_value(&output.alignment).unwrap(),
        expected["privateTranscriptOutputs"][0]["alignment"]
    );
    let mut program = serde_json::to_value(recorded.public.verify_ops()).unwrap();
    for op in program.as_array_mut().unwrap() {
        if let Some(pop) = op.get_mut("popeq") {
            pop["result"] = serde_json::Value::Null;
        }
    }
    let expected_ops: Vec<_> = expected["queries"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|q| q["program"].as_array().unwrap().iter().cloned())
        .collect();
    assert_eq!(program, serde_json::json!(expected_ops));
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
    let gas = serde_json::to_value(replay.gas_cost).unwrap();
    for dim in ["readTime", "computeTime", "bytesWritten", "bytesDeleted"] {
        assert_eq!(
            gas[dim],
            expected["replayGas"][dim]
                .as_str()
                .unwrap()
                .parse::<u64>()
                .unwrap()
        );
    }
    rehydrate(recorded.execution.context)
}

#[test]
fn typed_recording_preserves_preclear_messages_instances_and_witnesses() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/bboard-recording-oracle.json"
    ))
    .unwrap();
    let mut current = initial();
    let mut previous_poster = None;
    for (index, expected) in oracle["scenarios"].as_array().unwrap().iter().enumerate() {
        let before = state(current.query.state.get_ref().clone());
        let native_context = CircuitContext::from_contract_state(
            current.private_state,
            runtime::ledger::ContractAddress::default(),
            &before,
        );
        let witness = Witness::default();
        let native_witness = Witness::default();
        current = if expected["method"] == "post" {
            let message = OpaqueString::from(expected["message"].as_str().unwrap());
            let native = contract::post(native_context, &native_witness, message.clone()).unwrap();
            let recorded = contract::recorded::post(current, &witness, message).unwrap();
            let poster = compact_rust_test_center_bboard_fixture::ledger_slots::poster
                .inspect(recorded.execution.context.query.state.get_ref())
                .unwrap();
            if let Some(previous) = previous_poster {
                assert_ne!(poster, previous);
            }
            previous_poster = Some(poster);
            check(native, recorded, expected)
        } else {
            let native = contract::take_down(native_context, &native_witness).unwrap();
            let recorded = contract::recorded::take_down(current, &witness).unwrap();
            assert_eq!(recorded.execution.result.0, expected["result"]);
            assert_eq!(
                compact_rust_test_center_bboard_fixture::ledger_slots::instance
                    .inspect(recorded.execution.context.query.state.get_ref())
                    .unwrap(),
                2 + (index / 2) as u64
            );
            check(native, recorded, expected)
        };
        assert_eq!(*native_witness.calls.borrow(), *witness.calls.borrow());
        assert_eq!(
            serde_json::to_value(&*witness.calls.borrow()).unwrap(),
            expected["witnessCalls"]
        );
    }
    for name in ["emptyTakeDown", "occupiedPost", "wrongOwner", "oldInstance"] {
        let make = || {
            let mut context = initial();
            if name != "emptyTakeDown" {
                context = rehydrate(
                    contract::post(context, &Witness::default(), "owner".into())
                        .unwrap()
                        .context,
                );
            }
            if name == "oldInstance" {
                let old = compact_rust_test_center_bboard_fixture::ledger_slots::poster
                    .inspect(context.query.state.get_ref())
                    .unwrap();
                context = rehydrate(
                    contract::take_down(context, &Witness::default())
                        .unwrap()
                        .context,
                );
                context = rehydrate(
                    contract::post(context, &Witness::default(), "next instance".into())
                        .unwrap()
                        .context,
                );
                context = rehydrate(context.write_cell_at_path(&[3], old).unwrap().context);
            }
            context
        };
        let witness = Witness {
            key: if name == "wrongOwner" { 8 } else { 7 },
            ..Default::default()
        };
        let native_witness = Witness {
            key: witness.key,
            ..Default::default()
        };
        let (native, recorded) = if name == "occupiedPost" {
            (
                contract::post(make(), &native_witness, "blocked".into())
                    .err()
                    .unwrap(),
                contract::recorded::post(make(), &witness, "blocked".into())
                    .err()
                    .unwrap(),
            )
        } else {
            (
                contract::take_down(make(), &native_witness).err().unwrap(),
                contract::recorded::take_down(make(), &witness)
                    .err()
                    .unwrap(),
            )
        };
        assert_eq!(native.to_string(), recorded.to_string());
        assert_eq!(recorded.to_string(), oracle[name]["error"], "{name}");
        assert_eq!(*witness.calls.borrow(), *native_witness.calls.borrow());
        assert_eq!(
            serde_json::to_value(&*witness.calls.borrow()).unwrap(),
            oracle[name]["witnessCalls"]
        );
    }
}
