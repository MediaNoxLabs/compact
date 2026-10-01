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

use compact_rust_bug11_oracle_fixture::ledger_contract::{
    initial_state, set_medium, set_tiny, set_wide,
};
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
    for name in ["set_tiny", "set_medium", "set_wide"] {
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
fn exact_bug11_oracle_matches_non_power_of_two_uint_cell_widths() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/bug11-oracle.json"
    ))
    .unwrap();
    let initial = initial_state(ConstructorContext::new(())).unwrap();
    assert_eq!(
        state_hex(initial.ledger_state.get_ref().clone()),
        oracle["afterInit"]
    );
    let tiny = set_tiny(
        initial.into_circuit_context(ContractAddress::default()),
        runtime::BoundedUint::<99>::new(99).unwrap(),
    )
    .unwrap();
    assert_eq!(
        state_hex(tiny.context.query.state.get_ref().clone()),
        oracle["afterTiny99"]
    );
    let medium = set_medium(
        tiny.context,
        runtime::BoundedUint::<69999>::new(69999).unwrap(),
    )
    .unwrap();
    assert_eq!(
        state_hex(medium.context.query.state.get_ref().clone()),
        oracle["afterMedium69999"]
    );
    let wide = set_wide(
        medium.context,
        runtime::BoundedUint::<4999999999>::new(4999999999).unwrap(),
    )
    .unwrap();
    let state = wide.context.query.state.get_ref();
    assert_eq!(state_hex(state.clone()), oracle["afterWide4999999999"]);
    let tiny = runtime::ledger::read_root_cell::<runtime::BoundedUint<99>, _>(state, 0).unwrap();
    let medium =
        runtime::ledger::read_root_cell::<runtime::BoundedUint<69999>, _>(state, 1).unwrap();
    let wide =
        runtime::ledger::read_root_cell::<runtime::BoundedUint<4999999999>, _>(state, 2).unwrap();
    assert_eq!(tiny.value().to_string(), oracle["values"]["tiny"]);
    assert_eq!(medium.value().to_string(), oracle["values"]["medium"]);
    assert_eq!(wide.value().to_string(), oracle["values"]["wide"]);
    assert!(runtime::BoundedUint::<99>::new(100).is_err());
    assert!(runtime::BoundedUint::<69999>::new(70000).is_err());
    assert!(runtime::BoundedUint::<4999999999>::new(5000000000).is_err());
}
