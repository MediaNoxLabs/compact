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

//! Native cell operations, preserving context ownership and ledger query cost.

use super::{CircuitContext, CircuitResult};
use crate::ledger::{CellValue, DB};
use crate::{CompactError, ledger};

impl<Private, D: DB> CircuitContext<Private, D> {
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

    pub fn write_qualified_coin_cell<T: CellValue>(
        mut self,
        path: impl Into<ledger::LedgerPath>,
        coin: ledger::CoinInfo,
        recipient: ledger::CoinRecipient,
    ) -> Result<CircuitResult<Private, (), D>, CompactError> {
        let result = ledger::write_qualified_coin_cell::<T, D>(
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
        value.validate_cell_value()?;
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
}
