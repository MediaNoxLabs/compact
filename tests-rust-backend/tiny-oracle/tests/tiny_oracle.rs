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

use compact_rust_tiny_oracle_fixture::ledger_contract::{
    LedgerView, Witnesses, clear, get, initial_state, recorded, set,
};
use midnight_compact_runtime as runtime;
use midnight_onchain_state::state::{
    ContractMaintenanceAuthority, ContractOperation, ContractState, EntryPointBuf,
};
use midnight_onchain_vm::cost_model::INITIAL_COST_MODEL;
use midnight_serialize::tagged_serialize;
use midnight_storage::storage::HashMap;
use runtime::context::{ConstructorContext, WitnessContext};
use runtime::ledger::{ContractAddress, DefaultDB, StateValue};

struct FixedWitness;

struct WrongWitness;

impl Witnesses<()> for FixedWitness {
    fn private_secret_key(
        &self,
        _context: WitnessContext<'_, (), LedgerView<'_>>,
    ) -> ((), runtime::FixedBytes<32>) {
        ((), runtime::FixedBytes::new([7; 32]))
    }
}

impl Witnesses<()> for WrongWitness {
    fn private_secret_key(
        &self,
        _context: WitnessContext<'_, (), LedgerView<'_>>,
    ) -> ((), runtime::FixedBytes<32>) {
        ((), runtime::FixedBytes::new([8; 32]))
    }
}

fn state_hex(state: StateValue<DefaultDB>) -> String {
    let mut operations: HashMap<EntryPointBuf, ContractOperation, DefaultDB> = HashMap::new();
    for name in ["get", "set", "clear"] {
        operations = operations.insert(
            EntryPointBuf(name.as_bytes().to_vec()),
            ContractOperation::new(None),
        );
    }
    let contract_state =
        ContractState::new(state, operations, ContractMaintenanceAuthority::default());
    let mut bytes = Vec::new();
    tagged_serialize(&contract_state, &mut bytes).unwrap();
    hex::encode(bytes)
}

fn assert_no_witness_secret(label: &str, state_hex: &str) {
    assert!(
        state_hex.len() > 64,
        "{label}: serialized state is too small for the witness leak check"
    );
    assert!(
        !state_hex.contains(&"07".repeat(16)),
        "{label}: witness secret appears in serialized public state"
    );
}

fn assert_total_gas_matches_typescript_queries(
    label: &str,
    actual: &runtime::context::RunningCost,
    private_output_count: usize,
    oracle: &serde_json::Value,
) {
    let queries = oracle["queries"].as_array().unwrap();
    let reported = &oracle["reportedGas"];
    let last = &queries.last().unwrap()["gasCost"];
    let actual = serde_json::to_value(actual).unwrap();
    for key in ["readTime", "computeTime", "bytesWritten", "bytesDeleted"] {
        let total: u64 = queries
            .iter()
            .map(|query| {
                query["gasCost"][key]
                    .as_str()
                    .unwrap()
                    .parse::<u64>()
                    .unwrap()
            })
            .sum();
        assert_eq!(actual[key].as_u64().unwrap(), total, "{label}: total {key}");
        // Ledger-8 TypeScript currently reports only its final query's cost.
        assert_eq!(reported[key], last[key], "{label}: TS reported {key}");
    }
    assert_eq!(
        private_output_count,
        oracle["privateOutputCount"].as_u64().unwrap() as usize,
        "{label}: private output count"
    );
    assert_eq!(
        queries
            .iter()
            .map(|query| query["opTags"].as_array().unwrap().len())
            .sum::<usize>(),
        oracle["publicTranscript"].as_array().unwrap().len(),
        "{label}: TypeScript captured one public operation per query operation"
    );
}

fn assert_replay<Output>(recorded: &runtime::recording::RecordedCircuitResult<(), Output>) {
    let replay = recorded
        .public
        .initial()
        .query(recorded.public.verify_ops(), None, &INITIAL_COST_MODEL)
        .unwrap();
    assert_eq!(
        state_hex(replay.context.state.get_ref().clone()),
        state_hex(recorded.execution.context.query.state.get_ref().clone())
    );
    // Replay runs the complete Verify program as one query. The generated
    // circuit sums the separate compiler queries, which have different gas.
}

fn normalized_verify_ops<Output>(
    recorded: &runtime::recording::RecordedCircuitResult<(), Output>,
) -> serde_json::Value {
    fn normalize(value: &mut serde_json::Value) {
        match value {
            serde_json::Value::Object(object) => {
                if object.contains_key("alignment") {
                    if let Some(serde_json::Value::Array(chunks)) = object.get_mut("value") {
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

#[test]
fn tiny_constructor_clear_set_and_get_match_typescript() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/tiny-oracle.json"
    ))
    .unwrap();
    let gas_oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/tiny-gas-oracle.json"
    ))
    .unwrap();
    let witness = FixedWitness;
    let initial = initial_state(
        ConstructorContext::new(()),
        &witness,
        runtime::Field::from(42u64),
    )
    .unwrap();
    let initial_hex = state_hex(initial.ledger_state.get_ref().clone());
    assert_no_witness_secret("initial", &initial_hex);
    assert_eq!(initial_hex, oracle["afterInit"]["stateHex"]);
    let context = initial.into_circuit_context(ContractAddress::default());
    let cleared = clear(context, &witness).unwrap();
    assert_total_gas_matches_typescript_queries(
        "clear",
        &cleared.gas_cost,
        cleared.private_transcript_outputs.len(),
        &gas_oracle["clear"],
    );
    let cleared_hex = state_hex(cleared.context.query.state.get_ref().clone());
    assert_no_witness_secret("clear", &cleared_hex);
    assert_eq!(cleared_hex, oracle["afterClear"]["stateHex"]);
    let set99 = set(cleared.context, &witness, runtime::Field::from(99u64)).unwrap();
    assert_total_gas_matches_typescript_queries(
        "set",
        &set99.gas_cost,
        set99.private_transcript_outputs.len(),
        &gas_oracle["set"],
    );
    let set_hex = state_hex(set99.context.query.state.get_ref().clone());
    assert_no_witness_secret("set", &set_hex);
    assert_eq!(set_hex, oracle["afterSet99"]["stateHex"]);
    let got = get(set99.context).unwrap();
    assert_total_gas_matches_typescript_queries(
        "get",
        &got.gas_cost,
        got.private_transcript_outputs.len(),
        &gas_oracle["get"],
    );
    assert_eq!(got.result.is_some, oracle["getResult"]["isSome"]);
    assert_eq!(got.result.value, runtime::Field::from(99u64));
    let got_hex = state_hex(got.context.query.state.get_ref().clone());
    assert_no_witness_secret("get", &got_hex);
    assert_eq!(got_hex, oracle["afterSet99"]["stateHex"]);
}

#[test]
fn tiny_recorded_clear_and_set_match_native_typescript_and_replay() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/tiny-oracle.json"
    ))
    .unwrap();
    let gas_oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/tiny-gas-oracle.json"
    ))
    .unwrap();
    let witness = FixedWitness;
    let native = initial_state(
        ConstructorContext::new(()),
        &witness,
        runtime::Field::from(42u64),
    )
    .unwrap();
    let recorded = initial_state(
        ConstructorContext::new(()),
        &witness,
        runtime::Field::from(42u64),
    )
    .unwrap();
    let native = clear(
        native.into_circuit_context(ContractAddress::default()),
        &witness,
    )
    .unwrap();
    let recorded = recorded::clear(
        recorded.into_circuit_context(ContractAddress::default()),
        &witness,
    )
    .unwrap();
    assert_replay(&recorded);
    assert_eq!(recorded.public.verify_ops().len(), 15);
    assert_eq!(
        normalized_verify_ops(&recorded),
        gas_oracle["clear"]["publicTranscript"]
    );
    assert_eq!(native.gas_cost, recorded.execution.gas_cost);
    assert_eq!(
        native.private_transcript_outputs,
        recorded.execution.private_transcript_outputs
    );
    assert_total_gas_matches_typescript_queries(
        "recorded clear",
        &recorded.execution.gas_cost,
        recorded.execution.private_transcript_outputs.len(),
        &gas_oracle["clear"],
    );
    assert_eq!(
        state_hex(native.context.query.state.get_ref().clone()),
        oracle["afterClear"]["stateHex"]
    );
    assert_eq!(
        state_hex(recorded.execution.context.query.state.get_ref().clone()),
        oracle["afterClear"]["stateHex"]
    );

    let native = get(native.context).unwrap();
    let recorded = recorded::get(recorded.execution.context).unwrap();
    assert_replay(&recorded);
    assert_eq!(recorded.public.verify_ops().len(), 3);
    assert!(!recorded.execution.result.is_some);
    assert_eq!(native.result, recorded.execution.result);
    assert_eq!(native.gas_cost, recorded.execution.gas_cost);

    let native = set(native.context, &witness, runtime::Field::from(99u64)).unwrap();
    let recorded = recorded::set(
        recorded.execution.context,
        &witness,
        runtime::Field::from(99u64),
    )
    .unwrap();
    assert_replay(&recorded);
    assert_eq!(recorded.public.verify_ops().len(), 12);
    assert_eq!(
        normalized_verify_ops(&recorded),
        gas_oracle["set"]["publicTranscript"]
    );
    assert_eq!(native.gas_cost, recorded.execution.gas_cost);
    assert_eq!(
        native.private_transcript_outputs,
        recorded.execution.private_transcript_outputs
    );
    assert_total_gas_matches_typescript_queries(
        "recorded set",
        &recorded.execution.gas_cost,
        recorded.execution.private_transcript_outputs.len(),
        &gas_oracle["set"],
    );
    assert_eq!(
        state_hex(native.context.query.state.get_ref().clone()),
        oracle["afterSet99"]["stateHex"]
    );
    assert_eq!(
        state_hex(recorded.execution.context.query.state.get_ref().clone()),
        oracle["afterSet99"]["stateHex"]
    );

    let native = get(native.context).unwrap();
    let recorded = recorded::get(recorded.execution.context).unwrap();
    assert_replay(&recorded);
    assert_eq!(native.result, recorded.execution.result);
    assert_eq!(
        recorded.execution.result.is_some,
        oracle["getResult"]["isSome"]
    );
    assert_eq!(recorded.execution.result.value, runtime::Field::from(99u64));
    assert_eq!(native.gas_cost, recorded.execution.gas_cost);
    assert_eq!(recorded.public.verify_ops().len(), 6);
    assert_eq!(
        normalized_verify_ops(&recorded),
        gas_oracle["get"]["publicTranscript"]
    );
    assert_total_gas_matches_typescript_queries(
        "recorded get",
        &recorded.execution.gas_cost,
        recorded.execution.private_transcript_outputs.len(),
        &gas_oracle["get"],
    );
}

#[test]
fn tiny_recorded_assertions_match_native_failures() {
    let witness = FixedWitness;
    let initial = || {
        initial_state(
            ConstructorContext::new(()),
            &witness,
            runtime::Field::from(42u64),
        )
        .unwrap()
        .into_circuit_context(ContractAddress::default())
    };
    let native = set(initial(), &witness, runtime::Field::from(99u64))
        .err()
        .unwrap();
    let recorded = recorded::set(initial(), &witness, runtime::Field::from(99u64))
        .err()
        .unwrap();
    assert_eq!(native, recorded);
    assert_eq!(
        recorded,
        runtime::CompactError::AssertionFailed(
            "set: attempted to overwrite recorded value".to_owned()
        )
    );

    let native = clear(initial(), &WrongWitness).err().unwrap();
    let recorded = recorded::clear(initial(), &WrongWitness).err().unwrap();
    assert_eq!(native, recorded);
    assert_eq!(
        recorded,
        runtime::CompactError::AssertionFailed(
            "clear: attempted clear without proper authorization".to_owned()
        )
    );

    let native = clear(clear(initial(), &witness).unwrap().context, &witness)
        .err()
        .unwrap();
    let recorded = recorded::clear(clear(initial(), &witness).unwrap().context, &witness)
        .err()
        .unwrap();
    assert_eq!(native, recorded);
    assert_eq!(
        recorded,
        runtime::CompactError::AssertionFailed("clear: no value is currently recorded".to_owned())
    );
}
