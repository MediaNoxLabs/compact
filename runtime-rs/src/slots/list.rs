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

//! Typed List access for held state, witnesses, execution and recording.

use std::marker::PhantomData;

use crate::CompactError;
use crate::context::{CircuitContext, CircuitResult, WitnessReadMeter};
use crate::ledger::{
    CellValue, DB, ListView, MeteredListView, StateValue, list_view_at_path,
    metered_list_view_at_path,
};
use crate::recording::RecordingFrame;

/// A compiler-declared List with a fixed element type and physical path.
#[derive(Clone, Copy)]
pub struct ListSlot<T> {
    path: &'static [u8],
    element: PhantomData<fn() -> T>,
}

impl<T: CellValue> ListSlot<T> {
    pub const fn new(path: &'static [u8]) -> Self {
        Self {
            path,
            element: PhantomData,
        }
    }

    pub const fn path(self) -> &'static [u8] {
        self.path
    }

    /// Inspect this List in an already held public state without metering.
    pub fn inspect<'a, D: DB>(
        self,
        state: &'a StateValue<D>,
    ) -> Result<ListView<'a, T, D>, CompactError> {
        list_view_at_path(state, self.path)
    }

    /// Project this List through the metered witness view.
    pub fn witness_view<'a, D: DB>(
        self,
        meter: &'a WitnessReadMeter<'a, D>,
    ) -> Result<MeteredListView<'a, T, D>, CompactError> {
        metered_list_view_at_path(meter, self.path)
    }

    pub fn push_front<Private, D: DB>(
        self,
        context: CircuitContext<Private, D>,
        value: T,
    ) -> Result<CircuitResult<Private, (), D>, CompactError> {
        context.push_front_list(self.path, value)
    }

    pub fn pop_front<Private, D: DB>(
        self,
        context: CircuitContext<Private, D>,
    ) -> Result<CircuitResult<Private, (), D>, CompactError> {
        context.pop_front_list(self.path)
    }

    pub fn reset<Private, D: DB>(
        self,
        context: CircuitContext<Private, D>,
    ) -> Result<CircuitResult<Private, (), D>, CompactError> {
        context.reset_list(self.path)
    }

    pub fn length<Private, D: DB>(
        self,
        context: CircuitContext<Private, D>,
    ) -> Result<CircuitResult<Private, u64, D>, CompactError> {
        context.length_list(self.path)
    }

    pub fn is_empty<Private, D: DB>(
        self,
        context: CircuitContext<Private, D>,
    ) -> Result<CircuitResult<Private, bool, D>, CompactError> {
        context.is_empty_list(self.path)
    }

    pub fn head<M: CellValue, Private, D: DB>(
        self,
        context: CircuitContext<Private, D>,
    ) -> Result<CircuitResult<Private, M, D>, CompactError>
    where
        T: Default,
    {
        context.head_list::<T, M>(self.path)
    }

    pub fn record_push_front<Private, D: DB>(
        self,
        frame: RecordingFrame<Private, D>,
        value: T,
    ) -> Result<RecordingFrame<Private, D>, CompactError> {
        frame.push_front_list(self.path, value)
    }

    pub fn record_pop_front<Private, D: DB>(
        self,
        frame: RecordingFrame<Private, D>,
    ) -> Result<RecordingFrame<Private, D>, CompactError> {
        frame.pop_front_list(self.path)
    }

    pub fn record_reset<Private, D: DB>(
        self,
        frame: RecordingFrame<Private, D>,
    ) -> Result<RecordingFrame<Private, D>, CompactError> {
        frame.reset_list(self.path)
    }

    pub fn record_length<Private, D: DB>(
        self,
        frame: RecordingFrame<Private, D>,
    ) -> Result<(RecordingFrame<Private, D>, u64), CompactError> {
        frame.length_list(self.path)
    }

    pub fn record_is_empty<Private, D: DB>(
        self,
        frame: RecordingFrame<Private, D>,
    ) -> Result<(RecordingFrame<Private, D>, bool), CompactError> {
        frame.is_empty_list(self.path)
    }

    pub fn record_head<M: CellValue, Private, D: DB>(
        self,
        frame: RecordingFrame<Private, D>,
    ) -> Result<(RecordingFrame<Private, D>, M), CompactError>
    where
        T: Default,
    {
        frame.head_list::<T, M>(self.path)
    }
}
