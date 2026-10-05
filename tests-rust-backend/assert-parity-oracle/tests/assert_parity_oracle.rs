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

use compact_rust_assert_parity_oracle_fixture::ledger_contract::{
    initial_state, recorded, trigger_fail, trigger_ok,
};
use compact_rust_assert_parity_oracle_fixture::pure_circuits::require_true;
use midnight_compact_runtime as runtime;
use midnight_onchain_state::state::{
    ContractMaintenanceAuthority, ContractOperation, ContractState, EntryPointBuf,
};
use midnight_serialize::tagged_serialize;
use midnight_storage::storage::HashMap;
use runtime::context::ConstructorContext;
use runtime::ledger::{ContractAddress, DefaultDB, StateValue};

fn state_hex(state: StateValue<DefaultDB>) -> String {
    let mut operations: HashMap<EntryPointBuf, ContractOperation, DefaultDB> = HashMap::new();
    for name in ["trigger_ok", "trigger_fail", "ping"] {
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

#[test]
fn exact_assertion_oracle_returns_errors_without_panicking() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/assert-parity-oracle.json"
    ))
    .unwrap();
    assert_eq!(require_true(true).unwrap(), oracle["pureTrue"]["ok"]);
    let pure_error = require_true(false).unwrap_err();
    assert!(matches!(
        pure_error,
        runtime::CompactError::AssertionFailed(_)
    ));
    assert_eq!(pure_error.to_string(), oracle["pureFalse"]["error"]);
    assert_eq!(oracle["pureFalse"]["compactError"], true);

    let initial = initial_state(ConstructorContext::new(())).unwrap();
    assert_eq!(
        state_hex(initial.ledger_state.get_ref().clone()),
        oracle["afterInit"]
    );
    let ok = trigger_ok(initial.into_circuit_context(ContractAddress::default())).unwrap();
    assert_eq!(
        state_hex(ok.context.query.state.get_ref().clone()),
        oracle["afterTriggerOk"]
    );
    let flag =
        runtime::ledger::read_root_cell::<bool, _>(ok.context.query.state.get_ref(), 0).unwrap();
    assert_eq!(flag, oracle["flagAfterTriggerOk"]);
    let error = trigger_fail(ok.context)
        .err()
        .expect("assertion should fail");
    assert!(matches!(error, runtime::CompactError::AssertionFailed(_)));
    assert_eq!(error.to_string(), oracle["triggerFail"]["error"]);
    assert_eq!(oracle["triggerFail"]["compactError"], true);
    assert_eq!(oracle["afterTriggerFail"], oracle["afterTriggerOk"]);
}

#[test]
fn recorded_boolean_pure_assert_matches_typescript_success_and_failure() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/recorded-pure-assert.json"
    ))
    .unwrap();
    let success = &oracle["triggerOk"];
    let initial = initial_state(ConstructorContext::new(())).unwrap();
    assert_eq!(
        state_hex(initial.ledger_state.get_ref().clone()),
        success["initialStateHex"],
    );
    let native = trigger_ok(initial.into_circuit_context(ContractAddress::default())).unwrap();
    let initial = initial_state(ConstructorContext::new(())).unwrap();
    let recorded =
        recorded::trigger_ok(initial.into_circuit_context(ContractAddress::default())).unwrap();
    let _: () = native.result;
    let _: () = recorded.execution.result;
    assert_eq!(success["result"], "");
    assert_eq!(native.gas_cost, recorded.execution.gas_cost);
    assert_eq!(
        native.context.query.effects,
        recorded.execution.context.query.effects
    );
    assert_eq!(
        native.context.query.state.get_ref(),
        recorded.execution.context.query.state.get_ref(),
    );
    assert_eq!(
        state_hex(recorded.execution.context.query.state.get_ref().clone()),
        success["afterStateHex"],
    );
    assert!(native.private_transcript_outputs.is_empty());
    assert!(recorded.execution.private_transcript_outputs.is_empty());
    assert_eq!(success["privateTranscriptCount"], 0);
    let vm: serde_json::Value = serde_json::to_value(recorded.public.verify_ops()).unwrap();
    let shape = serde_json::Value::Array(
        vm.as_array()
            .unwrap()
            .iter()
            .map(|operation| {
                if let Some(push) = operation.get("push") {
                    serde_json::json!({"kind":"push", "storage":push["storage"]})
                } else if let Some(ins) = operation.get("ins") {
                    serde_json::json!({"kind":"ins", "cached":ins["cached"], "n":ins["n"]})
                } else {
                    panic!("unexpected VM operation: {operation}")
                }
            })
            .collect(),
    );
    assert_eq!(shape, success["publicTranscriptShape"]);
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
    let actual = serde_json::to_value(native.gas_cost).unwrap();
    for dimension in ["readTime", "computeTime", "bytesWritten", "bytesDeleted"] {
        assert_eq!(
            actual[dimension].as_u64().unwrap().to_string(),
            success["queries"][0]["gasCost"][dimension],
            "{dimension}",
        );
        assert_eq!(
            success["queries"][0]["gasCost"][dimension],
            success["reportedGas"][dimension],
        );
    }

    let failure = &oracle["triggerFail"];
    assert_eq!(failure["compactError"], true);
    assert_eq!(failure["queryCount"], 0);
    assert_eq!(failure["afterStateHex"], failure["initialStateHex"]);
    let initial = initial_state(ConstructorContext::new(())).unwrap();
    assert_eq!(
        state_hex(initial.ledger_state.get_ref().clone()),
        failure["initialStateHex"],
    );
    let native_error = trigger_fail(initial.into_circuit_context(ContractAddress::default()))
        .err()
        .expect("false assertion should fail");
    let initial = initial_state(ConstructorContext::new(())).unwrap();
    let recorded_error =
        recorded::trigger_fail(initial.into_circuit_context(ContractAddress::default()))
            .err()
            .expect("recorded false assertion should fail");
    assert!(matches!(
        recorded_error,
        runtime::CompactError::AssertionFailed(_)
    ));
    assert_eq!(native_error.to_string(), failure["error"]);
    assert_eq!(recorded_error.to_string(), failure["error"]);
}

fn assert_direct_trace(
    native: &midnight_compact_runtime::context::CircuitResult<(), ()>,
    recorded: &midnight_compact_runtime::recording::RecordedCircuitResult<(), ()>,
    row: &serde_json::Value,
) {
    assert_eq!(native.result, ());
    assert_eq!(recorded.execution.result, ());
    assert_eq!(row["result"], serde_json::json!([]));
    assert_eq!(
        state_hex(native.context.query.state.get_ref().clone()),
        row["after"]
    );
    assert_eq!(
        native.context.query.state,
        recorded.execution.context.query.state
    );
    assert_eq!(
        native.context.query.effects,
        recorded.execution.context.query.effects
    );
    assert_eq!(
        serde_json::json!(recorded.public.verify_ops()),
        row["publicTranscript"]
    );
    assert!(native.private_transcript_outputs.is_empty());
    assert!(recorded.execution.private_transcript_outputs.is_empty());
    assert_eq!(row["privateTranscript"], serde_json::json!([]));
    assert_eq!(native.gas_cost, recorded.execution.gas_cost);
    let gas = serde_json::json!(native.gas_cost);
    for dimension in ["readTime", "computeTime", "bytesWritten", "bytesDeleted"] {
        let queries = row["queries"].as_array().unwrap();
        let sum: u64 = queries
            .iter()
            .map(|q| {
                q["gasCost"][dimension]
                    .as_str()
                    .unwrap()
                    .parse::<u64>()
                    .unwrap()
            })
            .sum();
        assert_eq!(gas[dimension], sum);
        assert_eq!(
            row["reportedGas"][dimension],
            queries.last().unwrap()["gasCost"][dimension]
        );
    }
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
}

#[test]
fn ping_is_executed_and_matches_independent_typescript_recording() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/oracle-direct-behavior.json"
    ))
    .unwrap();
    let rows = oracle["stateful"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|row| row["export"] == "ping")
        .collect::<Vec<_>>();
    assert_eq!(rows.len(), 1);
    let row = rows[0];
    let fresh = || {
        initial_state(ConstructorContext::new(()))
            .unwrap()
            .into_circuit_context(ContractAddress::default())
    };
    assert_eq!(
        state_hex(fresh().query.state.get_ref().clone()),
        row["before"]
    );
    let native = compact_rust_assert_parity_oracle_fixture::ledger_contract::ping(fresh()).unwrap();
    let recorded = recorded::ping(fresh()).unwrap();
    assert_direct_trace(&native, &recorded, row);
    assert!(
        runtime::ledger::read_root_cell::<bool, _>(native.context.query.state.get_ref(), 0)
            .unwrap()
    );
    assert_ne!(row["before"], row["after"]);
}
