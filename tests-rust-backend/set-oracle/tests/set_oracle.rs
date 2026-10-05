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

use compact_rust_set_oracle_fixture::ledger_contract::{Contract, check, initial_state};
use midnight_compact_runtime as runtime;
use midnight_onchain_state::state::{
    ContractMaintenanceAuthority, ContractOperation, ContractState, EntryPointBuf,
};
use midnight_serialize::tagged_serialize;
use midnight_storage::storage::HashMap;
use runtime::context::ConstructorContext;
use runtime::ledger::{ContractAddress, DefaultDB, StateValue};

fn state_hex(state: StateValue<DefaultDB>) -> String {
    let operations: HashMap<EntryPointBuf, ContractOperation, DefaultDB> = HashMap::new().insert(
        EntryPointBuf(b"check".to_vec()),
        ContractOperation::new(None),
    );
    let contract_state =
        ContractState::new(state, operations, ContractMaintenanceAuthority::default());
    let mut bytes = Vec::new();
    tagged_serialize(&contract_state, &mut bytes).unwrap();
    hex::encode(bytes)
}

#[test]
fn exact_set_member_oracle_matches_typescript_state() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/set-oracle.json"
    ))
    .unwrap();
    let initial = initial_state(ConstructorContext::new(())).unwrap();
    assert_eq!(
        state_hex(initial.ledger_state.get_ref().clone()),
        oracle["afterInit"]
    );
    let check7 = check(
        initial.into_circuit_context(ContractAddress::default()),
        runtime::Field::from(7_u64),
    )
    .unwrap();
    assert_eq!(
        state_hex(check7.context.query.state.get_ref().clone()),
        oracle["afterCheck7"]
    );
    assert_eq!(
        runtime::ledger::read_root_cell::<bool, _>(check7.context.query.state.get_ref(), 0)
            .unwrap(),
        oracle["flagAfterCheck7"]
    );
    let check8 = check(check7.context, runtime::Field::from(8_u64)).unwrap();
    assert_eq!(
        state_hex(check8.context.query.state.get_ref().clone()),
        oracle["afterCheck8"]
    );
    assert_eq!(
        runtime::ledger::read_root_cell::<bool, _>(check8.context.query.state.get_ref(), 0)
            .unwrap(),
        oracle["flagAfterCheck8"]
    );
}

#[test]
fn recorded_set_member_and_cell_write_match_oracle_and_replay() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/set-oracle.json"
    ))
    .unwrap();
    let context = initial_state(ConstructorContext::new(()))
        .unwrap()
        .into_circuit_context(ContractAddress::default());
    let call = Contract::default()
        .recording
        .check(context, runtime::Field::from(7_u64))
        .unwrap();
    assert_eq!(
        state_hex(call.execution.context.query.state.get_ref().clone()),
        oracle["afterCheck7"]
    );
    let replay = call
        .public
        .initial()
        .query(
            call.public.verify_ops(),
            None,
            &call.execution.context.cost_model,
        )
        .unwrap();
    assert_eq!(
        replay.context.state.get_ref(),
        call.execution.context.query.state.get_ref()
    );
    assert_eq!(replay.context.effects, call.execution.context.query.effects);
}

#[path = "../../support/oracle_recorded_trace.rs"]
mod direct_trace;

#[test]
fn exported_set_check_recording_matches_both_typescript_traces() {
    use compact_rust_set_oracle_fixture::ledger_contract::recorded;
    let rows = direct_trace::cases("set_oracle", 2);
    let mut native_context = initial_state(ConstructorContext::new(()))
        .unwrap()
        .into_circuit_context(ContractAddress::default());
    let mut recorded_context = initial_state(ConstructorContext::new(()))
        .unwrap()
        .into_circuit_context(ContractAddress::default());
    for (row, label) in rows.iter().zip(["seven", "eight"]) {
        assert_eq!(row["id"], format!("set_oracle/check/{label}"));
        let x = runtime::Field::from(row["args"][0].as_str().unwrap().parse::<u64>().unwrap());
        let native = check(native_context, x).unwrap();
        let recorded = recorded::check(recorded_context, x).unwrap();
        direct_trace::assert_trace(&native, &recorded, row, state_hex);
        native_context = native.context;
        recorded_context = recorded.execution.context;
    }
}
