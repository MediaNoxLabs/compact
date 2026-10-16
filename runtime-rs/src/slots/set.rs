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

//! Typed Set access for held state, witnesses, execution and recording.

use std::marker::PhantomData;

use crate::CompactError;
use crate::context::{CircuitContext, CircuitResult, WitnessReadMeter};
use crate::ledger::{
    CellValue, DB, MeteredSetView, SetView, StateValue, metered_set_view_at_path, set_view_at_path,
};
use crate::recording::RecordingFrame;

/// A compiler-declared Set with a fixed element type and physical path.
#[derive(Clone, Copy)]
pub struct SetSlot<T> {
    path: &'static [u8],
    element: PhantomData<fn() -> T>,
}

impl<T: CellValue> SetSlot<T> {
    pub const fn new(path: &'static [u8]) -> Self {
        Self {
            path,
            element: PhantomData,
        }
    }

    pub const fn path(self) -> &'static [u8] {
        self.path
    }

    /// Inspect an already held public Set without a VM query or gas charge.
    pub fn inspect<'a, D: DB>(
        self,
        state: &'a StateValue<D>,
    ) -> Result<SetView<'a, T, D>, CompactError> {
        set_view_at_path(state, self.path)
    }

    /// Project this Set through the existing metered witness view.
    pub fn witness_view<'a, D: DB>(
        self,
        meter: &'a WitnessReadMeter<'a, D>,
    ) -> Result<MeteredSetView<'a, T, D>, CompactError> {
        metered_set_view_at_path(meter, self.path)
    }

    pub fn insert<Private, D: DB>(
        self,
        context: CircuitContext<Private, D>,
        value: T,
    ) -> Result<CircuitResult<Private, (), D>, CompactError> {
        context.insert_set(self.path, value)
    }

    /// Qualified coin insertion looks up the allocated Merkle index in the
    /// call context, then inserts through the ledger's dedicated VM program.
    pub fn insert_coin<Private, D: DB>(
        self,
        context: CircuitContext<Private, D>,
        coin: crate::ledger::CoinInfo,
        recipient: crate::ledger::CoinRecipient,
    ) -> Result<CircuitResult<Private, (), D>, CompactError> {
        context.insert_qualified_coin_set::<T>(self.path, coin, recipient)
    }

    pub fn remove<Private, D: DB>(
        self,
        context: CircuitContext<Private, D>,
        value: T,
    ) -> Result<CircuitResult<Private, (), D>, CompactError> {
        context.remove_set(self.path, value)
    }

    pub fn member<Private, D: DB>(
        self,
        context: CircuitContext<Private, D>,
        value: T,
    ) -> Result<CircuitResult<Private, bool, D>, CompactError> {
        context.member_set(self.path, value)
    }

    pub fn reset<Private, D: DB>(
        self,
        context: CircuitContext<Private, D>,
    ) -> Result<CircuitResult<Private, (), D>, CompactError> {
        context.reset_set(self.path)
    }

    pub fn size<Private, D: DB>(
        self,
        context: CircuitContext<Private, D>,
    ) -> Result<CircuitResult<Private, u64, D>, CompactError> {
        context.size_set(self.path)
    }

    pub fn is_empty<Private, D: DB>(
        self,
        context: CircuitContext<Private, D>,
    ) -> Result<CircuitResult<Private, bool, D>, CompactError> {
        context.is_empty_set(self.path)
    }

    pub fn record_insert<Private, D: DB>(
        self,
        frame: RecordingFrame<Private, D>,
        value: T,
    ) -> Result<RecordingFrame<Private, D>, CompactError> {
        frame.insert_set(self.path, value)
    }

    pub fn record_insert_coin<Private, D: DB>(
        self,
        frame: RecordingFrame<Private, D>,
        coin: crate::ledger::CoinInfo,
        recipient: crate::ledger::CoinRecipient,
    ) -> Result<RecordingFrame<Private, D>, CompactError> {
        frame.insert_qualified_coin_set::<T>(self.path, coin, recipient)
    }

    pub fn record_remove<Private, D: DB>(
        self,
        frame: RecordingFrame<Private, D>,
        value: T,
    ) -> Result<RecordingFrame<Private, D>, CompactError> {
        frame.remove_set(self.path, value)
    }

    pub fn record_member<Private, D: DB>(
        self,
        frame: RecordingFrame<Private, D>,
        value: T,
    ) -> Result<(RecordingFrame<Private, D>, bool), CompactError>
    where
        T: Clone,
    {
        frame.member_set(self.path, value)
    }

    pub fn record_reset<Private, D: DB>(
        self,
        frame: RecordingFrame<Private, D>,
    ) -> Result<RecordingFrame<Private, D>, CompactError> {
        frame.reset_set(self.path)
    }

    pub fn record_size<Private, D: DB>(
        self,
        frame: RecordingFrame<Private, D>,
    ) -> Result<(RecordingFrame<Private, D>, u64), CompactError> {
        frame.size_set(self.path)
    }

    pub fn record_is_empty<Private, D: DB>(
        self,
        frame: RecordingFrame<Private, D>,
    ) -> Result<(RecordingFrame<Private, D>, bool), CompactError> {
        frame.is_empty_set(self.path)
    }
}
