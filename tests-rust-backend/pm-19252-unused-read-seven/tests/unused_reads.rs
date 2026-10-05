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

use compact_rust_pm_19252_unused_read_eight_a_fixture::ledger_contract as eight_a;
use compact_rust_pm_19252_unused_read_eight_b_fixture::ledger_contract as eight_b;
use compact_rust_pm_19252_unused_read_seven_fixture::ledger_contract as seven;
use midnight_compact_runtime as runtime;
use midnight_onchain_state::state::{
    ContractMaintenanceAuthority, ContractOperation, ContractState, EntryPointBuf,
};
use midnight_serialize::tagged_serialize;
use midnight_storage::storage::HashMap;
use runtime::{
    Field,
    context::{CircuitContext, CircuitResult, ConstructorContext},
    ledger::{ContractAddress, DefaultDB, StateValue},
    recording::RecordedCircuitResult,
};

fn state_hex(state: StateValue<DefaultDB>, kind: &str) -> String {
    let mut operations = HashMap::new().insert(
        EntryPointBuf(b"test".to_vec()),
        ContractOperation::new(None),
    );
    if kind != "seven" {
        operations = operations.insert(
            EntryPointBuf(b"test1".to_vec()),
            ContractOperation::new(None),
        );
    }
    let state = ContractState::new(state, operations, ContractMaintenanceAuthority::default());
    let mut bytes = Vec::new();
    tagged_serialize(&state, &mut bytes).unwrap();
    hex::encode(bytes)
}

fn check(
    kind: &str,
    reference: &serde_json::Value,
    native: CircuitResult<(), ()>,
    recorded: RecordedCircuitResult<(), ()>,
) {
    assert_eq!(reference["result"], serde_json::json!([]));
    assert_eq!(reference["beforeStateHex"], reference["afterStateHex"]);
    assert_eq!(
        state_hex(native.context.query.state.get_ref().clone(), kind),
        reference["afterStateHex"]
    );
    assert_eq!(
        native.context.query.state,
        recorded.execution.context.query.state
    );
    assert_eq!(
        native.context.query.effects,
        recorded.execution.context.query.effects
    );
    assert_eq!(native.gas_cost, recorded.execution.gas_cost);
    assert!(native.private_transcript_outputs.is_empty());
    assert!(recorded.execution.private_transcript_outputs.is_empty());
    assert_eq!(reference["privateTranscriptCount"], 0);
    let queries = reference["queries"].as_array().unwrap();
    assert_eq!(queries.len(), 1);
    let gas = serde_json::to_value(recorded.execution.gas_cost).unwrap();
    for dimension in ["readTime", "computeTime", "bytesWritten", "bytesDeleted"] {
        assert_eq!(
            gas[dimension].as_u64().unwrap(),
            reference["gasCost"][dimension]
                .as_str()
                .unwrap()
                .parse::<u64>()
                .unwrap()
        );
        assert_eq!(
            queries[0]["gasCost"][dimension],
            reference["gasCost"][dimension]
        );
    }
    let program = serde_json::to_value(recorded.public.verify_ops()).unwrap();
    assert_eq!(program, reference["publicProgram"]);
    let mut gather = program;
    for op in gather.as_array_mut().unwrap() {
        if let Some(pop) = op.get_mut("popeq") {
            pop["result"] = serde_json::Value::Null;
        }
    }
    assert_eq!(gather, queries[0]["program"]);
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
    assert_eq!(replay.context.state, recorded.execution.context.query.state);
    assert_eq!(
        replay.context.effects,
        recorded.execution.context.query.effects
    );
}

#[test]
fn discarded_addition_still_records_the_original_cell_read() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/unused-field-read-seven.json"
    ))
    .unwrap();
    let context = || {
        seven::initial_state(ConstructorContext::new(()))
            .unwrap()
            .into_circuit_context(ContractAddress::default())
    };
    check(
        "seven",
        &oracle["cases"][0],
        seven::test(context(), Field::from(9)).unwrap(),
        seven::recorded::test(context(), Field::from(9)).unwrap(),
    );
}

#[test]
fn unused_public_and_private_cells_preserve_zero_and_populated_reads() {
    for (kind, raw) in [
        (
            "eight-a",
            include_str!("../../../runtime-rs/tests/fixtures/unused-field-read-eight-a.json"),
        ),
        (
            "eight-b",
            include_str!("../../../runtime-rs/tests/fixtures/unused-field-read-eight-b.json"),
        ),
    ] {
        let oracle: serde_json::Value = serde_json::from_str(raw).unwrap();
        for reference in oracle["cases"].as_array().unwrap() {
            let seed = reference["seed"].as_str().unwrap().parse::<u64>().unwrap();
            let context = || -> CircuitContext<()> {
                let initial = if kind == "eight-a" {
                    eight_a::initial_state(ConstructorContext::new(())).unwrap()
                } else {
                    eight_b::initial_state(ConstructorContext::new(())).unwrap()
                };
                let context = initial.into_circuit_context(ContractAddress::default());
                if seed == 0 {
                    context
                } else if kind == "eight-a" {
                    eight_a::test(context, Field::from(seed)).unwrap().context
                } else {
                    eight_b::test1(context, Field::from(seed)).unwrap().context
                }
            };
            let native = if kind == "eight-a" {
                eight_a::test1(context()).unwrap()
            } else {
                eight_b::test(context()).unwrap()
            };
            let recorded = if kind == "eight-a" {
                eight_a::recorded::test1(context()).unwrap()
            } else {
                eight_b::recorded::test(context()).unwrap()
            };
            check(kind, reference, native, recorded);
        }
    }
}
