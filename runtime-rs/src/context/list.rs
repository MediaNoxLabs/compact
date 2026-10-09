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

//! Native list operations, preserving context ownership and ledger query cost.

use super::{CircuitContext, CircuitResult};
use crate::ledger::{CellValue, DB};
use crate::{CompactError, ledger};

impl<Private, D: DB> CircuitContext<Private, D> {
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
}
