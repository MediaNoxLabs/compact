// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//  http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use compact_rust_recorded_enum_cell_fixture::ledger_contract::{
    choose, current, initial_state, recorded, selectNo,
};
use compact_rust_recorded_enum_cell_fixture::types::Choice;
use midnight_compact_runtime as runtime;
use midnight_onchain_state::state::{
    ContractMaintenanceAuthority, ContractOperation, ContractState, EntryPointBuf,
};
use midnight_onchain_vm::cost_model::INITIAL_COST_MODEL;
use midnight_serialize::tagged_serialize;
use midnight_storage::storage::HashMap;
use runtime::context::ConstructorContext;
use runtime::ledger::{ContractAddress, DefaultDB, StateValue};

fn state_hex(state: StateValue<DefaultDB>) -> String {
    let mut operations: HashMap<EntryPointBuf, ContractOperation, DefaultDB> = HashMap::new();
    for name in ["choose", "current", "selectNo"] {
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

fn assert_replay<Output>(recorded: &runtime::recording::RecordedCircuitResult<(), Output>) {
    let replay = recorded
        .public
        .initial()
        .query(recorded.public.verify_ops(), None, &INITIAL_COST_MODEL)
        .unwrap();
    assert_eq!(
        state_hex(replay.context.state.get_ref().clone()),
        state_hex(recorded.execution.context.query.state.get_ref().clone())
    );
    assert_eq!(replay.gas_cost, recorded.execution.gas_cost);
    assert!(recorded.execution.private_transcript_outputs.is_empty());
}

#[test]
fn enum_cell_native_and_recorded_calls_match_typescript_and_replay() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/recorded-enum-cell-oracle.json"
    ))
    .unwrap();
    let native = initial_state(ConstructorContext::new(())).unwrap();
    let recorded = initial_state(ConstructorContext::new(())).unwrap();
    assert_eq!(
        state_hex(native.ledger_state.get_ref().clone()),
        oracle["afterInit"]
    );

    let native = choose(
        native.into_circuit_context(ContractAddress::default()),
        Choice::no,
    )
    .unwrap();
    let recorded = recorded::choose(
        recorded.into_circuit_context(ContractAddress::default()),
        Choice::no,
    )
    .unwrap();
    assert_replay(&recorded);
    assert_eq!(recorded.public.verify_ops().len(), 3);
    assert_eq!(native.gas_cost, recorded.execution.gas_cost);
    assert_eq!(
        state_hex(native.context.query.state.get_ref().clone()),
        oracle["afterChooseNo"]
    );
    assert_eq!(
        state_hex(recorded.execution.context.query.state.get_ref().clone()),
        oracle["afterChooseNo"]
    );

    let native = current(native.context).unwrap();
    let recorded = recorded::current(recorded.execution.context).unwrap();
    assert_replay(&recorded);
    assert_eq!(recorded.public.verify_ops().len(), 3);
    assert_eq!(native.result, Choice::no);
    assert_eq!(recorded.execution.result, native.result);
    assert_eq!(oracle["currentNo"], 1);
    assert_eq!(native.gas_cost, recorded.execution.gas_cost);

    let native = choose(native.context, Choice::yes).unwrap();
    let recorded = recorded::choose(recorded.execution.context, Choice::yes).unwrap();
    assert_replay(&recorded);
    assert_eq!(native.gas_cost, recorded.execution.gas_cost);
    assert_eq!(
        state_hex(recorded.execution.context.query.state.get_ref().clone()),
        oracle["afterChooseYes"]
    );
    assert_eq!(
        state_hex(native.context.query.state.get_ref().clone()),
        oracle["afterChooseYes"]
    );

    let native = selectNo(native.context).unwrap();
    let recorded = recorded::selectNo(recorded.execution.context).unwrap();
    assert_replay(&recorded);
    assert_eq!(native.gas_cost, recorded.execution.gas_cost);
    assert_eq!(
        state_hex(recorded.execution.context.query.state.get_ref().clone()),
        oracle["afterSelectNo"]
    );
    assert_eq!(
        state_hex(native.context.query.state.get_ref().clone()),
        oracle["afterSelectNo"]
    );
}
