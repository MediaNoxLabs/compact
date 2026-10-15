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

//! Native map operations, preserving context ownership and ledger query cost.

use super::{CircuitContext, CircuitResult};
use crate::ledger::{CellValue, DB};
use crate::{CompactError, ledger};

impl<Private, D: DB> CircuitContext<Private, D> {
    pub fn insert_map<K: CellValue, V: CellValue>(
        mut self,
        path: impl Into<ledger::LedgerPath>,
        key: K,
        value: V,
    ) -> Result<CircuitResult<Private, (), D>, CompactError> {
        key.validate_cell_value()?;
        value.validate_cell_value()?;
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
}
