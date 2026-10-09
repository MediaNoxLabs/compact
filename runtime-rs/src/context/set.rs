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

//! Native set operations, preserving context ownership and ledger query cost.

use super::{CircuitContext, CircuitResult};
use crate::ledger::{CellValue, DB};
use crate::{CompactError, ledger};

impl<Private, D: DB> CircuitContext<Private, D> {
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

    /// Insert a coin into a qualified Set using the transaction's allocated
    /// commitment index and the exact ledger-8 insertion VM operation.
    pub fn insert_qualified_coin_set<T: CellValue>(
        mut self,
        path: impl Into<ledger::LedgerPath>,
        coin: ledger::CoinInfo,
        recipient: ledger::CoinRecipient,
    ) -> Result<CircuitResult<Private, (), D>, CompactError> {
        let result = ledger::insert_qualified_coin_set::<T, D>(
            &self.query,
            path,
            coin,
            recipient,
            self.gas_limit,
            &self.cost_model,
        )?;
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
}
