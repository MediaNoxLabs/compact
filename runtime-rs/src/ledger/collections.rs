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

//! List, Map and Set ledger operations and their shared native/recorded VM programs.

use super::{
    CellValue, CompactError, LedgerArray, LedgerHashMap, LedgerPath, QueryContext, QueryResults,
    StateValue, TranscriptRejected, aligned_cell_value, constructor_cell, decode_last_read,
    field_at_path, path_keys, read_cell,
};
use crate::BoundedUint;
use crate::context::WitnessReadMeter;
use midnight_base_crypto::cost_model::RunningCost;
use midnight_base_crypto::fab::AlignedValue;
use midnight_onchain_vm::cost_model::CostModel;
use midnight_onchain_vm::ops::{Key, Op};
use midnight_onchain_vm::result_mode::{ResultMode, ResultModeGather, ResultModeVerify};
use midnight_storage::db::DB;
use midnight_transient_crypto::fab::AlignmentExt;
use std::marker::PhantomData;

/// Read-only witness projection of a Compact Set backed by the ledger Map.
pub struct SetView<'a, T, D: DB> {
    map: &'a LedgerHashMap<AlignedValue, StateValue<D>, D>,
    marker: PhantomData<T>,
}

/// Witness-facing Set projection that charges the canonical ledger VM query
/// for every read. Query failures are returned to the witness implementation.
pub struct MeteredSetView<'a, T, D: DB> {
    meter: &'a WitnessReadMeter<'a, D>,
    path: LedgerPath,
    marker: PhantomData<T>,
}

pub fn metered_set_view_at_path<'a, T: CellValue, D: DB>(
    meter: &'a WitnessReadMeter<'a, D>,
    path: &[u8],
) -> Result<MeteredSetView<'a, T, D>, CompactError> {
    let _ = set_view_at_path::<T, D>(meter.state(), path)?;
    Ok(MeteredSetView {
        meter,
        path: path.into(),
        marker: PhantomData,
    })
}

impl<T: CellValue, D: DB> MeteredSetView<'_, T, D> {
    pub fn member(&self, value: T) -> Result<bool, CompactError> {
        self.meter.read_set_member(self.path.as_slice(), value)
    }

    pub fn size(&self) -> Result<BoundedUint<{ u64::MAX as u128 }>, CompactError> {
        BoundedUint::new(self.meter.read_set_size(self.path.as_slice())? as u128)
    }

    pub fn is_empty(&self) -> Result<bool, CompactError> {
        self.meter.read_set_is_empty(self.path.as_slice())
    }
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

/// Witness-facing Map projection that charges each canonical ledger VM query.
pub struct MeteredMapView<'a, K, V, D: DB> {
    meter: &'a WitnessReadMeter<'a, D>,
    path: LedgerPath,
    marker: PhantomData<(K, V)>,
}

pub fn metered_map_view_at_path<'a, K: CellValue, V: CellValue, D: DB>(
    meter: &'a WitnessReadMeter<'a, D>,
    path: &[u8],
) -> Result<MeteredMapView<'a, K, V, D>, CompactError> {
    let _ = map_view_at_path::<K, V, D>(meter.state(), path)?;
    Ok(MeteredMapView {
        meter,
        path: path.into(),
        marker: PhantomData,
    })
}

impl<K: CellValue, V: CellValue, D: DB> MeteredMapView<'_, K, V, D> {
    pub fn member(&self, key: K) -> Result<bool, CompactError> {
        self.meter.read_map_member(self.path.as_slice(), key)
    }

    pub fn lookup(&self, key: K) -> Result<V, CompactError> {
        self.meter.read_map_lookup(self.path.as_slice(), key)
    }

    pub fn size(&self) -> Result<BoundedUint<{ u64::MAX as u128 }>, CompactError> {
        BoundedUint::new(self.meter.read_map_size(self.path.as_slice())? as u128)
    }

    pub fn is_empty(&self) -> Result<bool, CompactError> {
        self.meter.read_map_is_empty(self.path.as_slice())
    }
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

/// Witness-facing List projection that charges each canonical ledger VM query.
pub struct MeteredListView<'a, T, D: DB> {
    meter: &'a WitnessReadMeter<'a, D>,
    path: LedgerPath,
    marker: PhantomData<T>,
}

pub fn metered_list_view<'a, T: CellValue, D: DB>(
    meter: &'a WitnessReadMeter<'a, D>,
    index: u8,
) -> Result<MeteredListView<'a, T, D>, CompactError> {
    metered_list_view_at_path(meter, &[index])
}

pub fn metered_list_view_at_path<'a, T: CellValue, D: DB>(
    meter: &'a WitnessReadMeter<'a, D>,
    path: &[u8],
) -> Result<MeteredListView<'a, T, D>, CompactError> {
    let _ = list_view_at_path::<T, D>(meter.state(), path)?;
    Ok(MeteredListView {
        meter,
        path: path.into(),
        marker: PhantomData,
    })
}

impl<T: CellValue, D: DB> MeteredListView<'_, T, D> {
    pub fn head(&self) -> Result<Option<T>, CompactError>
    where
        T: Default,
        midnight_base_crypto::fab::Value: From<T>,
    {
        self.meter.read_list_head::<T>(self.path.as_slice())
    }

    pub fn length(&self) -> Result<BoundedUint<{ u64::MAX as u128 }>, CompactError> {
        BoundedUint::new(self.meter.read_list_length(self.path.as_slice())? as u128)
    }

    pub fn is_empty(&self) -> Result<bool, CompactError> {
        self.meter.read_list_is_empty(self.path.as_slice())
    }
}

pub fn list_view<T: CellValue, D: DB>(
    state: &StateValue<D>,
    index: u8,
) -> Result<ListView<'_, T, D>, CompactError> {
    list_view_at_path(state, &[index])
}

pub fn list_view_at_path<'a, T: CellValue, D: DB>(
    state: &'a StateValue<D>,
    path: &[u8],
) -> Result<ListView<'a, T, D>, CompactError> {
    let StateValue::Array(fields) = field_at_path(state, path)? else {
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
            Some(value) => read_cell(value).map(Some),
            None => unreachable!("List shape checked at construction"),
        }
    }

    pub fn length(&self) -> Result<BoundedUint<{ u64::MAX as u128 }>, CompactError> {
        let length = read_cell::<u64, _>(self.fields.get(2).expect("List shape checked"))?;
        BoundedUint::new(length as u128)
    }

    pub fn is_empty(&self) -> bool {
        matches!(self.fields.get(1), Some(StateValue::Null))
    }
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

pub(crate) fn list_length_program<M: ResultMode<D>, D: DB>(
    path: impl Into<LedgerPath>,
    read_result: M::ReadResult,
) -> Vec<Op<M, D>> {
    let path = path.into();
    vec![
        Op::Dup { n: 0 },
        Op::Idx {
            cached: false,
            push_path: false,
            path: path_keys(path.as_slice()).into(),
        },
        Op::Idx {
            cached: false,
            push_path: false,
            path: vec![Key::Value(AlignedValue::from(2_u8))].into(),
        },
        Op::Popeq {
            cached: true,
            result: read_result,
        },
    ]
}

pub fn length_list<D: DB>(
    context: &QueryContext<D>,
    path: impl Into<LedgerPath>,
    gas_limit: Option<RunningCost>,
    cost_model: &CostModel,
) -> Result<(QueryResults<ResultModeGather, D>, u64), CompactError> {
    let program = list_length_program::<ResultModeGather, D>(path, ());
    let result = context
        .query(&program, gas_limit, cost_model)
        .map_err(|error| CompactError::LedgerQueryRejected(format!("{error:?}")))?;
    let decoded = decode_last_read::<u64, D>(&result)?;
    Ok((result, decoded))
}

pub(crate) fn list_is_empty_program<M: ResultMode<D>, D: DB>(
    path: impl Into<LedgerPath>,
    read_result: M::ReadResult,
) -> Vec<Op<M, D>> {
    let path = path.into();
    vec![
        Op::Dup { n: 0 },
        Op::Idx {
            cached: false,
            push_path: false,
            path: path_keys(path.as_slice()).into(),
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
            result: read_result,
        },
    ]
}

pub fn is_empty_list<D: DB>(
    context: &QueryContext<D>,
    path: impl Into<LedgerPath>,
    gas_limit: Option<RunningCost>,
    cost_model: &CostModel,
) -> Result<(QueryResults<ResultModeGather, D>, bool), CompactError> {
    let program = list_is_empty_program::<ResultModeGather, D>(path, ());
    let result = context
        .query(&program, gas_limit, cost_model)
        .map_err(|error| CompactError::LedgerQueryRejected(format!("{error:?}")))?;
    let decoded = decode_last_read::<bool, D>(&result)?;
    Ok((result, decoded))
}

pub(crate) fn list_head_program<T: CellValue + Default, R: ResultMode<D>, D: DB>(
    path: impl Into<LedgerPath>,
    read_result: R::ReadResult,
) -> Vec<Op<R, D>> {
    let path = path.into();
    let alignment = T::alignment();
    let default = AlignedValue::new(T::default().into(), alignment.clone())
        .expect("default CellValue must match its alignment");
    // Compact's VMmax-sizeof uses the alignment's maximum encoded size,
    // including for a zero value whose actual serialization is shorter.
    let concat_bound = (2 + alignment.max_aligned_size()) as u32;
    let absent = AlignedValue::concat([AlignedValue::from(0_u8), default].iter());
    vec![
        Op::Dup { n: 0 },
        Op::Idx {
            cached: false,
            push_path: false,
            path: path_keys(path.as_slice()).into(),
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
            result: read_result,
        },
    ]
}

pub fn head_list<T: CellValue + Default, M: CellValue, D: DB>(
    context: &QueryContext<D>,
    path: impl Into<LedgerPath>,
    gas_limit: Option<RunningCost>,
    cost_model: &CostModel,
) -> Result<(QueryResults<ResultModeGather, D>, M), CompactError> {
    let program = list_head_program::<T, ResultModeGather, D>(path, ());
    let result = context
        .query(&program, gas_limit, cost_model)
        .map_err(|error| CompactError::LedgerQueryRejected(format!("{error:?}")))?;
    let decoded = decode_last_read::<M, D>(&result)?;
    Ok((result, decoded))
}

pub(crate) fn list_push_front_program<T: CellValue, D: DB>(
    path: impl Into<LedgerPath>,
    value: T,
) -> Vec<Op<ResultModeVerify, D>> {
    let path = path.into();
    vec![
        Op::Idx {
            cached: false,
            push_path: true,
            path: path_keys(path.as_slice()).into(),
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
        Op::Ins {
            cached: true,
            n: path.as_slice().len() as u8 + 1,
        },
    ]
}

pub fn push_front_list<T: CellValue, D: DB>(
    context: &QueryContext<D>,
    path: impl Into<LedgerPath>,
    value: T,
    gas_limit: Option<RunningCost>,
    cost_model: &CostModel,
) -> Result<QueryResults<ResultModeVerify, D>, TranscriptRejected<D>> {
    let program = list_push_front_program(path, value);
    context.query(&program, gas_limit, cost_model)
}

pub(crate) fn list_pop_front_program<D: DB>(
    path: impl Into<LedgerPath>,
) -> Vec<Op<ResultModeVerify, D>> {
    let path = path.into();
    vec![
        Op::Idx {
            cached: false,
            push_path: true,
            path: path_keys(path.as_slice()).into(),
        },
        Op::Idx {
            cached: false,
            push_path: false,
            path: vec![Key::Value(AlignedValue::from(1_u8))].into(),
        },
        Op::Ins {
            cached: true,
            n: path.as_slice().len() as u8,
        },
    ]
}

pub fn pop_front_list<D: DB>(
    context: &QueryContext<D>,
    path: impl Into<LedgerPath>,
    gas_limit: Option<RunningCost>,
    cost_model: &CostModel,
) -> Result<QueryResults<ResultModeVerify, D>, TranscriptRejected<D>> {
    let program = list_pop_front_program(path);
    context.query(&program, gas_limit, cost_model)
}

pub(crate) fn list_reset_program<D: DB>(
    path: impl Into<LedgerPath>,
) -> Vec<Op<ResultModeVerify, D>> {
    let path = path.into();
    let (field_index, parent) = path.as_slice().split_last().expect("List path is nonempty");
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
            value: constructor_cell(*field_index),
        },
        Op::Push {
            storage: true,
            value: constructor_list(),
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

pub fn reset_list<D: DB>(
    context: &QueryContext<D>,
    path: impl Into<LedgerPath>,
    gas_limit: Option<RunningCost>,
    cost_model: &CostModel,
) -> Result<QueryResults<ResultModeVerify, D>, TranscriptRejected<D>> {
    let program = list_reset_program(path);
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
