// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//   http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use compact_contract_chunked_map::ledger_contract::{Contract, initial_state};
use compact_contract_chunked_map::ledger_slots::table;
use compact_contract_chunked_map::runtime::Field;
use compact_contract_chunked_map::runtime::context::ConstructorContext;
use compact_contract_chunked_map::runtime::ledger::{ContractAddress, ContractState, DefaultDB};
use compact_contract_chunked_map::runtime::transaction::{
    Observation, ObservedCallError, ObservedContractState, decode_verifier_key,
};

#[test]
fn chunked_map_calls_need_only_the_generated_crate() {
    assert_eq!(table.path(), &[1, 14]);
    let initial = initial_state(ConstructorContext::new(())).unwrap();
    let mut state = ContractState::<DefaultDB>::default();
    state.data = initial.ledger_state;
    let observed = ObservedContractState::new(
        ContractAddress::default(),
        state,
        Observation {
            transaction_hash: [1; 32],
            block_hash: [2; 32],
            block_height: 3,
        },
    );
    let contract = Contract::default();
    let root = std::env::var("COMPACT_RUST_CHUNKED_MAP_PROOF").unwrap();
    macro_rules! missing {
        ($name:literal, $call:expr) => {{
            let verifier = decode_verifier_key(
                &std::fs::read(format!("{root}/keys/{}.verifier", $name)).unwrap(),
            ).unwrap();
            assert!(matches!(
                $call.prepare(verifier, Field::from(0_u64)),
                Err(ObservedCallError::MissingOperation(actual)) if actual == $name
            ));
        }};
    }
    missing!("put", contract.recording.put_call(&observed, (), false, Field::from(7_u64)).unwrap());
    missing!("put_pair", contract.recording.put_pair_call(&observed, (), true, false, Field::from(42_u64)).unwrap());
    missing!("put_default", contract.recording.put_default_call(&observed, (), false).unwrap());
    missing!("has", contract.recording.has_call(&observed, (), true).unwrap());
    missing!("get", contract.recording.get_call(&observed, (), true).unwrap());
    missing!("remove_key", contract.recording.remove_key_call(&observed, (), true).unwrap());
    missing!("table_size", contract.recording.table_size_call(&observed, ()).unwrap());
    missing!("table_is_empty", contract.recording.table_is_empty_call(&observed, ()).unwrap());
    missing!("reset_table", contract.recording.reset_table_call(&observed, ()).unwrap());
}
