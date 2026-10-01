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

use compact_rust_chunked_ledger_oracle_fixture::ledger_contract::initial_state;
use compact_rust_chunked_ledger_oracle_fixture::pure_circuits::ping;
use midnight_compact_runtime::BoundedUint;
use midnight_compact_runtime::context::ConstructorContext;
use midnight_compact_runtime::ledger::{self, ContractAddress, DefaultDB, StateValue};
use midnight_onchain_state::state::{ContractMaintenanceAuthority, ContractState};
use midnight_serialize::tagged_serialize;
use midnight_storage::storage::HashMap;

fn state_hex(state: StateValue<DefaultDB>) -> String {
    let operations: HashMap<_, _, DefaultDB> = HashMap::new();
    let contract_state =
        ContractState::new(state, operations, ContractMaintenanceAuthority::default());
    let mut bytes = Vec::new();
    tagged_serialize(&contract_state, &mut bytes).unwrap();
    hex::encode(bytes)
}

#[test]
fn nested_state_matches_typescript_bytes_and_field_values() {
    let reference: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/chunked-ledger-oracle.json"
    ))
    .unwrap();
    let initial = initial_state(ConstructorContext::new(())).unwrap();
    let state = initial.ledger_state.get_ref().clone();
    assert_eq!(state_hex(state.clone()), reference["stateHex"]);
    for index in 0..17 {
        let path = if index < 3 {
            [0, index as u8]
        } else {
            [1, (index - 3) as u8]
        };
        let value =
            ledger::read_cell_at_path::<BoundedUint<{ u64::MAX as u128 }>, _>(&state, &path)
                .unwrap();
        assert_eq!(
            value.value().to_string(),
            reference["values"][index].as_str().unwrap()
        );
    }
    assert_eq!(
        ledger::read_cell_at_path::<bool, _>(&state, &[1, 14]).unwrap(),
        reference["active"].as_bool().unwrap()
    );
    assert_eq!(ping(true).unwrap(), reference["ping"].as_bool().unwrap());
}

#[test]
fn nested_cell_paths_support_vm_reads_and_writes() {
    let context = initial_state(ConstructorContext::new(()))
        .unwrap()
        .into_circuit_context(ContractAddress::default());
    let read = context
        .read_cell_at_path::<BoundedUint<{ u64::MAX as u128 }>>(&[1, 13])
        .unwrap();
    assert_eq!(read.result.value(), 42);
    let next = read
        .context
        .write_cell_at_path(
            &[1, 13],
            BoundedUint::<{ u64::MAX as u128 }>::new(99).unwrap(),
        )
        .unwrap();
    let state = next.context.query.state.get_ref();
    assert_eq!(
        ledger::read_cell_at_path::<BoundedUint<{ u64::MAX as u128 }>, _>(state, &[1, 13])
            .unwrap()
            .value(),
        99
    );
    assert_eq!(
        ledger::read_cell_at_path::<BoundedUint<{ u64::MAX as u128 }>, _>(state, &[0, 0])
            .unwrap()
            .value(),
        7
    );
}
