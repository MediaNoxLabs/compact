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

use compact_rust_counter_less_than_oracle_fixture::ledger_contract as contract;
use midnight_compact_runtime as runtime;
use midnight_onchain_state::state::{
    ContractMaintenanceAuthority, ContractOperation, ContractState, EntryPointBuf,
};
use midnight_serialize::tagged_serialize;
use midnight_storage::storage::HashMap;
use runtime::context::{CircuitContext, CircuitResult, ConstructorContext};
use runtime::ledger::{ContractAddress, DefaultDB, StateValue};
use runtime::recording::RecordedCircuitResult;

fn oracle() -> serde_json::Value {
    serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/counter-less-than-oracle.json"
    ))
    .unwrap()
}
fn context() -> CircuitContext<()> {
    contract::initial_state(ConstructorContext::new(()))
        .unwrap()
        .into_circuit_context(ContractAddress::default())
}
fn state_hex(state: StateValue<DefaultDB>) -> String {
    let mut operations = HashMap::new();
    for name in ["compare", "nested", "short_circuit", "checked"] {
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
fn shape(ops: serde_json::Value) -> serde_json::Value {
    serde_json::Value::Array(ops.as_array().unwrap().iter().map(|op| {
  if let Some(kind)=op.as_str(){return serde_json::json!({"kind":kind});}
  if let Some(x)=op.get("idx"){serde_json::json!({"kind":"idx","cached":x["cached"],"pushPath":x["pushPath"],"pathLength":x["path"].as_array().unwrap().len()})}
  else if let Some(x)=op.get("dup"){serde_json::json!({"kind":"dup","n":x["n"]})}
  else if let Some(x)=op.get("push"){serde_json::json!({"kind":"push","storage":x["storage"],"value":x["value"]})}
  else if let Some(x)=op.get("popeq"){serde_json::json!({"kind":"popeq","cached":x["cached"],"resultAtoms":x["result"]["value"]})}
  else{panic!("unexpected op {op}")}
 }).collect())
}
fn check<T: serde::Serialize + PartialEq + std::fmt::Debug>(
    row: &serde_json::Value,
    native: CircuitResult<(), T>,
    recorded: RecordedCircuitResult<(), T>,
) {
    assert_eq!(native.result, recorded.execution.result);
    assert_eq!(serde_json::to_value(&native.result).unwrap(), row["result"]);
    assert_eq!(native.gas_cost, recorded.execution.gas_cost);
    assert_eq!(
        native.context.query.state.get_ref(),
        recorded.execution.context.query.state.get_ref()
    );
    assert_eq!(
        native.context.query.effects,
        recorded.execution.context.query.effects
    );
    assert_eq!(
        state_hex(native.context.query.state.get_ref().clone()),
        row["afterStateHex"]
    );
    assert_eq!(
        shape(serde_json::to_value(recorded.public.verify_ops()).unwrap()),
        row["shape"]
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
        native.context.query.state.get_ref()
    );
    assert_eq!(replay.context.effects, native.context.query.effects);
    assert!(native.private_transcript_outputs.is_empty());
    assert!(recorded.execution.private_transcript_outputs.is_empty());
    assert_eq!(row["privateCount"], 0);
    let gas = serde_json::to_value(native.gas_cost).unwrap();
    for dimension in ["readTime", "computeTime", "bytesWritten", "bytesDeleted"] {
        let total: u64 = row["queries"]
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
        assert_eq!(gas[dimension].as_u64().unwrap(), total, "{dimension}");
    }
}
#[test]
fn counter_comparison_preserves_ledger_queries_and_short_circuiting() {
    let expected = oracle();
    assert_eq!(
        state_hex(context().query.state.get_ref().clone()),
        expected["initialStateHex"]
    );
    for (index, threshold) in [0_u64, 3, 4, u64::MAX].into_iter().enumerate() {
        let threshold = runtime::BoundedUint::new(threshold as u128).unwrap();
        check(
            &expected["rows"][index],
            contract::compare(context(), threshold).unwrap(),
            contract::recorded::compare(context(), threshold).unwrap(),
        );
    }
    check(
        &expected["rows"][4],
        contract::nested(context()).unwrap(),
        contract::recorded::nested(context()).unwrap(),
    );
    for (index, run) in [false, true].into_iter().enumerate() {
        check(
            &expected["rows"][index + 5],
            contract::short_circuit(context(), run).unwrap(),
            contract::recorded::short_circuit(context(), run).unwrap(),
        );
    }
    let four = runtime::BoundedUint::new(4).unwrap();
    check(
        &expected["rows"][7],
        contract::checked(context(), four).unwrap(),
        contract::recorded::checked(context(), four).unwrap(),
    );
    let three = runtime::BoundedUint::new(3).unwrap();
    assert!(
        contract::checked(context(), three)
            .err()
            .unwrap()
            .to_string()
            .contains("counter guard")
    );
    assert!(
        contract::recorded::checked(context(), three)
            .err()
            .unwrap()
            .to_string()
            .contains("counter guard")
    );
    assert!(
        expected["rows"][8]["error"]
            .as_str()
            .unwrap()
            .contains("counter guard")
    );
    assert_eq!(expected["rows"][5]["queries"].as_array().unwrap().len(), 0);
    assert_eq!(expected["rows"][6]["queries"].as_array().unwrap().len(), 2);
}
