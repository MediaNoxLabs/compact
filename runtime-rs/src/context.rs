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
use midnight_base_crypto::hash::HashOutput;
pub use midnight_coin_structure::coin::PublicKey as CoinPublicKey;
use midnight_onchain_vm::cost_model::{CostModel, INITIAL_COST_MODEL};
use midnight_zswap::local::State as ZswapLocalState;
use std::cell::RefCell;

use crate::ledger::{
    CellValue, ChargedState, ContractAddress, ContractState, DB, DefaultDB, QueryContext,
    StateValue,
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
            coin_public_key: None,
            cost_model: INITIAL_COST_MODEL.clone(),
            gas_limit: None,
        }
    }
}

pub struct CircuitContext<Private, D: DB = DefaultDB> {
    pub private_state: Private,
    pub query: QueryContext<D>,
    pub zswap_state: ZswapLocalState<D>,
    coin_public_key: Option<CoinPublicKey>,
    pub cost_model: CostModel,
    /// Ledger-8 VM guard for each individual query, including witness reads.
    /// CircuitResult::gas_cost separately sums the accepted query costs.
    pub gas_limit: Option<RunningCost>,
}

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
    query: &'a QueryContext<D>,
    cost_model: &'a CostModel,
    gas_limit: Option<RunningCost>,
    observed_gas: RefCell<RunningCost>,
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

pub struct CircuitResult<Private, Output, D: DB = DefaultDB> {
    pub context: CircuitContext<Private, D>,
    pub result: Output,
    /// Cost of the operations represented by this result. Generated circuits
    /// add the cost of each VM query rather than retaining only the last one.
    pub gas_cost: RunningCost,
    pub private_transcript_outputs: Vec<AlignedValue>,
}

/// Accumulates native circuit steps while leaving Compact control flow in Rust.
///
/// Each `apply` consumes the current context and adopts exactly one successor
/// result. A nested circuit can be passed to `apply` without special merging
/// code in the generated body. Public Verify operations require a separate
/// `RecordingFrame` when a replayable call is needed.
pub struct CircuitFrame<Private, D: DB = DefaultDB> {
    context: CircuitContext<Private, D>,
    gas_cost: RunningCost,
    private_outputs: Vec<AlignedValue>,
}

impl<Private, D: DB> CircuitFrame<Private, D> {
    pub fn new(context: CircuitContext<Private, D>) -> Self {
        Self {
            context,
            gas_cost: RunningCost::ZERO,
            private_outputs: Vec::new(),
        }
    }

    pub fn context(&self) -> &CircuitContext<Private, D> {
        &self.context
    }

    /// Invoke a witness against the current state and append its private FAB.
    pub fn witness<T, F>(mut self, call: F) -> (Self, T)
    where
        T: Clone,
        AlignedValue: From<T>,
        F: FnOnce(&CircuitContext<Private, D>) -> (Private, T),
    {
        let (next_private, value) = call(&self.context);
        self.context.private_state = next_private;
        self.private_outputs.push(AlignedValue::from(value.clone()));
        (self, value)
    }

    /// Invoke a witness with a ledger view that meters its projected reads.
    pub fn witness_metered<T, F>(mut self, call: F) -> (Self, T)
    where
        T: Clone,
        AlignedValue: From<T>,
        F: FnOnce(&CircuitContext<Private, D>, &WitnessReadMeter<'_, D>) -> (Private, T),
    {
        let meter = WitnessReadMeter::new(&self.context);
        let (next_private, value) = call(&self.context, &meter);
        self.gas_cost += meter.gas_cost();
        self.context.private_state = next_private;
        self.private_outputs.push(AlignedValue::from(value.clone()));
        (self, value)
    }

    /// Invoke a fallible witness and adopt its private effects only on success.
    pub fn try_witness_metered<T, F>(mut self, call: F) -> Result<(Self, T), CompactError>
    where
        T: Clone,
        AlignedValue: From<T>,
        F: FnOnce(
            &CircuitContext<Private, D>,
            &WitnessReadMeter<'_, D>,
        ) -> Result<(Private, T), CompactError>,
    {
        let meter = WitnessReadMeter::new(&self.context);
        let (next_private, value) = call(&self.context, &meter)?;
        self.gas_cost += meter.gas_cost();
        self.context.private_state = next_private;
        self.private_outputs.push(AlignedValue::from(value.clone()));
        Ok((self, value))
    }

    /// Run a native ledger step or complete nested circuit in sequence.
    pub fn apply<T, F>(self, operation: F) -> Result<(Self, T), CompactError>
    where
        F: FnOnce(CircuitContext<Private, D>) -> Result<CircuitResult<Private, T, D>, CompactError>,
    {
        let CircuitFrame {
            context,
            mut gas_cost,
            mut private_outputs,
        } = self;
        let result = operation(context)?;
        gas_cost += result.gas_cost;
        private_outputs.extend(result.private_transcript_outputs);
        Ok((
            Self {
                context: result.context,
                gas_cost,
                private_outputs,
            },
            result.result,
        ))
    }

    pub fn finish<T>(self, result: T) -> CircuitResult<Private, T, D> {
        CircuitResult {
            context: self.context,
            result,
            gas_cost: self.gas_cost,
            private_transcript_outputs: self.private_outputs,
        }
    }
}

impl<Private, D: DB> CircuitContext<Private, D> {
    /// Associate the callers pinned ledger coin key with native witness calls.
    pub fn with_coin_public_key(mut self, key: CoinPublicKey) -> Self {
        self.coin_public_key = Some(key);
        self
    }

    /// Use the 32-byte Compact representation of a ledger coin public key.
    pub fn with_coin_public_key_bytes(self, bytes: [u8; 32]) -> Self {
        self.with_coin_public_key(CoinPublicKey(HashOutput(bytes)))
    }

    /// Native `ownPublicKey()` reads the execution identity, not Zswap state.
    pub fn own_coin_public_key(&self) -> Result<[u8; 32], CompactError> {
        self.coin_public_key
            .map(|key| key.0.0)
            .ok_or(CompactError::MissingCoinPublicKey)
    }

    /// Start a new circuit call from an upstream ledger contract snapshot.
    ///
    /// The caller must associate this state with `address` and establish its
    /// network provenance. This conversion does not attest finality or fetch a
    /// full ledger snapshot.
    pub fn from_contract_state(
        private_state: Private,
        address: ContractAddress,
        contract: &ContractState<D>,
    ) -> Self {
        Self {
            private_state,
            query: QueryContext::new(contract.data.clone(), address),
            zswap_state: ZswapLocalState::default(),
            coin_public_key: None,
            cost_model: INITIAL_COST_MODEL.clone(),
            gas_limit: None,
        }
    }

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
        path: impl Into<ledger::LedgerPath>,
    ) -> Result<CircuitResult<Private, M, D>, CompactError> {
        let (result, value) =
            ledger::head_list::<T, M, D>(&self.query, path, self.gas_limit, &self.cost_model)?;
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
        path: impl Into<ledger::LedgerPath>,
    ) -> Result<CircuitResult<Private, (), D>, CompactError> {
        let result = ledger::pop_front_list(&self.query, path, self.gas_limit, &self.cost_model)
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
        path: impl Into<ledger::LedgerPath>,
    ) -> Result<CircuitResult<Private, (), D>, CompactError> {
        let result = ledger::reset_list(&self.query, path, self.gas_limit, &self.cost_model)
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
        path: impl Into<ledger::LedgerPath>,
        value: T,
    ) -> Result<CircuitResult<Private, (), D>, CompactError> {
        let result =
            ledger::push_front_list(&self.query, path, value, self.gas_limit, &self.cost_model)
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
        path: impl Into<ledger::LedgerPath>,
    ) -> Result<CircuitResult<Private, bool, D>, CompactError> {
        let (result, value) =
            ledger::is_empty_list(&self.query, path, self.gas_limit, &self.cost_model)?;
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
        path: impl Into<ledger::LedgerPath>,
    ) -> Result<CircuitResult<Private, u64, D>, CompactError> {
        let (result, value) =
            ledger::length_list(&self.query, path, self.gas_limit, &self.cost_model)?;
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
            self.gas_limit,
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
        let (result, present) =
            ledger::member_map(&self.query, path, key, self.gas_limit, &self.cost_model)?;
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
        let (result, value) =
            ledger::lookup_map(&self.query, path, key, self.gas_limit, &self.cost_model)?;
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
        let result = ledger::insert_set(&self.query, path, value, self.gas_limit, &self.cost_model)
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
        let (result, member) =
            ledger::member_set(&self.query, path, value, self.gas_limit, &self.cost_model)?;
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
        let result = ledger::remove_set(&self.query, path, value, self.gas_limit, &self.cost_model)
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
        let result = ledger::reset_set(&self.query, path, self.gas_limit, &self.cost_model)
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
        let (result, size) = ledger::size_set(&self.query, path, self.gas_limit, &self.cost_model)?;
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
            ledger::is_empty_set(&self.query, path, self.gas_limit, &self.cost_model)?;
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
            self.gas_limit,
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
            self.gas_limit,
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
            self.gas_limit,
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
        let result =
            ledger::merkle_insert(&self.query, path, item, self.gas_limit, &self.cost_model)
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
        let result =
            ledger::merkle_insert_hash(&self.query, path, hash, self.gas_limit, &self.cost_model)
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
            self.gas_limit,
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
            self.gas_limit,
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
            self.gas_limit,
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
        let result =
            ledger::historic_insert(&self.query, path, item, self.gas_limit, &self.cost_model)
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
        let result =
            ledger::historic_insert_hash(&self.query, path, hash, self.gas_limit, &self.cost_model)
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
        let result =
            ledger::historic_reset_history(&self.query, path, self.gas_limit, &self.cost_model)
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
            self.gas_limit,
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
            self.gas_limit,
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
        let (result, full) =
            ledger::historic_is_full(&self.query, path, depth, self.gas_limit, &self.cost_model)?;
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
        let (result, full) =
            ledger::merkle_is_full(&self.query, path, depth, self.gas_limit, &self.cost_model)?;
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
        let (result, found) =
            ledger::merkle_check_root(&self.query, path, root, self.gas_limit, &self.cost_model)?;
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
        let (result, found) =
            ledger::historic_check_root(&self.query, path, root, self.gas_limit, &self.cost_model)?;
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
            self.gas_limit,
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
        let result =
            ledger::write_cell_at_path(&self.query, path, value, self.gas_limit, &self.cost_model)
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
        let result =
            ledger::increment_counter(&self.query, path, amount, self.gas_limit, &self.cost_model)
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
        let result =
            ledger::decrement_counter(&self.query, path, amount, self.gas_limit, &self.cost_model)
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
