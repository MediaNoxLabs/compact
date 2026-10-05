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

use std::cell::RefCell;

use compact_rust_test_center_counter_fixture::ledger_contract::{
    LedgerView, PublicStateView, Witnesses, increment, initial_state, recorded,
};
use midnight_compact_runtime as runtime;
use midnight_onchain_state::state::{
    ContractMaintenanceAuthority, ContractOperation, ContractState, EntryPointBuf,
};
use midnight_serialize::tagged_serialize;
use midnight_storage::storage::HashMap;
use runtime::context::{ConstructorContext, WitnessContext};
use runtime::ledger::{ContractAddress, DefaultDB, StateValue};
use runtime::public_state::PublicStateSource;

#[derive(Default)]
struct AdvancingWitness(RefCell<Vec<(u64, u64)>>);

impl Witnesses<u64> for AdvancingWitness {
    fn private_increment(&self, context: WitnessContext<'_, u64, LedgerView<'_>>) -> (u64, ()) {
        let round = context.ledger.round().unwrap().value() as u64;
        self.0.borrow_mut().push((round, *context.private_state));
        (*context.private_state + 1, ())
    }
}

fn state_hex(state: StateValue<DefaultDB>) -> String {
    let operations: HashMap<EntryPointBuf, ContractOperation, DefaultDB> = HashMap::new();
    let operations = operations.insert(
        EntryPointBuf(b"increment".to_vec()),
        ContractOperation::new(None),
    );
    let contract_state =
        ContractState::new(state, operations, ContractMaintenanceAuthority::default());
    let mut bytes = Vec::new();
    tagged_serialize(&contract_state, &mut bytes).unwrap();
    hex::encode(bytes)
}

fn assert_gas(actual: &runtime::context::RunningCost, oracle: &serde_json::Value) {
    let actual = serde_json::to_value(actual).unwrap();
    let queries = oracle["queries"].as_array().unwrap();
    assert_eq!(queries.len(), 2);
    for key in ["readTime", "computeTime", "bytesWritten", "bytesDeleted"] {
        let expected: u64 = queries
            .iter()
            .map(|query| {
                query["gasCost"][key]
                    .as_str()
                    .unwrap()
                    .parse::<u64>()
                    .unwrap()
            })
            .sum();
        assert_eq!(actual[key].as_u64().unwrap(), expected, "{key}");
        assert_eq!(oracle["gasCost"][key], queries[0]["gasCost"][key]);
    }
}

fn assert_private_output(outputs: &[runtime::fab::AlignedValue], oracle: &serde_json::Value) {
    assert_eq!(outputs.len(), 1);
    let expected = &oracle["privateTranscriptOutputs"][0];
    let actual_atoms = outputs[0]
        .value
        .0
        .iter()
        .map(|atom| &atom.0)
        .collect::<Vec<_>>();
    assert_eq!(
        serde_json::to_value(actual_atoms).unwrap(),
        expected["valueAtoms"]
    );
    assert_eq!(
        serde_json::to_value(&outputs[0].alignment).unwrap(),
        expected["alignment"]
    );
}

#[test]
fn original_counter_preserves_public_then_private_effects_and_replays() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/test-center-counter.json"
    ))
    .unwrap();
    assert_eq!(
        oracle["source"],
        "test-center/test-contracts/counter.compact"
    );

    let native_witness = AdvancingWitness::default();
    let native_initial = initial_state(ConstructorContext::new(7_u64)).unwrap();
    assert_eq!(
        state_hex(native_initial.public_state().clone()),
        oracle["initialStateHex"]
    );
    let native = increment(
        native_initial.into_circuit_context(ContractAddress::default()),
        &native_witness,
    )
    .unwrap();

    let recorded_witness = AdvancingWitness::default();
    let recorded_initial = initial_state(ConstructorContext::new(7_u64)).unwrap();
    let recorded = recorded::increment(
        recorded_initial.into_circuit_context(ContractAddress::default()),
        &recorded_witness,
    )
    .unwrap();

    for seen in [&native_witness.0, &recorded_witness.0] {
        assert_eq!(*seen.borrow(), vec![(1, 7)]);
    }
    assert_eq!(
        serde_json::to_value(native_witness.0.borrow().as_slice()).unwrap(),
        serde_json::json!([[1, 7]])
    );
    assert_eq!(
        oracle["observed"],
        serde_json::json!([{"ledgerRound":"1","privateState":7}])
    );

    for (state, private, gas, outputs) in [
        (
            native.context.query.state.get_ref(),
            native.context.private_state,
            &native.gas_cost,
            &native.private_transcript_outputs,
        ),
        (
            recorded.execution.context.query.state.get_ref(),
            recorded.execution.context.private_state,
            &recorded.execution.gas_cost,
            &recorded.execution.private_transcript_outputs,
        ),
    ] {
        assert_eq!(PublicStateView::from(state).round().unwrap().value(), 1);
        assert_eq!(state_hex(state.clone()), oracle["afterCallStateHex"]);
        assert_eq!(private, oracle["privateState"].as_u64().unwrap());
        assert_gas(gas, &oracle);
        assert_private_output(outputs, &oracle);
    }
    assert_eq!(native.gas_cost, recorded.execution.gas_cost);
    assert_eq!(
        native.private_transcript_outputs,
        recorded.execution.private_transcript_outputs
    );
    assert_eq!(recorded.public.verify_ops().len(), 3);
    let ops = serde_json::to_value(recorded.public.verify_ops()).unwrap();
    let tags = ops
        .as_array()
        .unwrap()
        .iter()
        .map(|op| op.as_object().unwrap().keys().next().unwrap().as_str())
        .collect::<Vec<_>>();
    assert_eq!(serde_json::json!(tags), oracle["publicTranscriptOpTags"]);
    assert_eq!(serde_json::json!(tags), oracle["queries"][0]["opTags"]);

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
