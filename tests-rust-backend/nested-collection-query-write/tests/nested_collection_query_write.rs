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

use compact_rust_nested_collection_query_write_fixture::ledger_contract::{
    check_map_empty, check_member, check_set_empty, initial_state, map_empty_flag, member_flag,
    seed, set_empty_flag,
};
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
    for name in [
        "seed",
        "check_member",
        "check_set_empty",
        "check_map_empty",
        "member_flag",
        "set_empty_flag",
        "map_empty_flag",
    ] {
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
fn nested_queries_write_booleans_with_typescript_parity() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/nested-collection-query-write.json"
    ))
    .unwrap();
    let constructor = initial_state(ConstructorContext::new(())).unwrap();
    let mut context = constructor.into_circuit_context(ContractAddress::default());
    context = check_member(context, Field::from(42_u64)).unwrap().context;
    context = check_set_empty(context).unwrap().context;
    context = check_map_empty(context).unwrap().context;
    assert_eq!(
        state_hex(context.query.state.get_ref().clone()),
        oracle["before"]["stateHex"]
    );
    let member = member_flag(context).unwrap();
    assert_eq!(member.result, oracle["before"]["flags"]["member"]);
    let set_empty = set_empty_flag(member.context).unwrap();
    assert_eq!(set_empty.result, oracle["before"]["flags"]["setEmpty"]);
    let map_empty = map_empty_flag(set_empty.context).unwrap();
    assert_eq!(map_empty.result, oracle["before"]["flags"]["mapEmpty"]);
    context = seed(map_empty.context, Field::from(42_u64))
        .unwrap()
        .context;
    context = check_member(context, Field::from(42_u64)).unwrap().context;
    context = check_set_empty(context).unwrap().context;
    context = check_map_empty(context).unwrap().context;
    assert_eq!(
        state_hex(context.query.state.get_ref().clone()),
        oracle["after"]["stateHex"]
    );
    let member = member_flag(context).unwrap();
    assert_eq!(member.result, oracle["after"]["flags"]["member"]);
    let set_empty = set_empty_flag(member.context).unwrap();
    assert_eq!(set_empty.result, oracle["after"]["flags"]["setEmpty"]);
    let map_empty = map_empty_flag(set_empty.context).unwrap();
    assert_eq!(map_empty.result, oracle["after"]["flags"]["mapEmpty"]);
}

#[test]
fn nested_query_cost_includes_read_and_write() {
    let generated_context = initial_state(ConstructorContext::new(()))
        .unwrap()
        .into_circuit_context(ContractAddress::default());
    let generated = check_member(generated_context, Field::from(42_u64)).unwrap();

    let manual_context = initial_state(ConstructorContext::new(()))
        .unwrap()
        .into_circuit_context(ContractAddress::default());
    let query = manual_context.member_set(3, Field::from(42_u64)).unwrap();
    let write = query.context.write_cell(0, query.result).unwrap();
    assert_eq!(generated.gas_cost, query.gas_cost + write.gas_cost);
    assert_eq!(
        generated.context.query.state.get_ref(),
        write.context.query.state.get_ref(),
    );
}
