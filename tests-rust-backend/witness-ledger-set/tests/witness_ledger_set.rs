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

use compact_rust_witness_ledger_set_fixture::ledger_contract::{
    LedgerView, Witnesses, add_true, initial_state, private_contains,
};
use midnight_compact_runtime::context::{
    CircuitResult, ConstructorContext, ConstructorResult, RunningCost, WitnessContext,
    WitnessReadMeter,
};
use midnight_compact_runtime::ledger::ContractAddress;
use midnight_compact_runtime::ledger::{
    DefaultDB, StateValue, constructor_set, contract_state, is_empty_set, member_set,
    metered_set_view_at_path,
};
use midnight_onchain_state::state::{
    ContractMaintenanceAuthority, ContractOperation, ContractState, EntryPointBuf,
};
use midnight_serialize::tagged_serialize;
use midnight_storage::storage::HashMap;

fn state_hex(state: StateValue<DefaultDB>) -> String {
    let mut operations: HashMap<EntryPointBuf, ContractOperation, DefaultDB> = HashMap::new();
    for name in ["add_true"] {
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

struct ContainsTrue;

impl Witnesses<u64> for ContainsTrue {
    fn contains_true(&self, context: WitnessContext<'_, u64, LedgerView<'_>>) -> (u64, bool) {
        let set = context.ledger.seen().unwrap();
        assert_eq!(set.is_empty().unwrap(), set.size().unwrap().value() == 0);
        (*context.private_state + 1, set.member(true).unwrap())
    }
}

fn assert_oracle_output(result: &CircuitResult<u64, bool>, oracle: &serde_json::Value) {
    assert_eq!(result.result, oracle["result"].as_bool().unwrap());
    assert_eq!(
        result.context.private_state,
        oracle["privateState"].as_u64().unwrap()
    );
    assert_eq!(result.private_transcript_outputs.len(), 1);
    let output = &result.private_transcript_outputs[0];
    let atoms = output
        .value
        .0
        .iter()
        .map(|atom| &atom.0)
        .collect::<Vec<_>>();
    assert_eq!(
        serde_json::to_value(atoms).unwrap(),
        oracle["privateTranscriptOutputs"][0]["valueAtoms"]
    );
    assert_eq!(
        serde_json::to_value(&output.alignment).unwrap(),
        oracle["privateTranscriptOutputs"][0]["alignment"]
    );
    let actual_gas = serde_json::to_value(&result.gas_cost).unwrap();
    let queries = oracle["queryCosts"].as_array().unwrap();
    assert_eq!(queries.len(), 3, "isEmpty, size, and member queries");
    for key in ["readTime", "computeTime", "bytesWritten", "bytesDeleted"] {
        let total: u64 = queries
            .iter()
            .map(|query| query[key].as_str().unwrap().parse::<u64>().unwrap())
            .sum();
        assert_eq!(actual_gas[key].as_u64().unwrap(), total, "{key} total gas");
    }
}

#[test]
fn witness_reads_current_typed_set() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/witness-ledger-set-ts-output.json"
    ))
    .unwrap();
    let context = initial_state(ConstructorContext::new(7_u64))
        .unwrap()
        .into_circuit_context(ContractAddress::default());
    assert_eq!(
        state_hex(context.query.state.get_ref().clone()),
        oracle["initialState"],
    );
    let before = private_contains(context, &ContainsTrue).unwrap();
    assert_oracle_output(&before, &oracle["before"]);
    let write = add_true(before.context).unwrap();
    assert_eq!(
        state_hex(write.context.query.state.get_ref().clone()),
        oracle["afterState"],
    );
    assert!(write.private_transcript_outputs.is_empty());
    let after = private_contains(write.context, &ContainsTrue).unwrap();
    assert_oracle_output(&after, &oracle["after"]);
}

#[test]
fn failed_set_query_does_not_charge_the_witness_meter() {
    let context = initial_state(ConstructorContext::new(7_u64))
        .unwrap()
        .into_circuit_context(ContractAddress::default());
    let meter = WitnessReadMeter::new(&context);
    assert!(meter.read_set_member(&[1], true).is_err());
    assert_eq!(meter.gas_cost(), RunningCost::ZERO);
}

#[test]
fn repeated_set_member_reads_charge_each_vm_query() {
    let context = initial_state(ConstructorContext::new(7_u64))
        .unwrap()
        .into_circuit_context(ContractAddress::default());
    let (query, present) = member_set(
        &context.query,
        &[0],
        true,
        context.gas_limit.clone(),
        &context.cost_model,
    )
    .unwrap();
    assert!(!present);
    let expected_cost = query.gas_cost;
    let meter = WitnessReadMeter::new(&context);
    assert!(!meter.read_set_member(&[0], true).unwrap());
    assert!(!meter.read_set_member(&[0], true).unwrap());
    assert_eq!(meter.gas_cost(), expected_cost.clone() + expected_cost);
}

#[test]
fn set_projection_matches_each_typescript_query_cost() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/witness-ledger-set-ts-output.json"
    ))
    .unwrap();
    let queries = oracle["before"]["queryCosts"].as_array().unwrap();
    let context = initial_state(ConstructorContext::new(7_u64))
        .unwrap()
        .into_circuit_context(ContractAddress::default());
    let meter = WitnessReadMeter::new(&context);
    let view = metered_set_view_at_path::<bool, _>(&meter, &[0]).unwrap();
    let assert_prefix = |count: usize| {
        let total = serde_json::to_value(meter.gas_cost()).unwrap();
        for key in ["readTime", "computeTime", "bytesWritten", "bytesDeleted"] {
            let expected: u64 = queries[..count]
                .iter()
                .map(|query| query[key].as_str().unwrap().parse::<u64>().unwrap())
                .sum();
            assert_eq!(
                total[key].as_u64().unwrap(),
                expected,
                "{key} query {count}"
            );
        }
    };
    assert!(view.is_empty().unwrap());
    assert_prefix(1);
    assert_eq!(view.size().unwrap().value(), 0);
    assert_prefix(2);
    assert!(!view.member(true).unwrap());
    assert_prefix(3);
}

#[test]
fn nested_set_projection_queries_its_full_physical_path() {
    let nested: StateValue<DefaultDB> = StateValue::Array(vec![constructor_set()].into());
    let state = contract_state(vec![nested]);
    let context = ConstructorResult::new(ConstructorContext::new(7_u64), state)
        .into_circuit_context(ContractAddress::default());
    let (query, empty) = is_empty_set(
        &context.query,
        &[0, 0],
        context.gas_limit.clone(),
        &context.cost_model,
    )
    .unwrap();
    assert!(empty);
    let meter = WitnessReadMeter::new(&context);
    let view = metered_set_view_at_path::<bool, _>(&meter, &[0, 0]).unwrap();
    assert!(view.is_empty().unwrap());
    assert_eq!(meter.gas_cost(), query.gas_cost);
}
