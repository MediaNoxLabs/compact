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

use midnight_compact_runtime::ledger::{
    ChargedState, ContractAddress, DefaultDB, QueryContext, StateValue, constructor_counter,
    increment_counter, read_counter,
};
use midnight_onchain_state::state::{
    ContractMaintenanceAuthority, ContractOperation, ContractState, EntryPointBuf,
};
use midnight_onchain_vm::cost_model::INITIAL_COST_MODEL;
use midnight_serialize::tagged_serialize;
use midnight_storage::storage::HashMap;

#[test]
fn counter_state_bytes_match_the_typescript_oracle() {
    // The reference was produced by the codegen-rust branch's TS counter
    // fixture from the same ledger-8 line.
    let reference: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/counter-ts-state.json")).unwrap();
    let expected = hex::decode(reference["stateHex"].as_str().unwrap()).unwrap();

    let initial = StateValue::Array(vec![constructor_counter::<DefaultDB>()].into());
    let context = QueryContext::new(ChargedState::new(initial), ContractAddress::default());
    let result = increment_counter(&context, 0, 1, None, &INITIAL_COST_MODEL).unwrap();
    let final_state = result.context.state.get_ref().clone();

    let mut operations: HashMap<EntryPointBuf, ContractOperation, DefaultDB> = HashMap::new();
    operations = operations.insert(
        EntryPointBuf(b"increment".to_vec()),
        ContractOperation::new(None),
    );
    let contract_state: ContractState<DefaultDB> = ContractState::new(
        final_state.clone(),
        operations,
        ContractMaintenanceAuthority::default(),
    );
    let mut actual = Vec::new();
    tagged_serialize(&contract_state, &mut actual).unwrap();
    assert_eq!(actual, expected);

    let StateValue::Array(fields) = &final_state else {
        panic!("expected field array")
    };
    assert_eq!(
        read_counter(&fields.get(0).unwrap()).unwrap().to_string(),
        reference["counterValue"].as_str().unwrap()
    );
}
