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

use compact_rust_map_oracle_fixture::ledger_contract::{PublicStateView, initial_state, put};
use midnight_compact_runtime as runtime;
use midnight_onchain_state::state::{
    ContractMaintenanceAuthority, ContractOperation, ContractState, EntryPointBuf,
};
use midnight_serialize::tagged_serialize;
use midnight_storage::storage::HashMap;
use runtime::context::ConstructorContext;
use runtime::ledger::{ContractAddress, DefaultDB, StateValue};

fn state_hex(state: StateValue<DefaultDB>) -> String {
    let operations: HashMap<EntryPointBuf, ContractOperation, DefaultDB> =
        HashMap::new().insert(EntryPointBuf(b"put".to_vec()), ContractOperation::new(None));
    let contract_state =
        ContractState::new(state, operations, ContractMaintenanceAuthority::default());
    let mut bytes = Vec::new();
    tagged_serialize(&contract_state, &mut bytes).unwrap();
    hex::encode(bytes)
}

#[test]
fn exact_map_oracle_matches_typescript_insertion_and_replacement() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/map-oracle.json"
    ))
    .unwrap();
    let initial = initial_state(ConstructorContext::new(())).unwrap();
    assert!(PublicStateView::from(&initial).m().unwrap().is_empty());
    assert_eq!(
        state_hex(initial.ledger_state.get_ref().clone()),
        oracle["afterInit"]
    );
    let inserted = put(
        initial.into_circuit_context(ContractAddress::default()),
        runtime::Field::from(7_u64),
        runtime::Field::from(9_u64),
    )
    .unwrap();
    assert_eq!(
        state_hex(inserted.context.query.state.get_ref().clone()),
        oracle["afterPut7To9"]
    );
    let replaced = put(
        inserted.context,
        runtime::Field::from(7_u64),
        runtime::Field::from(11_u64),
    )
    .unwrap();
    assert_eq!(
        state_hex(replaced.context.query.state.get_ref().clone()),
        oracle["afterReplace7With11"]
    );
    let distinct = put(
        replaced.context,
        runtime::Field::from(8_u64),
        runtime::Field::from(13_u64),
    )
    .unwrap();
    assert_eq!(
        state_hex(distinct.context.query.state.get_ref().clone()),
        oracle["afterPut8To13"]
    );
    let view = runtime::ledger::map_view::<runtime::Field, runtime::Field, _>(
        distinct.context.query.state.get_ref(),
        0,
    )
    .unwrap();
    assert_eq!(view.size().unwrap().value(), 2);
    assert_eq!(
        view.lookup(runtime::Field::from(7_u64)).unwrap(),
        runtime::Field::from(11_u64)
    );
    assert_eq!(
        view.lookup(runtime::Field::from(8_u64)).unwrap(),
        runtime::Field::from(13_u64)
    );
    let typed = PublicStateView::from(&distinct).m().unwrap();
    assert_eq!(typed.size().unwrap(), view.size().unwrap());
    assert_eq!(
        typed.lookup(runtime::Field::from(7_u64)).unwrap(),
        view.lookup(runtime::Field::from(7_u64)).unwrap(),
    );
}

#[path = "../../support/oracle_recorded_trace.rs"]
mod direct_trace;

#[test]
fn exported_map_recording_matches_complete_typescript_insert_replace_and_distinct_traces() {
    use compact_rust_map_oracle_fixture::ledger_contract::recorded;
    let rows = direct_trace::cases("map_oracle", 3);
    let mut native_context = initial_state(ConstructorContext::new(()))
        .unwrap()
        .into_circuit_context(ContractAddress::default());
    let mut recorded_context = initial_state(ConstructorContext::new(()))
        .unwrap()
        .into_circuit_context(ContractAddress::default());
    for (row, label) in rows.iter().zip(["insert", "replace", "distinct"]) {
        assert_eq!(row["id"], format!("map_oracle/put/{label}"));
        let k = runtime::Field::from(row["args"][0].as_str().unwrap().parse::<u64>().unwrap());
        let v = runtime::Field::from(row["args"][1].as_str().unwrap().parse::<u64>().unwrap());
        let native = put(native_context, k, v).unwrap();
        let recorded = recorded::put(recorded_context, k, v).unwrap();
        direct_trace::assert_trace(&native, &recorded, row, state_hex);
        native_context = native.context;
        recorded_context = recorded.execution.context;
    }
}
