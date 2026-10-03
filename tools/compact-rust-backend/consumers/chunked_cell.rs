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

use compact_contract_chunked_cell::ledger_contract::{Contract, initial_state};
use compact_contract_chunked_cell::ledger_slots::{active, amount};
use compact_contract_chunked_cell::runtime::Field;
use compact_contract_chunked_cell::runtime::context::ConstructorContext;
use compact_contract_chunked_cell::runtime::ledger::{ContractAddress, ContractState, DefaultDB};
use compact_contract_chunked_cell::runtime::transaction::{
    Observation, ObservedCallError, ObservedContractState, decode_verifier_key,
};

#[test]
fn chunked_cell_calls_need_only_the_generated_crate() {
    assert_eq!(active.path(), &[1, 14]);
    assert_eq!(amount.path(), &[1, 13]);
    let initial = initial_state(ConstructorContext::new(())).unwrap();
    let mut state = ContractState::<DefaultDB>::default();
    state.data = initial.ledger_state;
    let observed = ObservedContractState::new(
        ContractAddress::default(), state,
        Observation { transaction_hash: [1; 32], block_hash: [2; 32], block_height: 3 },
    );
    let contract = Contract::default();
    let root = std::env::var("COMPACT_RUST_CHUNKED_CELL_PROOF").unwrap();
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
    missing!("set_active", contract.recording.set_active_call(&observed, (), false).unwrap());
    missing!("get_active", contract.recording.get_active_call(&observed, ()).unwrap());
    missing!("assert_active", contract.recording.assert_active_call(&observed, (), true).unwrap());
    missing!("set_amount", contract.recording.set_amount_call(&observed, (), Field::from(11_u64)).unwrap());
    missing!("get_amount", contract.recording.get_amount_call(&observed, ()).unwrap());
    missing!("add_amount", contract.recording.add_amount_call(&observed, (), Field::from(7_u64)).unwrap());
    missing!("active_equals", contract.recording.active_equals_call(&observed, (), true).unwrap());
    missing!("plus_amount", contract.recording.plus_amount_call(&observed, (), Field::from(7_u64)).unwrap());
    missing!("subtract_amount", contract.recording.subtract_amount_call(&observed, (), Field::from(2_u64)).unwrap());
    missing!("multiply_amount", contract.recording.multiply_amount_call(&observed, (), Field::from(7_u64)).unwrap());
}
