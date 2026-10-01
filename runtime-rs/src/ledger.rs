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

use crate::{BoundedUint, CompactError, Field, FixedBytes, FixedVector, JubjubPoint};
use midnight_base_crypto::cost_model::RunningCost;
use midnight_base_crypto::fab::{Aligned, AlignedValue, Value, ValueSlice};
use midnight_onchain_vm::cost_model::CostModel;
use midnight_onchain_vm::ops::{Key, Op};
use midnight_onchain_vm::result_mode::{GatherEvent, ResultModeGather, ResultModeVerify};
use midnight_serialize::Serializable;
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
    let program = [
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
            cached: true,
            result: (),
        },
    ];
    let result = context
        .query(&program, gas_limit, cost_model)
        .map_err(|error| CompactError::LedgerQueryRejected(format!("{error:?}")))?;
    let decoded = decode_last_read::<T, D>(&result)?;
    Ok((result, decoded))
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

pub fn insert_map<K: CellValue, V: CellValue, D: DB>(
    context: &QueryContext<D>,
    path: impl Into<LedgerPath>,
    key: K,
    value: V,
    gas_limit: Option<RunningCost>,
    cost_model: &CostModel,
) -> Result<QueryResults<ResultModeVerify, D>, TranscriptRejected<D>> {
    let path = path.into();
    let path = path.as_slice();
    let program = [
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
    ];
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

pub fn lookup_map<K: CellValue, V: CellValue, D: DB>(
    context: &QueryContext<D>,
    path: impl Into<LedgerPath>,
    key: K,
    gas_limit: Option<RunningCost>,
    cost_model: &CostModel,
) -> Result<(QueryResults<ResultModeGather, D>, V), CompactError> {
    let path = path.into();
    let path = path.as_slice();
    let key =
        AlignedValue::new(key.into(), K::alignment()).expect("CellValue must match its alignment");
    let program = [
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
            result: (),
        },
    ];
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
pub fn insert_set<T: CellValue, D: DB>(
    context: &QueryContext<D>,
    path: impl Into<LedgerPath>,
    value: T,
    gas_limit: Option<RunningCost>,
    cost_model: &CostModel,
) -> Result<QueryResults<ResultModeVerify, D>, TranscriptRejected<D>> {
    let path = path.into();
    let path = path.as_slice();
    let program = [
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
    ];
    context.query(&program, gas_limit, cost_model)
}

/// Test membership through a gather query and decode the ledger's Boolean Cell.
pub fn member_set<T: CellValue, D: DB>(
    context: &QueryContext<D>,
    path: impl Into<LedgerPath>,
    value: T,
    gas_limit: Option<RunningCost>,
    cost_model: &CostModel,
) -> Result<(QueryResults<ResultModeGather, D>, bool), CompactError> {
    let path = path.into();
    let path = path.as_slice();
    let program = [
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
            result: (),
        },
    ];
    let result = context
        .query(&program, gas_limit, cost_model)
        .map_err(|error| CompactError::LedgerQueryRejected(format!("{error:?}")))?;
    let decoded = decode_last_read::<bool, D>(&result)?;
    Ok((result, decoded))
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
    let program = [
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
    ];
    context.query(&program, gas_limit, cost_model)
}

pub fn reset_set<D: DB>(
    context: &QueryContext<D>,
    path: impl Into<LedgerPath>,
    gas_limit: Option<RunningCost>,
    cost_model: &CostModel,
) -> Result<QueryResults<ResultModeVerify, D>, TranscriptRejected<D>> {
    let path = path.into();
    let path = path.as_slice();
    let program = [
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
        Op::Ins {
            cached: true,
            n: path.len() as u8,
        },
    ];
    context.query(&program, gas_limit, cost_model)
}

pub fn size_set<D: DB>(
    context: &QueryContext<D>,
    path: impl Into<LedgerPath>,
    gas_limit: Option<RunningCost>,
    cost_model: &CostModel,
) -> Result<(QueryResults<ResultModeGather, D>, u64), CompactError> {
    let path = path.into();
    let path = path.as_slice();
    let program = [
        Op::Dup { n: 0 },
        Op::Idx {
            cached: false,
            push_path: false,
            path: path_keys(path).into(),
        },
        Op::Size,
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

pub fn is_empty_set<D: DB>(
    context: &QueryContext<D>,
    path: impl Into<LedgerPath>,
    gas_limit: Option<RunningCost>,
    cost_model: &CostModel,
) -> Result<(QueryResults<ResultModeGather, D>, bool), CompactError> {
    let path = path.into();
    let path = path.as_slice();
    let program = [
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
            result: (),
        },
    ];
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
    if path.len() == 2 {
        // Compact indexes the containing array, then inserts at its final
        // key and inserts the updated array back into the root.
        let program = [
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
        return context.query(&program, gas_limit, cost_model);
    }
    let program = [
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
    ];
    context.query(&program, gas_limit, cost_model)
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
    let arithmetic = if subtract {
        Op::Subi {
            immediate: amount.into(),
        }
    } else {
        Op::Addi {
            immediate: amount.into(),
        }
    };
    let program = [
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
    ];
    context.query(&program, gas_limit, cost_model)
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
