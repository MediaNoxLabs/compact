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

use compact_rust_test_center_bboard_fixture::ledger_contract as contract;
use midnight_compact_runtime as runtime;
use runtime::FixedBytes;
use runtime::context::{CircuitContext, ConstructorContext, WitnessContext};
use runtime::ledger::{ContractAddress, ContractState, DefaultDB, StateValue};
use std::cell::RefCell;

pub struct Witness {
    pub key: u8,
    pub calls: RefCell<Vec<u64>>,
}
impl Default for Witness {
    fn default() -> Self {
        Self {
            key: 7,
            calls: RefCell::default(),
        }
    }
}
impl contract::Witnesses<u64> for Witness {
    fn local_secret_key(
        &self,
        context: WitnessContext<'_, u64, contract::LedgerView<'_>>,
    ) -> (u64, FixedBytes<32>) {
        self.calls.borrow_mut().push(*context.private_state);
        (*context.private_state + 1, FixedBytes::new([self.key; 32]))
    }
}
pub fn initial() -> CircuitContext<u64> {
    contract::initial_state(ConstructorContext::new(5))
        .unwrap()
        .into_circuit_context(ContractAddress::default())
}
pub fn state(value: StateValue<DefaultDB>) -> ContractState<DefaultDB> {
    use midnight_onchain_state::state::{
        ContractMaintenanceAuthority, ContractOperation, EntryPointBuf,
    };
    let operations = midnight_storage::storage::HashMap::new()
        .insert(
            EntryPointBuf(b"post".to_vec()),
            ContractOperation::new(None),
        )
        .insert(
            EntryPointBuf(b"take_down".to_vec()),
            ContractOperation::new(None),
        );
    ContractState::new(value, operations, ContractMaintenanceAuthority::default())
}
pub fn rehydrate(context: CircuitContext<u64>) -> CircuitContext<u64> {
    CircuitContext::from_contract_state(
        context.private_state,
        ContractAddress::default(),
        &state(context.query.state.get_ref().clone()),
    )
}
