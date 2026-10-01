// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0 (the "License");

//! An opt-in path from native circuit execution to a replayable ledger program.
//!
//! Cell reads, writes, and Counter updates are supported so far. Generated contracts must
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
        mut self,
        path: impl Into<LedgerPath>,
        value: T,
    ) -> Result<Self, CompactError> {
        let path = path.into();
        let program = ledger::cell_write_program(path.as_slice(), value);
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
        mut self,
        path: impl Into<LedgerPath>,
        amount: u16,
        subtract: bool,
    ) -> Result<Self, CompactError> {
        let path = path.into();
        let program = ledger::counter_program(path.as_slice(), amount, subtract);
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
