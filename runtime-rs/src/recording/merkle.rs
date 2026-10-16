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

//! Plain and historic Merkle operations on a recording frame.
//!
//! Mutations reuse canonical ledger programs. Observations retain the actual
//! gathered FAB read before adopting query state, gas, and verification ops.
//! Frame storage and program application remain owned by the parent module.

use super::RecordingFrame;
use crate::CompactError;
use crate::ledger::{self, CellValue, DB, LedgerPath};
use midnight_onchain_vm::result_mode::{GatherEvent, ResultModeVerify};

impl<Private, D: DB> RecordingFrame<Private, D> {
    /// Append a typed leaf to a plain Merkle tree and retain its verifying VM program.
    pub fn insert_merkle<T: CellValue>(
        self,
        path: impl Into<LedgerPath>,
        value: T,
    ) -> Result<Self, CompactError> {
        self.apply_checked_verify_program(ledger::merkle_insert_program(path, value))
    }

    /// Append an already hashed leaf to a plain Merkle tree and retain its VM program.
    pub fn insert_merkle_hash(
        self,
        path: impl Into<LedgerPath>,
        hash: crate::FixedBytes<32>,
    ) -> Result<Self, CompactError> {
        self.apply_checked_verify_program(ledger::merkle_insert_hash_program(path, hash))
    }

    /// Reset a plain tree with the canonical metered ledger program.
    pub fn reset_merkle_to_default(
        self,
        path: impl Into<LedgerPath>,
        depth: u8,
    ) -> Result<Self, CompactError> {
        self.apply_checked_verify_program(ledger::merkle_reset_program(path, depth))
    }

    /// Append to a historic Merkle tree and retain its root-history VM update.
    pub fn insert_historic_merkle<T: CellValue>(
        self,
        path: impl Into<LedgerPath>,
        value: T,
    ) -> Result<Self, CompactError> {
        self.apply_checked_verify_program(ledger::historic_merkle_insert_program(path, value))
    }

    /// Append an already hashed leaf and retain the historic root-map update.
    pub fn insert_historic_merkle_hash(
        self,
        path: impl Into<LedgerPath>,
        hash: crate::FixedBytes<32>,
    ) -> Result<Self, CompactError> {
        self.apply_checked_verify_program(ledger::historic_merkle_insert_hash_program(path, hash))
    }

    /// Keep the current historic root and discard older root history.
    pub fn reset_historic_merkle_history(
        self,
        path: impl Into<LedgerPath>,
    ) -> Result<Self, CompactError> {
        self.apply_checked_verify_program(ledger::historic_reset_history_program(path.into()))
    }

    /// Replace a historic tree and seed the blank root in its new history.
    pub fn reset_historic_merkle_to_default(
        self,
        path: impl Into<LedgerPath>,
        depth: u8,
    ) -> Result<Self, CompactError> {
        self.apply_checked_verify_program(ledger::historic_reset_to_default_program(path, depth))
    }

    /// Insert a typed leaf at a declared index, retaining the ledger-8 VM program.
    pub fn insert_merkle_index<T: CellValue>(
        self,
        path: impl Into<LedgerPath>,
        value: T,
        position: crate::BoundedUint<{ u64::MAX as u128 }>,
    ) -> Result<Self, CompactError> {
        self.apply_checked_verify_program(ledger::merkle_insert_index_program(
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
        self.apply_checked_verify_program(ledger::merkle_insert_hash_index_program(
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
        self.apply_checked_verify_program(ledger::merkle_insert_index_default_program::<T, D>(
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
        self.apply_checked_verify_program(ledger::historic_merkle_insert_index_program(
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
        self.apply_checked_verify_program(ledger::historic_merkle_insert_hash_index_program(
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
        self.apply_checked_verify_program(ledger::historic_merkle_insert_index_default_program::<
            T,
            D,
        >(path, position.value() as u64))
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

    pub(super) fn read_merkle_is_full(
        mut self,
        path: LedgerPath,
        depth: u8,
        historic: bool,
    ) -> Result<(Self, bool), CompactError> {
        let (result, full) = if historic {
            ledger::historic_is_full(
                &self.state.context.query,
                path.as_slice(),
                depth,
                self.state.context.gas_limit,
                &self.state.context.cost_model,
            )?
        } else {
            ledger::merkle_is_full(
                &self.state.context.query,
                path.as_slice(),
                depth,
                self.state.context.gas_limit,
                &self.state.context.cost_model,
            )?
        };
        let Some(GatherEvent::Read(observed)) = result.events.last() else {
            return Err(CompactError::InvalidLedgerCell(
                "missing Merkle fullness event".into(),
            ));
        };
        let program =
            ledger::is_full_program::<ResultModeVerify, D>(path, depth, observed.clone())?;
        self.state.context.query = result.context;
        self.state.observed_gas += result.gas_cost;
        self.state.verify_ops.extend(program);
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
            &self.state.context.query,
            path.as_slice(),
            root.clone(),
            self.state.context.gas_limit,
            &self.state.context.cost_model,
        )?;
        let Some(GatherEvent::Read(observed)) = result.events.last() else {
            return Err(CompactError::InvalidLedgerCell(
                "missing Merkle root comparison event".into(),
            ));
        };
        let program = ledger::merkle_check_root_verify_program(path, root, observed.clone())?;
        self.state.context.query = result.context;
        self.state.observed_gas += result.gas_cost;
        self.state.verify_ops.extend(program);
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
            &self.state.context.query,
            path.as_slice(),
            root.clone(),
            self.state.context.gas_limit,
            &self.state.context.cost_model,
        )?;
        let Some(GatherEvent::Read(observed)) = result.events.last() else {
            return Err(CompactError::InvalidLedgerCell(
                "missing historic Merkle root membership event".into(),
            ));
        };
        let program = ledger::historic_check_root_verify_program(path, root, observed.clone())?;
        self.state.context.query = result.context;
        self.state.observed_gas += result.gas_cost;
        self.state.verify_ops.extend(program);
        Ok((self, known))
    }
}
