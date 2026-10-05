// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//  http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Current and historic Merkle ledger state and VM programs.

use super::{
    CellValue, CompactError, LedgerArray, LedgerHashMap, LedgerPath, MerklePath, QueryContext,
    QueryResults, StateValue, TranscriptRejected, aligned_cell_value, constructor_cell,
    constructor_map, decode_last_read, field_at_path, path_keys, read_cell,
};
use crate::context::WitnessReadMeter;
use crate::{BoundedUint, FixedBytes};
use midnight_base_crypto::cost_model::RunningCost;
use midnight_base_crypto::fab::AlignedValue;
use midnight_base_crypto::repr::BinaryHashRepr;
use midnight_onchain_vm::cost_model::CostModel;
use midnight_onchain_vm::ops::{Key, Op};
use midnight_onchain_vm::result_mode::{ResultMode, ResultModeGather, ResultModeVerify};
use midnight_storage::arena::Sp;
use midnight_storage::db::DB;
use midnight_transient_crypto::fab::ValueReprAlignedValue;
use midnight_transient_crypto::hash::HashOutput;
use midnight_transient_crypto::merkle_tree::{MerkleTree, leaf_hash};
use std::marker::PhantomData;
use std::ops::Deref;

/// Compact's plain MerkleTree stores the bounded tree and first free index.
pub fn constructor_merkle_tree<D: DB>(depth: u8) -> StateValue<D> {
    let tree: MerkleTree<(), D> = MerkleTree::blank(depth).rehash();
    StateValue::Array(vec![StateValue::BoundedMerkleTree(tree), constructor_cell(0_u64)].into())
}

/// Read-only witness projection of a plain MerkleTree.
pub struct MerkleTreeView<'a, D: DB> {
    fields: &'a LedgerArray<StateValue<D>, D>,
}

/// Witness-facing Merkle view. Structural methods are local; Compact `read`
/// methods execute and charge the canonical ledger VM programs.
pub struct MeteredMerkleTreeView<'a, Root, D: DB> {
    view: MerkleTreeView<'a, D>,
    meter: &'a WitnessReadMeter<'a, D>,
    path: LedgerPath,
    depth: u8,
    root: PhantomData<Root>,
}

pub fn metered_merkle_tree_view_at_path<'a, Root: CellValue, D: DB>(
    meter: &'a WitnessReadMeter<'a, D>,
    path: &[u8],
    depth: u8,
) -> Result<MeteredMerkleTreeView<'a, Root, D>, CompactError> {
    Ok(MeteredMerkleTreeView {
        view: merkle_tree_view_at_path(meter.state(), path)?,
        meter,
        path: path.into(),
        depth,
        root: PhantomData,
    })
}

impl<'a, Root, D: DB> Deref for MeteredMerkleTreeView<'a, Root, D> {
    type Target = MerkleTreeView<'a, D>;

    fn deref(&self) -> &Self::Target {
        &self.view
    }
}

impl<Root: CellValue, D: DB> MeteredMerkleTreeView<'_, Root, D> {
    pub fn is_full(&self) -> Result<bool, CompactError> {
        self.meter
            .read_merkle_is_full(self.path.as_slice(), self.depth)
    }

    pub fn check_root(&self, root: Root) -> Result<bool, CompactError> {
        self.meter
            .read_merkle_check_root(self.path.as_slice(), root)
    }
}

pub fn merkle_tree_view_at_path<'a, D: DB>(
    state: &'a StateValue<D>,
    path: &[u8],
) -> Result<MerkleTreeView<'a, D>, CompactError> {
    let StateValue::Array(fields) = field_at_path(state, path)? else {
        return Err(CompactError::InvalidLedgerCell(
            "expected MerkleTree array".into(),
        ));
    };
    if fields.len() != 2 || !matches!(fields.get(0), Some(StateValue::BoundedMerkleTree(_))) {
        return Err(CompactError::InvalidLedgerCell(
            "invalid MerkleTree layout".into(),
        ));
    }
    Ok(MerkleTreeView { fields })
}

impl<D: DB> MerkleTreeView<'_, D> {
    /// Height stored by the ledger tree, independent of any declaration.
    pub fn height(&self) -> u8 {
        bounded_tree(self.fields).height()
    }

    pub fn first_free(&self) -> Result<BoundedUint<{ u64::MAX as u128 }>, CompactError> {
        let value = read_cell::<u64, _>(self.fields.get(1).expect("tree shape checked"))?;
        BoundedUint::new(value as u128)
    }

    pub fn root(&self) -> Option<midnight_transient_crypto::merkle_tree::MerkleTreeDigest> {
        bounded_tree(self.fields).root()
    }

    pub fn path_for_leaf<T: BinaryHashRepr>(
        &self,
        index: u64,
        leaf: T,
    ) -> Result<MerklePath<T>, CompactError> {
        path_for_leaf(self.fields, index, leaf)
    }

    pub fn find_path_for_leaf<T: BinaryHashRepr>(&self, leaf: T) -> Option<MerklePath<T>> {
        bounded_tree(self.fields).find_path_for_leaf(leaf)
    }
}

fn bounded_tree<D: DB>(fields: &LedgerArray<StateValue<D>, D>) -> &MerkleTree<(), D> {
    let Some(StateValue::BoundedMerkleTree(tree)) = fields.get(0) else {
        unreachable!("tree shape checked")
    };
    tree
}

// Match ledger-8 pathForLeaf: the caller supplies the leaf, and only the index
// is checked. A mismatched leaf yields a path whose computed root differs.
fn path_for_leaf<T: BinaryHashRepr, D: DB>(
    fields: &LedgerArray<StateValue<D>, D>,
    index: u64,
    leaf: T,
) -> Result<MerklePath<T>, CompactError> {
    bounded_tree(fields)
        .path_for_leaf(index, leaf)
        .map_err(|error| CompactError::InvalidLedgerCell(error.to_string()))
}

/// Compact's HistoricMerkleTree seed includes the blank root in its history.
pub fn constructor_historic_merkle_tree<D: DB>(depth: u8) -> StateValue<D> {
    let tree: MerkleTree<(), D> = MerkleTree::blank(depth).rehash();
    let root = tree.root().expect("a rehashed blank tree has a root");
    let history = LedgerHashMap::new().insert(AlignedValue::from(root), StateValue::Null);
    StateValue::Array(
        vec![
            StateValue::BoundedMerkleTree(tree),
            constructor_cell(0_u64),
            StateValue::Map(history),
        ]
        .into(),
    )
}

fn blank_historic_merkle_tree<D: DB>(depth: u8) -> StateValue<D> {
    let tree: MerkleTree<(), D> = MerkleTree::blank(depth).rehash();
    StateValue::Array(
        vec![
            StateValue::BoundedMerkleTree(tree),
            constructor_cell(0_u64),
            StateValue::Map(LedgerHashMap::new()),
        ]
        .into(),
    )
}

/// Read-only witness projection of the ledger's tree, next index, and root history.
pub struct HistoricMerkleTreeView<'a, D: DB> {
    fields: &'a LedgerArray<StateValue<D>, D>,
}

/// Historic Merkle witness view with local tree/history projections and
/// charged VM reads for fullness and historical root membership.
pub struct MeteredHistoricMerkleTreeView<'a, Root, D: DB> {
    view: HistoricMerkleTreeView<'a, D>,
    meter: &'a WitnessReadMeter<'a, D>,
    path: LedgerPath,
    depth: u8,
    root: PhantomData<Root>,
}

pub fn metered_historic_merkle_tree_view_at_path<'a, Root: CellValue, D: DB>(
    meter: &'a WitnessReadMeter<'a, D>,
    path: &[u8],
    depth: u8,
) -> Result<MeteredHistoricMerkleTreeView<'a, Root, D>, CompactError> {
    Ok(MeteredHistoricMerkleTreeView {
        view: historic_merkle_tree_view_at_path(meter.state(), path)?,
        meter,
        path: path.into(),
        depth,
        root: PhantomData,
    })
}

impl<'a, Root, D: DB> Deref for MeteredHistoricMerkleTreeView<'a, Root, D> {
    type Target = HistoricMerkleTreeView<'a, D>;

    fn deref(&self) -> &Self::Target {
        &self.view
    }
}

impl<Root: CellValue, D: DB> MeteredHistoricMerkleTreeView<'_, Root, D> {
    pub fn is_full(&self) -> Result<bool, CompactError> {
        self.meter
            .read_historic_merkle_is_full(self.path.as_slice(), self.depth)
    }

    pub fn check_root(&self, root: Root) -> Result<bool, CompactError> {
        self.meter
            .read_historic_merkle_check_root(self.path.as_slice(), root)
    }
}

pub fn historic_merkle_tree_view_at_path<'a, D: DB>(
    state: &'a StateValue<D>,
    path: &[u8],
) -> Result<HistoricMerkleTreeView<'a, D>, CompactError> {
    let StateValue::Array(fields) = field_at_path(state, path)? else {
        return Err(CompactError::InvalidLedgerCell(
            "expected HistoricMerkleTree array".into(),
        ));
    };
    if fields.len() != 3
        || !matches!(fields.get(0), Some(StateValue::BoundedMerkleTree(_)))
        || !matches!(fields.get(2), Some(StateValue::Map(_)))
    {
        return Err(CompactError::InvalidLedgerCell(
            "invalid HistoricMerkleTree layout".into(),
        ));
    }
    Ok(HistoricMerkleTreeView { fields })
}

impl<D: DB> HistoricMerkleTreeView<'_, D> {
    /// Height stored by the ledger tree, independent of any declaration.
    pub fn height(&self) -> u8 {
        bounded_tree(self.fields).height()
    }

    pub fn first_free(&self) -> Result<BoundedUint<{ u64::MAX as u128 }>, CompactError> {
        let value = read_cell::<u64, _>(self.fields.get(1).expect("tree shape checked"))?;
        BoundedUint::new(value as u128)
    }

    pub fn root(&self) -> Option<midnight_transient_crypto::merkle_tree::MerkleTreeDigest> {
        bounded_tree(self.fields).root()
    }

    pub fn path_for_leaf<T: BinaryHashRepr>(
        &self,
        index: u64,
        leaf: T,
    ) -> Result<MerklePath<T>, CompactError> {
        path_for_leaf(self.fields, index, leaf)
    }

    pub fn find_path_for_leaf<T: BinaryHashRepr>(&self, leaf: T) -> Option<MerklePath<T>> {
        bounded_tree(self.fields).find_path_for_leaf(leaf)
    }

    pub fn history(
        &self,
    ) -> Result<Vec<midnight_transient_crypto::merkle_tree::MerkleTreeDigest>, CompactError> {
        let Some(StateValue::Map(history)) = self.fields.get(2) else {
            unreachable!("tree shape checked")
        };
        history
            .iter()
            .map(|entry| {
                midnight_transient_crypto::merkle_tree::MerkleTreeDigest::try_from(&*entry.0.value)
                    .map_err(|error| CompactError::InvalidLedgerCell(error.to_string()))
            })
            .collect()
    }

    pub fn contains_root(
        &self,
        root: midnight_transient_crypto::merkle_tree::MerkleTreeDigest,
    ) -> bool {
        let Some(StateValue::Map(history)) = self.fields.get(2) else {
            unreachable!("tree shape checked")
        };
        history.contains_key(&AlignedValue::from(root))
    }
}

pub fn merkle_insert_index_default<T: CellValue + Default, D: DB>(
    context: &QueryContext<D>,
    path: impl Into<LedgerPath>,
    position: u64,
    gas_limit: Option<RunningCost>,
    cost_model: &CostModel,
) -> Result<QueryResults<ResultModeVerify, D>, TranscriptRejected<D>> {
    merkle_insert_index_hashed(
        context,
        path,
        AlignedValue::from(leaf_hash_for(T::default())),
        position,
        MerkleHistory::CurrentOnly,
        gas_limit,
        cost_model,
    )
}

pub fn merkle_insert_index<T: CellValue, D: DB>(
    context: &QueryContext<D>,
    path: impl Into<LedgerPath>,
    item: T,
    position: u64,
    gas_limit: Option<RunningCost>,
    cost_model: &CostModel,
) -> Result<QueryResults<ResultModeVerify, D>, TranscriptRejected<D>> {
    merkle_insert_index_hashed(
        context,
        path,
        AlignedValue::from(leaf_hash_for(item)),
        position,
        MerkleHistory::CurrentOnly,
        gas_limit,
        cost_model,
    )
}

pub fn merkle_insert_hash_index<D: DB>(
    context: &QueryContext<D>,
    path: impl Into<LedgerPath>,
    hash: FixedBytes<32>,
    position: u64,
    gas_limit: Option<RunningCost>,
    cost_model: &CostModel,
) -> Result<QueryResults<ResultModeVerify, D>, TranscriptRejected<D>> {
    merkle_insert_index_hashed(
        context,
        path,
        aligned_cell_value(hash),
        position,
        MerkleHistory::CurrentOnly,
        gas_limit,
        cost_model,
    )
}

pub fn merkle_insert<T: CellValue, D: DB>(
    context: &QueryContext<D>,
    path: impl Into<LedgerPath>,
    item: T,
    gas_limit: Option<RunningCost>,
    cost_model: &CostModel,
) -> Result<QueryResults<ResultModeVerify, D>, TranscriptRejected<D>> {
    merkle_insert_hashed(
        context,
        path,
        AlignedValue::from(leaf_hash_for(item)),
        MerkleHistory::CurrentOnly,
        gas_limit,
        cost_model,
    )
}

pub fn merkle_insert_hash<D: DB>(
    context: &QueryContext<D>,
    path: impl Into<LedgerPath>,
    hash: FixedBytes<32>,
    gas_limit: Option<RunningCost>,
    cost_model: &CostModel,
) -> Result<QueryResults<ResultModeVerify, D>, TranscriptRejected<D>> {
    merkle_insert_hashed(
        context,
        path,
        aligned_cell_value(hash),
        MerkleHistory::CurrentOnly,
        gas_limit,
        cost_model,
    )
}

/// Execute ledger-8's HistoricMerkleTree.insertIndexDefault VM program.
pub fn historic_insert_index_default<T: CellValue + Default, D: DB>(
    context: &QueryContext<D>,
    path: impl Into<LedgerPath>,
    position: u64,
    gas_limit: Option<RunningCost>,
    cost_model: &CostModel,
) -> Result<QueryResults<ResultModeVerify, D>, TranscriptRejected<D>> {
    merkle_insert_index_hashed(
        context,
        path,
        AlignedValue::from(leaf_hash_for(T::default())),
        position,
        MerkleHistory::Historic,
        gas_limit,
        cost_model,
    )
}

fn leaf_hash_for<T: CellValue>(item: T) -> HashOutput {
    let aligned = AlignedValue::new(item.into(), T::alignment())
        .expect("a typed Compact value must fit its alignment");
    leaf_hash(&ValueReprAlignedValue(aligned))
}

/// Insert a typed leaf at a given index and record the resulting root.
pub fn historic_insert_index<T: CellValue, D: DB>(
    context: &QueryContext<D>,
    path: impl Into<LedgerPath>,
    item: T,
    position: u64,
    gas_limit: Option<RunningCost>,
    cost_model: &CostModel,
) -> Result<QueryResults<ResultModeVerify, D>, TranscriptRejected<D>> {
    merkle_insert_index_hashed(
        context,
        path,
        AlignedValue::from(leaf_hash_for(item)),
        position,
        MerkleHistory::Historic,
        gas_limit,
        cost_model,
    )
}

/// Insert a supplied 32-byte leaf hash at a given index.
pub fn historic_insert_hash_index<D: DB>(
    context: &QueryContext<D>,
    path: impl Into<LedgerPath>,
    hash: FixedBytes<32>,
    position: u64,
    gas_limit: Option<RunningCost>,
    cost_model: &CostModel,
) -> Result<QueryResults<ResultModeVerify, D>, TranscriptRejected<D>> {
    merkle_insert_index_hashed(
        context,
        path,
        aligned_cell_value(hash),
        position,
        MerkleHistory::Historic,
        gas_limit,
        cost_model,
    )
}

#[derive(Clone, Copy)]
enum MerkleHistory {
    CurrentOnly,
    Historic,
}

fn merkle_insert_index_hashed<D: DB>(
    context: &QueryContext<D>,
    path: impl Into<LedgerPath>,
    hash: AlignedValue,
    position: u64,
    history: MerkleHistory,
    gas_limit: Option<RunningCost>,
    cost_model: &CostModel,
) -> Result<QueryResults<ResultModeVerify, D>, TranscriptRejected<D>> {
    let program = merkle_insert_index_hashed_program::<D>(path.into(), hash, position, history);
    context.query(&program, gas_limit, cost_model)
}

fn merkle_insert_index_hashed_program<D: DB>(
    path: LedgerPath,
    hash: AlignedValue,
    position: u64,
    history: MerkleHistory,
) -> Vec<Op<ResultModeVerify, D>> {
    let keys = path_keys(path.as_slice());
    let index_key = |index| vec![Key::Value(AlignedValue::from(index))].into();
    let mut program = vec![
        Op::Idx {
            cached: false,
            push_path: true,
            path: keys.into(),
        },
        Op::Idx {
            cached: false,
            push_path: true,
            path: index_key(0_u8),
        },
        Op::Push {
            storage: false,
            value: constructor_cell(position),
        },
        Op::Push {
            storage: true,
            value: StateValue::Cell(Sp::new(hash)),
        },
        Op::Ins {
            cached: false,
            n: 2,
        },
        Op::Idx {
            cached: false,
            push_path: true,
            path: index_key(1_u8),
        },
        Op::Push {
            storage: false,
            value: constructor_cell(position),
        },
        Op::Addi { immediate: 1 },
        Op::Dup { n: 1 },
        Op::Dup { n: 1 },
        Op::Lt,
        Op::Branch { skip: 2 },
        Op::Pop,
        Op::Jmp { skip: 2 },
        Op::Swap { n: 0 },
        Op::Pop,
        Op::Ins {
            cached: false,
            n: 1,
        },
    ];
    if matches!(history, MerkleHistory::Historic) {
        program.extend([
            Op::Idx {
                cached: false,
                push_path: true,
                path: index_key(2_u8),
            },
            Op::Dup { n: 2 },
            Op::Idx {
                cached: false,
                push_path: false,
                path: index_key(0_u8),
            },
            Op::Root,
            Op::Push {
                storage: true,
                value: StateValue::Null,
            },
            Op::Ins {
                cached: false,
                n: 1,
            },
            Op::Ins {
                cached: true,
                n: path.as_slice().len() as u8 + 1,
            },
        ]);
    } else {
        program.push(Op::Ins {
            cached: true,
            n: path.as_slice().len() as u8,
        });
    }
    program
}

/// Build ledger-8's typed indexed insertion program for a recorded plain tree.
pub(crate) fn merkle_insert_index_program<T: CellValue, D: DB>(
    path: impl Into<LedgerPath>,
    item: T,
    position: u64,
) -> Vec<Op<ResultModeVerify, D>> {
    merkle_insert_index_hashed_program(
        path.into(),
        AlignedValue::from(leaf_hash_for(item)),
        position,
        MerkleHistory::CurrentOnly,
    )
}

/// Build native `insertHashIndex`'s verifying program from an already hashed leaf.
pub(crate) fn merkle_insert_hash_index_program<D: DB>(
    path: impl Into<LedgerPath>,
    hash: FixedBytes<32>,
    position: u64,
) -> Vec<Op<ResultModeVerify, D>> {
    merkle_insert_index_hashed_program(
        path.into(),
        aligned_cell_value(hash),
        position,
        MerkleHistory::CurrentOnly,
    )
}

/// Build ledger-8's default-leaf indexed insertion program for recording.
pub(crate) fn merkle_insert_index_default_program<T: CellValue + Default, D: DB>(
    path: impl Into<LedgerPath>,
    position: u64,
) -> Vec<Op<ResultModeVerify, D>> {
    merkle_insert_index_program(path, T::default(), position)
}

pub(crate) fn historic_merkle_insert_index_program<T: CellValue, D: DB>(
    path: impl Into<LedgerPath>,
    item: T,
    position: u64,
) -> Vec<Op<ResultModeVerify, D>> {
    merkle_insert_index_hashed_program(
        path.into(),
        AlignedValue::from(leaf_hash_for(item)),
        position,
        MerkleHistory::Historic,
    )
}

/// Build native historic `insertHashIndex`'s verifying program with root history.
pub(crate) fn historic_merkle_insert_hash_index_program<D: DB>(
    path: impl Into<LedgerPath>,
    hash: FixedBytes<32>,
    position: u64,
) -> Vec<Op<ResultModeVerify, D>> {
    merkle_insert_index_hashed_program(
        path.into(),
        aligned_cell_value(hash),
        position,
        MerkleHistory::Historic,
    )
}

pub(crate) fn historic_merkle_insert_index_default_program<T: CellValue + Default, D: DB>(
    path: impl Into<LedgerPath>,
    position: u64,
) -> Vec<Op<ResultModeVerify, D>> {
    historic_merkle_insert_index_program(path, T::default(), position)
}

/// Insert a typed leaf at the first free index and record the resulting root.
pub fn historic_insert<T: CellValue, D: DB>(
    context: &QueryContext<D>,
    path: impl Into<LedgerPath>,
    item: T,
    gas_limit: Option<RunningCost>,
    cost_model: &CostModel,
) -> Result<QueryResults<ResultModeVerify, D>, TranscriptRejected<D>> {
    merkle_insert_hashed(
        context,
        path,
        AlignedValue::from(leaf_hash_for(item)),
        MerkleHistory::Historic,
        gas_limit,
        cost_model,
    )
}

/// Insert a supplied 32-byte leaf hash at the first free index.
pub fn historic_insert_hash<D: DB>(
    context: &QueryContext<D>,
    path: impl Into<LedgerPath>,
    hash: FixedBytes<32>,
    gas_limit: Option<RunningCost>,
    cost_model: &CostModel,
) -> Result<QueryResults<ResultModeVerify, D>, TranscriptRejected<D>> {
    merkle_insert_hashed(
        context,
        path,
        aligned_cell_value(hash),
        MerkleHistory::Historic,
        gas_limit,
        cost_model,
    )
}

fn merkle_insert_hashed<D: DB>(
    context: &QueryContext<D>,
    path: impl Into<LedgerPath>,
    hash: AlignedValue,
    history: MerkleHistory,
    gas_limit: Option<RunningCost>,
    cost_model: &CostModel,
) -> Result<QueryResults<ResultModeVerify, D>, TranscriptRejected<D>> {
    let program = merkle_insert_hashed_program::<D>(path.into(), hash, history);
    context.query(&program, gas_limit, cost_model)
}

fn merkle_insert_hashed_program<D: DB>(
    path: LedgerPath,
    hash: AlignedValue,
    history: MerkleHistory,
) -> Vec<Op<ResultModeVerify, D>> {
    let keys = path_keys(path.as_slice());
    let index_key = |index| vec![Key::Value(AlignedValue::from(index))].into();
    let mut program = vec![
        Op::Idx {
            cached: false,
            push_path: true,
            path: keys.into(),
        },
        Op::Idx {
            cached: false,
            push_path: true,
            path: index_key(0_u8),
        },
        Op::Dup { n: 2 },
        Op::Idx {
            cached: false,
            push_path: false,
            path: index_key(1_u8),
        },
        Op::Push {
            storage: true,
            value: StateValue::Cell(Sp::new(hash)),
        },
        Op::Ins {
            cached: false,
            n: 1,
        },
        Op::Ins { cached: true, n: 1 },
        Op::Idx {
            cached: false,
            push_path: true,
            path: index_key(1_u8),
        },
        Op::Addi { immediate: 1 },
    ];
    if matches!(history, MerkleHistory::Historic) {
        program.extend([
            Op::Ins { cached: true, n: 1 },
            Op::Idx {
                cached: false,
                push_path: true,
                path: index_key(2_u8),
            },
            Op::Dup { n: 2 },
            Op::Idx {
                cached: false,
                push_path: false,
                path: index_key(0_u8),
            },
            Op::Root,
            Op::Push {
                storage: true,
                value: StateValue::Null,
            },
            Op::Ins {
                cached: false,
                n: 1,
            },
            Op::Ins {
                cached: true,
                n: path.as_slice().len() as u8 + 1,
            },
        ]);
    } else {
        program.push(Op::Ins {
            cached: true,
            n: path.as_slice().len() as u8 + 1,
        });
    }
    program
}

/// Build the same verifying program used by native append, for a recorded circuit.
pub(crate) fn merkle_insert_program<T: CellValue, D: DB>(
    path: impl Into<LedgerPath>,
    item: T,
) -> Vec<Op<ResultModeVerify, D>> {
    merkle_insert_hashed_program(
        path.into(),
        AlignedValue::from(leaf_hash_for(item)),
        MerkleHistory::CurrentOnly,
    )
}

/// Build native `insertHash`'s verifying program from the supplied digest.
/// The digest is already a leaf hash; hashing it again changes the tree.
pub(crate) fn merkle_insert_hash_program<D: DB>(
    path: impl Into<LedgerPath>,
    hash: FixedBytes<32>,
) -> Vec<Op<ResultModeVerify, D>> {
    merkle_insert_hashed_program(
        path.into(),
        aligned_cell_value(hash),
        MerkleHistory::CurrentOnly,
    )
}

/// Build native historic `insertHash`'s verifying program, including root history.
pub(crate) fn historic_merkle_insert_hash_program<D: DB>(
    path: impl Into<LedgerPath>,
    hash: FixedBytes<32>,
) -> Vec<Op<ResultModeVerify, D>> {
    merkle_insert_hashed_program(
        path.into(),
        aligned_cell_value(hash),
        MerkleHistory::Historic,
    )
}

/// Build the native historic append program, including its root-history update.
pub(crate) fn historic_merkle_insert_program<T: CellValue, D: DB>(
    path: impl Into<LedgerPath>,
    item: T,
) -> Vec<Op<ResultModeVerify, D>> {
    merkle_insert_hashed_program(
        path.into(),
        AlignedValue::from(leaf_hash_for(item)),
        MerkleHistory::Historic,
    )
}

/// Keep only the current root in a HistoricMerkleTree's history.
pub fn historic_reset_history<D: DB>(
    context: &QueryContext<D>,
    path: impl Into<LedgerPath>,
    gas_limit: Option<RunningCost>,
    cost_model: &CostModel,
) -> Result<QueryResults<ResultModeVerify, D>, TranscriptRejected<D>> {
    let program = historic_reset_history_program::<D>(path.into());
    context.query(&program, gas_limit, cost_model)
}

pub(crate) fn historic_reset_history_program<D: DB>(
    path: LedgerPath,
) -> Vec<Op<ResultModeVerify, D>> {
    let keys = path_keys(path.as_slice());
    let index_key = |index| vec![Key::Value(AlignedValue::from(index))].into();
    vec![
        Op::Idx {
            cached: false,
            push_path: true,
            path: keys.into(),
        },
        Op::Push {
            storage: false,
            value: constructor_cell(2_u8),
        },
        Op::Push {
            storage: true,
            value: constructor_map(),
        },
        Op::Dup { n: 2 },
        Op::Idx {
            cached: false,
            push_path: false,
            path: index_key(0_u8),
        },
        Op::Root,
        Op::Push {
            storage: true,
            value: StateValue::Null,
        },
        Op::Ins {
            cached: true,
            n: path.as_slice().len() as u8 + 2,
        },
    ]
}

/// Reset a HistoricMerkleTree and seed its new blank root through the VM.
pub fn historic_reset_to_default<D: DB>(
    context: &QueryContext<D>,
    path: impl Into<LedgerPath>,
    depth: u8,
    gas_limit: Option<RunningCost>,
    cost_model: &CostModel,
) -> Result<QueryResults<ResultModeVerify, D>, TranscriptRejected<D>> {
    context.query(
        &historic_reset_to_default_program(path, depth),
        gas_limit,
        cost_model,
    )
}

/// Canonical historic reset program shared by native and recorded execution.
pub(crate) fn historic_reset_to_default_program<D: DB>(
    path: impl Into<LedgerPath>,
    depth: u8,
) -> Vec<Op<ResultModeVerify, D>> {
    let path = path.into();
    let parts = path.as_slice();
    let (&field_index, parent) = parts
        .split_last()
        .expect("ledger path contains a field index");
    let index_key = |index| vec![Key::Value(AlignedValue::from(index))].into();
    let mut program = Vec::new();
    if !parent.is_empty() {
        program.push(Op::Idx {
            cached: false,
            push_path: true,
            path: path_keys(parent).into(),
        });
    }
    program.extend([
        Op::Push {
            storage: false,
            value: constructor_cell(field_index),
        },
        Op::Push {
            storage: true,
            value: blank_historic_merkle_tree(depth),
        },
        Op::Idx {
            cached: false,
            push_path: true,
            path: index_key(2_u8),
        },
        Op::Dup { n: 2 },
        Op::Idx {
            cached: false,
            push_path: false,
            path: index_key(0_u8),
        },
        Op::Root,
        Op::Push {
            storage: true,
            value: StateValue::Null,
        },
        Op::Ins { cached: true, n: 2 },
        Op::Ins {
            cached: false,
            n: 1,
        },
    ]);
    if !parent.is_empty() {
        program.push(Op::Ins {
            cached: true,
            n: parent.len() as u8,
        });
    }
    program
}

/// Reset a plain MerkleTree through the canonical ledger VM sequence.
pub fn merkle_reset_to_default<D: DB>(
    context: &QueryContext<D>,
    path: impl Into<LedgerPath>,
    depth: u8,
    gas_limit: Option<RunningCost>,
    cost_model: &CostModel,
) -> Result<QueryResults<ResultModeVerify, D>, TranscriptRejected<D>> {
    context.query(&merkle_reset_program(path, depth), gas_limit, cost_model)
}

/// Canonical plain-tree reset program shared by native and recorded execution.
pub(crate) fn merkle_reset_program<D: DB>(
    path: impl Into<LedgerPath>,
    depth: u8,
) -> Vec<Op<ResultModeVerify, D>> {
    let path = path.into();
    let (&field_index, parent) = path
        .as_slice()
        .split_last()
        .expect("ledger path contains a field index");
    let mut program = Vec::new();
    if !parent.is_empty() {
        program.push(Op::Idx {
            cached: false,
            push_path: true,
            path: path_keys(parent).into(),
        });
    }
    program.extend([
        Op::Push {
            storage: false,
            value: constructor_cell(field_index),
        },
        Op::Push {
            storage: true,
            value: constructor_merkle_tree(depth),
        },
        Op::Ins {
            cached: false,
            n: 1,
        },
    ]);
    if !parent.is_empty() {
        program.push(Op::Ins {
            cached: true,
            n: parent.len() as u8,
        });
    }
    program
}

/// Ask the ledger VM whether the next free index has reached tree capacity.
pub fn historic_is_full<D: DB>(
    context: &QueryContext<D>,
    path: impl Into<LedgerPath>,
    depth: u8,
    gas_limit: Option<RunningCost>,
    cost_model: &CostModel,
) -> Result<(QueryResults<ResultModeGather, D>, bool), CompactError> {
    let program = is_full_program::<ResultModeGather, D>(path.into(), depth, ())?;
    let result = context
        .query(&program, gas_limit, cost_model)
        .map_err(|error| CompactError::LedgerQueryRejected(format!("{error:?}")))?;
    let decoded = decode_last_read::<bool, D>(&result)?;
    Ok((result, decoded))
}

pub(crate) fn is_full_program<M: ResultMode<D>, D: DB>(
    path: LedgerPath,
    depth: u8,
    read_result: M::ReadResult,
) -> Result<Vec<Op<M, D>>, CompactError> {
    let capacity = 1_u64.checked_shl(depth as u32).ok_or_else(|| {
        CompactError::InvalidLedgerCell(format!("invalid HistoricMerkleTree depth {depth}"))
    })?;
    let index_key = vec![Key::Value(AlignedValue::from(1_u8))].into();
    Ok(vec![
        Op::Dup { n: 0 },
        Op::Idx {
            cached: false,
            push_path: false,
            path: path_keys(path.as_slice()).into(),
        },
        Op::Idx {
            cached: false,
            push_path: false,
            path: index_key,
        },
        Op::Push {
            storage: false,
            value: constructor_cell(capacity),
        },
        Op::Lt,
        Op::Neg,
        Op::Popeq {
            cached: true,
            result: read_result,
        },
    ])
}

pub fn merkle_is_full<D: DB>(
    context: &QueryContext<D>,
    path: impl Into<LedgerPath>,
    depth: u8,
    gas_limit: Option<RunningCost>,
    cost_model: &CostModel,
) -> Result<(QueryResults<ResultModeGather, D>, bool), CompactError> {
    historic_is_full(context, path, depth, gas_limit, cost_model)
}

/// Compare a supplied digest to the current root of a plain MerkleTree.
pub fn merkle_check_root<T: CellValue, D: DB>(
    context: &QueryContext<D>,
    path: impl Into<LedgerPath>,
    root: T,
    gas_limit: Option<RunningCost>,
    cost_model: &CostModel,
) -> Result<(QueryResults<ResultModeGather, D>, bool), CompactError> {
    let program = check_root_program::<T, ResultModeGather, D>(
        path.into(),
        root,
        MerkleHistory::CurrentOnly,
        (),
    );
    let result = context
        .query(&program, gas_limit, cost_model)
        .map_err(|error| CompactError::LedgerQueryRejected(format!("{error:?}")))?;
    let decoded = decode_last_read::<bool, D>(&result)?;
    Ok((result, decoded))
}

fn check_root_program<T: CellValue, M: ResultMode<D>, D: DB>(
    path: LedgerPath,
    root: T,
    history: MerkleHistory,
    read_result: M::ReadResult,
) -> Vec<Op<M, D>> {
    let historic = matches!(history, MerkleHistory::Historic);
    let index = if historic { 2_u8 } else { 0_u8 };
    let index_key = vec![Key::Value(AlignedValue::from(index))].into();
    let mut program = vec![
        Op::Dup { n: 0 },
        Op::Idx {
            cached: false,
            push_path: false,
            path: path_keys(path.as_slice()).into(),
        },
        Op::Idx {
            cached: false,
            push_path: false,
            path: index_key,
        },
    ];
    if historic {
        program.extend([
            Op::Push {
                storage: false,
                value: constructor_cell(root),
            },
            Op::Member,
        ]);
    } else {
        program.extend([
            Op::Root,
            Op::Push {
                storage: false,
                value: constructor_cell(root),
            },
            Op::Eq,
        ]);
    }
    program.push(Op::Popeq {
        cached: true,
        result: read_result,
    });
    program
}

pub(crate) fn merkle_check_root_verify_program<T: CellValue, D: DB>(
    path: LedgerPath,
    root: T,
    observed: AlignedValue,
) -> Vec<Op<ResultModeVerify, D>> {
    check_root_program(path, root, MerkleHistory::CurrentOnly, observed)
}

pub(crate) fn historic_check_root_verify_program<T: CellValue, D: DB>(
    path: LedgerPath,
    root: T,
    observed: AlignedValue,
) -> Vec<Op<ResultModeVerify, D>> {
    check_root_program(path, root, MerkleHistory::Historic, observed)
}

/// Query membership in a HistoricMerkleTree's root history.
pub fn historic_check_root<T: CellValue, D: DB>(
    context: &QueryContext<D>,
    path: impl Into<LedgerPath>,
    root: T,
    gas_limit: Option<RunningCost>,
    cost_model: &CostModel,
) -> Result<(QueryResults<ResultModeGather, D>, bool), CompactError> {
    let program = check_root_program::<T, ResultModeGather, D>(
        path.into(),
        root,
        MerkleHistory::Historic,
        (),
    );
    let result = context
        .query(&program, gas_limit, cost_model)
        .map_err(|error| CompactError::LedgerQueryRejected(format!("{error:?}")))?;
    let decoded = decode_last_read::<bool, D>(&result)?;
    Ok((result, decoded))
}

#[cfg(test)]
mod query_program_tests {
    use super::*;
    use crate::ledger::DefaultDB;
    use midnight_onchain_vm::result_mode::ResultMode;

    fn serialized_program<M: ResultMode<DefaultDB>>(
        program: &[Op<M, DefaultDB>],
    ) -> Vec<serde_json::Value>
    where
        Op<M, DefaultDB>: serde::Serialize,
    {
        program
            .iter()
            .map(|operation| serde_json::to_value(operation).unwrap())
            .collect()
    }

    fn tags(program: &[serde_json::Value]) -> Vec<String> {
        program
            .iter()
            .map(|value| match value {
                serde_json::Value::String(tag) => tag.clone(),
                serde_json::Value::Object(fields) if fields.len() == 1 => {
                    fields.keys().next().unwrap().clone()
                }
                other => panic!("unexpected serialized VM operation: {other}"),
            })
            .collect()
    }

    fn expected_tags(oracle: &serde_json::Value, call: &str) -> Vec<String> {
        let queries = oracle["nativeQueries"][call]["queries"].as_array().unwrap();
        assert_eq!(queries.len(), 1, "{call}: expected one ledger-8 query");
        queries[0]["opTags"]
            .as_array()
            .unwrap()
            .iter()
            .map(|tag| tag.as_str().unwrap().to_owned())
            .collect()
    }

    fn assert_program<M: ResultMode<DefaultDB>>(
        program: &[Op<M, DefaultDB>],
        oracle: &serde_json::Value,
        call: &str,
    ) where
        Op<M, DefaultDB>: serde::Serialize,
    {
        let queries = oracle["nativeQueries"][call]["queries"].as_array().unwrap();
        assert_eq!(queries.len(), 1, "{call}: expected one ledger-8 query");
        let actual = serialized_program(program);
        assert_eq!(tags(&actual), expected_tags(oracle, call), "{call}: tags");
        assert_eq!(
            actual.as_slice(),
            queries[0]["program"].as_array().unwrap().as_slice(),
            "{call}: VM operands"
        );
    }

    #[test]
    fn native_merkle_programs_match_captured_ledger8_operation_order() {
        let plain: serde_json::Value =
            serde_json::from_str(include_str!("../../tests/fixtures/merkle-tree-oracle.json"))
                .unwrap();
        let historic: serde_json::Value =
            serde_json::from_str(include_str!("../../tests/fixtures/hmt-insert-oracle.json"))
                .unwrap();
        let path = || LedgerPath::from(&[0_u8][..]);
        let hash = || AlignedValue::from(leaf_hash_for(BoundedUint::<255>::new(7).unwrap()));
        let plain_state = StateValue::Array(vec![constructor_merkle_tree::<DefaultDB>(3)].into());
        let plain_root = merkle_tree_view_at_path(&plain_state, &[0])
            .unwrap()
            .root()
            .unwrap();
        let historic_state =
            StateValue::Array(vec![constructor_historic_merkle_tree::<DefaultDB>(3)].into());
        let historic_root = historic_merkle_tree_view_at_path(&historic_state, &[0])
            .unwrap()
            .root()
            .unwrap();

        let full = is_full_program::<ResultModeGather, DefaultDB>(path(), 3, ()).unwrap();
        assert_program(&full, &plain, "fullAtInit");
        assert_program(&full, &historic, "fullAtInit");
        assert_program(
            &check_root_program::<_, ResultModeGather, DefaultDB>(
                path(),
                plain_root.0,
                MerkleHistory::CurrentOnly,
                (),
            ),
            &plain,
            "knownAtInit",
        );
        assert_program(
            &check_root_program::<_, ResultModeGather, DefaultDB>(
                path(),
                historic_root.0,
                MerkleHistory::Historic,
                (),
            ),
            &historic,
            "knownAtInit",
        );
        assert_program(
            &merkle_insert_hashed_program::<DefaultDB>(path(), hash(), MerkleHistory::CurrentOnly),
            &plain,
            "append7",
        );
        assert_program(
            &merkle_insert_hashed_program::<DefaultDB>(path(), hash(), MerkleHistory::Historic),
            &historic,
            "append7",
        );
        assert_program(
            &historic_reset_history_program::<DefaultDB>(path()),
            &historic,
            "forgetHistory",
        );
    }
}
