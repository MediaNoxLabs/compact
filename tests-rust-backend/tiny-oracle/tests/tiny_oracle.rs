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

use compact_rust_tiny_oracle_fixture::ledger_contract::{
    LedgerView, Witnesses, clear, get, initial_state, set,
};
use midnight_compact_runtime as runtime;
use midnight_onchain_state::state::{
    ContractMaintenanceAuthority, ContractOperation, ContractState, EntryPointBuf,
};
use midnight_serialize::tagged_serialize;
use midnight_storage::storage::HashMap;
use runtime::context::{ConstructorContext, WitnessContext};
use runtime::ledger::{ContractAddress, DefaultDB, StateValue};

struct FixedWitness;

impl Witnesses<()> for FixedWitness {
    fn private_secret_key(
        &self,
        _context: WitnessContext<'_, (), LedgerView<'_>>,
    ) -> ((), runtime::FixedBytes<32>) {
        ((), runtime::FixedBytes::new([7; 32]))
    }
}

fn state_hex(state: StateValue<DefaultDB>) -> String {
    let mut operations: HashMap<EntryPointBuf, ContractOperation, DefaultDB> = HashMap::new();
    for name in ["get", "set", "clear"] {
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
fn tiny_constructor_clear_set_and_get_match_typescript() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/tiny-oracle.json"
    ))
    .unwrap();
    let witness = FixedWitness;
    let initial = initial_state(
        ConstructorContext::new(()),
        &witness,
        runtime::Field::from(42u64),
    )
    .unwrap();
    assert_eq!(
        state_hex(initial.ledger_state.get_ref().clone()),
        oracle["afterInit"]["stateHex"]
    );
    let context = initial.into_circuit_context(ContractAddress::default());
    let cleared = clear(context, &witness).unwrap();
    assert_eq!(
        state_hex(cleared.context.query.state.get_ref().clone()),
        oracle["afterClear"]["stateHex"]
    );
    let set99 = set(cleared.context, &witness, runtime::Field::from(99u64)).unwrap();
    assert_eq!(
        state_hex(set99.context.query.state.get_ref().clone()),
        oracle["afterSet99"]["stateHex"]
    );
    let got = get(set99.context).unwrap();
    assert_eq!(got.result.is_some, oracle["getResult"]["isSome"]);
    assert_eq!(got.result.value, runtime::Field::from(99u64));
    assert_eq!(
        state_hex(got.context.query.state.get_ref().clone()),
        oracle["afterSet99"]["stateHex"]
    );
}
