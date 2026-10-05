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

//! An opt-in path from native circuit execution to a replayable ledger program.
//!
//! Cell, Counter, Set, Map, List, and plain/historic Merkle append and fullness reads are supported so far. Generated contracts must
//! not claim a transaction-ready trace until every operation they use records
//! its corresponding verifying VM instruction.

mod kernel;
mod zswap;

use midnight_base_crypto::cost_model::RunningCost;
use midnight_base_crypto::fab::AlignedValue;
use midnight_onchain_vm::ops::Op;
use midnight_onchain_vm::result_mode::{GatherEvent, ResultModeVerify};

use crate::CompactError;
use crate::context::{CircuitContext, CircuitResult, WitnessReadMeter};
use crate::ledger::{self, CellValue, DB, DefaultDB, LedgerPath, QueryContext};

/// The starting ledger context and the ordered VM program for one circuit.
pub struct PublicTrace<D: DB = DefaultDB> {
    initial_coin_public_key: Option<[u8; 32]>,
    final_coin_public_key: Option<[u8; 32]>,
    initial_intents: crate::CircuitZswapPlan,
    final_intents: crate::CircuitZswapPlan,
    initial: QueryContext<D>,
    verify_ops: Vec<Op<ResultModeVerify, D>>,
}

impl<D: DB> PublicTrace<D> {
    /// Caller-selected execution identity at recording entry, not wallet authorization.
    pub fn initial_coin_public_key(&self) -> Option<[u8; 32]> {
        self.initial_coin_public_key
    }

    /// Sealed execution identity at recording completion.
    pub fn final_coin_public_key(&self) -> Option<[u8; 32]> {
        self.final_coin_public_key
    }

    pub fn initial_intents(&self) -> &crate::CircuitZswapPlan {
        &self.initial_intents
    }
    pub fn final_intents(&self) -> &crate::CircuitZswapPlan {
        &self.final_intents
    }
    pub fn initial(&self) -> &QueryContext<D> {
        &self.initial
    }

    pub fn verify_ops(&self) -> &[Op<ResultModeVerify, D>] {
        &self.verify_ops
    }

    /// Transfer ownership to a ledger transaction builder.
    pub fn into_parts(self) -> (QueryContext<D>, Vec<Op<ResultModeVerify, D>>) {
        (self.initial, self.verify_ops)
    }
}

/// Native result paired with an independently replayable public trace.
pub struct RecordedCircuitResult<Private, Output, D: DB = DefaultDB> {
    pub execution: CircuitResult<Private, Output, D>,
    pub public: PublicTrace<D>,
}

/// Records the VM instructions while executing a small supported circuit.
pub struct RecordingFrame<Private, D: DB = DefaultDB> {
    context: CircuitContext<Private, D>,
    initial_coin_public_key: Option<[u8; 32]>,
    initial_intents: crate::CircuitZswapPlan,
    initial: QueryContext<D>,
    verify_ops: Vec<Op<ResultModeVerify, D>>,
    private_outputs: Vec<AlignedValue>,
    observed_gas: RunningCost,
}

impl<Private, D: DB> RecordingFrame<Private, D> {
    pub fn new(context: CircuitContext<Private, D>) -> Self {
        let initial = context.query.clone();
        let initial_intents = context.circuit_zswap().clone();
        let initial_coin_public_key = context.own_coin_public_key().ok();
        Self {
            context,
            initial_coin_public_key,
            initial_intents,
            initial,
            verify_ops: Vec::new(),
            private_outputs: Vec::new(),
            observed_gas: RunningCost::ZERO,
        }
    }

    pub fn context(&self) -> &CircuitContext<Private, D> {
        &self.context
    }

    /// Read the configured execution coin key and record the native witness output.
    /// This contributes neither a public query nor gas or private-state changes.
    pub fn own_coin_public_key(mut self) -> Result<(Self, [u8; 32]), CompactError> {
        let key = self.context.own_coin_public_key()?;
        self.private_outputs.push(AlignedValue::from(key));
        Ok((self, key))
    }

    /// Observe kernel.self() and retain the exact VM address read.
    pub fn kernel_self(mut self) -> Result<(Self, ledger::ContractAddress), CompactError> {
        let result = ledger::query_kernel_self(
            &self.context.query,
            self.context.gas_limit,
            &self.context.cost_model,
        )?;
        let Some(GatherEvent::Read(observed)) = result.events.last() else {
            return Err(CompactError::InvalidLedgerCell(
                "missing kernel.self read event".into(),
            ));
        };
        let program = ledger::kernel_self_program::<ResultModeVerify, D>(observed.clone());
        self.context.query = result.context;
        self.observed_gas += result.gas_cost;
        self.verify_ops.extend(program);
        let address = self.context.query.address;
        Ok((self, address))
    }

    /// Invoke a witness against the current context and record its FAB result.
    /// The closure may project a generated ledger view from the context.
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

    /// Invoke a witness and charge ledger reads made through its projection.
    pub fn witness_metered<T, F>(mut self, call: F) -> (Self, T)
    where
        T: Clone,
        AlignedValue: From<T>,
        F: FnOnce(&CircuitContext<Private, D>, &WitnessReadMeter<'_, D>) -> (Private, T),
    {
        let meter = WitnessReadMeter::new(&self.context);
        let (next_private, value) = call(&self.context, &meter);
        self.observed_gas += meter.gas_cost();
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
        self.observed_gas += meter.gas_cost();
        self.context.private_state = next_private;
        self.private_outputs.push(AlignedValue::from(value.clone()));
        Ok((self, value))
    }

    /// Adopt a generated helper that has been statically audited as local.
    ///
    /// The helper may change private state, emit private witness values, and
    /// incur metered witness-read gas. It cannot add a public VM operation or
    /// modify the ledger, Zswap, call context, or execution policy. A runtime
    /// check enforces that boundary before its effects enter the recording.
    pub fn call_local<Output, F>(mut self, call: F) -> Result<(Self, Output), CompactError>
    where
        F: FnOnce(
            CircuitContext<Private, D>,
        ) -> Result<CircuitResult<Private, Output, D>, CompactError>,
    {
        let prior_query = self.context.query.clone();
        let prior_zswap = self.context.zswap_state.clone();
        let prior_circuit_zswap = self.context.circuit_zswap.clone();
        let prior_coin_key = self.context.own_coin_public_key().ok();
        let prior_cost_model = self.context.cost_model.clone();
        let prior_gas_limit = self.context.gas_limit;
        let result = call(self.context)?;
        let next = &result.context;
        let before_call = &prior_query.call_context;
        let after_call = &next.query.call_context;
        if prior_query.state != next.query.state
            || prior_query.effects != next.query.effects
            || prior_query.address != next.query.address
            || before_call.own_address != after_call.own_address
            || before_call.tblock != after_call.tblock
            || before_call.tblock_err != after_call.tblock_err
            || before_call.parent_block_hash != after_call.parent_block_hash
            || before_call.caller != after_call.caller
            || before_call.balance != after_call.balance
            || before_call.com_indices != after_call.com_indices
            || before_call.last_block_time != after_call.last_block_time
            || prior_circuit_zswap != next.circuit_zswap
            || prior_zswap.coins != next.zswap_state.coins
            || prior_zswap.pending_spends != next.zswap_state.pending_spends
            || prior_zswap.pending_outputs != next.zswap_state.pending_outputs
            || prior_zswap.merkle_tree != next.zswap_state.merkle_tree
            || prior_zswap.first_free != next.zswap_state.first_free
            || prior_coin_key != next.own_coin_public_key().ok()
            || prior_cost_model != next.cost_model
            || prior_gas_limit != next.gas_limit
        {
            return Err(CompactError::InvalidLedgerCell(
                "local helper changed public or Zswap execution context".into(),
            ));
        }
        self.context = result.context;
        self.observed_gas += result.gas_cost;
        self.private_outputs
            .extend(result.private_transcript_outputs);
        Ok((self, result.result))
    }

    pub fn write_cell<T: CellValue>(
        self,
        path: impl Into<LedgerPath>,
        value: T,
    ) -> Result<Self, CompactError> {
        let path = path.into();
        let program = ledger::cell_write_program(path.as_slice(), value);
        self.apply_verify_program(program)
    }

    /// Replace a qualified coin Cell using the actual offer-allocated index.
    pub fn write_qualified_coin_cell<T: CellValue>(
        self,
        path: impl Into<LedgerPath>,
        coin: ledger::CoinInfo,
        recipient: ledger::CoinRecipient,
    ) -> Result<Self, CompactError> {
        let path = path.into();
        let program = ledger::qualified_coin_cell_write_program_for_context::<T, D>(
            &self.context.query,
            path.as_slice(),
            coin,
            recipient,
        )?;
        self.apply_verify_program(program)
    }

    pub fn increment_counter(
        self,
        path: impl Into<LedgerPath>,
        amount: u16,
    ) -> Result<Self, CompactError> {
        self.update_counter(path, amount, false)
    }

    pub fn decrement_counter(
        self,
        path: impl Into<LedgerPath>,
        amount: u16,
    ) -> Result<Self, CompactError> {
        self.update_counter(path, amount, true)
    }

    fn update_counter(
        self,
        path: impl Into<LedgerPath>,
        amount: u16,
        subtract: bool,
    ) -> Result<Self, CompactError> {
        let path = path.into();
        let program = ledger::counter_program(path.as_slice(), amount, subtract);
        self.apply_verify_program(program)
    }

    fn apply_verify_program(
        mut self,
        program: Vec<Op<ResultModeVerify, D>>,
    ) -> Result<Self, CompactError> {
        let result = self
            .context
            .query
            .query(&program, self.context.gas_limit, &self.context.cost_model)
            .map_err(|error| CompactError::LedgerQueryRejected(format!("{error:?}")))?;
        self.context.query = result.context;
        self.observed_gas += result.gas_cost;
        self.verify_ops.extend(program);
        Ok(self)
    }

    pub fn insert_set<T: CellValue>(
        self,
        path: impl Into<LedgerPath>,
        value: T,
    ) -> Result<Self, CompactError> {
        let path = path.into();
        self.apply_verify_program(ledger::set_insert_program(path.as_slice(), value))
    }

    /// Insert a qualified coin using its allocated transaction commitment index.
    pub fn insert_qualified_coin_set<T: CellValue>(
        self,
        path: impl Into<LedgerPath>,
        coin: ledger::CoinInfo,
        recipient: ledger::CoinRecipient,
    ) -> Result<Self, CompactError> {
        let path = path.into();
        let program = ledger::qualified_coin_set_insert_program::<T, D>(
            &self.context.query,
            path.as_slice(),
            coin,
            recipient,
        )?;
        self.apply_verify_program(program)
    }

    /// Append a typed leaf to a plain Merkle tree and retain its verifying VM program.
    pub fn insert_merkle<T: CellValue>(
        self,
        path: impl Into<LedgerPath>,
        value: T,
    ) -> Result<Self, CompactError> {
        self.apply_verify_program(ledger::merkle_insert_program(path, value))
    }

    /// Append an already hashed leaf to a plain Merkle tree and retain its VM program.
    pub fn insert_merkle_hash(
        self,
        path: impl Into<LedgerPath>,
        hash: crate::FixedBytes<32>,
    ) -> Result<Self, CompactError> {
        self.apply_verify_program(ledger::merkle_insert_hash_program(path, hash))
    }

    /// Reset a plain tree with the canonical metered ledger program.
    pub fn reset_merkle_to_default(
        self,
        path: impl Into<LedgerPath>,
        depth: u8,
    ) -> Result<Self, CompactError> {
        self.apply_verify_program(ledger::merkle_reset_program(path, depth))
    }

    /// Append to a historic Merkle tree and retain its root-history VM update.
    pub fn insert_historic_merkle<T: CellValue>(
        self,
        path: impl Into<LedgerPath>,
        value: T,
    ) -> Result<Self, CompactError> {
        self.apply_verify_program(ledger::historic_merkle_insert_program(path, value))
    }

    /// Append an already hashed leaf and retain the historic root-map update.
    pub fn insert_historic_merkle_hash(
        self,
        path: impl Into<LedgerPath>,
        hash: crate::FixedBytes<32>,
    ) -> Result<Self, CompactError> {
        self.apply_verify_program(ledger::historic_merkle_insert_hash_program(path, hash))
    }

    /// Keep the current historic root and discard older root history.
    pub fn reset_historic_merkle_history(
        self,
        path: impl Into<LedgerPath>,
    ) -> Result<Self, CompactError> {
        self.apply_verify_program(ledger::historic_reset_history_program(path.into()))
    }

    /// Replace a historic tree and seed the blank root in its new history.
    pub fn reset_historic_merkle_to_default(
        self,
        path: impl Into<LedgerPath>,
        depth: u8,
    ) -> Result<Self, CompactError> {
        self.apply_verify_program(ledger::historic_reset_to_default_program(path, depth))
    }

    /// Insert a typed leaf at a declared index, retaining the ledger-8 VM program.
    pub fn insert_merkle_index<T: CellValue>(
        self,
        path: impl Into<LedgerPath>,
        value: T,
        position: crate::BoundedUint<{ u64::MAX as u128 }>,
    ) -> Result<Self, CompactError> {
        self.apply_verify_program(ledger::merkle_insert_index_program(
            path,
            value,
            position.value() as u64,
        ))
    }

    /// Place an already hashed leaf at a typed plain-tree index and record its VM program.
    pub fn insert_merkle_hash_index(
        self,
        path: impl Into<LedgerPath>,
        hash: crate::FixedBytes<32>,
        position: crate::BoundedUint<{ u64::MAX as u128 }>,
    ) -> Result<Self, CompactError> {
        self.apply_verify_program(ledger::merkle_insert_hash_index_program(
            path,
            hash,
            position.value() as u64,
        ))
    }

    pub fn insert_merkle_index_default<T: CellValue + Default>(
        self,
        path: impl Into<LedgerPath>,
        position: crate::BoundedUint<{ u64::MAX as u128 }>,
    ) -> Result<Self, CompactError> {
        self.apply_verify_program(ledger::merkle_insert_index_default_program::<T, D>(
            path,
            position.value() as u64,
        ))
    }

    pub fn insert_historic_merkle_index<T: CellValue>(
        self,
        path: impl Into<LedgerPath>,
        value: T,
        position: crate::BoundedUint<{ u64::MAX as u128 }>,
    ) -> Result<Self, CompactError> {
        self.apply_verify_program(ledger::historic_merkle_insert_index_program(
            path,
            value,
            position.value() as u64,
        ))
    }

    /// Place an already hashed leaf at a historic index and retain root history.
    pub fn insert_historic_merkle_hash_index(
        self,
        path: impl Into<LedgerPath>,
        hash: crate::FixedBytes<32>,
        position: crate::BoundedUint<{ u64::MAX as u128 }>,
    ) -> Result<Self, CompactError> {
        self.apply_verify_program(ledger::historic_merkle_insert_hash_index_program(
            path,
            hash,
            position.value() as u64,
        ))
    }

    pub fn insert_historic_merkle_index_default<T: CellValue + Default>(
        self,
        path: impl Into<LedgerPath>,
        position: crate::BoundedUint<{ u64::MAX as u128 }>,
    ) -> Result<Self, CompactError> {
        self.apply_verify_program(
            ledger::historic_merkle_insert_index_default_program::<T, D>(
                path,
                position.value() as u64,
            ),
        )
    }

    /// Read plain-tree fullness through the ledger VM and retain the observed
    /// read value in the verifying program.
    pub fn merkle_is_full(
        self,
        path: impl Into<LedgerPath>,
        depth: u8,
    ) -> Result<(Self, bool), CompactError> {
        self.read_merkle_is_full(path.into(), depth, false)
    }

    /// Historic trees use the same ledger fullness VM program.
    pub fn historic_merkle_is_full(
        self,
        path: impl Into<LedgerPath>,
        depth: u8,
    ) -> Result<(Self, bool), CompactError> {
        self.read_merkle_is_full(path.into(), depth, true)
    }

    fn read_merkle_is_full(
        mut self,
        path: LedgerPath,
        depth: u8,
        historic: bool,
    ) -> Result<(Self, bool), CompactError> {
        let (result, full) = if historic {
            ledger::historic_is_full(
                &self.context.query,
                path.as_slice(),
                depth,
                self.context.gas_limit,
                &self.context.cost_model,
            )?
        } else {
            ledger::merkle_is_full(
                &self.context.query,
                path.as_slice(),
                depth,
                self.context.gas_limit,
                &self.context.cost_model,
            )?
        };
        let Some(GatherEvent::Read(observed)) = result.events.last() else {
            return Err(CompactError::InvalidLedgerCell(
                "missing Merkle fullness event".into(),
            ));
        };
        let program =
            ledger::is_full_program::<ResultModeVerify, D>(path, depth, observed.clone())?;
        self.context.query = result.context;
        self.observed_gas += result.gas_cost;
        self.verify_ops.extend(program);
        Ok((self, full))
    }

    /// Compare a typed digest with the current Merkle root and retain its
    /// observed Boolean in the ledger verification program.
    pub fn merkle_check_root<T: CellValue + Clone>(
        mut self,
        path: impl Into<LedgerPath>,
        root: T,
    ) -> Result<(Self, bool), CompactError> {
        let path = path.into();
        let (result, known) = ledger::merkle_check_root(
            &self.context.query,
            path.as_slice(),
            root.clone(),
            self.context.gas_limit,
            &self.context.cost_model,
        )?;
        let Some(GatherEvent::Read(observed)) = result.events.last() else {
            return Err(CompactError::InvalidLedgerCell(
                "missing Merkle root comparison event".into(),
            ));
        };
        let program = ledger::merkle_check_root_verify_program(path, root, observed.clone());
        self.context.query = result.context;
        self.observed_gas += result.gas_cost;
        self.verify_ops.extend(program);
        Ok((self, known))
    }

    /// Check the historic root map and retain its observed Boolean Verify operation.
    pub fn historic_merkle_check_root<T: CellValue + Clone>(
        mut self,
        path: impl Into<LedgerPath>,
        root: T,
    ) -> Result<(Self, bool), CompactError> {
        let path = path.into();
        let (result, known) = ledger::historic_check_root(
            &self.context.query,
            path.as_slice(),
            root.clone(),
            self.context.gas_limit,
            &self.context.cost_model,
        )?;
        let Some(GatherEvent::Read(observed)) = result.events.last() else {
            return Err(CompactError::InvalidLedgerCell(
                "missing historic Merkle root membership event".into(),
            ));
        };
        let program = ledger::historic_check_root_verify_program(path, root, observed.clone());
        self.context.query = result.context;
        self.observed_gas += result.gas_cost;
        self.verify_ops.extend(program);
        Ok((self, known))
    }

    pub fn remove_set<T: CellValue>(
        self,
        path: impl Into<LedgerPath>,
        value: T,
    ) -> Result<Self, CompactError> {
        let path = path.into();
        self.apply_verify_program(ledger::set_remove_program(path.as_slice(), value))
    }

    pub fn reset_set(self, path: impl Into<LedgerPath>) -> Result<Self, CompactError> {
        let path = path.into();
        self.apply_verify_program(ledger::set_reset_program(path.as_slice()))
    }

    pub fn member_set<T: CellValue + Clone>(
        mut self,
        path: impl Into<LedgerPath>,
        value: T,
    ) -> Result<(Self, bool), CompactError> {
        let path = path.into();
        let (result, member) = ledger::member_set(
            &self.context.query,
            path.as_slice(),
            value.clone(),
            self.context.gas_limit,
            &self.context.cost_model,
        )?;
        let Some(GatherEvent::Read(observed)) = result.events.last() else {
            return Err(CompactError::InvalidLedgerCell(
                "missing Set member event".into(),
            ));
        };
        let program = ledger::set_member_program(path.as_slice(), value, observed.clone());
        self.context.query = result.context;
        self.observed_gas += result.gas_cost;
        self.verify_ops.extend(program);
        Ok((self, member))
    }

    pub fn size_set(mut self, path: impl Into<LedgerPath>) -> Result<(Self, u64), CompactError> {
        let path = path.into();
        let (result, size) = ledger::size_set(
            &self.context.query,
            path.as_slice(),
            self.context.gas_limit,
            &self.context.cost_model,
        )?;
        let Some(GatherEvent::Read(observed)) = result.events.last() else {
            return Err(CompactError::InvalidLedgerCell(
                "missing Set size event".into(),
            ));
        };
        let program = ledger::set_size_program(path.as_slice(), observed.clone());
        self.context.query = result.context;
        self.observed_gas += result.gas_cost;
        self.verify_ops.extend(program);
        Ok((self, size))
    }

    pub fn is_empty_set(
        mut self,
        path: impl Into<LedgerPath>,
    ) -> Result<(Self, bool), CompactError> {
        let path = path.into();
        let (result, empty) = ledger::is_empty_set(
            &self.context.query,
            path.as_slice(),
            self.context.gas_limit,
            &self.context.cost_model,
        )?;
        let Some(GatherEvent::Read(observed)) = result.events.last() else {
            return Err(CompactError::InvalidLedgerCell(
                "missing Set emptiness event".into(),
            ));
        };
        let program = ledger::set_is_empty_program(path.as_slice(), observed.clone());
        self.context.query = result.context;
        self.observed_gas += result.gas_cost;
        self.verify_ops.extend(program);
        Ok((self, empty))
    }

    pub fn insert_map<K: CellValue, V: CellValue>(
        self,
        path: impl Into<LedgerPath>,
        key: K,
        value: V,
    ) -> Result<Self, CompactError> {
        let path = path.into();
        self.apply_verify_program(ledger::map_insert_program(path.as_slice(), key, value))
    }

    pub fn remove_map<K: CellValue>(
        self,
        path: impl Into<LedgerPath>,
        key: K,
    ) -> Result<Self, CompactError> {
        self.remove_set(path, key)
    }

    pub fn reset_map(self, path: impl Into<LedgerPath>) -> Result<Self, CompactError> {
        self.reset_set(path)
    }

    pub fn member_map<K: CellValue + Clone>(
        self,
        path: impl Into<LedgerPath>,
        key: K,
    ) -> Result<(Self, bool), CompactError> {
        self.member_set(path, key)
    }

    pub fn lookup_map<K: CellValue + Clone, V: CellValue>(
        mut self,
        path: impl Into<LedgerPath>,
        key: K,
    ) -> Result<(Self, V), CompactError> {
        let path = path.into();
        let (result, value) = ledger::lookup_map::<K, V, D>(
            &self.context.query,
            path.as_slice(),
            key.clone(),
            self.context.gas_limit,
            &self.context.cost_model,
        )?;
        let Some(GatherEvent::Read(observed)) = result.events.last() else {
            return Err(CompactError::InvalidLedgerCell(
                "missing Map lookup event".into(),
            ));
        };
        let program = ledger::map_lookup_program(path.as_slice(), key, observed.clone());
        self.context.query = result.context;
        self.observed_gas += result.gas_cost;
        self.verify_ops.extend(program);
        Ok((self, value))
    }

    pub fn size_map(self, path: impl Into<LedgerPath>) -> Result<(Self, u64), CompactError> {
        self.size_set(path)
    }

    pub fn is_empty_map(self, path: impl Into<LedgerPath>) -> Result<(Self, bool), CompactError> {
        self.is_empty_set(path)
    }

    pub fn push_front_list<T: CellValue>(
        self,
        path: impl Into<LedgerPath>,
        value: T,
    ) -> Result<Self, CompactError> {
        self.apply_verify_program(ledger::list_push_front_program(path, value))
    }

    pub fn pop_front_list(self, path: impl Into<LedgerPath>) -> Result<Self, CompactError> {
        self.apply_verify_program(ledger::list_pop_front_program(path))
    }

    pub fn reset_list(self, path: impl Into<LedgerPath>) -> Result<Self, CompactError> {
        self.apply_verify_program(ledger::list_reset_program(path))
    }

    pub fn length_list(mut self, path: impl Into<LedgerPath>) -> Result<(Self, u64), CompactError> {
        let path = path.into();
        let (result, length) = ledger::length_list(
            &self.context.query,
            path.as_slice(),
            self.context.gas_limit,
            &self.context.cost_model,
        )?;
        let Some(GatherEvent::Read(observed)) = result.events.last() else {
            return Err(CompactError::InvalidLedgerCell(
                "missing List length event".into(),
            ));
        };
        let program = ledger::list_length_program(path.as_slice(), observed.clone());
        self.context.query = result.context;
        self.observed_gas += result.gas_cost;
        self.verify_ops.extend(program);
        Ok((self, length))
    }

    pub fn is_empty_list(
        mut self,
        path: impl Into<LedgerPath>,
    ) -> Result<(Self, bool), CompactError> {
        let path = path.into();
        let (result, empty) = ledger::is_empty_list(
            &self.context.query,
            path.as_slice(),
            self.context.gas_limit,
            &self.context.cost_model,
        )?;
        let Some(GatherEvent::Read(observed)) = result.events.last() else {
            return Err(CompactError::InvalidLedgerCell(
                "missing List emptiness event".into(),
            ));
        };
        let program = ledger::list_is_empty_program(path.as_slice(), observed.clone());
        self.context.query = result.context;
        self.observed_gas += result.gas_cost;
        self.verify_ops.extend(program);
        Ok((self, empty))
    }

    pub fn head_list<T: CellValue + Default, M: CellValue>(
        mut self,
        path: impl Into<LedgerPath>,
    ) -> Result<(Self, M), CompactError> {
        let path = path.into();
        let (result, head) = ledger::head_list::<T, M, D>(
            &self.context.query,
            path.as_slice(),
            self.context.gas_limit,
            &self.context.cost_model,
        )?;
        let Some(GatherEvent::Read(observed)) = result.events.last() else {
            return Err(CompactError::InvalidLedgerCell(
                "missing List head event".into(),
            ));
        };
        let program =
            ledger::list_head_program::<T, ResultModeVerify, D>(path.as_slice(), observed.clone());
        self.context.query = result.context;
        self.observed_gas += result.gas_cost;
        self.verify_ops.extend(program);
        Ok((self, head))
    }

    pub fn read_cell<T: CellValue>(
        mut self,
        path: impl Into<LedgerPath>,
    ) -> Result<(Self, T), CompactError> {
        let path = path.into();
        let (result, value) = ledger::query_cell_at_path::<T, D>(
            &self.context.query,
            path.as_slice(),
            self.context.gas_limit,
            &self.context.cost_model,
        )?;
        let Some(GatherEvent::Read(observed)) = result.events.last() else {
            return Err(CompactError::InvalidLedgerCell(
                "missing ledger read event".into(),
            ));
        };
        // Preserve the actual ledger FAB value. Re-encoding `value` could
        // change its alignment and make the verification transcript invalid.
        let program = ledger::cell_read_program(path.as_slice(), observed.clone());
        self.context.query = result.context;
        self.observed_gas += result.gas_cost;
        self.verify_ops.extend(program);
        Ok((self, value))
    }

    pub(crate) fn counter_less_than(
        mut self,
        path: impl Into<LedgerPath>,
        threshold: u64,
    ) -> Result<(Self, bool), CompactError> {
        let path = path.into();
        let (result, value) = ledger::query_counter_less_than(
            &self.context.query,
            path.as_slice(),
            threshold,
            self.context.gas_limit,
            &self.context.cost_model,
        )?;
        let Some(GatherEvent::Read(observed)) = result.events.last() else {
            return Err(CompactError::InvalidLedgerCell(
                "missing Counter comparison event".into(),
            ));
        };
        let program =
            ledger::counter_less_than_program(path.as_slice(), threshold, observed.clone());
        self.context.query = result.context;
        self.observed_gas += result.gas_cost;
        self.verify_ops.extend(program);
        Ok((self, value))
    }

    pub(crate) fn read_counter(
        mut self,
        path: impl Into<LedgerPath>,
    ) -> Result<(Self, u64), CompactError> {
        let path = path.into();
        let (result, value) = ledger::query_counter_at_path(
            &self.context.query,
            path.as_slice(),
            self.context.gas_limit,
            &self.context.cost_model,
        )?;
        let Some(GatherEvent::Read(observed)) = result.events.last() else {
            return Err(CompactError::InvalidLedgerCell(
                "missing Counter read event".into(),
            ));
        };
        let program = ledger::counter_read_program(path.as_slice(), observed.clone());
        self.context.query = result.context;
        self.observed_gas += result.gas_cost;
        self.verify_ops.extend(program);
        Ok((self, value))
    }

    pub fn finish<Output>(self, output: Output) -> RecordedCircuitResult<Private, Output, D> {
        let final_intents = self.context.circuit_zswap().clone();
        let final_coin_public_key = self.context.own_coin_public_key().ok();
        RecordedCircuitResult {
            execution: CircuitResult {
                context: self.context,
                result: output,
                gas_cost: self.observed_gas,
                private_transcript_outputs: self.private_outputs,
            },
            public: PublicTrace {
                initial_coin_public_key: self.initial_coin_public_key,
                final_coin_public_key,
                initial_intents: self.initial_intents,
                final_intents,
                initial: self.initial,
                verify_ops: self.verify_ops,
            },
        }
    }
}
