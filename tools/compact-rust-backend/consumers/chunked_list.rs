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

use compact_contract_chunked_list::ledger_contract::{Contract, initial_state};
use compact_contract_chunked_list::ledger_slots::items;
use compact_contract_chunked_list::runtime::Field;
use compact_contract_chunked_list::runtime::context::ConstructorContext;
use compact_contract_chunked_list::runtime::ledger::{ContractAddress, ContractState, DefaultDB};
use compact_contract_chunked_list::runtime::transaction::{
    Observation, ObservedCallError, ObservedContractState, decode_verifier_key,
};

#[test]
fn chunked_list_calls_need_only_the_generated_crate() {
    assert_eq!(items.path(), &[1, 14]);
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
    let root = std::env::var("COMPACT_RUST_CHUNKED_LIST_PROOF").unwrap();
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
    missing!("item_count", contract.recording.item_count_call(&observed, ()).unwrap());
    missing!("items_empty", contract.recording.items_empty_call(&observed, ()).unwrap());
    missing!("first_item", contract.recording.first_item_call(&observed, ()).unwrap());
    missing!("prepend", contract.recording.prepend_call(&observed, (), Field::from(42_u64)).unwrap());
    missing!("drop_first", contract.recording.drop_first_call(&observed, ()).unwrap());
    missing!("clear_items", contract.recording.clear_items_call(&observed, ()).unwrap());
}
