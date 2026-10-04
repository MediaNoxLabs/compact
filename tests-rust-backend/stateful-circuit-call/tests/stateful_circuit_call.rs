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

use compact_rust_stateful_circuit_call_fixture::ledger_contract::{
    Contract, add_twice, bump_twice, initial_state,
};
use midnight_compact_runtime::BoundedUint;
use midnight_compact_runtime::context::ConstructorContext;
use midnight_compact_runtime::ledger::{ContractAddress, DefaultDB, StateValue, read_counter};
use midnight_onchain_state::state::{
    ContractMaintenanceAuthority, ContractOperation, ContractState, EntryPointBuf,
};
use midnight_serialize::tagged_serialize;
use midnight_storage::storage::HashMap;

fn state_hex(state: StateValue<DefaultDB>) -> String {
    let mut operations: HashMap<EntryPointBuf, ContractOperation, DefaultDB> = HashMap::new();
    for name in ["bump", "bump_twice", "add", "add_twice"] {
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
fn stateful_calls_execute_twice_and_match_typescript_state() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/stateful-circuit-call.json"
    ))
    .unwrap();
    let context = initial_state(ConstructorContext::new(()))
        .unwrap()
        .into_circuit_context(ContractAddress::default());
    let called = bump_twice(context).unwrap();
    let state = called.context.query.state.get_ref();
    let StateValue::Array(fields) = state else {
        panic!("expected ledger field array")
    };
    assert_eq!(read_counter(fields.get(0).unwrap()).unwrap(), 2);
    assert_eq!(state_hex(state.clone()), oracle["afterBumpHex"]);
    let added = add_twice(called.context, BoundedUint::<65535>::new(3).unwrap()).unwrap();
    let state = added.context.query.state.get_ref();
    let StateValue::Array(fields) = state else {
        panic!("expected ledger field array")
    };
    assert_eq!(read_counter(fields.get(0).unwrap()).unwrap(), 8);
    assert_eq!(state_hex(state.clone()), oracle["afterHex"]);
}

#[test]
fn nested_recorded_calls_preserve_oracle_state_and_replay_effects() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/stateful-circuit-call.json"
    ))
    .unwrap();
    let context = initial_state(ConstructorContext::new(()))
        .unwrap()
        .into_circuit_context(ContractAddress::default());
    let contract = Contract::default();
    let first = contract.recording.bump_twice(context).unwrap();
    assert_eq!(first.public.verify_ops().len(), 6);
    let replay = first
        .public
        .initial()
        .query(
            first.public.verify_ops(),
            None,
            &first.execution.context.cost_model,
        )
        .unwrap();
    assert_eq!(
        replay.context.effects,
        first.execution.context.query.effects
    );
    assert_eq!(
        state_hex(first.execution.context.query.state.get_ref().clone()),
        oracle["afterBumpHex"]
    );

    let second = contract
        .recording
        .add_twice(
            first.execution.context,
            BoundedUint::<65535>::new(3).unwrap(),
        )
        .unwrap();
    assert_eq!(second.public.verify_ops().len(), 6);
    let replay = second
        .public
        .initial()
        .query(
            second.public.verify_ops(),
            None,
            &second.execution.context.cost_model,
        )
        .unwrap();
    assert_eq!(
        replay.context.effects,
        second.execution.context.query.effects
    );
    assert_eq!(
        state_hex(second.execution.context.query.state.get_ref().clone()),
        oracle["afterHex"]
    );
}
