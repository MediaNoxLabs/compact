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

use compact_rust_wide_uint_oracle_fixture::ledger_contract::{
    LedgerView, Witnesses, initial_state, readWide, recorded, writeWide,
};
use compact_rust_wide_uint_oracle_fixture::pure_circuits::maxWide;
use midnight_compact_runtime::WideUint;
use midnight_compact_runtime::context::{ConstructorContext, RunningCost, WitnessContext};
use midnight_compact_runtime::ledger::{ContractAddress, DefaultDB, StateValue};
use midnight_compact_runtime::recording::RecordedCircuitResult;
use midnight_onchain_state::state::{
    ContractMaintenanceAuthority, ContractOperation, ContractState, EntryPointBuf,
};
use midnight_serialize::tagged_serialize;
use midnight_storage::storage::HashMap;

type Uint248 = WideUint<{ (1_u128 << 120) - 1 }, { u128::MAX }>;

struct OracleWitness;

impl Witnesses<u64> for OracleWitness {
    fn nextWide(&self, context: WitnessContext<'_, u64, LedgerView<'_>>) -> (u64, Uint248) {
        (
            *context.private_state + 1,
            Uint248::from_le_bytes(&[0xff; 31]).unwrap(),
        )
    }
}

fn state_hex(state: StateValue<DefaultDB>) -> String {
    let mut operations: HashMap<EntryPointBuf, ContractOperation, DefaultDB> = HashMap::new();
    for name in ["readWide", "writeWide"] {
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

fn assert_gas(actual: &RunningCost, expected: &serde_json::Value) {
    assert_eq!(
        actual.read_time.into_picoseconds().to_string(),
        expected["readTime"]
    );
    assert_eq!(
        actual.compute_time.into_picoseconds().to_string(),
        expected["computeTime"]
    );
    assert_eq!(actual.bytes_written.to_string(), expected["bytesWritten"]);
    assert_eq!(actual.bytes_deleted.to_string(), expected["bytesDeleted"]);
}

fn normalized_verify_ops<Output>(
    recorded: &RecordedCircuitResult<u64, Output>,
) -> serde_json::Value {
    fn normalize(value: &mut serde_json::Value) {
        match value {
            serde_json::Value::Object(object) => {
                if object.contains_key("alignment")
                    && let Some(serde_json::Value::Array(chunks)) = object.get_mut("value")
                {
                    for chunk in chunks {
                        if let serde_json::Value::Array(bytes) = chunk {
                            let bytes = bytes
                                .iter()
                                .map(|byte| byte.as_u64().unwrap() as u8)
                                .collect::<Vec<_>>();
                            *chunk = serde_json::json!({ "bytesHex": hex::encode(bytes) });
                        }
                    }
                }
                for child in object.values_mut() {
                    normalize(child);
                }
            }
            serde_json::Value::Array(values) => {
                for child in values {
                    normalize(child);
                }
            }
            _ => {}
        }
    }
    let mut ops = serde_json::to_value(recorded.public.verify_ops()).unwrap();
    normalize(&mut ops);
    ops
}

fn assert_recorded_capture<Output>(
    recorded: &RecordedCircuitResult<u64, Output>,
    expected: &serde_json::Value,
) {
    let queries = expected["queries"].as_array().unwrap();
    assert_eq!(queries.len(), 1);
    assert_eq!(expected["reportedGas"], queries[0]["gasCost"]);
    assert_gas(&recorded.execution.gas_cost, &expected["reportedGas"]);
    let ops = normalized_verify_ops(recorded);
    assert_eq!(ops, expected["publicTranscript"]);
    let tags = ops
        .as_array()
        .unwrap()
        .iter()
        .map(|op| op.as_object().unwrap().keys().next().unwrap().as_str())
        .collect::<Vec<_>>();
    assert_eq!(serde_json::json!(tags), queries[0]["opTags"]);
    let outputs = expected["privateTranscriptOutputs"].as_array().unwrap();
    assert_eq!(
        outputs.len(),
        expected["privateOutputCount"].as_u64().unwrap() as usize
    );
    assert_eq!(
        recorded.execution.private_transcript_outputs.len(),
        outputs.len()
    );
    for (actual, expected) in recorded
        .execution
        .private_transcript_outputs
        .iter()
        .zip(outputs)
    {
        let atoms = actual
            .value
            .0
            .iter()
            .map(|atom| &atom.0)
            .collect::<Vec<_>>();
        assert_eq!(serde_json::to_value(atoms).unwrap(), expected["valueAtoms"]);
        assert_eq!(
            serde_json::to_value(&actual.alignment).unwrap(),
            expected["alignment"]
        );
    }
}

#[test]
fn wide_uint_cell_and_witness_transcript_match_typescript() {
    let reference: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/wide-uint-oracle.json"
    ))
    .unwrap();
    let maximum = Uint248::from_le_bytes(&[0xff; 31]).unwrap();
    assert_eq!(maxWide().unwrap(), maximum);
    assert_eq!(maximum.as_field().as_le_bytes().len(), 32);
    assert_eq!(maximum.as_le_bytes().len(), 31);
    assert_eq!(
        reference["maxWide"],
        "452312848583266388373324160190187140051835877600158453279131187530910662655"
    );

    let initial = initial_state(ConstructorContext::new(7_u64)).unwrap();
    assert_eq!(
        state_hex(initial.ledger_state.get_ref().clone()),
        reference["initialHex"]
    );
    let context = initial.into_circuit_context(ContractAddress::default());
    let write = writeWide(context, &OracleWitness).unwrap();
    assert_eq!(
        write.context.private_state,
        reference["privateState"].as_u64().unwrap()
    );
    assert_eq!(
        state_hex(write.context.query.state.get_ref().clone()),
        reference["afterWriteHex"]
    );
    let outputs = reference["privateTranscriptOutputs"].as_array().unwrap();
    assert_eq!(write.private_transcript_outputs.len(), outputs.len());
    let atoms = write.private_transcript_outputs[0]
        .value
        .0
        .iter()
        .map(|atom| &atom.0)
        .collect::<Vec<_>>();
    assert_eq!(
        serde_json::to_value(atoms).unwrap(),
        outputs[0]["valueAtoms"]
    );
    assert_eq!(
        serde_json::to_value(&write.private_transcript_outputs[0].alignment).unwrap(),
        outputs[0]["alignment"]
    );
    assert_eq!(readWide(write.context).unwrap().result, maximum);
    assert_eq!(reference["read"], reference["maxWide"]);
}

#[test]
fn witnessed_wide_uint_write_and_read_record_typescript_parity_and_replay() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/wide-uint-oracle.json"
    ))
    .unwrap();
    let initial = || {
        initial_state(ConstructorContext::new(7_u64))
            .unwrap()
            .into_circuit_context(ContractAddress::default())
    };
    let native_write = writeWide(initial(), &OracleWitness).unwrap();
    let recorded_write = recorded::writeWide(initial(), &OracleWitness).unwrap();
    let write_replay = recorded_write
        .public
        .initial()
        .query(
            recorded_write.public.verify_ops(),
            None,
            &recorded_write.execution.context.cost_model,
        )
        .unwrap();
    assert_recorded_capture(&recorded_write, &oracle["writeWide"]);
    assert_eq!(native_write.gas_cost, recorded_write.execution.gas_cost);
    assert_eq!(recorded_write.execution.gas_cost, write_replay.gas_cost);
    assert_eq!(
        native_write.context.private_state,
        recorded_write.execution.context.private_state
    );
    assert_eq!(
        recorded_write.execution.context.private_state,
        oracle["privateState"]
    );
    assert_eq!(
        native_write.private_transcript_outputs,
        recorded_write.execution.private_transcript_outputs
    );
    assert_eq!(
        native_write.context.query.effects,
        recorded_write.execution.context.query.effects
    );
    assert_eq!(
        native_write.context.query.effects,
        write_replay.context.effects
    );
    for state in [
        native_write.context.query.state.get_ref(),
        recorded_write.execution.context.query.state.get_ref(),
        write_replay.context.state.get_ref(),
    ] {
        assert_eq!(state_hex(state.clone()), oracle["afterWriteHex"]);
    }

    let native_read = readWide(native_write.context).unwrap();
    let recorded_read = recorded::readWide(recorded_write.execution.context).unwrap();
    let read_replay = recorded_read
        .public
        .initial()
        .query(
            recorded_read.public.verify_ops(),
            None,
            &recorded_read.execution.context.cost_model,
        )
        .unwrap();
    assert_recorded_capture(&recorded_read, &oracle["readWide"]);
    assert_eq!(native_read.gas_cost, recorded_read.execution.gas_cost);
    assert_eq!(recorded_read.execution.gas_cost, read_replay.gas_cost);
    assert_eq!(native_read.result, recorded_read.execution.result);
    assert_eq!(
        native_read.result,
        Uint248::from_le_bytes(&[0xff; 31]).unwrap()
    );
    assert_eq!(oracle["read"], oracle["maxWide"]);
    assert_eq!(
        oracle["readWide"]["publicTranscript"][2]["popeq"]["result"]["alignment"][0]["value"]["length"],
        31
    );
    assert_eq!(
        native_read.context.query.effects,
        recorded_read.execution.context.query.effects
    );
    assert_eq!(
        native_read.context.query.effects,
        read_replay.context.effects
    );
    for state in [
        native_read.context.query.state.get_ref(),
        recorded_read.execution.context.query.state.get_ref(),
        read_replay.context.state.get_ref(),
    ] {
        assert_eq!(state_hex(state.clone()), oracle["afterWriteHex"]);
    }
    assert!(
        recorded_read
            .execution
            .private_transcript_outputs
            .is_empty()
    );
    assert!(Uint248::from_le_bytes(&[0xff; 32]).is_err());
}
