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

//! Native trees operations, preserving context ownership and ledger query cost.

use super::{CircuitContext, CircuitResult};
use crate::ledger::{CellValue, DB};
use crate::{CompactError, ledger};

impl<Private, D: DB> CircuitContext<Private, D> {
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
}
