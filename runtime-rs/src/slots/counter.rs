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

//! Typed Counter access for held state, witnesses, execution and recording.

use crate::CompactError;
use crate::context::{CircuitContext, CircuitResult, WitnessReadMeter};
use crate::ledger::{DB, StateValue};
use crate::recording::RecordingFrame;

#[derive(Clone, Copy)]
pub struct CounterSlot {
    path: &'static [u8],
}

impl CounterSlot {
    pub const fn new(path: &'static [u8]) -> Self {
        Self { path }
    }

    pub const fn path(self) -> &'static [u8] {
        self.path
    }

    /// Inspect an already held public Counter without a VM query or gas charge.
    pub fn inspect<D: DB>(self, state: &StateValue<D>) -> Result<u64, CompactError> {
        crate::ledger::read_cell_at_path(state, self.path)
    }

    /// Read the raw ledger Counter through the witness read meter.
    pub fn witness_read<D: DB>(self, meter: &WitnessReadMeter<'_, D>) -> Result<u64, CompactError> {
        meter.read_cell::<u64>(self.path)
    }

    /// Execute the ledger Counter's lessThan program with its query gas.
    pub fn less_than<Private, D: DB>(
        self,
        context: CircuitContext<Private, D>,
        threshold: u64,
    ) -> Result<CircuitResult<Private, bool, D>, CompactError> {
        context.counter_less_than_at_path(self.path, threshold)
    }

    pub fn record_less_than<Private, D: DB>(
        self,
        frame: RecordingFrame<Private, D>,
        threshold: u64,
    ) -> Result<(RecordingFrame<Private, D>, bool), CompactError> {
        frame.counter_less_than(self.path, threshold)
    }

    pub fn read<Private, D: DB>(
        self,
        context: CircuitContext<Private, D>,
    ) -> Result<CircuitResult<Private, u64, D>, CompactError> {
        context.read_counter_at_path(self.path)
    }

    pub fn increment<Private, D: DB>(
        self,
        context: CircuitContext<Private, D>,
        amount: u16,
    ) -> Result<CircuitResult<Private, (), D>, CompactError> {
        context.increment_counter(self.path, amount)
    }

    pub fn decrement<Private, D: DB>(
        self,
        context: CircuitContext<Private, D>,
        amount: u16,
    ) -> Result<CircuitResult<Private, (), D>, CompactError> {
        context.decrement_counter(self.path, amount)
    }

    pub fn reset<Private, D: DB>(
        self,
        context: CircuitContext<Private, D>,
    ) -> Result<CircuitResult<Private, (), D>, CompactError> {
        context.write_cell_at_path(self.path, 0_u64)
    }

    pub fn record_read<Private, D: DB>(
        self,
        frame: RecordingFrame<Private, D>,
    ) -> Result<(RecordingFrame<Private, D>, u64), CompactError> {
        frame.read_counter(self.path)
    }

    pub fn record_increment<Private, D: DB>(
        self,
        frame: RecordingFrame<Private, D>,
        amount: u16,
    ) -> Result<RecordingFrame<Private, D>, CompactError> {
        frame.increment_counter(self.path, amount)
    }

    pub fn record_decrement<Private, D: DB>(
        self,
        frame: RecordingFrame<Private, D>,
        amount: u16,
    ) -> Result<RecordingFrame<Private, D>, CompactError> {
        frame.decrement_counter(self.path, amount)
    }

    pub fn record_reset<Private, D: DB>(
        self,
        frame: RecordingFrame<Private, D>,
    ) -> Result<RecordingFrame<Private, D>, CompactError> {
        frame.write_cell(self.path, 0_u64)
    }
}
