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

//! Ledger-8 contract state and query types.
//!
//! Keep these as the ledger's types. Compact-specific behavior belongs in
//! small, tested functions around them, not duplicate state structures.

pub use midnight_coin_structure::contract::ContractAddress;
pub use midnight_onchain_runtime::context::QueryContext;
pub use midnight_onchain_runtime::context::QueryResults;
pub use midnight_onchain_runtime::error::TranscriptRejected;
pub use midnight_onchain_state::state::{ChargedState, StateValue};
pub use midnight_storage::DefaultDB;
pub use midnight_storage::db::DB;
pub use midnight_storage::storage::Array as LedgerArray;
pub use midnight_storage::storage::HashMap as LedgerHashMap;
pub use midnight_transient_crypto::merkle_tree::{MerklePath, MerklePathEntry, MerkleTreeDigest};

use crate::{BoundedUint, CompactError, Field, FixedBytes, FixedVector, JubjubPoint};
use midnight_base_crypto::cost_model::RunningCost;
use midnight_base_crypto::fab::{Aligned, AlignedValue, Value, ValueSlice};
use midnight_base_crypto::repr::BinaryHashRepr;
use midnight_onchain_vm::cost_model::CostModel;
use midnight_onchain_vm::ops::{Key, Op};
use midnight_onchain_vm::result_mode::{
    GatherEvent, ResultMode, ResultModeGather, ResultModeVerify,
};
use midnight_serialize::Serializable;
use midnight_storage::arena::Sp;
use midnight_transient_crypto::fab::ValueReprAlignedValue;
use midnight_transient_crypto::hash::HashOutput;
use midnight_transient_crypto::merkle_tree::{MerkleTree, leaf_hash};
use std::marker::PhantomData;

/// A physical path through Compact's chunked ledger root. A single field
/// index and a nested array path use the same query operations.
pub struct LedgerPath(Vec<u8>);

impl LedgerPath {
    pub fn as_slice(&self) -> &[u8] {
        &self.0
    }
}

impl From<u8> for LedgerPath {
    fn from(index: u8) -> Self {
        Self(vec![index])
    }
}

impl From<&[u8]> for LedgerPath {
    fn from(path: &[u8]) -> Self {
        Self(path.to_vec())
    }
}

impl<const N: usize> From<&[u8; N]> for LedgerPath {
    fn from(path: &[u8; N]) -> Self {
        Self(path.to_vec())
    }
}

fn path_keys(path: &[u8]) -> Vec<Key> {
    path.iter()
        .map(|index| Key::Value(AlignedValue::from(*index)))
        .collect()
}

/// Compact values that can be stored in a ledger Cell. Implementations use
/// upstream FAB conversions while keeping each type's declared alignment.
pub trait CellValue: Aligned + Into<Value> + Sized {
    fn decode_cell_value(value: &ValueSlice) -> Result<Self, CompactError>;
}

/// Expose the ledger address bytes for Compact's stdlib ContractAddress struct.
pub fn contract_address_bytes(address: &ContractAddress) -> FixedBytes<32> {
    FixedBytes(address.0.0)
}

macro_rules! primitive_cell_value {
    ($($ty:ty),* $(,)?) => {$ (
        impl CellValue for $ty {
            fn decode_cell_value(value: &ValueSlice) -> Result<Self, CompactError> {
                Self::try_from(value).map_err(|error| CompactError::InvalidLedgerCell(error.to_string()))
            }
        }
    )* };
}

primitive_cell_value!(bool, u8, u16, u32, u64, u128, Field, JubjubPoint);

impl<const MAX: u128> CellValue for BoundedUint<MAX> {
    fn decode_cell_value(value: &ValueSlice) -> Result<Self, CompactError> {
        Self::try_from(value)
    }
}

impl<const HIGH: u128, const LOW: u128> CellValue for crate::WideUint<HIGH, LOW> {
    fn decode_cell_value(value: &ValueSlice) -> Result<Self, CompactError> {
        Self::try_from(value)
    }
}

impl<const N: usize> CellValue for [u8; N] {
    fn decode_cell_value(value: &ValueSlice) -> Result<Self, CompactError> {
        Self::try_from(Value(value.0.to_vec()))
            .map_err(|error| CompactError::InvalidLedgerCell(error.to_string()))
    }
}

impl<const N: usize> CellValue for FixedBytes<N> {
    fn decode_cell_value(value: &ValueSlice) -> Result<Self, CompactError> {
        <[u8; N]>::try_from(Value(value.0.to_vec()))
            .map(Self::new)
            .map_err(|error| CompactError::InvalidLedgerCell(error.to_string()))
    }
}

impl<T: CellValue, const N: usize> CellValue for FixedVector<T, N> {
    fn decode_cell_value(value: &ValueSlice) -> Result<Self, CompactError> {
        let atoms_per_element = T::alignment().0.len();
        if value.0.len() != N * atoms_per_element {
            return Err(CompactError::InvalidLedgerCell(
                "vector atom count differs from declared type".into(),
            ));
        }
        let elements = (0..N)
            .map(|index| {
                let start = index * atoms_per_element;
                T::decode_cell_value(&value[start..start + atoms_per_element])
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self::new(elements.try_into().map_err(|_| {
            CompactError::InvalidLedgerCell("invalid vector length".into())
        })?))
    }
}

impl CellValue for () {
    fn decode_cell_value(value: &ValueSlice) -> Result<Self, CompactError> {
        if value.0.is_empty() {
            Ok(())
        } else {
            Err(CompactError::InvalidLedgerCell(
                "expected empty tuple value".into(),
            ))
        }
    }
}

macro_rules! tuple_cell_value {
    ($($name:ident),+ $(,)?) => {
        #[allow(non_snake_case)]
        impl<$($name: CellValue),+> CellValue for ($($name,)+)
        where
            $(Value: From<$name>,)+
        {
            fn decode_cell_value(value: &ValueSlice) -> Result<Self, CompactError> {
                let mut offset = 0;
                $(let $name = {
                    let length = $name::alignment().0.len();
                    if offset + length > value.0.len() {
                        return Err(CompactError::InvalidLedgerCell("tuple atom count differs from declared type".into()));
                    }
                    let decoded = $name::decode_cell_value(&value[offset..offset + length])?;
                    offset += length;
                    decoded
                };)+
                if offset != value.0.len() {
                    return Err(CompactError::InvalidLedgerCell("tuple has trailing atoms".into()));
                }
                Ok(($($name,)+))
            }
        }
    };
}

tuple_cell_value!(A);
tuple_cell_value!(A, B);
tuple_cell_value!(A, B, C);
tuple_cell_value!(A, B, C, D);
tuple_cell_value!(A, B, C, D, E);
tuple_cell_value!(A, B, C, D, E, F);
tuple_cell_value!(A, B, C, D, E, F, G);
tuple_cell_value!(A, B, C, D, E, F, G, H);

/// Construct the ledger's Cell shape using its FAB alignment and value rules.
pub fn constructor_cell<T, D>(value: T) -> StateValue<D>
where
    T: CellValue,
    D: DB,
{
    StateValue::from(aligned_cell_value(value))
}

fn aligned_cell_value<T: CellValue>(value: T) -> AlignedValue {
    AlignedValue::new(value.into(), T::alignment()).expect("CellValue must match its alignment")
}

/// Read a Compact Cell after checking the declared type's exact alignment.
pub fn read_cell<T, D>(state: &StateValue<D>) -> Result<T, CompactError>
where
    T: CellValue,
    D: DB,
{
    let StateValue::Cell(cell) = state else {
        return Err(CompactError::InvalidLedgerCell(
            "expected Cell state".into(),
        ));
    };
    if cell.alignment != T::alignment() {
        return Err(CompactError::InvalidLedgerCell(
            "alignment differs from declared type".into(),
        ));
    }
    T::decode_cell_value(&cell.as_slice())
}

/// Decode one declared root Cell from a contract's current ledger state.
pub fn read_root_cell<T, D>(state: &StateValue<D>, index: u8) -> Result<T, CompactError>
where
    T: CellValue,
    D: DB,
{
    read_cell_at_path(state, &[index])
}

/// Read a Cell at the compiler's public ledger path, including chunked roots.
pub fn read_cell_at_path<T, D>(state: &StateValue<D>, path: &[u8]) -> Result<T, CompactError>
where
    T: CellValue,
    D: DB,
{
    read_cell(field_at_path(state, path)?)
}

fn field_at_path<'a, D: DB>(
    state: &'a StateValue<D>,
    path: &[u8],
) -> Result<&'a StateValue<D>, CompactError> {
    if path.is_empty() {
        return Err(CompactError::InvalidLedgerCell("empty ledger path".into()));
    }
    let mut current = state;
    for &index in path {
        let StateValue::Array(fields) = current else {
            return Err(CompactError::InvalidLedgerCell(
                "expected ledger array on path".into(),
            ));
        };
        current = fields.get(index as usize).ok_or_else(|| {
            CompactError::InvalidLedgerCell(format!("missing ledger path index {index}"))
        })?;
    }
    Ok(current)
}

fn root_field<D: DB>(state: &StateValue<D>, index: u8) -> Result<&StateValue<D>, CompactError> {
    let StateValue::Array(fields) = state else {
        return Err(CompactError::InvalidLedgerCell(
            "expected root ledger field array".into(),
        ));
    };
    fields.get(index as usize).ok_or_else(|| {
        CompactError::InvalidLedgerCell(format!("missing root ledger field {index}"))
    })
}

/// Read-only witness projection of a Compact Set backed by the ledger Map.
pub struct SetView<'a, T, D: DB> {
    map: &'a LedgerHashMap<AlignedValue, StateValue<D>, D>,
    marker: PhantomData<T>,
}

pub fn set_view<T: CellValue, D: DB>(
    state: &StateValue<D>,
    index: u8,
) -> Result<SetView<'_, T, D>, CompactError> {
    set_view_at_path(state, &[index])
}

pub fn set_view_at_path<'a, T: CellValue, D: DB>(
    state: &'a StateValue<D>,
    path: &[u8],
) -> Result<SetView<'a, T, D>, CompactError> {
    let StateValue::Map(map) = field_at_path(state, path)? else {
        return Err(CompactError::InvalidLedgerCell("expected Set map".into()));
    };
    Ok(SetView {
        map,
        marker: PhantomData,
    })
}

impl<T: CellValue, D: DB> SetView<'_, T, D> {
    pub fn member(&self, value: T) -> bool {
        self.map.contains_key(&aligned_cell_value(value))
    }

    pub fn size(&self) -> Result<BoundedUint<{ u64::MAX as u128 }>, CompactError> {
        let size = u64::try_from(self.map.size())
            .map_err(|_| CompactError::InvalidLedgerCell("Set size exceeds Uint<64>".into()))?;
        BoundedUint::new(size as u128)
    }

    pub fn is_empty(&self) -> bool {
        self.map.size() == 0
    }
}

/// Read-only witness projection of a Compact Map backed by the ledger Map.
pub struct MapView<'a, K, V, D: DB> {
    map: &'a LedgerHashMap<AlignedValue, StateValue<D>, D>,
    marker: PhantomData<(K, V)>,
}

pub fn map_view<K: CellValue, V: CellValue, D: DB>(
    state: &StateValue<D>,
    index: u8,
) -> Result<MapView<'_, K, V, D>, CompactError> {
    map_view_at_path(state, &[index])
}

pub fn map_view_at_path<'a, K: CellValue, V: CellValue, D: DB>(
    state: &'a StateValue<D>,
    path: &[u8],
) -> Result<MapView<'a, K, V, D>, CompactError> {
    let StateValue::Map(map) = field_at_path(state, path)? else {
        return Err(CompactError::InvalidLedgerCell("expected Map state".into()));
    };
    Ok(MapView {
        map,
        marker: PhantomData,
    })
}

impl<K: CellValue, V: CellValue, D: DB> MapView<'_, K, V, D> {
    pub fn member(&self, key: K) -> bool {
        self.map.contains_key(&aligned_cell_value(key))
    }

    pub fn lookup(&self, key: K) -> Result<V, CompactError> {
        let value = self
            .map
            .get(&aligned_cell_value(key))
            .ok_or_else(|| CompactError::InvalidLedgerCell("Map key is absent".into()))?;
        read_cell(&value)
    }

    pub fn size(&self) -> Result<BoundedUint<{ u64::MAX as u128 }>, CompactError> {
        let size = u64::try_from(self.map.size())
            .map_err(|_| CompactError::InvalidLedgerCell("Map size exceeds Uint<64>".into()))?;
        BoundedUint::new(size as u128)
    }

    pub fn is_empty(&self) -> bool {
        self.map.size() == 0
    }
}

/// Read-only witness projection of the ledger's head/tail/length List array.
pub struct ListView<'a, T, D: DB> {
    fields: &'a LedgerArray<StateValue<D>, D>,
    marker: PhantomData<T>,
}

pub fn list_view<T: CellValue, D: DB>(
    state: &StateValue<D>,
    index: u8,
) -> Result<ListView<'_, T, D>, CompactError> {
    let StateValue::Array(fields) = root_field(state, index)? else {
        return Err(CompactError::InvalidLedgerCell(
            "expected List array".into(),
        ));
    };
    if fields.len() != 3 {
        return Err(CompactError::InvalidLedgerCell(
            "expected List head, tail, and length".into(),
        ));
    }
    Ok(ListView {
        fields,
        marker: PhantomData,
    })
}

impl<T: CellValue, D: DB> ListView<'_, T, D> {
    pub fn head(&self) -> Result<Option<T>, CompactError> {
        match self.fields.get(0) {
            Some(StateValue::Null) => Ok(None),
            Some(value) => read_cell(&value).map(Some),
            None => unreachable!("List shape checked at construction"),
        }
    }

    pub fn length(&self) -> Result<BoundedUint<{ u64::MAX as u128 }>, CompactError> {
        let length = read_cell::<u64, _>(&self.fields.get(2).expect("List shape checked"))?;
        BoundedUint::new(length as u128)
    }

    pub fn is_empty(&self) -> bool {
        matches!(self.fields.get(1), Some(StateValue::Null))
    }
}

/// Read a root Cell through the ledger VM and gather its typed read event.
pub fn query_cell<T: CellValue, D: DB>(
    context: &QueryContext<D>,
    field_index: u8,
    gas_limit: Option<RunningCost>,
    cost_model: &CostModel,
) -> Result<(QueryResults<ResultModeGather, D>, T), CompactError> {
    query_cell_at_path(context, &[field_index], gas_limit, cost_model)
}

pub fn query_cell_at_path<T: CellValue, D: DB>(
    context: &QueryContext<D>,
    path: &[u8],
    gas_limit: Option<RunningCost>,
    cost_model: &CostModel,
) -> Result<(QueryResults<ResultModeGather, D>, T), CompactError> {
    if path.is_empty() {
        return Err(CompactError::InvalidLedgerCell("empty ledger path".into()));
    }
    let program = cell_read_program::<ResultModeGather, D>(path, ());
    let result = context
        .query(&program, gas_limit, cost_model)
        .map_err(|error| CompactError::LedgerQueryRejected(format!("{error:?}")))?;
    let decoded = decode_last_read::<T, D>(&result)?;
    Ok((result, decoded))
}

pub(crate) fn cell_read_program<M: ResultMode<D>, D: DB>(
    path: &[u8],
    read_result: M::ReadResult,
) -> Vec<Op<M, D>> {
    vec![
        Op::Dup { n: 0 },
        Op::Idx {
            cached: false,
            push_path: false,
            path: path
                .iter()
                .map(|index| Key::Value(AlignedValue::from(*index)))
                .collect::<Vec<_>>()
                .into(),
        },
        Op::Popeq {
            cached: false,
            result: read_result,
        },
    ]
}

fn decode_last_read<T: CellValue, D: DB>(
    result: &QueryResults<ResultModeGather, D>,
) -> Result<T, CompactError> {
    let Some(GatherEvent::Read(value)) = result.events.last() else {
        return Err(CompactError::InvalidLedgerCell(
            "missing ledger read event".into(),
        ));
    };
    if value.alignment != T::alignment() {
        return Err(CompactError::InvalidLedgerCell(
            "alignment differs from declared type".into(),
        ));
    }
    T::decode_cell_value(&value.value)
}

/// Compact Counter initializes as a ledger Cell of fixed-width Uint64.
pub fn constructor_counter<D: DB>() -> StateValue<D> {
    constructor_cell::<u64, D>(0)
}

pub fn constructor_set<D: DB>() -> StateValue<D> {
    constructor_map()
}

pub fn constructor_map<D: DB>() -> StateValue<D> {
    StateValue::Map(LedgerHashMap::new())
}

/// Compact List stores the head, tail, and fixed-width length in an array.
pub fn constructor_list<D: DB>() -> StateValue<D> {
    StateValue::Array(vec![StateValue::Null, StateValue::Null, constructor_cell(0_u64)].into())
}

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

pub fn length_list<D: DB>(
    context: &QueryContext<D>,
    field_index: u8,
    gas_limit: Option<RunningCost>,
    cost_model: &CostModel,
) -> Result<(QueryResults<ResultModeGather, D>, u64), CompactError> {
    let program = [
        Op::Dup { n: 0 },
        Op::Idx {
            cached: false,
            push_path: false,
            path: vec![Key::Value(AlignedValue::from(field_index))].into(),
        },
        Op::Idx {
            cached: false,
            push_path: false,
            path: vec![Key::Value(AlignedValue::from(2_u8))].into(),
        },
        Op::Popeq {
            cached: true,
            result: (),
        },
    ];
    let result = context
        .query(&program, gas_limit, cost_model)
        .map_err(|error| CompactError::LedgerQueryRejected(format!("{error:?}")))?;
    let decoded = decode_last_read::<u64, D>(&result)?;
    Ok((result, decoded))
}

pub fn is_empty_list<D: DB>(
    context: &QueryContext<D>,
    field_index: u8,
    gas_limit: Option<RunningCost>,
    cost_model: &CostModel,
) -> Result<(QueryResults<ResultModeGather, D>, bool), CompactError> {
    let program = [
        Op::Dup { n: 0 },
        Op::Idx {
            cached: false,
            push_path: false,
            path: vec![Key::Value(AlignedValue::from(field_index))].into(),
        },
        Op::Idx {
            cached: false,
            push_path: false,
            path: vec![Key::Value(AlignedValue::from(1_u8))].into(),
        },
        Op::Type,
        Op::Push {
            storage: false,
            value: constructor_cell::<u8, D>(1),
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

pub fn head_list<T: CellValue + Default, M: CellValue, D: DB>(
    context: &QueryContext<D>,
    field_index: u8,
    gas_limit: Option<RunningCost>,
    cost_model: &CostModel,
) -> Result<(QueryResults<ResultModeGather, D>, M), CompactError> {
    let default = AlignedValue::new(T::default().into(), T::alignment())
        .expect("default CellValue must match its alignment");
    let concat_bound = (Serializable::serialized_size(&AlignedValue::from(1_u8))
        + Serializable::serialized_size(&default)) as u32;
    let absent = AlignedValue::concat([AlignedValue::from(0_u8), default].iter());
    let program = [
        Op::Dup { n: 0 },
        Op::Idx {
            cached: false,
            push_path: false,
            path: vec![Key::Value(AlignedValue::from(field_index))].into(),
        },
        Op::Idx {
            cached: false,
            push_path: false,
            path: vec![Key::Value(AlignedValue::from(0_u8))].into(),
        },
        Op::Dup { n: 0 },
        Op::Type,
        Op::Push {
            storage: false,
            value: constructor_cell::<u8, D>(1),
        },
        Op::Eq,
        Op::Branch { skip: 4 },
        Op::Push {
            storage: false,
            value: constructor_cell::<u8, D>(1),
        },
        Op::Swap { n: 0 },
        Op::Concat {
            cached: false,
            n: concat_bound,
        },
        Op::Jmp { skip: 2 },
        Op::Pop,
        Op::Push {
            storage: false,
            value: StateValue::from(absent),
        },
        Op::Popeq {
            cached: true,
            result: (),
        },
    ];
    let result = context
        .query(&program, gas_limit, cost_model)
        .map_err(|error| CompactError::LedgerQueryRejected(format!("{error:?}")))?;
    let decoded = decode_last_read::<M, D>(&result)?;
    Ok((result, decoded))
}

pub fn push_front_list<T: CellValue, D: DB>(
    context: &QueryContext<D>,
    field_index: u8,
    value: T,
    gas_limit: Option<RunningCost>,
    cost_model: &CostModel,
) -> Result<QueryResults<ResultModeVerify, D>, TranscriptRejected<D>> {
    let program = [
        Op::Idx {
            cached: false,
            push_path: true,
            path: vec![Key::Value(AlignedValue::from(field_index))].into(),
        },
        Op::Dup { n: 0 },
        Op::Idx {
            cached: false,
            push_path: false,
            path: vec![Key::Value(AlignedValue::from(2_u8))].into(),
        },
        Op::Addi { immediate: 1 },
        Op::Push {
            storage: true,
            value: StateValue::Array(
                vec![constructor_cell(value), StateValue::Null, StateValue::Null].into(),
            ),
        },
        Op::Swap { n: 0 },
        Op::Push {
            storage: false,
            value: constructor_cell(2_u8),
        },
        Op::Swap { n: 0 },
        Op::Ins { cached: true, n: 1 },
        Op::Swap { n: 0 },
        Op::Push {
            storage: false,
            value: constructor_cell(1_u8),
        },
        Op::Swap { n: 0 },
        Op::Ins { cached: true, n: 2 },
    ];
    context.query(&program, gas_limit, cost_model)
}

pub fn pop_front_list<D: DB>(
    context: &QueryContext<D>,
    field_index: u8,
    gas_limit: Option<RunningCost>,
    cost_model: &CostModel,
) -> Result<QueryResults<ResultModeVerify, D>, TranscriptRejected<D>> {
    let program = [
        Op::Idx {
            cached: false,
            push_path: true,
            path: vec![Key::Value(AlignedValue::from(field_index))].into(),
        },
        Op::Idx {
            cached: false,
            push_path: false,
            path: vec![Key::Value(AlignedValue::from(1_u8))].into(),
        },
        Op::Ins { cached: true, n: 1 },
    ];
    context.query(&program, gas_limit, cost_model)
}

pub fn reset_list<D: DB>(
    context: &QueryContext<D>,
    field_index: u8,
    gas_limit: Option<RunningCost>,
    cost_model: &CostModel,
) -> Result<QueryResults<ResultModeVerify, D>, TranscriptRejected<D>> {
    let program = [
        Op::Idx {
            cached: false,
            push_path: true,
            path: vec![].into(),
        },
        Op::Push {
            storage: false,
            value: constructor_cell(field_index),
        },
        Op::Push {
            storage: true,
            value: constructor_list(),
        },
        Op::Ins {
            cached: false,
            n: 1,
        },
        Op::Ins { cached: true, n: 0 },
    ];
    context.query(&program, gas_limit, cost_model)
}

pub(crate) fn map_insert_program<K: CellValue, V: CellValue, D: DB>(
    path: &[u8],
    key: K,
    value: V,
) -> Vec<Op<ResultModeVerify, D>> {
    vec![
        Op::Idx {
            cached: false,
            push_path: true,
            path: path_keys(path).into(),
        },
        Op::Push {
            storage: false,
            value: constructor_cell(key),
        },
        Op::Push {
            storage: true,
            value: constructor_cell(value),
        },
        Op::Ins {
            cached: false,
            n: 1,
        },
        Op::Ins {
            cached: true,
            n: path.len() as u8,
        },
    ]
}

pub fn insert_map<K: CellValue, V: CellValue, D: DB>(
    context: &QueryContext<D>,
    path: impl Into<LedgerPath>,
    key: K,
    value: V,
    gas_limit: Option<RunningCost>,
    cost_model: &CostModel,
) -> Result<QueryResults<ResultModeVerify, D>, TranscriptRejected<D>> {
    let path = path.into();
    let program = map_insert_program(path.as_slice(), key, value);
    context.query(&program, gas_limit, cost_model)
}

pub fn member_map<K: CellValue, D: DB>(
    context: &QueryContext<D>,
    path: impl Into<LedgerPath>,
    key: K,
    gas_limit: Option<RunningCost>,
    cost_model: &CostModel,
) -> Result<(QueryResults<ResultModeGather, D>, bool), CompactError> {
    member_set(context, path, key, gas_limit, cost_model)
}

pub(crate) fn map_lookup_program<K: CellValue, M: ResultMode<D>, D: DB>(
    path: &[u8],
    key: K,
    read_result: M::ReadResult,
) -> Vec<Op<M, D>> {
    let key =
        AlignedValue::new(key.into(), K::alignment()).expect("CellValue must match its alignment");
    vec![
        Op::Dup { n: 0 },
        Op::Idx {
            cached: false,
            push_path: false,
            path: path_keys(path).into(),
        },
        Op::Idx {
            cached: false,
            push_path: false,
            path: vec![Key::Value(key)].into(),
        },
        Op::Popeq {
            cached: false,
            result: read_result,
        },
    ]
}

pub fn lookup_map<K: CellValue, V: CellValue, D: DB>(
    context: &QueryContext<D>,
    path: impl Into<LedgerPath>,
    key: K,
    gas_limit: Option<RunningCost>,
    cost_model: &CostModel,
) -> Result<(QueryResults<ResultModeGather, D>, V), CompactError> {
    let path = path.into();
    let program = map_lookup_program::<K, ResultModeGather, D>(path.as_slice(), key, ());
    let result = context
        .query(&program, gas_limit, cost_model)
        .map_err(|error| CompactError::LedgerQueryRejected(format!("{error:?}")))?;
    let decoded = decode_last_read::<V, D>(&result)?;
    Ok((result, decoded))
}

pub fn remove_map<K: CellValue, D: DB>(
    context: &QueryContext<D>,
    path: impl Into<LedgerPath>,
    key: K,
    gas_limit: Option<RunningCost>,
    cost_model: &CostModel,
) -> Result<QueryResults<ResultModeVerify, D>, TranscriptRejected<D>> {
    remove_set(context, path, key, gas_limit, cost_model)
}

pub fn size_map<D: DB>(
    context: &QueryContext<D>,
    path: impl Into<LedgerPath>,
    gas_limit: Option<RunningCost>,
    cost_model: &CostModel,
) -> Result<(QueryResults<ResultModeGather, D>, u64), CompactError> {
    size_set(context, path, gas_limit, cost_model)
}

pub fn is_empty_map<D: DB>(
    context: &QueryContext<D>,
    path: impl Into<LedgerPath>,
    gas_limit: Option<RunningCost>,
    cost_model: &CostModel,
) -> Result<(QueryResults<ResultModeGather, D>, bool), CompactError> {
    is_empty_set(context, path, gas_limit, cost_model)
}

pub fn reset_map<D: DB>(
    context: &QueryContext<D>,
    path: impl Into<LedgerPath>,
    gas_limit: Option<RunningCost>,
    cost_model: &CostModel,
) -> Result<QueryResults<ResultModeVerify, D>, TranscriptRejected<D>> {
    reset_set(context, path, gas_limit, cost_model)
}

/// Insert a typed element into a Set through the ledger VM.
pub(crate) fn set_insert_program<T: CellValue, D: DB>(
    path: &[u8],
    value: T,
) -> Vec<Op<ResultModeVerify, D>> {
    vec![
        Op::Idx {
            cached: false,
            push_path: true,
            path: path_keys(path).into(),
        },
        Op::Push {
            storage: false,
            value: constructor_cell(value),
        },
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
            n: path.len() as u8,
        },
    ]
}

pub fn insert_set<T: CellValue, D: DB>(
    context: &QueryContext<D>,
    path: impl Into<LedgerPath>,
    value: T,
    gas_limit: Option<RunningCost>,
    cost_model: &CostModel,
) -> Result<QueryResults<ResultModeVerify, D>, TranscriptRejected<D>> {
    let path = path.into();
    let path = path.as_slice();
    let program = set_insert_program(path, value);
    context.query(&program, gas_limit, cost_model)
}

/// Test membership through a gather query and decode the ledger's Boolean Cell.
pub(crate) fn set_member_program<T: CellValue, M: ResultMode<D>, D: DB>(
    path: &[u8],
    value: T,
    read_result: M::ReadResult,
) -> Vec<Op<M, D>> {
    vec![
        Op::Dup { n: 0 },
        Op::Idx {
            cached: false,
            push_path: false,
            path: path_keys(path).into(),
        },
        Op::Push {
            storage: false,
            value: constructor_cell(value),
        },
        Op::Member,
        Op::Popeq {
            cached: true,
            result: read_result,
        },
    ]
}

pub fn member_set<T: CellValue, D: DB>(
    context: &QueryContext<D>,
    path: impl Into<LedgerPath>,
    value: T,
    gas_limit: Option<RunningCost>,
    cost_model: &CostModel,
) -> Result<(QueryResults<ResultModeGather, D>, bool), CompactError> {
    let path = path.into();
    let path = path.as_slice();
    let program = set_member_program::<T, ResultModeGather, D>(path, value, ());
    let result = context
        .query(&program, gas_limit, cost_model)
        .map_err(|error| CompactError::LedgerQueryRejected(format!("{error:?}")))?;
    let decoded = decode_last_read::<bool, D>(&result)?;
    Ok((result, decoded))
}

pub(crate) fn set_remove_program<T: CellValue, D: DB>(
    path: &[u8],
    value: T,
) -> Vec<Op<ResultModeVerify, D>> {
    vec![
        Op::Idx {
            cached: false,
            push_path: true,
            path: path_keys(path).into(),
        },
        Op::Push {
            storage: false,
            value: constructor_cell(value),
        },
        Op::Rem { cached: false },
        Op::Ins {
            cached: true,
            n: path.len() as u8,
        },
    ]
}

pub fn remove_set<T: CellValue, D: DB>(
    context: &QueryContext<D>,
    path: impl Into<LedgerPath>,
    value: T,
    gas_limit: Option<RunningCost>,
    cost_model: &CostModel,
) -> Result<QueryResults<ResultModeVerify, D>, TranscriptRejected<D>> {
    let path = path.into();
    let path = path.as_slice();
    let program = set_remove_program(path, value);
    context.query(&program, gas_limit, cost_model)
}

pub(crate) fn set_reset_program<D: DB>(path: &[u8]) -> Vec<Op<ResultModeVerify, D>> {
    let Some((field, parent)) = path.split_last() else {
        // Preserve the previous low-level empty-path program; compiler-declared
        // Set paths are nonempty and use the keyed replacement below.
        return vec![
            Op::Idx {
                cached: false,
                push_path: true,
                path: path_keys(path).into(),
            },
            Op::Pop,
            Op::Push {
                storage: true,
                value: constructor_set(),
            },
            Op::Ins { cached: true, n: 0 },
        ];
    };
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
            value: constructor_cell(*field),
        },
        Op::Push {
            storage: true,
            value: constructor_set(),
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

pub fn reset_set<D: DB>(
    context: &QueryContext<D>,
    path: impl Into<LedgerPath>,
    gas_limit: Option<RunningCost>,
    cost_model: &CostModel,
) -> Result<QueryResults<ResultModeVerify, D>, TranscriptRejected<D>> {
    let path = path.into();
    let path = path.as_slice();
    let program = set_reset_program(path);
    context.query(&program, gas_limit, cost_model)
}

pub(crate) fn set_size_program<M: ResultMode<D>, D: DB>(
    path: &[u8],
    read_result: M::ReadResult,
) -> Vec<Op<M, D>> {
    vec![
        Op::Dup { n: 0 },
        Op::Idx {
            cached: false,
            push_path: false,
            path: path_keys(path).into(),
        },
        Op::Size,
        Op::Popeq {
            cached: true,
            result: read_result,
        },
    ]
}

pub fn size_set<D: DB>(
    context: &QueryContext<D>,
    path: impl Into<LedgerPath>,
    gas_limit: Option<RunningCost>,
    cost_model: &CostModel,
) -> Result<(QueryResults<ResultModeGather, D>, u64), CompactError> {
    let path = path.into();
    let path = path.as_slice();
    let program = set_size_program::<ResultModeGather, D>(path, ());
    let result = context
        .query(&program, gas_limit, cost_model)
        .map_err(|error| CompactError::LedgerQueryRejected(format!("{error:?}")))?;
    let decoded = decode_last_read::<u64, D>(&result)?;
    Ok((result, decoded))
}

pub(crate) fn set_is_empty_program<M: ResultMode<D>, D: DB>(
    path: &[u8],
    read_result: M::ReadResult,
) -> Vec<Op<M, D>> {
    vec![
        Op::Dup { n: 0 },
        Op::Idx {
            cached: false,
            push_path: false,
            path: path_keys(path).into(),
        },
        Op::Size,
        Op::Push {
            storage: false,
            value: constructor_cell::<u64, D>(0),
        },
        Op::Eq,
        Op::Popeq {
            cached: true,
            result: read_result,
        },
    ]
}

pub fn is_empty_set<D: DB>(
    context: &QueryContext<D>,
    path: impl Into<LedgerPath>,
    gas_limit: Option<RunningCost>,
    cost_model: &CostModel,
) -> Result<(QueryResults<ResultModeGather, D>, bool), CompactError> {
    let path = path.into();
    let path = path.as_slice();
    let program = set_is_empty_program::<ResultModeGather, D>(path, ());
    let result = context
        .query(&program, gas_limit, cost_model)
        .map_err(|error| CompactError::LedgerQueryRejected(format!("{error:?}")))?;
    let decoded = decode_last_read::<bool, D>(&result)?;
    Ok((result, decoded))
}

pub fn read_counter<D: DB>(state: &StateValue<D>) -> Result<u64, CompactError> {
    read_cell::<u64, D>(state)
}

/// Replace an existing root Cell through ledger VM execution.
pub fn write_cell<T: CellValue, D: DB>(
    context: &QueryContext<D>,
    field_index: u8,
    value: T,
    gas_limit: Option<RunningCost>,
    cost_model: &CostModel,
) -> Result<QueryResults<ResultModeVerify, D>, TranscriptRejected<D>> {
    write_cell_at_path(context, &[field_index], value, gas_limit, cost_model)
}

pub fn write_cell_at_path<T: CellValue, D: DB>(
    context: &QueryContext<D>,
    path: &[u8],
    value: T,
    gas_limit: Option<RunningCost>,
    cost_model: &CostModel,
) -> Result<QueryResults<ResultModeVerify, D>, TranscriptRejected<D>> {
    context.query(&cell_write_program(path, value), gas_limit, cost_model)
}

pub(crate) fn cell_write_program<T: CellValue, D: DB>(
    path: &[u8],
    value: T,
) -> Vec<Op<ResultModeVerify, D>> {
    if path.len() == 1 {
        // Compact emits a root Cell replacement as an insertion keyed by a
        // temporary Cell containing the ledger index. Preserve that program
        // so the ledger transcript matches the circuit's ZKIR public inputs.
        return vec![
            Op::Push {
                storage: false,
                value: constructor_cell(path[0]),
            },
            Op::Push {
                storage: true,
                value: constructor_cell(value),
            },
            Op::Ins {
                cached: false,
                n: 1,
            },
        ];
    }
    if path.len() == 2 {
        // Compact indexes the containing array, then inserts at its final
        // key and inserts the updated array back into the root.
        return vec![
            Op::Idx {
                cached: false,
                push_path: true,
                path: vec![Key::Value(AlignedValue::from(path[0]))].into(),
            },
            Op::Push {
                storage: false,
                value: constructor_cell(path[1]),
            },
            Op::Push {
                storage: true,
                value: constructor_cell(value),
            },
            Op::Ins {
                cached: false,
                n: 1,
            },
            Op::Ins { cached: true, n: 1 },
        ];
    }
    vec![
        Op::Idx {
            cached: false,
            push_path: true,
            path: path
                .iter()
                .map(|index| Key::Value(AlignedValue::from(*index)))
                .collect::<Vec<_>>()
                .into(),
        },
        Op::Pop,
        Op::Push {
            storage: true,
            value: constructor_cell(value),
        },
        Op::Ins { cached: true, n: 1 },
    ]
}

/// Run Compact Counter's increment program through ledger query execution.
pub fn increment_counter<D: DB>(
    context: &QueryContext<D>,
    path: impl Into<LedgerPath>,
    amount: u16,
    gas_limit: Option<RunningCost>,
    cost_model: &CostModel,
) -> Result<QueryResults<ResultModeVerify, D>, TranscriptRejected<D>> {
    let path = path.into();
    update_counter(
        context,
        path.as_slice(),
        amount,
        false,
        gas_limit,
        cost_model,
    )
}

/// Run Compact Counter's decrement program through ledger query execution.
pub fn decrement_counter<D: DB>(
    context: &QueryContext<D>,
    path: impl Into<LedgerPath>,
    amount: u16,
    gas_limit: Option<RunningCost>,
    cost_model: &CostModel,
) -> Result<QueryResults<ResultModeVerify, D>, TranscriptRejected<D>> {
    let path = path.into();
    update_counter(
        context,
        path.as_slice(),
        amount,
        true,
        gas_limit,
        cost_model,
    )
}

fn update_counter<D: DB>(
    context: &QueryContext<D>,
    path: &[u8],
    amount: u16,
    subtract: bool,
    gas_limit: Option<RunningCost>,
    cost_model: &CostModel,
) -> Result<QueryResults<ResultModeVerify, D>, TranscriptRejected<D>> {
    context.query(
        &counter_program(path, amount, subtract),
        gas_limit,
        cost_model,
    )
}

pub(crate) fn counter_program<D: DB>(
    path: &[u8],
    amount: u16,
    subtract: bool,
) -> Vec<Op<ResultModeVerify, D>> {
    let arithmetic = if subtract {
        Op::Subi {
            immediate: amount.into(),
        }
    } else {
        Op::Addi {
            immediate: amount.into(),
        }
    };
    vec![
        Op::Idx {
            cached: false,
            push_path: true,
            path: path_keys(path).into(),
        },
        arithmetic,
        Op::Ins {
            cached: true,
            n: path.len() as u8,
        },
    ]
}

/// The root ledger state for a contract with no public ledger fields.
pub fn empty_contract_state() -> ChargedState<DefaultDB> {
    contract_state(Vec::new())
}

/// Build a contract root from ordered ledger fields.
pub fn contract_state<D: DB>(fields: Vec<StateValue<D>>) -> ChargedState<D> {
    let root = chunk_ledger_fields(fields);
    ChargedState::new(root)
}

fn chunk_ledger_fields<D: DB>(fields: Vec<StateValue<D>>) -> StateValue<D> {
    const SEGMENT: usize = 15;
    if fields.len() <= SEGMENT {
        return StateValue::Array(fields.into());
    }
    let remainder = fields.len() % SEGMENT;
    let mut iter = fields.into_iter();
    let mut chunks = Vec::new();
    if remainder != 0 {
        chunks.push(StateValue::Array(
            iter.by_ref().take(remainder).collect::<Vec<_>>().into(),
        ));
    }
    loop {
        let chunk = iter.by_ref().take(SEGMENT).collect::<Vec<_>>();
        if chunk.is_empty() {
            break;
        }
        chunks.push(StateValue::Array(chunk.into()));
    }
    chunk_ledger_fields(chunks)
}

/// A query context for a newly created empty contract.
pub fn empty_query_context() -> QueryContext<DefaultDB> {
    QueryContext::new(empty_contract_state(), ContractAddress::default())
}
