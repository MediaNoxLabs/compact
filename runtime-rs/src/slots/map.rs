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

//! Typed Map access and the marker for nested Map declarations.

use std::marker::PhantomData;

use crate::CompactError;
use crate::context::{CircuitContext, CircuitResult, WitnessReadMeter};
use crate::ledger::{
    CellValue, DB, MapView, MeteredMapView, StateValue, map_view_at_path, metered_map_view_at_path,
};
use crate::recording::RecordingFrame;

/// A ledger Map nested as another Map's value, rather than a scalar `CellValue`.
///
/// This marker has no value codec. It lets a generated slot name the nested
/// structure while keeping scalar lookup and insert methods unavailable.
///
/// Removing a nested child does not require a scalar codec. Scalar access stays
/// unavailable, including its recorded forms:
///
/// ```compile_fail,E0599
/// use midnight_compact_runtime::{Field, context::CircuitContext, slots::{MapNode, MapSlot}};
/// fn scalar_lookup(context: CircuitContext<()>) {
///     let slot = MapSlot::<bool, MapNode<bool, Field>>::new(&[0]);
///     slot.lookup(context, true);
/// }
/// ```
///
/// ```compile_fail,E0599
/// use midnight_compact_runtime::{Field, context::CircuitContext, slots::{MapNode, MapSlot}};
/// fn scalar_insert(context: CircuitContext<()>, value: MapNode<bool, Field>) {
///     let slot = MapSlot::<bool, MapNode<bool, Field>>::new(&[0]);
///     slot.insert(context, true, value);
/// }
/// ```
///
/// ```compile_fail,E0599
/// use midnight_compact_runtime::{Field, recording::RecordingFrame, slots::{MapNode, MapSlot}};
/// fn scalar_lookup(frame: RecordingFrame<()>) {
///     let slot = MapSlot::<bool, MapNode<bool, Field>>::new(&[0]);
///     slot.record_lookup(frame, true);
/// }
/// ```
///
/// ```compile_fail,E0599
/// use midnight_compact_runtime::{Field, recording::RecordingFrame, slots::{MapNode, MapSlot}};
/// fn scalar_insert(frame: RecordingFrame<()>, value: MapNode<bool, Field>) {
///     let slot = MapSlot::<bool, MapNode<bool, Field>>::new(&[0]);
///     slot.record_insert(frame, true, value);
/// }
/// ```
#[derive(Clone, Copy, Debug)]
pub struct MapNode<K, V>(PhantomData<fn() -> (K, V)>);

/// A compiler-declared Map with fixed key/value shape and a physical path.
#[derive(Clone, Copy)]
pub struct MapSlot<K, V> {
    path: &'static [u8],
    types: PhantomData<fn() -> (K, V)>,
}

impl<K: CellValue, V> MapSlot<K, V> {
    pub const fn new(path: &'static [u8]) -> Self {
        Self {
            path,
            types: PhantomData,
        }
    }

    pub const fn path(self) -> &'static [u8] {
        self.path
    }

    pub fn member<Private, D: DB>(
        self,
        context: CircuitContext<Private, D>,
        key: K,
    ) -> Result<CircuitResult<Private, bool, D>, CompactError> {
        context.member_map(self.path, key)
    }

    pub fn remove<Private, D: DB>(
        self,
        context: CircuitContext<Private, D>,
        key: K,
    ) -> Result<CircuitResult<Private, (), D>, CompactError> {
        context.remove_map(self.path, key)
    }

    pub fn size<Private, D: DB>(
        self,
        context: CircuitContext<Private, D>,
    ) -> Result<CircuitResult<Private, u64, D>, CompactError> {
        context.size_map(self.path)
    }

    pub fn is_empty<Private, D: DB>(
        self,
        context: CircuitContext<Private, D>,
    ) -> Result<CircuitResult<Private, bool, D>, CompactError> {
        context.is_empty_map(self.path)
    }

    pub fn record_member<Private, D: DB>(
        self,
        frame: RecordingFrame<Private, D>,
        key: K,
    ) -> Result<(RecordingFrame<Private, D>, bool), CompactError>
    where
        K: Clone,
    {
        frame.member_map(self.path, key)
    }

    pub fn record_remove<Private, D: DB>(
        self,
        frame: RecordingFrame<Private, D>,
        key: K,
    ) -> Result<RecordingFrame<Private, D>, CompactError> {
        frame.remove_map(self.path, key)
    }

    pub fn record_size<Private, D: DB>(
        self,
        frame: RecordingFrame<Private, D>,
    ) -> Result<(RecordingFrame<Private, D>, u64), CompactError> {
        frame.size_map(self.path)
    }

    pub fn record_is_empty<Private, D: DB>(
        self,
        frame: RecordingFrame<Private, D>,
    ) -> Result<(RecordingFrame<Private, D>, bool), CompactError> {
        frame.is_empty_map(self.path)
    }
}

impl<K: CellValue, V: CellValue> MapSlot<K, V> {
    /// Inspect an already held public Map without a VM query or gas charge.
    pub fn inspect<'a, D: DB>(
        self,
        state: &'a StateValue<D>,
    ) -> Result<MapView<'a, K, V, D>, CompactError> {
        map_view_at_path(state, self.path)
    }

    /// Project this scalar-valued Map through the existing metered witness view.
    pub fn witness_view<'a, D: DB>(
        self,
        meter: &'a WitnessReadMeter<'a, D>,
    ) -> Result<MeteredMapView<'a, K, V, D>, CompactError> {
        metered_map_view_at_path(meter, self.path)
    }

    pub fn insert<Private, D: DB>(
        self,
        context: CircuitContext<Private, D>,
        key: K,
        value: V,
    ) -> Result<CircuitResult<Private, (), D>, CompactError> {
        context.insert_map(self.path, key, value)
    }

    pub fn insert_default<Private, D: DB>(
        self,
        context: CircuitContext<Private, D>,
        key: K,
    ) -> Result<CircuitResult<Private, (), D>, CompactError>
    where
        V: Default,
    {
        self.insert(context, key, V::default())
    }

    pub fn reset<Private, D: DB>(
        self,
        context: CircuitContext<Private, D>,
    ) -> Result<CircuitResult<Private, (), D>, CompactError> {
        context.reset_map(self.path)
    }

    pub fn lookup<Private, D: DB>(
        self,
        context: CircuitContext<Private, D>,
        key: K,
    ) -> Result<CircuitResult<Private, V, D>, CompactError> {
        context.lookup_map(self.path, key)
    }

    pub fn record_insert<Private, D: DB>(
        self,
        frame: RecordingFrame<Private, D>,
        key: K,
        value: V,
    ) -> Result<RecordingFrame<Private, D>, CompactError> {
        frame.insert_map(self.path, key, value)
    }

    pub fn record_insert_default<Private, D: DB>(
        self,
        frame: RecordingFrame<Private, D>,
        key: K,
    ) -> Result<RecordingFrame<Private, D>, CompactError>
    where
        V: Default,
    {
        self.record_insert(frame, key, V::default())
    }

    pub fn record_reset<Private, D: DB>(
        self,
        frame: RecordingFrame<Private, D>,
    ) -> Result<RecordingFrame<Private, D>, CompactError> {
        frame.reset_map(self.path)
    }

    pub fn record_lookup<Private, D: DB>(
        self,
        frame: RecordingFrame<Private, D>,
        key: K,
    ) -> Result<(RecordingFrame<Private, D>, V), CompactError>
    where
        K: Clone,
    {
        frame.lookup_map(self.path, key)
    }
}
