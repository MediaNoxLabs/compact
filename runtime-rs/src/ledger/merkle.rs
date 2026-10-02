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
use crate::{BoundedUint, FixedBytes};
use midnight_base_crypto::cost_model::RunningCost;
use midnight_base_crypto::fab::AlignedValue;
use midnight_base_crypto::repr::BinaryHashRepr;
use midnight_onchain_vm::cost_model::CostModel;
use midnight_onchain_vm::ops::{Key, Op};
use midnight_onchain_vm::result_mode::{ResultModeGather, ResultModeVerify};
use midnight_storage::arena::Sp;
use midnight_storage::db::DB;
use midnight_transient_crypto::fab::ValueReprAlignedValue;
use midnight_transient_crypto::hash::HashOutput;
use midnight_transient_crypto::merkle_tree::{MerkleTree, leaf_hash};

/// Compact's plain MerkleTree stores the bounded tree and first free index.
pub fn constructor_merkle_tree<D: DB>(depth: u8) -> StateValue<D> {
    let tree: MerkleTree<(), D> = MerkleTree::blank(depth).rehash();
    StateValue::Array(vec![StateValue::BoundedMerkleTree(tree), constructor_cell(0_u64)].into())
}

/// Read-only witness projection of a plain MerkleTree.
pub struct MerkleTreeView<'a, D: DB> {
    fields: &'a LedgerArray<StateValue<D>, D>,
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
    pub fn first_free(&self) -> Result<BoundedUint<{ u64::MAX as u128 }>, CompactError> {
        let value = read_cell::<u64, _>(&self.fields.get(1).expect("tree shape checked"))?;
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
    pub fn first_free(&self) -> Result<BoundedUint<{ u64::MAX as u128 }>, CompactError> {
        let value = read_cell::<u64, _>(&self.fields.get(1).expect("tree shape checked"))?;
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
    let path = path.into();
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
    context.query(&program, gas_limit, cost_model)
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
    let path = path.into();
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
    context.query(&program, gas_limit, cost_model)
}

/// Keep only the current root in a HistoricMerkleTree's history.
pub fn historic_reset_history<D: DB>(
    context: &QueryContext<D>,
    path: impl Into<LedgerPath>,
    gas_limit: Option<RunningCost>,
    cost_model: &CostModel,
) -> Result<QueryResults<ResultModeVerify, D>, TranscriptRejected<D>> {
    let path = path.into();
    let keys = path_keys(path.as_slice());
    let index_key = |index| vec![Key::Value(AlignedValue::from(index))].into();
    let program = [
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
    ];
    context.query(&program, gas_limit, cost_model)
}

/// Reset a HistoricMerkleTree and seed its new blank root through the VM.
pub fn historic_reset_to_default<D: DB>(
    context: &QueryContext<D>,
    path: impl Into<LedgerPath>,
    depth: u8,
    gas_limit: Option<RunningCost>,
    cost_model: &CostModel,
) -> Result<QueryResults<ResultModeVerify, D>, TranscriptRejected<D>> {
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
    context.query(&program, gas_limit, cost_model)
}

/// Reset a plain MerkleTree through the canonical ledger VM sequence.
pub fn merkle_reset_to_default<D: DB>(
    context: &QueryContext<D>,
    path: impl Into<LedgerPath>,
    depth: u8,
    gas_limit: Option<RunningCost>,
    cost_model: &CostModel,
) -> Result<QueryResults<ResultModeVerify, D>, TranscriptRejected<D>> {
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
    context.query(&program, gas_limit, cost_model)
}

/// Ask the ledger VM whether the next free index has reached tree capacity.
pub fn historic_is_full<D: DB>(
    context: &QueryContext<D>,
    path: impl Into<LedgerPath>,
    depth: u8,
    gas_limit: Option<RunningCost>,
    cost_model: &CostModel,
) -> Result<(QueryResults<ResultModeGather, D>, bool), CompactError> {
    let capacity = 1_u64.checked_shl(depth as u32).ok_or_else(|| {
        CompactError::InvalidLedgerCell(format!("invalid HistoricMerkleTree depth {depth}"))
    })?;
    let path = path.into();
    let index_key = vec![Key::Value(AlignedValue::from(1_u8))].into();
    let program = [
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
            result: (),
        },
    ];
    let result = context
        .query(&program, gas_limit, cost_model)
        .map_err(|error| CompactError::LedgerQueryRejected(format!("{error:?}")))?;
    let decoded = decode_last_read::<bool, D>(&result)?;
    Ok((result, decoded))
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
    let path = path.into();
    let index_key = vec![Key::Value(AlignedValue::from(0_u8))].into();
    let program = [
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
        Op::Root,
        Op::Push {
            storage: false,
            value: constructor_cell(root),
        },
        Op::Eq,
        Op::Popeq {
            cached: true,
            result: (),
        },
    ];
    let result = context
        .query(&program, gas_limit, cost_model)
        .map_err(|error| CompactError::LedgerQueryRejected(format!("{error:?}")))?;
    let decoded = decode_last_read::<bool, D>(&result)?;
    Ok((result, decoded))
}

/// Query membership in a HistoricMerkleTree's root history.
pub fn historic_check_root<T: CellValue, D: DB>(
    context: &QueryContext<D>,
    path: impl Into<LedgerPath>,
    root: T,
    gas_limit: Option<RunningCost>,
    cost_model: &CostModel,
) -> Result<(QueryResults<ResultModeGather, D>, bool), CompactError> {
    let path = path.into();
    let index_key = vec![Key::Value(AlignedValue::from(2_u8))].into();
    let program = [
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
            value: constructor_cell(root),
        },
        Op::Member,
        Op::Popeq {
            cached: true,
            result: (),
        },
    ];
    let result = context
        .query(&program, gas_limit, cost_model)
        .map_err(|error| CompactError::LedgerQueryRejected(format!("{error:?}")))?;
    let decoded = decode_last_read::<bool, D>(&result)?;
    Ok((result, decoded))
}
