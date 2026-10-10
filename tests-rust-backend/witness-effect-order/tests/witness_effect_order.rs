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

//! Native effect ordering and current recorded Set membership qualification.
//! The independent captured query programs belong to TypeScript.

use compact_rust_witness_effect_order_fixture::ledger_contract as contract;
use midnight_compact_runtime as runtime;
use midnight_onchain_state::state::{
    ContractMaintenanceAuthority, ContractOperation, ContractState, EntryPointBuf,
};
use midnight_storage::storage::HashMap;
use runtime::context::{CircuitResult, ConstructorContext, WitnessContext};
use runtime::fab::AlignedValue;
use runtime::ledger::{ContractAddress, DefaultDB, StateValue};
use runtime::{CompactError, Field};
use serde_json::{Value, json};
use std::cell::Cell;

const REFUSAL: &str = "intentional witness refusal";

#[derive(Clone)]
struct CountingWitness {
    choice: Value,
    gate_calls: std::rc::Rc<Cell<usize>>,
    key_calls: std::rc::Rc<Cell<usize>>,
}

impl CountingWitness {
    fn new(choice: &Value) -> Self {
        Self {
            choice: choice.clone(),
            gate_calls: std::rc::Rc::new(Cell::new(0)),
            key_calls: std::rc::Rc::new(Cell::new(0)),
        }
    }

    fn next_private(&self, private: u64) -> Result<u64, CompactError> {
        assert_eq!(
            private, 7,
            "the one callback sees the initial private state"
        );
        if self.choice == "error" {
            Err(CompactError::AssertionFailed(REFUSAL.into()))
        } else {
            Ok(private + 1)
        }
    }
}

impl contract::TryWitnesses<u64> for CountingWitness {
    fn gate(
        &self,
        context: WitnessContext<'_, u64, contract::LedgerView<'_>>,
    ) -> Result<(u64, bool), CompactError> {
        self.gate_calls.set(self.gate_calls.get() + 1);
        Ok((
            self.next_private(*context.private_state)?,
            self.choice.as_bool().unwrap(),
        ))
    }

    fn key(
        &self,
        context: WitnessContext<'_, u64, contract::LedgerView<'_>>,
    ) -> Result<(u64, Field), CompactError> {
        self.key_calls.set(self.key_calls.get() + 1);
        Ok((
            self.next_private(*context.private_state)?,
            Field::from(self.choice.as_u64().unwrap()),
        ))
    }
}

fn oracle_rows() -> Vec<Value> {
    let capture: Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/witness-effect-order.json"
    ))
    .unwrap();
    let rows = capture["rows"].as_array().unwrap().clone();
    assert_eq!(rows.len(), 18);
    rows
}

fn state_hex(state: StateValue<DefaultDB>) -> String {
    // equal_result has no proof-required ledger operation in the TS contract.
    let operations =
        ["set_member", "map_member", "map_lookup"]
            .into_iter()
            .fold(HashMap::new(), |map, name| {
                map.insert(
                    EntryPointBuf(name.as_bytes().to_vec()),
                    ContractOperation::new(None),
                )
            });
    let state = ContractState::new(state, operations, ContractMaintenanceAuthority::default());
    let mut bytes = Vec::new();
    midnight_serialize::tagged_serialize(&state, &mut bytes).unwrap();
    hex::encode(bytes)
}

fn check_result<T: Clone + Into<AlignedValue>>(
    result: Result<CircuitResult<u64, T>, CompactError>,
    row: &Value,
    expected: AlignedValue,
) {
    if row["choice"] == "error" {
        assert_eq!(row["error"], "Error: intentional witness refusal");
        let error = result
            .err()
            .expect("witness refusal must not produce success");
        assert!(
            matches!(&error, CompactError::AssertionFailed(message) if message == REFUSAL),
            "{error:?}"
        );
        return;
    }
    if row["zeroGas"] == true {
        assert_eq!(row["error"], "CompactError: Error: ran out of gas budget");
        let error = result.err().expect("zero gas must reject the query");
        assert!(
            matches!(&error, CompactError::LedgerQueryRejected(message)
            if message == "Execution(OutOfGas)"),
            "{error:?}"
        );
        return;
    }
    if row["name"] == "map_lookup" && row["choice"] == 9 {
        assert_eq!(
            row["error"],
            "CompactError: Error: expected a cell, received null"
        );
        let error = result.err().expect("missing map entry must reject lookup");
        assert!(
            matches!(&error, CompactError::LedgerQueryRejected(message)
            if message == "Execution(ExpectedCell(null))"),
            "{error:?}"
        );
        return;
    }
    assert!(row.get("error").is_none());
    let out = result.unwrap();
    let actual: AlignedValue = out.result.clone().into();
    assert_eq!(actual, expected, "source-semantic result");
    assert_eq!(json!(actual), row["output"]);
    assert_eq!(
        state_hex(out.context.query.state.get_ref().clone()),
        row["after"]
    );
    assert_eq!(row["before"], row["after"], "queries preserve public state");
    assert_eq!(json!(out.context.query.effects), row["effects"]);
    assert_eq!(out.context.private_state, 8);
    assert_eq!(row["privateState"], 8);
    let witness_output = if row["name"] == "equal_result" {
        AlignedValue::from(row["choice"].as_bool().unwrap())
    } else {
        AlignedValue::from(Field::from(row["choice"].as_u64().unwrap()))
    };
    assert_eq!(out.private_transcript_outputs, [witness_output]);
    assert_eq!(json!(out.private_transcript_outputs), row["privateOutputs"]);
    assert!(out.context.circuit_zswap().inputs().is_empty());
    assert!(out.context.circuit_zswap().outputs().is_empty());
    for dimension in ["readTime", "computeTime", "bytesWritten", "bytesDeleted"] {
        let expected: u64 = row["queries"]
            .as_array()
            .unwrap()
            .iter()
            .map(|query| {
                query["gas"][dimension]
                    .as_str()
                    .unwrap()
                    .parse::<u64>()
                    .unwrap()
            })
            .sum();
        assert_eq!(json!(out.gas_cost)[dimension], expected);
        assert_eq!(row["gas"][dimension], expected.to_string());
    }
}

fn check_case(row: &Value) {
    let mut context = contract::initial_state(ConstructorContext::new(7_u64))
        .unwrap()
        .into_circuit_context(ContractAddress::default());
    assert_eq!(
        state_hex(context.query.state.get_ref().clone()),
        row["before"]
    );
    let initial_effects = json!(context.query.effects);
    if row["zeroGas"] == true {
        context.gas_limit = Some(Default::default());
    }
    let witness = CountingWitness::new(&row["choice"]);
    let name = row["name"].as_str().unwrap();
    let queries = row["queries"].as_array().unwrap();
    let queried = name != "equal_result" && row["choice"] != "error";
    assert_eq!(queries.len(), usize::from(queried), "TS query count");
    let witness_name = if name == "equal_result" {
        "gate"
    } else {
        "key"
    };
    assert_eq!(
        row["events"],
        if queried {
            json!([witness_name, "query"])
        } else {
            json!([witness_name])
        }
    );
    if row.get("error").is_none() {
        assert_eq!(row["effects"], initial_effects);
    }
    match name {
        "equal_result" => check_result(
            contract::equal_result(context, &witness, Field::from(42_u64)),
            row,
            AlignedValue::from(Field::from(42_u64)),
        ),
        "set_member" => check_result(
            contract::set_member(context, &witness),
            row,
            AlignedValue::from(row["choice"] == 7),
        ),
        "map_member" => check_result(
            contract::map_member(context, &witness),
            row,
            AlignedValue::from(row["choice"] == 7),
        ),
        "map_lookup" => check_result(
            contract::map_lookup(context, &witness),
            row,
            AlignedValue::from(Field::from(42_u64)),
        ),
        other => panic!("unexpected oracle circuit {other}"),
    }
    let calls = json!({"gate": witness.gate_calls.get(), "key": witness.key_calls.get()});
    let expected = if name == "equal_result" {
        json!({"gate": 1, "key": 0})
    } else {
        json!({"gate": 0, "key": 1})
    };
    assert_eq!(
        calls, expected,
        "each native callback executes once, including errors"
    );
    assert_eq!(row["calls"], expected);
}

#[test]
fn equal_arms_preserve_true_false_and_refused_witness_effects() {
    let rows: Vec<_> = oracle_rows()
        .into_iter()
        .filter(|r| r["name"] == "equal_result")
        .collect();
    assert_eq!(rows.len(), 3);
    for row in rows {
        check_case(&row);
    }
}

#[test]
fn membership_queries_use_one_witness_answer_for_present_absent_and_refused_keys() {
    let rows: Vec<_> = oracle_rows()
        .into_iter()
        .filter(|r| {
            (r["name"] == "set_member" || r["name"] == "map_member") && r["zeroGas"] == false
        })
        .collect();
    assert_eq!(rows.len(), 6);
    for row in rows {
        check_case(&row);
    }
}

#[test]
fn lookup_preserves_present_missing_and_refused_key_outcomes() {
    let rows: Vec<_> = oracle_rows()
        .into_iter()
        .filter(|r| r["name"] == "map_lookup" && r["zeroGas"] == false)
        .collect();
    assert_eq!(rows.len(), 3);
    for row in rows {
        check_case(&row);
    }
}

#[test]
fn witness_refusal_precedes_query_gas_failure_and_success_reaches_the_query() {
    let rows: Vec<_> = oracle_rows()
        .into_iter()
        .filter(|r| r["zeroGas"] == true)
        .collect();
    assert_eq!(rows.len(), 6);
    for row in rows {
        check_case(&row);
    }
}

#[cfg(feature = "ledger-transaction")]
#[test]
fn new_set_recording_preserves_callback_before_query_and_ts_results() {
    use runtime::transaction::{Observation, ObservedContractState};
    for row in oracle_rows()
        .into_iter()
        .filter(|r| r["name"] == "set_member")
    {
        let initial = contract::initial_state(ConstructorContext::new(7_u64))
            .unwrap()
            .into_circuit_context(Default::default());
        let state = ObservedContractState::new(
            Default::default(),
            ContractState::new(
                initial.query.state.get_ref().clone(),
                Default::default(),
                Default::default(),
            ),
            Observation {
                transaction_hash: [0; 32],
                block_hash: [0; 32],
                block_height: 0,
            },
        );
        let context = || {
            let mut c = state.circuit_context(7);
            if row["zeroGas"] == true {
                c.gas_limit = Some(Default::default());
            }
            c
        };
        let (a, b, w) = (
            CountingWitness::new(&row["choice"]),
            CountingWitness::new(&row["choice"]),
            CountingWitness::new(&row["choice"]),
        );
        let native = contract::set_member(context(), &a);
        let recorded = contract::recorded::set_member(context(), &b);
        if row.get("error").is_some() {
            assert_eq!(
                native.as_ref().err().unwrap(),
                recorded.as_ref().err().unwrap()
            );
            // Existing TS oracle validation distinguishes callback refusal from a
            // VM gas refusal after the one callback.
            check_result(native, &row, AlignedValue::from(false));
            if row["zeroGas"] == false {
                let generated = contract::Contract::from(w.clone());
                let call = generated.recording().set_member_call(&state, 7);
                assert_eq!(
                    call.err().unwrap(),
                    CompactError::AssertionFailed(REFUSAL.into())
                );
                assert_eq!(w.key_calls.get(), 1);
            }
        } else {
            let native = native.unwrap();
            let recorded = recorded.unwrap();
            let generated = contract::Contract::from(w.clone());
            let call = generated.recording().set_member_call(&state, 7).unwrap();
            for actual in [&recorded.execution, &call.recorded().execution] {
                assert_eq!(actual.result, native.result);
                assert_eq!(actual.context.query.state, native.context.query.state);
                assert_eq!(actual.context.query.effects, native.context.query.effects);
                assert_eq!(actual.context.private_state, 8);
                assert_eq!(actual.gas_cost, native.gas_cost);
                assert_eq!(
                    actual.private_transcript_outputs,
                    native.private_transcript_outputs
                );
            }
            assert!(!recorded.public.verify_ops().is_empty());
            let replay = recorded
                .public
                .initial()
                .query(
                    recorded.public.verify_ops(),
                    None,
                    &recorded.execution.context.cost_model,
                )
                .unwrap();
            assert_eq!(replay.context.state, native.context.query.state);
            assert_eq!(replay.context.effects, native.context.query.effects);
            assert_eq!(w.key_calls.get(), 1);
            assert_eq!(w.gate_calls.get(), 0);
            check_result(Ok(native), &row, AlignedValue::from(row["choice"] == 7));
        }
        for witness in [&a, &b] {
            assert_eq!(witness.key_calls.get(), 1);
            assert_eq!(witness.gate_calls.get(), 0);
        }
    }
}
