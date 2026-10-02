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
use midnight_base_crypto::fab::{Aligned, AlignedValue, Value, ValueSlice};
use midnight_onchain_vm::ops::Key;

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

mod cell;
mod collections;
mod counter;
mod merkle;

pub(crate) use cell::{
    aligned_cell_value, cell_read_program, cell_write_program, decode_last_read,
};
pub use cell::{
    constructor_cell, query_cell, query_cell_at_path, read_cell, read_cell_at_path, read_root_cell,
    write_cell, write_cell_at_path,
};
pub use collections::{
    ListView, MapView, MeteredListView, MeteredMapView, MeteredSetView, SetView, constructor_list,
    constructor_map, constructor_set, head_list, insert_map, insert_set, is_empty_list,
    is_empty_map, is_empty_set, length_list, list_view, lookup_map, map_view, map_view_at_path,
    member_map, member_set, metered_list_view, metered_map_view_at_path, metered_set_view_at_path,
    pop_front_list, push_front_list, remove_map, remove_set, reset_list, reset_map, reset_set,
    set_view, set_view_at_path, size_map, size_set,
};
pub(crate) use collections::{
    list_head_program, list_is_empty_program, list_length_program, list_pop_front_program,
    list_push_front_program, list_reset_program, map_insert_program, map_lookup_program,
    set_insert_program, set_is_empty_program, set_member_program, set_remove_program,
    set_reset_program, set_size_program,
};
pub(crate) use counter::counter_program;
pub use counter::{constructor_counter, decrement_counter, increment_counter, read_counter};
pub(crate) use merkle::merkle_insert_program;
pub use merkle::{
    HistoricMerkleTreeView, MerkleTreeView, MeteredHistoricMerkleTreeView, MeteredMerkleTreeView,
    constructor_historic_merkle_tree, constructor_merkle_tree, historic_check_root,
    historic_insert, historic_insert_hash, historic_insert_hash_index, historic_insert_index,
    historic_insert_index_default, historic_is_full, historic_merkle_tree_view_at_path,
    historic_reset_history, historic_reset_to_default, merkle_check_root, merkle_insert,
    merkle_insert_hash, merkle_insert_hash_index, merkle_insert_index, merkle_insert_index_default,
    merkle_is_full, merkle_reset_to_default, merkle_tree_view_at_path,
    metered_historic_merkle_tree_view_at_path, metered_merkle_tree_view_at_path,
};

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
