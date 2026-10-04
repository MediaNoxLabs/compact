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

use compact_rust_internal_pure_call_fixture::ledger_contract::{
    initial_state, read_stored, recorded, save,
};
#[path = "../../boolean_observation_assertions.rs"]
mod boolean_observation_assertions;
use compact_rust_internal_pure_call_fixture::pure_circuits::double_increment;
use midnight_compact_runtime::Field;
use midnight_compact_runtime::context::ConstructorContext;
use midnight_compact_runtime::ledger::{ContractAddress, DefaultDB, StateValue};
use midnight_onchain_state::state::{
    ContractMaintenanceAuthority, ContractOperation, ContractState, EntryPointBuf,
};
use midnight_serialize::tagged_serialize;
use midnight_storage::storage::HashMap;

fn state_hex(state: StateValue<DefaultDB>) -> String {
    let mut operations: HashMap<EntryPointBuf, ContractOperation, DefaultDB> = HashMap::new();
    for name in ["save", "read_stored"] {
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
fn local_pure_helper_is_callable_without_becoming_an_export() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/internal-pure-call.json"
    ))
    .unwrap();
    assert_eq!(
        double_increment(Field::from(7_u64)).unwrap(),
        Field::from(9_u64)
    );
    assert_eq!(oracle["exportedPure"], "9");
    let context = initial_state(ConstructorContext::new(()))
        .unwrap()
        .into_circuit_context(ContractAddress::default());
    let write = save(context, Field::from(7_u64)).unwrap();
    let read = read_stored(write.context).unwrap();
    assert_eq!(read.result, Field::from(8_u64));
    assert_eq!(oracle["stored"], "8");
    assert_eq!(
        state_hex(read.context.query.state.get_ref().clone()),
        oracle["stateHex"]
    );
}

#[test]
fn recorded_scalar_pure_argument_matches_typescript_trace_and_gas() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/internal-pure-call.json"
    ))
    .unwrap();
    let native = save(
        initial_state(ConstructorContext::new(()))
            .unwrap()
            .into_circuit_context(ContractAddress::default()),
        Field::from(7_u64),
    )
    .unwrap();
    let recorded = recorded::save(
        initial_state(ConstructorContext::new(()))
            .unwrap()
            .into_circuit_context(ContractAddress::default()),
        Field::from(7_u64),
    )
    .unwrap();
    boolean_observation_assertions::assert_ts_trace(
        "save",
        &native,
        &recorded,
        &oracle["saveTrace"],
    );
    assert_eq!(native.context.private_state, ());
    assert_eq!(recorded.execution.context.private_state, ());
    assert!(native.private_transcript_outputs.is_empty());
    assert!(recorded.execution.private_transcript_outputs.is_empty());
    assert_eq!(
        native.context.query.state.get_ref(),
        recorded.execution.context.query.state.get_ref(),
    );
    assert_eq!(
        state_hex(recorded.execution.context.query.state.get_ref().clone()),
        oracle["stateHex"],
    );
}
