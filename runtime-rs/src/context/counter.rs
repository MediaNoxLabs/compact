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

//! Native counter operations, preserving context ownership and ledger query cost.

use super::{CircuitContext, CircuitResult};
use crate::ledger::DB;
use crate::{CompactError, ledger};

impl<Private, D: DB> CircuitContext<Private, D> {
    pub(crate) fn counter_less_than_at_path(
        mut self,
        path: &[u8],
        threshold: u64,
    ) -> Result<CircuitResult<Private, bool, D>, CompactError> {
        let (result, value) = ledger::query_counter_less_than(
            &self.query,
            path,
            threshold,
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

    pub(crate) fn read_counter_at_path(
        mut self,
        path: &[u8],
    ) -> Result<CircuitResult<Private, u64, D>, CompactError> {
        let (result, value) =
            ledger::query_counter_at_path(&self.query, path, self.gas_limit, &self.cost_model)?;
        self.query = result.context;
        Ok(CircuitResult {
            context: self,
            result: value,
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
