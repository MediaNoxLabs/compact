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

//! Typed plain and historic Merkle slots with their local state projections.

use std::marker::PhantomData;

use midnight_base_crypto::repr::BinaryHashRepr;

use crate::CompactError;
use crate::context::{CircuitContext, CircuitResult, WitnessReadMeter};
use crate::ledger::{
    CellValue, DB, HistoricMerkleTreeView, MerklePath, MerkleTreeDigest, MerkleTreeView,
    MeteredHistoricMerkleTreeView, MeteredMerkleTreeView, StateValue,
    historic_merkle_tree_view_at_path, merkle_tree_view_at_path,
    metered_historic_merkle_tree_view_at_path, metered_merkle_tree_view_at_path,
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
    /// Record membership in the declared historic root map.
    pub fn record_check_root<Private, D: DB, R: CellValue + Clone>(
        self,
        frame: RecordingFrame<Private, D>,
        root: R,
    ) -> Result<(RecordingFrame<Private, D>, bool), CompactError> {
        frame.historic_merkle_check_root(self.path, root)
    }

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
