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

use midnight_base_crypto::cost_model::RunningCost;
use midnight_base_crypto::fab::AlignedValue;
use midnight_onchain_vm::ops::Op;
use midnight_onchain_vm::result_mode::{GatherEvent, ResultModeVerify};

use crate::CompactError;
use crate::context::{CircuitContext, CircuitResult, WitnessReadMeter};
use crate::ledger::{self, CellValue, DB, DefaultDB, LedgerPath, QueryContext};

/// The starting ledger context and the ordered VM program for one circuit.
pub struct PublicTrace<D: DB = DefaultDB> {
    initial: QueryContext<D>,
    verify_ops: Vec<Op<ResultModeVerify, D>>,
}

impl<D: DB> PublicTrace<D> {
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
    initial: QueryContext<D>,
    verify_ops: Vec<Op<ResultModeVerify, D>>,
    private_outputs: Vec<AlignedValue>,
    observed_gas: RunningCost,
}

impl<Private, D: DB> RecordingFrame<Private, D> {
    pub fn new(context: CircuitContext<Private, D>) -> Self {
        let initial = context.query.clone();
        Self {
            context,
            initial,
            verify_ops: Vec::new(),
            private_outputs: Vec::new(),
            observed_gas: RunningCost::ZERO,
        }
    }

    pub fn context(&self) -> &CircuitContext<Private, D> {
        &self.context
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

    pub fn write_cell<T: CellValue>(
        self,
        path: impl Into<LedgerPath>,
        value: T,
    ) -> Result<Self, CompactError> {
        let path = path.into();
        let program = ledger::cell_write_program(path.as_slice(), value);
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

    pub fn finish<Output>(self, output: Output) -> RecordedCircuitResult<Private, Output, D> {
        RecordedCircuitResult {
            execution: CircuitResult {
                context: self.context,
                result: output,
                gas_cost: self.observed_gas,
                private_transcript_outputs: self.private_outputs,
            },
            public: PublicTrace {
                initial: self.initial,
                verify_ops: self.verify_ops,
            },
        }
    }
}
