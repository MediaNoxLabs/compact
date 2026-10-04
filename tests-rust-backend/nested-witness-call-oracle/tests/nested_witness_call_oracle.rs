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
    Contract, LedgerView, TryWitnesses, Witnesses, initial_state, outer, outerValue, outerValue2,
    outerValueExpr,
};
use midnight_compact_runtime::context::{ConstructorContext, WitnessContext};
use midnight_compact_runtime::ledger::{ContractAddress, DefaultDB, StateValue};
use midnight_compact_runtime::{CompactError, Field};
use midnight_onchain_state::state::{
    ContractMaintenanceAuthority, ContractOperation, ContractState, EntryPointBuf,
};
use midnight_serialize::tagged_serialize;
use midnight_storage::storage::HashMap;
use std::cell::Cell;
use std::rc::Rc;

struct OracleWitness;

struct RejectingWitness;

struct RejectThirdOrderedWitness;

struct RejectExpressionLeftWitness {
    ordered_calls: Rc<Cell<usize>>,
}

impl TryWitnesses<u64> for RejectExpressionLeftWitness {
    fn secret(
        &self,
        _context: WitnessContext<'_, u64, LedgerView<'_>>,
    ) -> Result<(u64, Field), CompactError> {
        Err(CompactError::AssertionFailed(
            "left witness rejected".into(),
        ))
    }

    fn ordered(
        &self,
        context: WitnessContext<'_, u64, LedgerView<'_>>,
    ) -> Result<(u64, Field), CompactError> {
        self.ordered_calls.set(self.ordered_calls.get() + 1);
        Ok((*context.private_state + 1, Field::from(99_u64)))
    }
}

impl TryWitnesses<u64> for RejectingWitness {
    fn secret(
        &self,
        _context: WitnessContext<'_, u64, LedgerView<'_>>,
    ) -> Result<(u64, Field), CompactError> {
        Err(CompactError::AssertionFailed("rejected witness".into()))
    }

    fn ordered(
        &self,
        _context: WitnessContext<'_, u64, LedgerView<'_>>,
    ) -> Result<(u64, Field), CompactError> {
        Err(CompactError::AssertionFailed(
            "rejected ordered witness".into(),
        ))
    }
}

impl TryWitnesses<u64> for RejectThirdOrderedWitness {
    fn secret(
        &self,
        _context: WitnessContext<'_, u64, LedgerView<'_>>,
    ) -> Result<(u64, Field), CompactError> {
        Err(CompactError::AssertionFailed(
            "unexpected secret witness".into(),
        ))
    }

    fn ordered(
        &self,
        context: WitnessContext<'_, u64, LedgerView<'_>>,
    ) -> Result<(u64, Field), CompactError> {
        let private = *context.private_state;
        if private == 9 {
            return Err(CompactError::AssertionFailed(
                "third witness rejected".into(),
            ));
        }
        Ok((private + 1, Field::from(private)))
    }
}

impl Witnesses<u64> for OracleWitness {
    fn secret(&self, context: WitnessContext<'_, u64, LedgerView<'_>>) -> (u64, Field) {
        (*context.private_state + 1, Field::from(7_u64))
    }

    fn ordered(&self, context: WitnessContext<'_, u64, LedgerView<'_>>) -> (u64, Field) {
        let private = *context.private_state;
        (private + 1, Field::from(private))
    }
}

fn state_hex(state: StateValue<DefaultDB>) -> String {
    let mut operations: HashMap<EntryPointBuf, ContractOperation, DefaultDB> = HashMap::new();
    for name in ["outer", "outerValue", "outerValue2", "outerValueExpr"] {
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
    let native_context = initial_state(ConstructorContext::new(7_u64))
        .unwrap()
        .into_circuit_context(ContractAddress::default());
    let native_outer = outer(native_context, &OracleWitness).unwrap();
    let native_outer_value = outerValue(native_outer.context, &OracleWitness).unwrap();
    let contract = Contract::from(OracleWitness);
    let outer = contract.recording().outer(context).unwrap();
    assert_eq!(outer.execution.gas_cost, native_outer.gas_cost);
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
    assert_eq!(outer_value.execution.gas_cost, native_outer_value.gas_cost);
    assert_eq!(
        outer_value.execution.context.query.effects,
        native_outer_value.context.query.effects
    );
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

#[test]
fn recorded_value_helper_propagates_witness_rejection() {
    let context = initial_state(ConstructorContext::new(7_u64))
        .unwrap()
        .into_circuit_context(ContractAddress::default());
    let contract = Contract::from(RejectingWitness);
    assert!(matches!(
        contract.recording().outerValue(context),
        Err(CompactError::AssertionFailed(message)) if message == "rejected witness"
    ));
}

#[test]
fn parameterized_value_helper_preserves_argument_order_and_verify_program() {
    let reference: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/nested-witness-call-oracle.json"
    ))
    .unwrap();
    let native_context = initial_state(ConstructorContext::new(7_u64))
        .unwrap()
        .into_circuit_context(ContractAddress::default());
    let native_outer = outer(native_context, &OracleWitness).unwrap();
    let native_outer_value = outerValue(native_outer.context, &OracleWitness).unwrap();
    let native = outerValue2(native_outer_value.context, &OracleWitness).unwrap();

    let recorded_context = initial_state(ConstructorContext::new(7_u64))
        .unwrap()
        .into_circuit_context(ContractAddress::default());
    let contract = Contract::from(OracleWitness);
    let outer = contract.recording().outer(recorded_context).unwrap();
    let outer_value = contract
        .recording()
        .outerValue(outer.execution.context)
        .unwrap();
    let recorded = contract
        .recording()
        .outerValue2(outer_value.execution.context)
        .unwrap();

    assert_eq!(recorded.execution.gas_cost, native.gas_cost);
    assert_eq!(
        recorded.execution.context.query.effects,
        native.context.query.effects
    );
    assert_eq!(
        recorded.execution.context.private_state,
        reference["afterOuterValue2PrivateState"].as_u64().unwrap()
    );
    assert_eq!(
        state_hex(recorded.execution.context.query.state.get_ref().clone()),
        reference["afterOuterValue2Hex"]
    );
    assert_transcript(
        &recorded.execution.private_transcript_outputs,
        &reference["outerValue2Transcript"],
    );
    let queries = reference["outerValue2Queries"].as_array().unwrap();
    assert_eq!(
        queries.len(),
        1,
        "only the final Cell write queries the ledger"
    );
    let actual_gas = serde_json::to_value(recorded.execution.gas_cost).unwrap();
    for key in ["readTime", "computeTime", "bytesWritten", "bytesDeleted"] {
        let expected = queries[0]["gasCost"][key]
            .as_str()
            .unwrap()
            .parse::<u64>()
            .unwrap();
        assert_eq!(actual_gas[key].as_u64().unwrap(), expected, "{key}");
        assert_eq!(reference["outerValue2Gas"][key], queries[0]["gasCost"][key]);
    }
    assert_eq!(
        recorded.public.verify_ops().len(),
        queries[0]["opTags"].as_array().unwrap().len()
    );
    assert_eq!(
        normalized_verify_ops(&recorded),
        reference["outerValue2PublicTranscript"]
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
        replay.context.effects,
        recorded.execution.context.query.effects
    );
}

#[test]
fn expression_nested_value_helper_matches_typescript_and_replays() {
    let reference: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/nested-witness-call-oracle.json"
    ))
    .unwrap();

    let native_context = initial_state(ConstructorContext::new(7_u64))
        .unwrap()
        .into_circuit_context(ContractAddress::default());
    let native_outer = outer(native_context, &OracleWitness).unwrap();
    let native_value = outerValue(native_outer.context, &OracleWitness).unwrap();
    let native_value2 = outerValue2(native_value.context, &OracleWitness).unwrap();
    let native = outerValueExpr(native_value2.context, &OracleWitness).unwrap();

    let recorded_context = initial_state(ConstructorContext::new(7_u64))
        .unwrap()
        .into_circuit_context(ContractAddress::default());
    let contract = Contract::from(OracleWitness);
    let recorded_outer = contract.recording().outer(recorded_context).unwrap();
    let recorded_value = contract
        .recording()
        .outerValue(recorded_outer.execution.context)
        .unwrap();
    let recorded_value2 = contract
        .recording()
        .outerValue2(recorded_value.execution.context)
        .unwrap();
    let recorded = contract
        .recording()
        .outerValueExpr(recorded_value2.execution.context)
        .unwrap();

    assert_eq!(recorded.execution.gas_cost, native.gas_cost);
    assert_eq!(
        recorded.execution.context.query.effects,
        native.context.query.effects
    );
    assert_eq!(
        recorded.execution.context.private_state,
        reference["afterOuterValueExprPrivateState"]
            .as_u64()
            .unwrap()
    );
    assert_eq!(
        state_hex(recorded.execution.context.query.state.get_ref().clone()),
        reference["afterOuterValueExprHex"]
    );
    assert_eq!(
        midnight_compact_runtime::ledger::read_root_cell::<Field, _>(
            recorded.execution.context.query.state.get_ref(),
            0,
        )
        .unwrap(),
        Field::from(20_u64)
    );
    assert_transcript(
        &recorded.execution.private_transcript_outputs,
        &reference["outerValueExprTranscript"],
    );
    let queries = reference["outerValueExprQueries"].as_array().unwrap();
    assert_eq!(queries.len(), 1, "only the Cell write queries the ledger");
    let actual_gas = serde_json::to_value(recorded.execution.gas_cost).unwrap();
    for key in ["readTime", "computeTime", "bytesWritten", "bytesDeleted"] {
        let expected = queries[0]["gasCost"][key]
            .as_str()
            .unwrap()
            .parse::<u64>()
            .unwrap();
        assert_eq!(actual_gas[key].as_u64().unwrap(), expected, "{key}");
        assert_eq!(
            reference["outerValueExprGas"][key],
            queries[0]["gasCost"][key]
        );
    }
    assert_eq!(
        normalized_verify_ops(&recorded),
        reference["outerValueExprPublicTranscript"]
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
        replay.context.effects,
        recorded.execution.context.query.effects
    );
}

#[test]
fn expression_nested_value_helper_stops_before_right_witness_on_left_error() {
    let context = initial_state(ConstructorContext::new(7_u64))
        .unwrap()
        .into_circuit_context(ContractAddress::default());
    let ordered_calls = Rc::new(Cell::new(0));
    let witness = RejectExpressionLeftWitness {
        ordered_calls: Rc::clone(&ordered_calls),
    };
    let contract = Contract::from(witness);
    assert!(matches!(
        contract.recording().outerValueExpr(context),
        Err(CompactError::AssertionFailed(message)) if message == "left witness rejected"
    ));
    assert_eq!(ordered_calls.get(), 0);
}

#[test]
fn parameterized_value_helper_short_circuits_on_callee_witness_error() {
    let context = initial_state(ConstructorContext::new(7_u64))
        .unwrap()
        .into_circuit_context(ContractAddress::default());
    let contract = Contract::from(RejectThirdOrderedWitness);
    assert!(matches!(
        contract.recording().outerValue2(context),
        Err(CompactError::AssertionFailed(message)) if message == "third witness rejected"
    ));
}

fn normalized_verify_ops(
    recorded: &midnight_compact_runtime::recording::RecordedCircuitResult<u64, ()>,
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
