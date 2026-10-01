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

//! Compact-level envelopes around ledger query and shielded state.
//!
//! The state, query, cost, and Zswap fields are upstream ledger types. This
//! module only ties them to a contract's private state and records ownership
//! transitions between constructor and circuit calls.

pub use midnight_base_crypto::cost_model::RunningCost;
use midnight_base_crypto::fab::AlignedValue;
use midnight_onchain_vm::cost_model::{CostModel, INITIAL_COST_MODEL};
use midnight_zswap::local::State as ZswapLocalState;

use crate::ledger::{
    CellValue, ChargedState, ContractAddress, DB, DefaultDB, QueryContext, StateValue,
};
use crate::{CompactError, ledger};

pub struct ConstructorContext<Private, D: DB = DefaultDB> {
    pub private_state: Private,
    pub zswap_state: ZswapLocalState<D>,
}

impl<Private, D: DB> ConstructorContext<Private, D> {
    pub fn new(private_state: Private) -> Self {
        Self {
            private_state,
            zswap_state: ZswapLocalState::default(),
        }
    }
}

pub struct ConstructorResult<Private, D: DB = DefaultDB> {
    pub ledger_state: ChargedState<D>,
    pub private_state: Private,
    pub zswap_state: ZswapLocalState<D>,
}

impl<Private, D: DB> ConstructorResult<Private, D> {
    pub fn new(context: ConstructorContext<Private, D>, ledger_state: ChargedState<D>) -> Self {
        Self {
            ledger_state,
            private_state: context.private_state,
            zswap_state: context.zswap_state,
        }
    }

    pub fn into_circuit_context(self, address: ContractAddress) -> CircuitContext<Private, D> {
        CircuitContext {
            private_state: self.private_state,
            query: QueryContext::new(self.ledger_state, address),
            zswap_state: self.zswap_state,
            cost_model: INITIAL_COST_MODEL.clone(),
            gas_limit: None,
        }
    }
}

pub struct CircuitContext<Private, D: DB = DefaultDB> {
    pub private_state: Private,
    pub query: QueryContext<D>,
    pub zswap_state: ZswapLocalState<D>,
    pub cost_model: CostModel,
    pub gas_limit: Option<RunningCost>,
}

/// The ledger, private state, and address visible to a Compact witness.
pub struct WitnessContext<'a, Private, Ledger = &'a StateValue<DefaultDB>> {
    pub ledger: Ledger,
    pub private_state: &'a Private,
    pub contract_address: &'a ContractAddress,
}

pub struct CircuitResult<Private, Output, D: DB = DefaultDB> {
    pub context: CircuitContext<Private, D>,
    pub result: Output,
    pub gas_cost: RunningCost,
    pub private_transcript_outputs: Vec<AlignedValue>,
}

impl<Private, D: DB> CircuitContext<Private, D> {
    pub fn into_constructor_result(self) -> ConstructorResult<Private, D> {
        ConstructorResult {
            ledger_state: self.query.state,
            private_state: self.private_state,
            zswap_state: self.zswap_state,
        }
    }

    pub fn witness_context(&self) -> WitnessContext<'_, Private, &StateValue<D>> {
        WitnessContext {
            ledger: self.query.state.get_ref(),
            private_state: &self.private_state,
            contract_address: &self.query.address,
        }
    }

    pub fn witness_context_with<'a, Ledger>(
        &'a self,
        ledger: Ledger,
    ) -> WitnessContext<'a, Private, Ledger> {
        WitnessContext {
            ledger,
            private_state: &self.private_state,
            contract_address: &self.query.address,
        }
    }

    pub fn head_list<T: CellValue + Default, M: CellValue>(
        mut self,
        field_index: u8,
    ) -> Result<CircuitResult<Private, M, D>, CompactError> {
        let (result, value) = ledger::head_list::<T, M, D>(
            &self.query,
            field_index,
            self.gas_limit.clone(),
            &self.cost_model,
        )?;
        self.query = result.context;
        Ok(CircuitResult {
            context: self,
            result: value,
            gas_cost: result.gas_cost,
            private_transcript_outputs: Vec::new(),
        })
    }

    pub fn pop_front_list(
        mut self,
        field_index: u8,
    ) -> Result<CircuitResult<Private, (), D>, CompactError> {
        let result = ledger::pop_front_list(
            &self.query,
            field_index,
            self.gas_limit.clone(),
            &self.cost_model,
        )
        .map_err(|error| CompactError::LedgerQueryRejected(format!("{error:?}")))?;
        self.query = result.context;
        Ok(CircuitResult {
            context: self,
            result: (),
            gas_cost: result.gas_cost,
            private_transcript_outputs: Vec::new(),
        })
    }

    pub fn reset_list(
        mut self,
        field_index: u8,
    ) -> Result<CircuitResult<Private, (), D>, CompactError> {
        let result = ledger::reset_list(
            &self.query,
            field_index,
            self.gas_limit.clone(),
            &self.cost_model,
        )
        .map_err(|error| CompactError::LedgerQueryRejected(format!("{error:?}")))?;
        self.query = result.context;
        Ok(CircuitResult {
            context: self,
            result: (),
            gas_cost: result.gas_cost,
            private_transcript_outputs: Vec::new(),
        })
    }

    pub fn push_front_list<T: CellValue>(
        mut self,
        field_index: u8,
        value: T,
    ) -> Result<CircuitResult<Private, (), D>, CompactError> {
        let result = ledger::push_front_list(
            &self.query,
            field_index,
            value,
            self.gas_limit.clone(),
            &self.cost_model,
        )
        .map_err(|error| CompactError::LedgerQueryRejected(format!("{error:?}")))?;
        self.query = result.context;
        Ok(CircuitResult {
            context: self,
            result: (),
            gas_cost: result.gas_cost,
            private_transcript_outputs: Vec::new(),
        })
    }

    pub fn is_empty_list(
        mut self,
        field_index: u8,
    ) -> Result<CircuitResult<Private, bool, D>, CompactError> {
        let (result, value) = ledger::is_empty_list(
            &self.query,
            field_index,
            self.gas_limit.clone(),
            &self.cost_model,
        )?;
        self.query = result.context;
        Ok(CircuitResult {
            context: self,
            result: value,
            gas_cost: result.gas_cost,
            private_transcript_outputs: Vec::new(),
        })
    }

    pub fn length_list(
        mut self,
        field_index: u8,
    ) -> Result<CircuitResult<Private, u64, D>, CompactError> {
        let (result, value) = ledger::length_list(
            &self.query,
            field_index,
            self.gas_limit.clone(),
            &self.cost_model,
        )?;
        self.query = result.context;
        Ok(CircuitResult {
            context: self,
            result: value,
            gas_cost: result.gas_cost,
            private_transcript_outputs: Vec::new(),
        })
    }

    pub fn insert_map<K: CellValue, V: CellValue>(
        mut self,
        path: impl Into<ledger::LedgerPath>,
        key: K,
        value: V,
    ) -> Result<CircuitResult<Private, (), D>, CompactError> {
        let result = ledger::insert_map(
            &self.query,
            path,
            key,
            value,
            self.gas_limit.clone(),
            &self.cost_model,
        )
        .map_err(|error| CompactError::LedgerQueryRejected(format!("{error:?}")))?;
        self.query = result.context;
        Ok(CircuitResult {
            context: self,
            result: (),
            gas_cost: result.gas_cost,
            private_transcript_outputs: Vec::new(),
        })
    }

    pub fn member_map<K: CellValue>(
        mut self,
        path: impl Into<ledger::LedgerPath>,
        key: K,
    ) -> Result<CircuitResult<Private, bool, D>, CompactError> {
        let (result, present) = ledger::member_map(
            &self.query,
            path,
            key,
            self.gas_limit.clone(),
            &self.cost_model,
        )?;
        self.query = result.context;
        Ok(CircuitResult {
            context: self,
            result: present,
            gas_cost: result.gas_cost,
            private_transcript_outputs: Vec::new(),
        })
    }

    pub fn lookup_map<K: CellValue, V: CellValue>(
        mut self,
        path: impl Into<ledger::LedgerPath>,
        key: K,
    ) -> Result<CircuitResult<Private, V, D>, CompactError> {
        let (result, value) = ledger::lookup_map(
            &self.query,
            path,
            key,
            self.gas_limit.clone(),
            &self.cost_model,
        )?;
        self.query = result.context;
        Ok(CircuitResult {
            context: self,
            result: value,
            gas_cost: result.gas_cost,
            private_transcript_outputs: Vec::new(),
        })
    }

    pub fn remove_map<K: CellValue>(
        self,
        path: impl Into<ledger::LedgerPath>,
        key: K,
    ) -> Result<CircuitResult<Private, (), D>, CompactError> {
        self.remove_set(path, key)
    }

    pub fn size_map(
        self,
        path: impl Into<ledger::LedgerPath>,
    ) -> Result<CircuitResult<Private, u64, D>, CompactError> {
        self.size_set(path)
    }

    pub fn is_empty_map(
        self,
        path: impl Into<ledger::LedgerPath>,
    ) -> Result<CircuitResult<Private, bool, D>, CompactError> {
        self.is_empty_set(path)
    }

    pub fn reset_map(
        self,
        path: impl Into<ledger::LedgerPath>,
    ) -> Result<CircuitResult<Private, (), D>, CompactError> {
        self.reset_set(path)
    }
    pub fn insert_set<T: CellValue>(
        mut self,
        path: impl Into<ledger::LedgerPath>,
        value: T,
    ) -> Result<CircuitResult<Private, (), D>, CompactError> {
        let result = ledger::insert_set(
            &self.query,
            path,
            value,
            self.gas_limit.clone(),
            &self.cost_model,
        )
        .map_err(|error| CompactError::LedgerQueryRejected(format!("{error:?}")))?;
        self.query = result.context;
        Ok(CircuitResult {
            context: self,
            result: (),
            gas_cost: result.gas_cost,
            private_transcript_outputs: Vec::new(),
        })
    }

    pub fn member_set<T: CellValue>(
        mut self,
        path: impl Into<ledger::LedgerPath>,
        value: T,
    ) -> Result<CircuitResult<Private, bool, D>, CompactError> {
        let (result, member) = ledger::member_set(
            &self.query,
            path,
            value,
            self.gas_limit.clone(),
            &self.cost_model,
        )?;
        self.query = result.context;
        Ok(CircuitResult {
            context: self,
            result: member,
            gas_cost: result.gas_cost,
            private_transcript_outputs: Vec::new(),
        })
    }

    pub fn remove_set<T: CellValue>(
        mut self,
        path: impl Into<ledger::LedgerPath>,
        value: T,
    ) -> Result<CircuitResult<Private, (), D>, CompactError> {
        let result = ledger::remove_set(
            &self.query,
            path,
            value,
            self.gas_limit.clone(),
            &self.cost_model,
        )
        .map_err(|error| CompactError::LedgerQueryRejected(format!("{error:?}")))?;
        self.query = result.context;
        Ok(CircuitResult {
            context: self,
            result: (),
            gas_cost: result.gas_cost,
            private_transcript_outputs: Vec::new(),
        })
    }

    pub fn reset_set(
        mut self,
        path: impl Into<ledger::LedgerPath>,
    ) -> Result<CircuitResult<Private, (), D>, CompactError> {
        let result = ledger::reset_set(&self.query, path, self.gas_limit.clone(), &self.cost_model)
            .map_err(|error| CompactError::LedgerQueryRejected(format!("{error:?}")))?;
        self.query = result.context;
        Ok(CircuitResult {
            context: self,
            result: (),
            gas_cost: result.gas_cost,
            private_transcript_outputs: Vec::new(),
        })
    }

    pub fn size_set(
        mut self,
        path: impl Into<ledger::LedgerPath>,
    ) -> Result<CircuitResult<Private, u64, D>, CompactError> {
        let (result, size) =
            ledger::size_set(&self.query, path, self.gas_limit.clone(), &self.cost_model)?;
        self.query = result.context;
        Ok(CircuitResult {
            context: self,
            result: size,
            gas_cost: result.gas_cost,
            private_transcript_outputs: Vec::new(),
        })
    }

    pub fn is_empty_set(
        mut self,
        path: impl Into<ledger::LedgerPath>,
    ) -> Result<CircuitResult<Private, bool, D>, CompactError> {
        let (result, empty) =
            ledger::is_empty_set(&self.query, path, self.gas_limit.clone(), &self.cost_model)?;
        self.query = result.context;
        Ok(CircuitResult {
            context: self,
            result: empty,
            gas_cost: result.gas_cost,
            private_transcript_outputs: Vec::new(),
        })
    }

    pub fn merkle_insert_index_default<T: CellValue + Default>(
        mut self,
        path: impl Into<ledger::LedgerPath>,
        position: crate::BoundedUint<{ u64::MAX as u128 }>,
    ) -> Result<CircuitResult<Private, (), D>, CompactError> {
        let result = ledger::merkle_insert_index_default::<T, D>(
            &self.query,
            path,
            position.value() as u64,
            self.gas_limit.clone(),
            &self.cost_model,
        )
        .map_err(|error| CompactError::LedgerQueryRejected(format!("{error:?}")))?;
        self.query = result.context;
        Ok(CircuitResult {
            context: self,
            result: (),
            gas_cost: result.gas_cost,
            private_transcript_outputs: Vec::new(),
        })
    }

    pub fn merkle_insert_index<T: CellValue>(
        mut self,
        path: impl Into<ledger::LedgerPath>,
        item: T,
        position: crate::BoundedUint<{ u64::MAX as u128 }>,
    ) -> Result<CircuitResult<Private, (), D>, CompactError> {
        let result = ledger::merkle_insert_index(
            &self.query,
            path,
            item,
            position.value() as u64,
            self.gas_limit.clone(),
            &self.cost_model,
        )
        .map_err(|error| CompactError::LedgerQueryRejected(format!("{error:?}")))?;
        self.query = result.context;
        Ok(CircuitResult {
            context: self,
            result: (),
            gas_cost: result.gas_cost,
            private_transcript_outputs: Vec::new(),
        })
    }

    pub fn merkle_insert_hash_index(
        mut self,
        path: impl Into<ledger::LedgerPath>,
        hash: crate::FixedBytes<32>,
        position: crate::BoundedUint<{ u64::MAX as u128 }>,
    ) -> Result<CircuitResult<Private, (), D>, CompactError> {
        let result = ledger::merkle_insert_hash_index(
            &self.query,
            path,
            hash,
            position.value() as u64,
            self.gas_limit.clone(),
            &self.cost_model,
        )
        .map_err(|error| CompactError::LedgerQueryRejected(format!("{error:?}")))?;
        self.query = result.context;
        Ok(CircuitResult {
            context: self,
            result: (),
            gas_cost: result.gas_cost,
            private_transcript_outputs: Vec::new(),
        })
    }

    pub fn merkle_insert<T: CellValue>(
        mut self,
        path: impl Into<ledger::LedgerPath>,
        item: T,
    ) -> Result<CircuitResult<Private, (), D>, CompactError> {
        let result = ledger::merkle_insert(
            &self.query,
            path,
            item,
            self.gas_limit.clone(),
            &self.cost_model,
        )
        .map_err(|error| CompactError::LedgerQueryRejected(format!("{error:?}")))?;
        self.query = result.context;
        Ok(CircuitResult {
            context: self,
            result: (),
            gas_cost: result.gas_cost,
            private_transcript_outputs: Vec::new(),
        })
    }

    pub fn merkle_insert_hash(
        mut self,
        path: impl Into<ledger::LedgerPath>,
        hash: crate::FixedBytes<32>,
    ) -> Result<CircuitResult<Private, (), D>, CompactError> {
        let result = ledger::merkle_insert_hash(
            &self.query,
            path,
            hash,
            self.gas_limit.clone(),
            &self.cost_model,
        )
        .map_err(|error| CompactError::LedgerQueryRejected(format!("{error:?}")))?;
        self.query = result.context;
        Ok(CircuitResult {
            context: self,
            result: (),
            gas_cost: result.gas_cost,
            private_transcript_outputs: Vec::new(),
        })
    }

    pub fn historic_insert_index_default<T: CellValue + Default>(
        mut self,
        path: impl Into<ledger::LedgerPath>,
        position: crate::BoundedUint<{ u64::MAX as u128 }>,
    ) -> Result<CircuitResult<Private, (), D>, CompactError> {
        let result = ledger::historic_insert_index_default::<T, D>(
            &self.query,
            path,
            position.value() as u64,
            self.gas_limit.clone(),
            &self.cost_model,
        )
        .map_err(|error| CompactError::LedgerQueryRejected(format!("{error:?}")))?;
        self.query = result.context;
        Ok(CircuitResult {
            context: self,
            result: (),
            gas_cost: result.gas_cost,
            private_transcript_outputs: Vec::new(),
        })
    }

    pub fn historic_insert_index<T: CellValue>(
        mut self,
        path: impl Into<ledger::LedgerPath>,
        item: T,
        position: crate::BoundedUint<{ u64::MAX as u128 }>,
    ) -> Result<CircuitResult<Private, (), D>, CompactError> {
        let result = ledger::historic_insert_index(
            &self.query,
            path,
            item,
            position.value() as u64,
            self.gas_limit.clone(),
            &self.cost_model,
        )
        .map_err(|error| CompactError::LedgerQueryRejected(format!("{error:?}")))?;
        self.query = result.context;
        Ok(CircuitResult {
            context: self,
            result: (),
            gas_cost: result.gas_cost,
            private_transcript_outputs: Vec::new(),
        })
    }

    pub fn historic_insert_hash_index(
        mut self,
        path: impl Into<ledger::LedgerPath>,
        hash: crate::FixedBytes<32>,
        position: crate::BoundedUint<{ u64::MAX as u128 }>,
    ) -> Result<CircuitResult<Private, (), D>, CompactError> {
        let result = ledger::historic_insert_hash_index(
            &self.query,
            path,
            hash,
            position.value() as u64,
            self.gas_limit.clone(),
            &self.cost_model,
        )
        .map_err(|error| CompactError::LedgerQueryRejected(format!("{error:?}")))?;
        self.query = result.context;
        Ok(CircuitResult {
            context: self,
            result: (),
            gas_cost: result.gas_cost,
            private_transcript_outputs: Vec::new(),
        })
    }

    pub fn historic_insert<T: CellValue>(
        mut self,
        path: impl Into<ledger::LedgerPath>,
        item: T,
    ) -> Result<CircuitResult<Private, (), D>, CompactError> {
        let result = ledger::historic_insert(
            &self.query,
            path,
            item,
            self.gas_limit.clone(),
            &self.cost_model,
        )
        .map_err(|error| CompactError::LedgerQueryRejected(format!("{error:?}")))?;
        self.query = result.context;
        Ok(CircuitResult {
            context: self,
            result: (),
            gas_cost: result.gas_cost,
            private_transcript_outputs: Vec::new(),
        })
    }

    pub fn historic_insert_hash(
        mut self,
        path: impl Into<ledger::LedgerPath>,
        hash: crate::FixedBytes<32>,
    ) -> Result<CircuitResult<Private, (), D>, CompactError> {
        let result = ledger::historic_insert_hash(
            &self.query,
            path,
            hash,
            self.gas_limit.clone(),
            &self.cost_model,
        )
        .map_err(|error| CompactError::LedgerQueryRejected(format!("{error:?}")))?;
        self.query = result.context;
        Ok(CircuitResult {
            context: self,
            result: (),
            gas_cost: result.gas_cost,
            private_transcript_outputs: Vec::new(),
        })
    }

    pub fn historic_reset_history(
        mut self,
        path: impl Into<ledger::LedgerPath>,
    ) -> Result<CircuitResult<Private, (), D>, CompactError> {
        let result = ledger::historic_reset_history(
            &self.query,
            path,
            self.gas_limit.clone(),
            &self.cost_model,
        )
        .map_err(|error| CompactError::LedgerQueryRejected(format!("{error:?}")))?;
        self.query = result.context;
        Ok(CircuitResult {
            context: self,
            result: (),
            gas_cost: result.gas_cost,
            private_transcript_outputs: Vec::new(),
        })
    }

    pub fn historic_reset_to_default(
        mut self,
        path: impl Into<ledger::LedgerPath>,
        depth: u8,
    ) -> Result<CircuitResult<Private, (), D>, CompactError> {
        let result = ledger::historic_reset_to_default(
            &self.query,
            path,
            depth,
            self.gas_limit.clone(),
            &self.cost_model,
        )
        .map_err(|error| CompactError::LedgerQueryRejected(format!("{error:?}")))?;
        self.query = result.context;
        Ok(CircuitResult {
            context: self,
            result: (),
            gas_cost: result.gas_cost,
            private_transcript_outputs: Vec::new(),
        })
    }

    pub fn merkle_reset_to_default(
        mut self,
        path: impl Into<ledger::LedgerPath>,
        depth: u8,
    ) -> Result<CircuitResult<Private, (), D>, CompactError> {
        let result = ledger::merkle_reset_to_default(
            &self.query,
            path,
            depth,
            self.gas_limit.clone(),
            &self.cost_model,
        )
        .map_err(|error| CompactError::LedgerQueryRejected(format!("{error:?}")))?;
        self.query = result.context;
        Ok(CircuitResult {
            context: self,
            result: (),
            gas_cost: result.gas_cost,
            private_transcript_outputs: Vec::new(),
        })
    }

    pub fn historic_is_full(
        mut self,
        path: impl Into<ledger::LedgerPath>,
        depth: u8,
    ) -> Result<CircuitResult<Private, bool, D>, CompactError> {
        let (result, full) = ledger::historic_is_full(
            &self.query,
            path,
            depth,
            self.gas_limit.clone(),
            &self.cost_model,
        )?;
        self.query = result.context;
        Ok(CircuitResult {
            context: self,
            result: full,
            gas_cost: result.gas_cost,
            private_transcript_outputs: Vec::new(),
        })
    }

    pub fn merkle_is_full(
        mut self,
        path: impl Into<ledger::LedgerPath>,
        depth: u8,
    ) -> Result<CircuitResult<Private, bool, D>, CompactError> {
        let (result, full) = ledger::merkle_is_full(
            &self.query,
            path,
            depth,
            self.gas_limit.clone(),
            &self.cost_model,
        )?;
        self.query = result.context;
        Ok(CircuitResult {
            context: self,
            result: full,
            gas_cost: result.gas_cost,
            private_transcript_outputs: Vec::new(),
        })
    }

    pub fn merkle_check_root<T: CellValue>(
        mut self,
        path: impl Into<ledger::LedgerPath>,
        root: T,
    ) -> Result<CircuitResult<Private, bool, D>, CompactError> {
        let (result, found) = ledger::merkle_check_root(
            &self.query,
            path,
            root,
            self.gas_limit.clone(),
            &self.cost_model,
        )?;
        self.query = result.context;
        Ok(CircuitResult {
            context: self,
            result: found,
            gas_cost: result.gas_cost,
            private_transcript_outputs: Vec::new(),
        })
    }

    pub fn historic_check_root<T: CellValue>(
        mut self,
        path: impl Into<ledger::LedgerPath>,
        root: T,
    ) -> Result<CircuitResult<Private, bool, D>, CompactError> {
        let (result, found) = ledger::historic_check_root(
            &self.query,
            path,
            root,
            self.gas_limit.clone(),
            &self.cost_model,
        )?;
        self.query = result.context;
        Ok(CircuitResult {
            context: self,
            result: found,
            gas_cost: result.gas_cost,
            private_transcript_outputs: Vec::new(),
        })
    }

    pub fn read_cell<T: CellValue>(
        self,
        path: impl Into<ledger::LedgerPath>,
    ) -> Result<CircuitResult<Private, T, D>, CompactError> {
        self.read_cell_at_path(path.into().as_slice())
    }

    pub fn read_cell_at_path<T: CellValue>(
        mut self,
        path: &[u8],
    ) -> Result<CircuitResult<Private, T, D>, CompactError> {
        let (result, value) = ledger::query_cell_at_path::<T, D>(
            &self.query,
            path,
            self.gas_limit.clone(),
            &self.cost_model,
        )?;
        self.query = result.context;
        Ok(CircuitResult {
            context: self,
            result: value,
            gas_cost: result.gas_cost,
            private_transcript_outputs: Vec::new(),
        })
    }

    pub fn write_cell<T: CellValue>(
        self,
        path: impl Into<ledger::LedgerPath>,
        value: T,
    ) -> Result<CircuitResult<Private, (), D>, CompactError> {
        self.write_cell_at_path(path.into().as_slice(), value)
    }

    pub fn write_cell_at_path<T: CellValue>(
        mut self,
        path: &[u8],
        value: T,
    ) -> Result<CircuitResult<Private, (), D>, CompactError> {
        let result = ledger::write_cell_at_path(
            &self.query,
            path,
            value,
            self.gas_limit.clone(),
            &self.cost_model,
        )
        .map_err(|error| CompactError::LedgerQueryRejected(format!("{error:?}")))?;
        self.query = result.context;
        Ok(CircuitResult {
            context: self,
            result: (),
            gas_cost: result.gas_cost,
            private_transcript_outputs: Vec::new(),
        })
    }

    pub fn increment_counter(
        mut self,
        path: impl Into<ledger::LedgerPath>,
        amount: u16,
    ) -> Result<CircuitResult<Private, (), D>, CompactError> {
        let result = ledger::increment_counter(
            &self.query,
            path,
            amount,
            self.gas_limit.clone(),
            &self.cost_model,
        )
        .map_err(|error| CompactError::LedgerQueryRejected(format!("{error:?}")))?;
        self.query = result.context;
        Ok(CircuitResult {
            context: self,
            result: (),
            gas_cost: result.gas_cost,
            private_transcript_outputs: Vec::new(),
        })
    }

    pub fn decrement_counter(
        mut self,
        path: impl Into<ledger::LedgerPath>,
        amount: u16,
    ) -> Result<CircuitResult<Private, (), D>, CompactError> {
        let result = ledger::decrement_counter(
            &self.query,
            path,
            amount,
            self.gas_limit.clone(),
            &self.cost_model,
        )
        .map_err(|error| CompactError::LedgerQueryRejected(format!("{error:?}")))?;
        self.query = result.context;
        Ok(CircuitResult {
            context: self,
            result: (),
            gas_cost: result.gas_cost,
            private_transcript_outputs: Vec::new(),
        })
    }
}
