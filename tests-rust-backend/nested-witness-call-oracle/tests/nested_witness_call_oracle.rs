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

use compact_rust_nested_witness_call_oracle_fixture::ledger_contract::{
    Contract, LedgerView, Witnesses, initial_state, outer, outerValue,
};
use midnight_compact_runtime::Field;
use midnight_compact_runtime::context::{ConstructorContext, WitnessContext};
use midnight_compact_runtime::ledger::{ContractAddress, DefaultDB, StateValue};
use midnight_onchain_state::state::{
    ContractMaintenanceAuthority, ContractOperation, ContractState, EntryPointBuf,
};
use midnight_serialize::tagged_serialize;
use midnight_storage::storage::HashMap;

struct OracleWitness;

impl Witnesses<u64> for OracleWitness {
    fn secret(&self, context: WitnessContext<'_, u64, LedgerView<'_>>) -> (u64, Field) {
        (*context.private_state + 1, Field::from(7_u64))
    }
}

fn state_hex(state: StateValue<DefaultDB>) -> String {
    let mut operations: HashMap<EntryPointBuf, ContractOperation, DefaultDB> = HashMap::new();
    for name in ["outer", "outerValue"] {
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
fn nested_witness_call_preserves_state_and_transcript_order() {
    let reference: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/nested-witness-call-oracle.json"
    ))
    .unwrap();
    let initial = initial_state(ConstructorContext::new(7_u64)).unwrap();
    assert_eq!(
        state_hex(initial.ledger_state.get_ref().clone()),
        reference["initialHex"]
    );
    let context = initial.into_circuit_context(ContractAddress::default());
    let called = outer(context, &OracleWitness).unwrap();
    assert_eq!(
        called.context.private_state,
        reference["privateState"].as_u64().unwrap()
    );
    assert_eq!(
        state_hex(called.context.query.state.get_ref().clone()),
        reference["afterOuterHex"]
    );
    assert_transcript(
        &called.private_transcript_outputs,
        &reference["privateTranscriptOutputs"],
    );
    let value_call = outerValue(called.context, &OracleWitness).unwrap();
    assert_eq!(
        value_call.context.private_state,
        reference["afterOuterValuePrivateState"].as_u64().unwrap()
    );
    assert_eq!(
        state_hex(value_call.context.query.state.get_ref().clone()),
        reference["afterOuterValueHex"]
    );
    assert_transcript(
        &value_call.private_transcript_outputs,
        &reference["outerValueTranscript"],
    );
}

#[test]
fn recorded_nested_expressions_match_typescript_and_replay() {
    let reference: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/nested-witness-call-oracle.json"
    ))
    .unwrap();
    let context = initial_state(ConstructorContext::new(7_u64))
        .unwrap()
        .into_circuit_context(ContractAddress::default());
    let contract = Contract::from(OracleWitness);
    let outer = contract.recording().outer(context).unwrap();
    assert_eq!(
        outer.execution.context.private_state,
        reference["privateState"].as_u64().unwrap()
    );
    assert_eq!(
        state_hex(outer.execution.context.query.state.get_ref().clone()),
        reference["afterOuterHex"]
    );
    assert_transcript(
        &outer.execution.private_transcript_outputs,
        &reference["privateTranscriptOutputs"],
    );
    let replay = outer
        .public
        .initial()
        .query(
            outer.public.verify_ops(),
            None,
            &outer.execution.context.cost_model,
        )
        .unwrap();
    assert_eq!(
        replay.context.effects,
        outer.execution.context.query.effects
    );

    let outer_value = contract
        .recording()
        .outerValue(outer.execution.context)
        .unwrap();
    assert_eq!(
        outer_value.execution.context.private_state,
        reference["afterOuterValuePrivateState"].as_u64().unwrap()
    );
    assert_eq!(
        state_hex(outer_value.execution.context.query.state.get_ref().clone()),
        reference["afterOuterValueHex"]
    );
    assert_transcript(
        &outer_value.execution.private_transcript_outputs,
        &reference["outerValueTranscript"],
    );
    let replay = outer_value
        .public
        .initial()
        .query(
            outer_value.public.verify_ops(),
            None,
            &outer_value.execution.context.cost_model,
        )
        .unwrap();
    assert_eq!(
        replay.context.effects,
        outer_value.execution.context.query.effects
    );
}

fn assert_transcript(
    actual: &[midnight_compact_runtime::fab::AlignedValue],
    expected: &serde_json::Value,
) {
    let outputs = expected.as_array().unwrap();
    assert_eq!(actual.len(), outputs.len());
    for (actual, expected) in actual.iter().zip(outputs) {
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
