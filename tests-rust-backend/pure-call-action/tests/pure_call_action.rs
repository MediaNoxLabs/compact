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

use compact_rust_pure_call_action_fixture::ledger_contract::{initial_state, recorded, save};
use midnight_compact_runtime::Field;
use midnight_compact_runtime::context::ConstructorContext;
use midnight_compact_runtime::ledger::{ContractAddress, DefaultDB, StateValue};
use midnight_onchain_state::state::{
    ContractMaintenanceAuthority, ContractOperation, ContractState, EntryPointBuf,
};
use midnight_serialize::tagged_serialize;
use midnight_storage::storage::HashMap;

fn state_hex(state: StateValue<DefaultDB>) -> String {
    let operations: HashMap<EntryPointBuf, ContractOperation, DefaultDB> = HashMap::new().insert(
        EntryPointBuf(b"save".to_vec()),
        ContractOperation::new(None),
    );
    let contract_state =
        ContractState::new(state, operations, ContractMaintenanceAuthority::default());
    let mut bytes = Vec::new();
    tagged_serialize(&contract_state, &mut bytes).unwrap();
    hex::encode(bytes)
}

#[test]
fn pure_call_action_propagates_error_before_write_and_matches_success_state() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/pure-call-action.json"
    ))
    .unwrap();
    let constructor = initial_state(ConstructorContext::new(())).unwrap();
    let context = constructor.into_circuit_context(ContractAddress::default());
    let error = match save(context, Field::from(0_u64)) {
        Err(error) => error,
        Ok(_) => panic!("zero input unexpectedly passed"),
    };
    assert_eq!(error.to_string(), oracle["zeroError"]);

    let constructor = initial_state(ConstructorContext::new(())).unwrap();
    let context = constructor.into_circuit_context(ContractAddress::default());
    let saved = save(context, Field::from(7_u64)).unwrap();
    assert_eq!(
        state_hex(saved.context.query.state.get_ref().clone()),
        oracle["afterHex"]
    );
}

#[test]
fn recorded_pure_assert_call_matches_typescript_success_and_failure() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/recorded-pure-assert.json"
    ))
    .unwrap();
    let success = &oracle["saveSuccess"];
    let initial = initial_state(ConstructorContext::new(())).unwrap();
    assert_eq!(
        state_hex(initial.ledger_state.get_ref().clone()),
        success["initialStateHex"],
    );
    let native = save(
        initial.into_circuit_context(ContractAddress::default()),
        Field::from(7_u64),
    )
    .unwrap();
    let initial = initial_state(ConstructorContext::new(())).unwrap();
    let recorded = recorded::save(
        initial.into_circuit_context(ContractAddress::default()),
        Field::from(7_u64),
    )
    .unwrap();
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

    let failure = &oracle["saveFailure"];
    assert_eq!(failure["compactError"], true);
    assert_eq!(failure["queryCount"], 0);
    assert_eq!(failure["afterStateHex"], failure["initialStateHex"]);
    let initial = initial_state(ConstructorContext::new(())).unwrap();
    assert_eq!(
        state_hex(initial.ledger_state.get_ref().clone()),
        failure["initialStateHex"],
    );
    let native_error = save(
        initial.into_circuit_context(ContractAddress::default()),
        Field::from(0_u64),
    )
    .err()
    .expect("zero input should fail");
    let initial = initial_state(ConstructorContext::new(())).unwrap();
    let recorded_error = recorded::save(
        initial.into_circuit_context(ContractAddress::default()),
        Field::from(0_u64),
    )
    .err()
    .expect("recorded zero input should fail");
    assert!(matches!(
        recorded_error,
        midnight_compact_runtime::CompactError::AssertionFailed(_)
    ));
    assert_eq!(native_error.to_string(), failure["error"]);
    assert_eq!(recorded_error.to_string(), failure["error"]);
}

#[test]
fn direct_pure_guard_accepts_nonzero_and_preserves_zero_error() {
    use compact_rust_pure_call_action_fixture::pure_circuits::require_positive;
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/pure-call-action.json"
    ))
    .unwrap();
    require_positive(Field::from(7_u64)).unwrap();
    let error = require_positive(Field::from(0_u64)).unwrap_err();
    assert_eq!(
        error,
        midnight_compact_runtime::CompactError::AssertionFailed("value must be positive".into())
    );
    assert_eq!(error.to_string(), oracle["zeroError"]);
}
