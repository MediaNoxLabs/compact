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

use compact_rust_assert_witness_fixture::ledger_contract::{
    LedgerView, Witnesses, checked_value, checked_write, initial_state, read_cell, recorded,
};
use midnight_compact_runtime::Field;
use midnight_compact_runtime::context::{ConstructorContext, WitnessContext};
use midnight_compact_runtime::ledger::{ContractAddress, DefaultDB, StateValue};
use midnight_onchain_state::state::{
    ContractMaintenanceAuthority, ContractOperation, ContractState, EntryPointBuf,
};
use midnight_serialize::tagged_serialize;
use midnight_storage::storage::HashMap;
use std::cell::RefCell;

#[derive(Default)]
struct Echo {
    observed: RefCell<Vec<(u64, bool)>>,
}

impl Witnesses<u64> for Echo {
    fn echo(&self, context: WitnessContext<'_, u64, LedgerView<'_>>, value: bool) -> (u64, bool) {
        self.observed
            .borrow_mut()
            .push((*context.private_state, value));
        (*context.private_state + 1, value)
    }
}

fn observed_matches(witnesses: &Echo, oracle: &serde_json::Value) {
    let actual = witnesses
        .observed
        .borrow()
        .iter()
        .map(|(private_state, value)| {
            serde_json::json!({
                "privateState": private_state,
                "value": value,
            })
        })
        .collect::<Vec<_>>();
    assert_eq!(serde_json::to_value(actual).unwrap(), oracle["observed"]);
}

fn state_hex(state: StateValue<DefaultDB>) -> String {
    let mut operations = HashMap::new();
    for name in ["checked_write", "read_cell"] {
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

fn public_shape(ops: &impl serde::Serialize) -> serde_json::Value {
    let ops = serde_json::to_value(ops).unwrap();
    serde_json::Value::Array(
        ops.as_array()
            .unwrap()
            .iter()
            .map(|op| {
                if let Some(kind) = op.as_str() {
                    return serde_json::json!({"kind": kind});
                }
                if let Some(push) = op.get("push") {
                    return serde_json::json!({"kind":"push", "storage":push["storage"]});
                }
                if let Some(ins) = op.get("ins") {
                    return serde_json::json!({
                        "kind":"ins", "cached":ins["cached"], "n":ins["n"]
                    });
                }
                panic!("unexpected assertion write VM operation: {op}");
            })
            .collect(),
    )
}

fn assert_private_outputs(
    outputs: &[midnight_compact_runtime::fab::AlignedValue],
    oracle: &serde_json::Value,
) {
    let actual = outputs
        .iter()
        .map(|output| {
            serde_json::json!({
                "valueAtoms": output.value.0.iter().map(|atom| &atom.0).collect::<Vec<_>>(),
                "alignment": output.alignment,
            })
        })
        .collect::<Vec<_>>();
    assert_eq!(serde_json::to_value(actual).unwrap(), *oracle);
}

#[test]
fn recorded_witness_assertion_matches_typescript_and_replays() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/assert-witness.json"
    ))
    .unwrap();
    let expected = &oracle["writePass"];
    let native_witnesses = Echo::default();
    let recorded_witnesses = Echo::default();
    let initial = initial_state(ConstructorContext::new(7_u64)).unwrap();
    assert_eq!(
        state_hex(initial.ledger_state.get_ref().clone()),
        expected["initialStateHex"],
    );
    let native = checked_write(
        initial_state(ConstructorContext::new(7_u64))
            .unwrap()
            .into_circuit_context(ContractAddress::default()),
        &native_witnesses,
        true,
        Field::from(42_u64),
    )
    .unwrap();
    let recorded = recorded::checked_write(
        initial.into_circuit_context(ContractAddress::default()),
        &recorded_witnesses,
        true,
        Field::from(42_u64),
    )
    .unwrap();
    observed_matches(&native_witnesses, expected);
    observed_matches(&recorded_witnesses, expected);
    assert_eq!(
        recorded.execution.context.private_state,
        expected["privateState"]
    );
    assert_eq!(
        native.context.private_state,
        recorded.execution.context.private_state
    );
    assert_eq!(native.gas_cost, recorded.execution.gas_cost);
    assert_eq!(
        state_hex(recorded.execution.context.query.state.get_ref().clone()),
        expected["afterCall"],
    );
    assert_eq!(
        native.context.query.state.get_ref(),
        recorded.execution.context.query.state.get_ref(),
    );
    assert_private_outputs(
        &recorded.execution.private_transcript_outputs,
        &expected["privateTranscriptOutputs"],
    );
    assert_private_outputs(
        &native.private_transcript_outputs,
        &expected["privateTranscriptOutputs"],
    );
    let gas = serde_json::to_value(recorded.execution.gas_cost).unwrap();
    for name in ["readTime", "computeTime", "bytesWritten", "bytesDeleted"] {
        assert_eq!(
            gas[name].as_u64().unwrap().to_string(),
            expected["gasCost"][name]
        );
    }
    assert_eq!(
        public_shape(&recorded.public.verify_ops()),
        expected["publicTranscriptShape"],
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
    assert_eq!(replay.gas_cost, recorded.execution.gas_cost);
    assert_eq!(
        replay.context.state.get_ref(),
        native.context.query.state.get_ref()
    );

    let failed_witnesses = Echo::default();
    let failed_initial = initial_state(ConstructorContext::new(7_u64)).unwrap();
    assert_eq!(
        state_hex(failed_initial.ledger_state.get_ref().clone()),
        oracle["writeFails"]["initialStateHex"],
    );
    let failure = recorded::checked_write(
        failed_initial.into_circuit_context(ContractAddress::default()),
        &failed_witnesses,
        false,
        Field::from(42_u64),
    )
    .err()
    .expect("assertion must stop the ledger write");
    assert_eq!(failure.to_string(), oracle["writeFails"]["error"]);
    assert_eq!(oracle["writeFails"]["queryCount"], 0);
    observed_matches(&failed_witnesses, &oracle["writeFails"]);
}

#[test]
fn witnessed_assertions_short_circuit_in_typescript_order() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/assert-witness.json"
    ))
    .unwrap();
    for (name, first, second) in [
        ("pass", true, true),
        ("firstFails", false, false),
        ("secondFails", true, false),
    ] {
        let witnesses = Echo::default();
        let context = initial_state(ConstructorContext::new(7_u64))
            .unwrap()
            .into_circuit_context(ContractAddress::default());
        let result = checked_value(context, &witnesses, first, second, Field::from(42_u64));
        let expected = &oracle[name];
        observed_matches(&witnesses, expected);
        if name == "pass" {
            let result = result.unwrap();
            assert_eq!(result.result, Field::from(42_u64));
            assert_eq!(result.context.private_state, expected["privateState"]);
            let outputs = expected["privateTranscriptOutputs"].as_array().unwrap();
            assert_eq!(result.private_transcript_outputs.len(), outputs.len());
            for (output, expected) in result.private_transcript_outputs.iter().zip(outputs) {
                let atoms = output
                    .value
                    .0
                    .iter()
                    .map(|atom| &atom.0)
                    .collect::<Vec<_>>();
                assert_eq!(serde_json::to_value(atoms).unwrap(), expected["valueAtoms"]);
                assert_eq!(
                    serde_json::to_value(&output.alignment).unwrap(),
                    expected["alignment"]
                );
            }
        } else {
            assert_eq!(result.err().unwrap().to_string(), expected["error"]);
        }
    }
}

#[test]
fn witnessed_assertion_guards_ledger_write() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/assert-witness.json"
    ))
    .unwrap();
    for (name, flag) in [("writePass", true), ("writeFails", false)] {
        let witnesses = Echo::default();
        let context = initial_state(ConstructorContext::new(7_u64))
            .unwrap()
            .into_circuit_context(ContractAddress::default());
        let result = checked_write(context, &witnesses, flag, Field::from(42_u64));
        let expected = &oracle[name];
        observed_matches(&witnesses, expected);
        if flag {
            let written = result.unwrap();
            assert_eq!(written.context.private_state, expected["privateState"]);
            let read = read_cell(written.context).unwrap();
            assert_eq!(read.result, Field::from(42_u64));
            assert_eq!(
                read.result,
                Field::from(expected["value"].as_str().unwrap().parse::<u64>().unwrap())
            );
        } else {
            assert_eq!(result.err().unwrap().to_string(), expected["error"]);
        }
    }
}
