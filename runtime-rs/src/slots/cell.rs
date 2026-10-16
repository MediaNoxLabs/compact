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

//! Typed Cell access for held state, witnesses, execution and recording.

use std::marker::PhantomData;

use crate::CompactError;
use crate::context::{CircuitContext, CircuitResult, WitnessReadMeter};
use crate::ledger::{CellValue, DB, StateValue};
use crate::recording::RecordingFrame;

#[derive(Clone, Copy)]
pub struct CellSlot<T> {
    path: &'static [u8],
    value: PhantomData<fn() -> T>,
}

impl<T: CellValue> CellSlot<T> {
    pub const fn new(path: &'static [u8]) -> Self {
        Self {
            path,
            value: PhantomData,
        }
    }

    pub const fn path(self) -> &'static [u8] {
        self.path
    }

    /// Inspect an already held public state without a VM query or gas charge.
    pub fn inspect<D: DB>(self, state: &StateValue<D>) -> Result<T, CompactError> {
        crate::ledger::read_cell_at_path(state, self.path)
    }

    /// Read this declared Cell through the witness read meter.
    pub fn witness_read<D: DB>(self, meter: &WitnessReadMeter<'_, D>) -> Result<T, CompactError> {
        meter.read_cell::<T>(self.path)
    }

    pub fn read<Private, D: DB>(
        self,
        context: CircuitContext<Private, D>,
    ) -> Result<CircuitResult<Private, T, D>, CompactError> {
        context.read_cell_at_path(self.path)
    }

    /// Qualify a coin using the transaction context and replace this Cell.
    pub fn write_coin<Private, D: DB>(
        self,
        context: CircuitContext<Private, D>,
        coin: crate::ledger::CoinInfo,
        recipient: crate::ledger::CoinRecipient,
    ) -> Result<CircuitResult<Private, (), D>, CompactError> {
        context.write_qualified_coin_cell::<T>(self.path, coin, recipient)
    }

    /// Record a qualified coin replacement through the declared Cell slot.
    pub fn record_write_coin<Private, D: DB>(
        self,
        frame: RecordingFrame<Private, D>,
        coin: crate::ledger::CoinInfo,
        recipient: crate::ledger::CoinRecipient,
    ) -> Result<RecordingFrame<Private, D>, CompactError> {
        frame.write_qualified_coin_cell::<T>(self.path, coin, recipient)
    }

    pub fn write<Private, D: DB>(
        self,
        context: CircuitContext<Private, D>,
        value: T,
    ) -> Result<CircuitResult<Private, (), D>, CompactError> {
        context.write_cell_at_path(self.path, value)
    }

    pub fn record_read<Private, D: DB>(
        self,
        frame: RecordingFrame<Private, D>,
    ) -> Result<(RecordingFrame<Private, D>, T), CompactError> {
        frame.read_cell(self.path)
    }

    pub fn record_write<Private, D: DB>(
        self,
        frame: RecordingFrame<Private, D>,
        value: T,
    ) -> Result<RecordingFrame<Private, D>, CompactError> {
        frame.write_cell(self.path, value)
    }
}
