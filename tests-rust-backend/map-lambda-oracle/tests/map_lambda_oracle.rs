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

use compact_rust_map_lambda_oracle_fixture::ledger_contract::{initial_state, ping};
use midnight_compact_runtime::context::ConstructorContext;
use midnight_compact_runtime::ledger::{self, ContractAddress, DefaultDB, StateValue};
use midnight_compact_runtime::{BoundedUint, FixedVector};
use midnight_onchain_state::state::{
    ContractMaintenanceAuthority, ContractOperation, ContractState, EntryPointBuf,
};
use midnight_serialize::tagged_serialize;
use midnight_storage::storage::HashMap;

fn state_hex(state: StateValue<DefaultDB>) -> String {
    let operations = HashMap::new().insert(
        EntryPointBuf(b"ping".to_vec()),
        ContractOperation::new(None),
    );
    let state = ContractState::new(state, operations, ContractMaintenanceAuthority::default());
    let mut bytes = Vec::new();
    tagged_serialize(&state, &mut bytes).unwrap();
    hex::encode(bytes)
}

fn values(state: &StateValue<DefaultDB>) -> Vec<String> {
    let vector =
        ledger::read_root_cell::<FixedVector<BoundedUint<{ u64::MAX as u128 }>, 3>, _>(state, 0)
            .unwrap();
    vector
        .into_array()
        .into_iter()
        .map(|value| value.value().to_string())
        .collect()
}

#[test]
fn vector_map_arithmetic_matches_typescript_state() {
    let reference: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/map-lambda-oracle.json"
    ))
    .unwrap();
    let initial = initial_state(ConstructorContext::new(())).unwrap();
    let state = initial.ledger_state.get_ref().clone();
    assert_eq!(state_hex(state.clone()), reference["afterInit"]["stateHex"]);
    assert_eq!(
        serde_json::to_value(values(&state)).unwrap(),
        reference["afterInit"]["values"]
    );

    let next = ping(initial.into_circuit_context(ContractAddress::default())).unwrap();
    let state = next.context.query.state.get_ref().clone();
    assert_eq!(state_hex(state.clone()), reference["afterPing"]["stateHex"]);
    assert_eq!(
        serde_json::to_value(values(&state)).unwrap(),
        reference["afterPing"]["values"]
    );
}
