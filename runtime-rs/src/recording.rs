// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0 (the "License");

//! An opt-in path from native circuit execution to a replayable ledger program.
//!
//! Cell, Counter, Set, and Map operations are supported so far. Generated contracts must
//! not claim a transaction-ready trace until every operation they use records
//! its corresponding verifying VM instruction.

use midnight_base_crypto::cost_model::RunningCost;
use midnight_base_crypto::fab::AlignedValue;
use midnight_onchain_vm::ops::Op;
use midnight_onchain_vm::result_mode::{GatherEvent, ResultModeVerify};

use crate::CompactError;
use crate::context::{CircuitContext, CircuitResult};
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
