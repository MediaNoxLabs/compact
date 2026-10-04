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

use compact_rust_cross_circuit_oracle_fixture::ledger_contract::{
    initial_state, recorded, reset, reset_and_set,
};
use midnight_compact_runtime as runtime;
use midnight_onchain_state::state::{
    ContractMaintenanceAuthority, ContractOperation, ContractState, EntryPointBuf,
};
use midnight_onchain_vm::ops::Op;
use midnight_serialize::tagged_serialize;
use midnight_storage::storage::HashMap;
use runtime::context::ConstructorContext;
use runtime::ledger::{ContractAddress, DefaultDB, StateValue};

fn state_hex(state: StateValue<DefaultDB>) -> String {
    let mut operations: HashMap<EntryPointBuf, ContractOperation, DefaultDB> = HashMap::new();
    for name in ["reset", "reset_and_set"] {
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

fn value(state: &StateValue<DefaultDB>) -> u128 {
    runtime::ledger::read_root_cell::<runtime::BoundedUint<{ u64::MAX as u128 }>, _>(state, 0)
        .unwrap()
        .value()
}

#[test]
fn exported_stateful_circuit_call_matches_typescript_state() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/cross-circuit-oracle.json"
    ))
    .unwrap();
    let initial = initial_state(ConstructorContext::new(())).unwrap();
    let state = initial.ledger_state.get_ref();
    assert_eq!(state_hex(state.clone()), oracle["afterInit"]);
    assert_eq!(value(state).to_string(), oracle["initialValue"]);
    let context = initial.into_circuit_context(ContractAddress::default());
    let set7 = reset_and_set(context, runtime::BoundedUint::new(7).unwrap()).unwrap();
    assert_eq!(
        state_hex(set7.context.query.state.get_ref().clone()),
        oracle["afterSet7"]
    );
    assert_eq!(value(set7.context.query.state.get_ref()), 7);
    let set13 = reset_and_set(set7.context, runtime::BoundedUint::new(13).unwrap()).unwrap();
    assert_eq!(
        state_hex(set13.context.query.state.get_ref().clone()),
        oracle["afterSet13"]
    );
    assert_eq!(value(set13.context.query.state.get_ref()), 13);
    let cleared = reset(set13.context).unwrap();
    assert_eq!(
        state_hex(cleared.context.query.state.get_ref().clone()),
        oracle["afterReset"]
    );
    assert_eq!(value(cleared.context.query.state.get_ref()), 0);
}

fn assert_gas(cost: &runtime::context::RunningCost, expected: &serde_json::Value) {
    assert_eq!(
        cost.read_time.into_picoseconds().to_string(),
        expected["readTime"]
    );
    assert_eq!(
        cost.compute_time.into_picoseconds().to_string(),
        expected["computeTime"]
    );
    assert_eq!(cost.bytes_written.to_string(), expected["bytesWritten"]);
    assert_eq!(cost.bytes_deleted.to_string(), expected["bytesDeleted"]);
}

fn assert_recorded_step(
    native: &runtime::context::CircuitResult<(), ()>,
    recorded: &runtime::recording::RecordedCircuitResult<(), ()>,
    expected: &serde_json::Value,
    expected_state: &serde_json::Value,
    expected_value: u128,
    writes: usize,
) {
    assert_eq!(native.gas_cost, recorded.execution.gas_cost);
    assert_gas(&native.gas_cost, &expected["totalGas"]);
    assert_gas(&recorded.execution.gas_cost, &expected["totalGas"]);
    if writes == 1 {
        assert_eq!(expected["reportedGas"], expected["totalGas"]);
    } else {
        assert_ne!(expected["reportedGas"], expected["totalGas"]);
    }

    let write_transcript = [
        serde_json::json!({ "kind": "push", "storage": false }),
        serde_json::json!({ "kind": "push", "storage": true }),
        serde_json::json!({ "kind": "ins", "cached": false, "n": 1 }),
    ];
    let expected_transcript = (0..writes)
        .flat_map(|_| write_transcript.iter().cloned())
        .collect::<Vec<_>>();
    assert_eq!(
        expected["transcript"],
        serde_json::json!(expected_transcript)
    );
    assert_eq!(expected["queries"].as_array().unwrap().len(), writes);
    for query in expected["queries"].as_array().unwrap() {
        assert_eq!(query["transcript"], serde_json::json!(write_transcript));
    }
    let ops = recorded.public.verify_ops();
    match writes {
        1 => assert!(matches!(
            ops,
            [
                Op::Push { storage: false, .. },
                Op::Push { storage: true, .. },
                Op::Ins {
                    cached: false,
                    n: 1
                },
            ]
        )),
        2 => assert!(matches!(
            ops,
            [
                Op::Push { storage: false, .. },
                Op::Push { storage: true, .. },
                Op::Ins {
                    cached: false,
                    n: 1
                },
                Op::Push { storage: false, .. },
                Op::Push { storage: true, .. },
                Op::Ins {
                    cached: false,
                    n: 1
                },
            ]
        )),
        _ => panic!("unsupported number of Cell writes in this oracle"),
    }

    let replay = recorded
        .public
        .initial()
        .query(ops, None, &recorded.execution.context.cost_model)
        .unwrap();
    assert_eq!(
        native.context.query.effects,
        recorded.execution.context.query.effects
    );
    assert_eq!(native.context.query.effects, replay.context.effects);
    for state in [
        native.context.query.state.get_ref(),
        recorded.execution.context.query.state.get_ref(),
        replay.context.state.get_ref(),
    ] {
        assert_eq!(state_hex(state.clone()), *expected_state);
        assert_eq!(value(state), expected_value);
    }
    assert!(native.private_transcript_outputs.is_empty());
    assert!(recorded.execution.private_transcript_outputs.is_empty());
    assert_eq!(expected["privateTranscriptCount"], 0);
}

#[test]
fn exported_stateful_circuit_call_records_and_replays_typescript_trace() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/cross-circuit-oracle.json"
    ))
    .unwrap();
    let native = initial_state(ConstructorContext::new(())).unwrap();
    let recorded_state = initial_state(ConstructorContext::new(())).unwrap();
    let native_context = native.into_circuit_context(ContractAddress::default());
    let recorded_context = recorded_state.into_circuit_context(ContractAddress::default());

    let native = reset_and_set(native_context, runtime::BoundedUint::new(7).unwrap()).unwrap();
    let recorded =
        recorded::reset_and_set(recorded_context, runtime::BoundedUint::new(7).unwrap()).unwrap();
    assert_recorded_step(
        &native,
        &recorded,
        &oracle["set7"],
        &oracle["afterSet7"],
        7,
        2,
    );

    let native = reset_and_set(native.context, runtime::BoundedUint::new(13).unwrap()).unwrap();
    let recorded = recorded::reset_and_set(
        recorded.execution.context,
        runtime::BoundedUint::new(13).unwrap(),
    )
    .unwrap();
    assert_recorded_step(
        &native,
        &recorded,
        &oracle["set13"],
        &oracle["afterSet13"],
        13,
        2,
    );

    let native = reset(native.context).unwrap();
    let recorded = recorded::reset(recorded.execution.context).unwrap();
    assert_recorded_step(
        &native,
        &recorded,
        &oracle["reset"],
        &oracle["afterReset"],
        0,
        1,
    );
}
