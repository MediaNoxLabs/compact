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

//! Typed names for compiler-declared ledger fields.
//!
//! Generated contracts expose one constant per supported ledger declaration. The runtime
//! owns VM execution and read-only state decoding; these descriptors tie a
//! value type to its physical ledger path. Their constructors are public to
//! support generated crates and are not an access-control boundary.

use std::marker::PhantomData;

use midnight_base_crypto::repr::BinaryHashRepr;

use crate::CompactError;
use crate::context::{CircuitContext, CircuitResult, WitnessReadMeter};
use crate::ledger::{
    CellValue, DB, HistoricMerkleTreeView, ListView, MapView, MerklePath, MerkleTreeDigest,
    MerkleTreeView, MeteredHistoricMerkleTreeView, MeteredListView, MeteredMapView,
    MeteredMerkleTreeView, MeteredSetView, SetView, StateValue, historic_merkle_tree_view_at_path,
    list_view_at_path, map_view_at_path, merkle_tree_view_at_path,
    metered_historic_merkle_tree_view_at_path, metered_list_view_at_path, metered_map_view_at_path,
    metered_merkle_tree_view_at_path, metered_set_view_at_path, set_view_at_path,
};
use crate::recording::RecordingFrame;

/// Local, read-only projection of a declared plain Merkle tree.
///
/// This view checks the declared height but does not execute ledger VM reads.
pub struct PlainMerkleStateView<'a, Leaf, const DEPTH: u8, D: DB> {
    view: MerkleTreeView<'a, D>,
    leaf: PhantomData<fn() -> Leaf>,
}

impl<Leaf, const DEPTH: u8, D: DB> PlainMerkleStateView<'_, Leaf, DEPTH, D> {
    pub const DECLARED_DEPTH: u8 = DEPTH;

    pub fn root(&self) -> Option<MerkleTreeDigest> {
        self.view.root()
    }

    pub fn first_free(&self) -> Result<crate::BoundedUint<{ u64::MAX as u128 }>, CompactError> {
        self.view.first_free()
    }

    /// Build a path with a declared leaf; the supplied leaf is not checked against the tree.
    pub fn path_for_leaf(&self, index: u64, leaf: Leaf) -> Result<MerklePath<Leaf>, CompactError>
    where
        Leaf: BinaryHashRepr,
    {
        self.view.path_for_leaf(index, leaf)
    }

    pub fn find_path_for_leaf(&self, leaf: Leaf) -> Option<MerklePath<Leaf>>
    where
        Leaf: BinaryHashRepr,
    {
        self.view.find_path_for_leaf(leaf)
    }
}

/// Local, read-only projection of a declared historic Merkle tree.
///
/// History inspection reads the held state; VM-metered root checks remain on witness views.
pub struct HistoricMerkleStateView<'a, Leaf, const DEPTH: u8, D: DB> {
    view: HistoricMerkleTreeView<'a, D>,
    leaf: PhantomData<fn() -> Leaf>,
}

impl<Leaf, const DEPTH: u8, D: DB> HistoricMerkleStateView<'_, Leaf, DEPTH, D> {
    pub const DECLARED_DEPTH: u8 = DEPTH;

    pub fn root(&self) -> Option<MerkleTreeDigest> {
        self.view.root()
    }

    pub fn first_free(&self) -> Result<crate::BoundedUint<{ u64::MAX as u128 }>, CompactError> {
        self.view.first_free()
    }

    /// Build a path with a declared leaf; the supplied leaf is not checked against the tree.
    pub fn path_for_leaf(&self, index: u64, leaf: Leaf) -> Result<MerklePath<Leaf>, CompactError>
    where
        Leaf: BinaryHashRepr,
    {
        self.view.path_for_leaf(index, leaf)
    }

    pub fn find_path_for_leaf(&self, leaf: Leaf) -> Option<MerklePath<Leaf>>
    where
        Leaf: BinaryHashRepr,
    {
        self.view.find_path_for_leaf(leaf)
    }

    pub fn history(&self) -> Result<Vec<MerkleTreeDigest>, CompactError> {
        self.view.history()
    }

    /// Check the held history map locally, without the ledger VM's root-check program.
    pub fn contains_root(&self, root: MerkleTreeDigest) -> bool {
        self.view.contains_root(root)
    }
}

/// A compiler-declared Merkle tree. The leaf type, depth, and historic kind
/// are fixed at the generated declaration rather than repeated at call sites.
#[derive(Clone, Copy)]
pub struct MerkleSlot<T, const DEPTH: u8, const HISTORIC: bool> {
    path: &'static [u8],
    leaf: PhantomData<fn() -> T>,
}

impl<T: CellValue, const DEPTH: u8, const HISTORIC: bool> MerkleSlot<T, DEPTH, HISTORIC> {
    pub const fn new(path: &'static [u8]) -> Self {
        Self {
            path,
            leaf: PhantomData,
        }
    }

    pub const fn path(self) -> &'static [u8] {
        self.path
    }

    pub fn insert<Private, D: DB>(
        self,
        context: CircuitContext<Private, D>,
        value: T,
    ) -> Result<CircuitResult<Private, (), D>, CompactError> {
        if HISTORIC {
            context.historic_insert(self.path, value)
        } else {
            context.merkle_insert(self.path, value)
        }
    }

    pub fn insert_index<Private, D: DB>(
        self,
        context: CircuitContext<Private, D>,
        value: T,
        position: crate::BoundedUint<{ u64::MAX as u128 }>,
    ) -> Result<CircuitResult<Private, (), D>, CompactError> {
        if HISTORIC {
            context.historic_insert_index(self.path, value, position)
        } else {
            context.merkle_insert_index(self.path, value, position)
        }
    }

    /// Record a typed indexed insertion using the declared tree kind and leaf type.
    pub fn record_insert_index<Private, D: DB>(
        self,
        frame: RecordingFrame<Private, D>,
        value: T,
        position: crate::BoundedUint<{ u64::MAX as u128 }>,
    ) -> Result<RecordingFrame<Private, D>, CompactError> {
        if HISTORIC {
            frame.insert_historic_merkle_index(self.path, value, position)
        } else {
            frame.insert_merkle_index(self.path, value, position)
        }
    }

    pub fn insert_hash<Private, D: DB>(
        self,
        context: CircuitContext<Private, D>,
        hash: crate::FixedBytes<32>,
    ) -> Result<CircuitResult<Private, (), D>, CompactError> {
        if HISTORIC {
            context.historic_insert_hash(self.path, hash)
        } else {
            context.merkle_insert_hash(self.path, hash)
        }
    }

    pub fn insert_hash_index<Private, D: DB>(
        self,
        context: CircuitContext<Private, D>,
        hash: crate::FixedBytes<32>,
        position: crate::BoundedUint<{ u64::MAX as u128 }>,
    ) -> Result<CircuitResult<Private, (), D>, CompactError> {
        if HISTORIC {
            context.historic_insert_hash_index(self.path, hash, position)
        } else {
            context.merkle_insert_hash_index(self.path, hash, position)
        }
    }

    pub fn insert_index_default<Private, D: DB>(
        self,
        context: CircuitContext<Private, D>,
        position: crate::BoundedUint<{ u64::MAX as u128 }>,
    ) -> Result<CircuitResult<Private, (), D>, CompactError>
    where
        T: Default,
    {
        if HISTORIC {
            context.historic_insert_index_default::<T>(self.path, position)
        } else {
            context.merkle_insert_index_default::<T>(self.path, position)
        }
    }

    /// Record insertion of the declared default leaf at a typed index.
    pub fn record_insert_index_default<Private, D: DB>(
        self,
        frame: RecordingFrame<Private, D>,
        position: crate::BoundedUint<{ u64::MAX as u128 }>,
    ) -> Result<RecordingFrame<Private, D>, CompactError>
    where
        T: Default,
    {
        if HISTORIC {
            frame.insert_historic_merkle_index_default::<T>(self.path, position)
        } else {
            frame.insert_merkle_index_default::<T>(self.path, position)
        }
    }

    pub fn reset_to_default<Private, D: DB>(
        self,
        context: CircuitContext<Private, D>,
    ) -> Result<CircuitResult<Private, (), D>, CompactError> {
        if HISTORIC {
            context.historic_reset_to_default(self.path, DEPTH)
        } else {
            context.merkle_reset_to_default(self.path, DEPTH)
        }
    }

    pub fn is_full<Private, D: DB>(
        self,
        context: CircuitContext<Private, D>,
    ) -> Result<CircuitResult<Private, bool, D>, CompactError> {
        if HISTORIC {
            context.historic_is_full(self.path, DEPTH)
        } else {
            context.merkle_is_full(self.path, DEPTH)
        }
    }

    /// Record a metered fullness read for proof and ledger replay.
    pub fn record_is_full<Private, D: DB>(
        self,
        frame: RecordingFrame<Private, D>,
    ) -> Result<(RecordingFrame<Private, D>, bool), CompactError> {
        if HISTORIC {
            frame.historic_merkle_is_full(self.path, DEPTH)
        } else {
            frame.merkle_is_full(self.path, DEPTH)
        }
    }

    pub fn check_root<Private, D: DB, R: CellValue>(
        self,
        context: CircuitContext<Private, D>,
        root: R,
    ) -> Result<CircuitResult<Private, bool, D>, CompactError> {
        if HISTORIC {
            context.historic_check_root(self.path, root)
        } else {
            context.merkle_check_root(self.path, root)
        }
    }
}

impl<T: CellValue, const DEPTH: u8> MerkleSlot<T, DEPTH, false> {
    pub fn record_check_root<Private, D: DB, R: CellValue + Clone>(
        self,
        frame: RecordingFrame<Private, D>,
        root: R,
    ) -> Result<(RecordingFrame<Private, D>, bool), CompactError> {
        frame.merkle_check_root(self.path, root)
    }

    /// Inspect an already held plain tree, rejecting a mismatched ledger height.
    pub fn inspect<'a, D: DB>(
        self,
        state: &'a StateValue<D>,
    ) -> Result<PlainMerkleStateView<'a, T, DEPTH, D>, CompactError> {
        let view = merkle_tree_view_at_path(state, self.path)?;
        if view.height() != DEPTH {
            return Err(CompactError::InvalidLedgerCell(format!(
                "expected MerkleTree height {DEPTH}, got {}",
                view.height()
            )));
        }
        Ok(PlainMerkleStateView {
            view,
            leaf: PhantomData,
        })
    }

    /// Project the declared plain tree through the existing metered witness view.
    pub fn witness_view<'a, Root: CellValue, D: DB>(
        self,
        meter: &'a WitnessReadMeter<'a, D>,
    ) -> Result<MeteredMerkleTreeView<'a, Root, D>, CompactError> {
        metered_merkle_tree_view_at_path(meter, self.path, DEPTH)
    }

    /// Record a plain-tree reset at its declared path and depth.
    pub fn record_reset_to_default<Private, D: DB>(
        self,
        frame: RecordingFrame<Private, D>,
    ) -> Result<RecordingFrame<Private, D>, CompactError> {
        frame.reset_merkle_to_default(self.path, DEPTH)
    }

    /// Record a complete plain-tree append trace for replay and proof.
    pub fn record_insert<Private, D: DB>(
        self,
        frame: RecordingFrame<Private, D>,
        value: T,
    ) -> Result<RecordingFrame<Private, D>, CompactError> {
        frame.insert_merkle(self.path, value)
    }

    /// Record a plain Merkle `insertHash` using the declared tree path.
    pub fn record_insert_hash<Private, D: DB>(
        self,
        frame: RecordingFrame<Private, D>,
        hash: crate::FixedBytes<32>,
    ) -> Result<RecordingFrame<Private, D>, CompactError> {
        frame.insert_merkle_hash(self.path, hash)
    }

    /// Record a plain Merkle `insertHashIndex` using its declared tree path.
    pub fn record_insert_hash_index<Private, D: DB>(
        self,
        frame: RecordingFrame<Private, D>,
        hash: crate::FixedBytes<32>,
        position: crate::BoundedUint<{ u64::MAX as u128 }>,
    ) -> Result<RecordingFrame<Private, D>, CompactError> {
        frame.insert_merkle_hash_index(self.path, hash, position)
    }
}

impl<T: CellValue, const DEPTH: u8> MerkleSlot<T, DEPTH, true> {
    /// Record the depth-specific historic reset and blank-root history seed.
    pub fn record_reset_to_default<Private, D: DB>(
        self,
        frame: RecordingFrame<Private, D>,
    ) -> Result<RecordingFrame<Private, D>, CompactError> {
        frame.reset_historic_merkle_to_default(self.path, DEPTH)
    }

    /// Inspect an already held historic tree, rejecting a mismatched ledger height.
    pub fn inspect<'a, D: DB>(
        self,
        state: &'a StateValue<D>,
    ) -> Result<HistoricMerkleStateView<'a, T, DEPTH, D>, CompactError> {
        let view = historic_merkle_tree_view_at_path(state, self.path)?;
        if view.height() != DEPTH {
            return Err(CompactError::InvalidLedgerCell(format!(
                "expected HistoricMerkleTree height {DEPTH}, got {}",
                view.height()
            )));
        }
        Ok(HistoricMerkleStateView {
            view,
            leaf: PhantomData,
        })
    }

    /// Project the declared historic tree through the existing metered witness view.
    pub fn witness_view<'a, Root: CellValue, D: DB>(
        self,
        meter: &'a WitnessReadMeter<'a, D>,
    ) -> Result<MeteredHistoricMerkleTreeView<'a, Root, D>, CompactError> {
        metered_historic_merkle_tree_view_at_path(meter, self.path, DEPTH)
    }

    /// Record a complete historic-tree append, including the root-history update.
    pub fn record_insert<Private, D: DB>(
        self,
        frame: RecordingFrame<Private, D>,
        value: T,
    ) -> Result<RecordingFrame<Private, D>, CompactError> {
        frame.insert_historic_merkle(self.path, value)
    }

    /// Record historic `insertHash`, including the root-history update.
    pub fn record_insert_hash<Private, D: DB>(
        self,
        frame: RecordingFrame<Private, D>,
        hash: crate::FixedBytes<32>,
    ) -> Result<RecordingFrame<Private, D>, CompactError> {
        frame.insert_historic_merkle_hash(self.path, hash)
    }

    /// Record historic `insertHashIndex`, including its root-history update.
    pub fn record_insert_hash_index<Private, D: DB>(
        self,
        frame: RecordingFrame<Private, D>,
        hash: crate::FixedBytes<32>,
        position: crate::BoundedUint<{ u64::MAX as u128 }>,
    ) -> Result<RecordingFrame<Private, D>, CompactError> {
        frame.insert_historic_merkle_hash_index(self.path, hash, position)
    }

    pub fn reset_history<Private, D: DB>(
        self,
        context: CircuitContext<Private, D>,
    ) -> Result<CircuitResult<Private, (), D>, CompactError> {
        context.historic_reset_history(self.path)
    }

    /// Record the same ledger-8 reset-history VM program as the native call.
    pub fn record_reset_history<Private, D: DB>(
        self,
        frame: RecordingFrame<Private, D>,
    ) -> Result<RecordingFrame<Private, D>, CompactError> {
        frame.reset_historic_merkle_history(self.path)
    }
}

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

    pub fn read<Private, D: DB>(
        self,
        context: CircuitContext<Private, D>,
    ) -> Result<CircuitResult<Private, u64, D>, CompactError> {
        context.read_cell_at_path(self.path)
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
        frame.read_cell(self.path)
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

/// A ledger Map nested as another Map's value, rather than a scalar `CellValue`.
///
/// This marker has no value codec. It lets a generated slot name the nested
/// structure while keeping scalar lookup and insert methods unavailable.
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

    pub fn remove<Private, D: DB>(
        self,
        context: CircuitContext<Private, D>,
        key: K,
    ) -> Result<CircuitResult<Private, (), D>, CompactError> {
        context.remove_map(self.path, key)
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

    pub fn record_remove<Private, D: DB>(
        self,
        frame: RecordingFrame<Private, D>,
        key: K,
    ) -> Result<RecordingFrame<Private, D>, CompactError> {
        frame.remove_map(self.path, key)
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
