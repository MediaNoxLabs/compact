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

use compact_rust_wide_field_literal_fixture::ledger_contract::{initial_state, read_large};
use compact_rust_wide_field_literal_fixture::pure_circuits::constant;
use midnight_compact_runtime::Field;
use midnight_compact_runtime::context::ConstructorContext;
use midnight_compact_runtime::ledger::{ContractAddress, DefaultDB, StateValue};
use midnight_onchain_state::state::{
    ContractMaintenanceAuthority, ContractOperation, ContractState, EntryPointBuf,
};
use midnight_serialize::tagged_serialize;
use midnight_storage::storage::HashMap;
use num_bigint::BigUint;

fn state_hex(state: StateValue<DefaultDB>) -> String {
    let operations: HashMap<EntryPointBuf, ContractOperation, DefaultDB> = HashMap::new();
    let operations = operations.insert(
        EntryPointBuf(b"read_large".to_vec()),
        ContractOperation::new(None),
    );
    let contract_state =
        ContractState::new(state, operations, ContractMaintenanceAuthority::default());
    let mut bytes = Vec::new();
    tagged_serialize(&contract_state, &mut bytes).unwrap();
    hex::encode(bytes)
}

#[test]
fn wide_field_literal_matches_typescript_state_and_pure_result() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/wide-field-literal.json"
    ))
    .unwrap();
    let constructor = initial_state(ConstructorContext::new(())).unwrap();
    assert_eq!(
        state_hex(constructor.ledger_state.get_ref().clone()),
        oracle["initialHex"]
    );
    let expected_decimal =
        "819310549611346726241370945440405716213240158234039660170669895299022906775";
    let expected = Field::from_le_bytes(
        &BigUint::parse_bytes(expected_decimal.as_bytes(), 10)
            .unwrap()
            .to_bytes_le(),
    )
    .unwrap();
    let read = read_large(constructor.into_circuit_context(ContractAddress::default())).unwrap();
    assert_eq!(read.result, expected);
    assert_eq!(constant().unwrap(), expected);
    assert_eq!(oracle["constant"], oracle["read"]);
    assert_eq!(oracle["read"], expected_decimal);
}
