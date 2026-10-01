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

use compact_rust_multi_pl_call_oracle_fixture::ledger_contract::{initial_state, record_update};
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
        EntryPointBuf(b"record_update".to_vec()),
        ContractOperation::new(None),
    );
    let contract_state =
        ContractState::new(state, operations, ContractMaintenanceAuthority::default());
    let mut bytes = Vec::new();
    tagged_serialize(&contract_state, &mut bytes).unwrap();
    hex::encode(bytes)
}

fn values(state: &StateValue<DefaultDB>) -> (String, String, String) {
    let StateValue::Array(fields) = state else {
        panic!("expected ledger field array")
    };
    let ops = runtime::ledger::read_counter(&fields.get(0).unwrap()).unwrap();
    let ver = runtime::ledger::read_counter(&fields.get(1).unwrap()).unwrap();
    let updated =
        runtime::ledger::read_root_cell::<runtime::BoundedUint<{ u64::MAX as u128 }>, _>(state, 2)
            .unwrap();
    (
        ops.to_string(),
        ver.to_string(),
        updated.value().to_string(),
    )
}

#[test]
fn exact_multi_ledger_call_oracle_preserves_action_order() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/multi-pl-call-oracle.json"
    ))
    .unwrap();
    let initial = initial_state(ConstructorContext::new(())).unwrap();
    assert_eq!(
        state_hex(initial.ledger_state.get_ref().clone()),
        oracle["afterInit"]
    );
    let update7 = record_update(
        initial.into_circuit_context(ContractAddress::default()),
        runtime::BoundedUint::new(7).unwrap(),
    )
    .unwrap();
    assert_eq!(
        state_hex(update7.context.query.state.get_ref().clone()),
        oracle["afterUpdate7"]
    );
    let (ops, ver, updated) = values(update7.context.query.state.get_ref());
    assert_eq!(ops, oracle["fieldsAfterUpdate7"]["ops"]);
    assert_eq!(ver, oracle["fieldsAfterUpdate7"]["ver"]);
    assert_eq!(updated, oracle["fieldsAfterUpdate7"]["updated"]);
    let update13 = record_update(update7.context, runtime::BoundedUint::new(13).unwrap()).unwrap();
    assert_eq!(
        state_hex(update13.context.query.state.get_ref().clone()),
        oracle["afterUpdate13"]
    );
    let (ops, ver, updated) = values(update13.context.query.state.get_ref());
    assert_eq!(ops, oracle["fieldsAfterUpdate13"]["ops"]);
    assert_eq!(ver, oracle["fieldsAfterUpdate13"]["ver"]);
    assert_eq!(updated, oracle["fieldsAfterUpdate13"]["updated"]);
}
