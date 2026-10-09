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

//! Borrowed witness context and metered reads through the official ledger VM.

use std::cell::RefCell;

use midnight_onchain_vm::cost_model::CostModel;

use super::{CircuitContext, RunningCost};
use crate::ledger::{CellValue, ContractAddress, DB, DefaultDB, QueryContext, StateValue};
use crate::{CompactError, ledger};

/// The ledger, private state, and address visible to a Compact witness.
pub struct WitnessContext<'a, Private, Ledger = &'a StateValue<DefaultDB>> {
    pub ledger: Ledger,
    pub private_state: &'a Private,
    pub contract_address: &'a ContractAddress,
}

/// Charges ledger reads performed through a generated witness ledger view.
///
/// The witness only borrows the current query state. Each projected Cell read
/// runs the same ledger VM program as a circuit read and adds that query's cost
/// without adding a public proof operation to the witness result. The context's
/// gas limit is applied to each query, matching ledger-8 and TypeScript.
pub struct WitnessReadMeter<'a, D: DB = DefaultDB> {
    pub(super) query: &'a QueryContext<D>,
    pub(super) cost_model: &'a CostModel,
    pub(super) gas_limit: Option<RunningCost>,
    pub(super) observed_gas: RefCell<RunningCost>,
}

impl<'a, D: DB> WitnessReadMeter<'a, D> {
    pub fn new<Private>(context: &'a CircuitContext<Private, D>) -> Self {
        Self {
            query: &context.query,
            cost_model: &context.cost_model,
            gas_limit: context.gas_limit,
            observed_gas: RefCell::new(RunningCost::ZERO),
        }
    }

    pub(crate) fn state(&self) -> &StateValue<D> {
        self.query.state.get_ref()
    }

    pub fn read_cell<T: CellValue>(&self, path: &[u8]) -> Result<T, CompactError> {
        let (result, value) =
            ledger::query_cell_at_path::<T, D>(self.query, path, self.gas_limit, self.cost_model)?;
        *self.observed_gas.borrow_mut() += result.gas_cost;
        Ok(value)
    }

    pub fn read_set_member<T: CellValue>(
        &self,
        path: &[u8],
        value: T,
    ) -> Result<bool, CompactError> {
        let (result, present) =
            ledger::member_set(self.query, path, value, self.gas_limit, self.cost_model)?;
        *self.observed_gas.borrow_mut() += result.gas_cost;
        Ok(present)
    }

    pub fn read_set_size(&self, path: &[u8]) -> Result<u64, CompactError> {
        let (result, size) = ledger::size_set(self.query, path, self.gas_limit, self.cost_model)?;
        *self.observed_gas.borrow_mut() += result.gas_cost;
        Ok(size)
    }

    pub fn read_set_is_empty(&self, path: &[u8]) -> Result<bool, CompactError> {
        let (result, empty) =
            ledger::is_empty_set(self.query, path, self.gas_limit, self.cost_model)?;
        *self.observed_gas.borrow_mut() += result.gas_cost;
        Ok(empty)
    }

    pub fn read_map_member<K: CellValue>(&self, path: &[u8], key: K) -> Result<bool, CompactError> {
        let (result, present) =
            ledger::member_map(self.query, path, key, self.gas_limit, self.cost_model)?;
        *self.observed_gas.borrow_mut() += result.gas_cost;
        Ok(present)
    }

    pub fn read_map_lookup<K: CellValue, V: CellValue>(
        &self,
        path: &[u8],
        key: K,
    ) -> Result<V, CompactError> {
        let (result, value) =
            ledger::lookup_map(self.query, path, key, self.gas_limit, self.cost_model)?;
        *self.observed_gas.borrow_mut() += result.gas_cost;
        Ok(value)
    }

    pub fn read_map_size(&self, path: &[u8]) -> Result<u64, CompactError> {
        let (result, size) = ledger::size_map(self.query, path, self.gas_limit, self.cost_model)?;
        *self.observed_gas.borrow_mut() += result.gas_cost;
        Ok(size)
    }

    pub fn read_map_is_empty(&self, path: &[u8]) -> Result<bool, CompactError> {
        let (result, empty) =
            ledger::is_empty_map(self.query, path, self.gas_limit, self.cost_model)?;
        *self.observed_gas.borrow_mut() += result.gas_cost;
        Ok(empty)
    }

    pub fn read_list_head<T: CellValue + Default>(
        &self,
        path: impl Into<ledger::LedgerPath>,
    ) -> Result<Option<T>, CompactError>
    where
        midnight_base_crypto::fab::Value: From<T>,
    {
        let (result, (present, value)) = ledger::head_list::<T, (bool, T), D>(
            self.query,
            path,
            self.gas_limit,
            self.cost_model,
        )?;
        *self.observed_gas.borrow_mut() += result.gas_cost;
        Ok(present.then_some(value))
    }

    pub fn read_list_is_empty(
        &self,
        path: impl Into<ledger::LedgerPath>,
    ) -> Result<bool, CompactError> {
        let (result, empty) =
            ledger::is_empty_list(self.query, path, self.gas_limit, self.cost_model)?;
        *self.observed_gas.borrow_mut() += result.gas_cost;
        Ok(empty)
    }

    pub fn read_list_length(
        &self,
        path: impl Into<ledger::LedgerPath>,
    ) -> Result<u64, CompactError> {
        let (result, length) =
            ledger::length_list(self.query, path, self.gas_limit, self.cost_model)?;
        *self.observed_gas.borrow_mut() += result.gas_cost;
        Ok(length)
    }

    pub fn read_merkle_is_full(&self, path: &[u8], depth: u8) -> Result<bool, CompactError> {
        let (result, full) =
            ledger::merkle_is_full(self.query, path, depth, self.gas_limit, self.cost_model)?;
        *self.observed_gas.borrow_mut() += result.gas_cost;
        Ok(full)
    }

    pub fn read_merkle_check_root<T: CellValue>(
        &self,
        path: &[u8],
        root: T,
    ) -> Result<bool, CompactError> {
        let (result, known) =
            ledger::merkle_check_root(self.query, path, root, self.gas_limit, self.cost_model)?;
        *self.observed_gas.borrow_mut() += result.gas_cost;
        Ok(known)
    }

    pub fn read_historic_merkle_is_full(
        &self,
        path: &[u8],
        depth: u8,
    ) -> Result<bool, CompactError> {
        let (result, full) =
            ledger::historic_is_full(self.query, path, depth, self.gas_limit, self.cost_model)?;
        *self.observed_gas.borrow_mut() += result.gas_cost;
        Ok(full)
    }

    pub fn read_historic_merkle_check_root<T: CellValue>(
        &self,
        path: &[u8],
        root: T,
    ) -> Result<bool, CompactError> {
        let (result, known) =
            ledger::historic_check_root(self.query, path, root, self.gas_limit, self.cost_model)?;
        *self.observed_gas.borrow_mut() += result.gas_cost;
        Ok(known)
    }

    pub fn gas_cost(&self) -> RunningCost {
        *self.observed_gas.borrow()
    }
}
