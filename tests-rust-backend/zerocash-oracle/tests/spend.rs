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
use compact_rust_zerocash_oracle_fixture::ledger_contract as contract;
use midnight_compact_runtime as runtime;
use runtime::context::CircuitContext;
use support::*;
fn state_hex(context: &CircuitContext<Private>) -> String {
    let mut bytes = Vec::new();
    midnight_serialize::tagged_serialize(
        &contract_state(context.query.state.get_ref().clone()),
        &mut bytes,
    )
    .unwrap();
    hex::encode(bytes)
}
fn check<T: PartialEq + std::fmt::Debug>(
    native: runtime::context::CircuitResult<Private, T>,
    recorded: runtime::recording::RecordedCircuitResult<Private, T>,
    expected: &serde_json::Value,
) -> CircuitContext<Private> {
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
    let private = &recorded.execution.context.private_state;
    assert_eq!(
        serde_json::json!({"calls":private.calls,"next":private.next,"owned":private.owned}),
        expected["privateState"]
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
    assert_eq!(recorded.execution.private_transcript_outputs.len(), 5);
    for (output, expected) in recorded
        .execution
        .private_transcript_outputs
        .iter()
        .zip(expected["privateTranscriptOutputs"].as_array().unwrap())
    {
        let atoms: Vec<_> = output.value.0.iter().map(|atom| &atom.0).collect();
        assert_eq!(serde_json::to_value(atoms).unwrap(), expected["value"]);
        assert_eq!(
            serde_json::to_value(&output.alignment).unwrap(),
            expected["alignment"]
        );
    }
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
fn spend_records_current_and_historic_roots_recipient_bytes_and_ordered_private_effects() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/zerocash-spend.json"
    ))
    .unwrap();
    for expected in oracle["scenarios"].as_array().unwrap() {
        let historical = expected["historical"].as_bool().unwrap();
        let (context, path) = seeded(historical).unwrap();
        let (native_context, _) = seeded(historical).unwrap();
        let witness = Witness {
            path: historical.then_some(path.clone()),
            ..Default::default()
        };
        let native_witness = Witness {
            path: historical.then_some(path),
            ..Default::default()
        };
        let native =
            contract::spend(native_context, &native_witness, destination(), coin(3)).unwrap();
        let recorded =
            contract::recorded::spend(context, &witness, destination(), coin(3)).unwrap();
        let ciphertext = compact_rust_zerocash_oracle_fixture::ledger_slots::ciphertexts
            .inspect(recorded.execution.context.query.state.get_ref())
            .unwrap();
        assert_eq!(
            ciphertext,
            support::ciphertext(
                &destination().encryption,
                &coin(if historical { 7 } else { 5 })
            )
        );
        let _ = check(native, recorded, expected);
        assert_eq!(*native_witness.calls.borrow(), *witness.calls.borrow());
        assert_eq!(
            serde_json::to_value(&*witness.calls.borrow()).unwrap(),
            expected["witnessCalls"]
        );
    }
    for name in [
        "duplicate",
        "wrong_root",
        "wrong_leaf",
        "malformed",
        "encryption_failure",
    ] {
        let make = || {
            let (mut context, _) = seeded(name == "wrong_leaf").unwrap();
            if name == "duplicate" {
                context = rehydrate(
                    contract::spend(context, &Witness::default(), destination(), coin(3))
                        .unwrap()
                        .context,
                );
            }
            context
        };
        let mode = match name {
            "wrong_root" => Mode::WrongRoot,
            "wrong_leaf" => Mode::WrongLeaf,
            "malformed" => Mode::Malformed,
            "encryption_failure" => Mode::EncryptionFailure,
            _ => Mode::Normal,
        };
        let native_witness = Witness {
            mode,
            ..Default::default()
        };
        let witness = Witness {
            mode,
            ..Default::default()
        };
        let native = contract::spend(make(), &native_witness, destination(), coin(3))
            .err()
            .unwrap()
            .to_string();
        let recorded = contract::recorded::spend(make(), &witness, destination(), coin(3))
            .err()
            .unwrap()
            .to_string();
        assert_eq!(native, recorded, "{name}");
        if name == "malformed" {
            assert!(recorded.contains("Merkle path depth"));
            assert!(
                oracle["failures"][name]["error"]
                    .as_str()
                    .unwrap()
                    .starts_with("type error:")
            );
        } else {
            assert_eq!(recorded, oracle["failures"][name]["error"], "{name}");
        }
        assert_eq!(*native_witness.calls.borrow(), *witness.calls.borrow());
        assert_eq!(
            serde_json::to_value(&*witness.calls.borrow()).unwrap(),
            oracle["failures"][name]["witnessCalls"]
        );
    }
}
