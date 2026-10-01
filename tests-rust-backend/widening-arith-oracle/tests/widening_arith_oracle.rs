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

use compact_rust_widening_arith_oracle_fixture::ledger_contract::{initial_state, recordArea};
use compact_rust_widening_arith_oracle_fixture::pure_circuits::{
    ageThresholdDays, areaOf, productBytes, sumBytes,
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
    let operations: HashMap<EntryPointBuf, ContractOperation, DefaultDB> = HashMap::new().insert(
        EntryPointBuf(b"recordArea".to_vec()),
        ContractOperation::new(None),
    );
    let contract_state =
        ContractState::new(state, operations, ContractMaintenanceAuthority::default());
    let mut bytes = Vec::new();
    tagged_serialize(&contract_state, &mut bytes).unwrap();
    hex::encode(bytes)
}

fn count(state: &StateValue<DefaultDB>) -> String {
    let StateValue::Array(fields) = state else {
        panic!("expected ledger field array")
    };
    runtime::ledger::read_counter(&fields.get(0).unwrap())
        .unwrap()
        .to_string()
}

#[test]
fn exact_widening_arithmetic_oracle_matches_typescript_boundaries() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/widening-arith-oracle.json"
    ))
    .unwrap();
    let byte = runtime::BoundedUint::<255>::new(255).unwrap();
    let word = runtime::BoundedUint::<65535>::new(65535).unwrap();
    assert_eq!(
        sumBytes(byte, byte).unwrap().value().to_string(),
        oracle["sumBytes"]
    );
    assert_eq!(
        ageThresholdDays(byte).unwrap().value().to_string(),
        oracle["ageThresholdDays"]
    );
    assert_eq!(
        productBytes(byte, byte).unwrap().value().to_string(),
        oracle["productBytes"]
    );
    assert_eq!(
        areaOf(word, word).unwrap().value().to_string(),
        oracle["areaOf"]
    );

    let initial = initial_state(ConstructorContext::new(())).unwrap();
    assert_eq!(
        state_hex(initial.ledger_state.get_ref().clone()),
        oracle["afterInit"]
    );
    let max = recordArea(
        initial.into_circuit_context(ContractAddress::default()),
        word,
        word,
    )
    .unwrap();
    assert_eq!(
        state_hex(max.context.query.state.get_ref().clone()),
        oracle["afterRecordMax"]
    );
    assert_eq!(
        count(max.context.query.state.get_ref()),
        oracle["countAfterRecordMax"]
    );
    let small = recordArea(
        max.context,
        runtime::BoundedUint::new(5).unwrap(),
        runtime::BoundedUint::new(7).unwrap(),
    )
    .unwrap();
    assert_eq!(
        state_hex(small.context.query.state.get_ref().clone()),
        oracle["afterRecordSmall"]
    );
    assert_eq!(
        count(small.context.query.state.get_ref()),
        oracle["countAfterRecordSmall"]
    );
}
