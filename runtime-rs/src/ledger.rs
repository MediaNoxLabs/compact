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
pub use midnight_storage::storage::HashMap as LedgerHashMap;

use crate::{BoundedUint, CompactError, Field, FixedBytes, FixedVector};
use midnight_base_crypto::cost_model::RunningCost;
use midnight_base_crypto::fab::{Aligned, AlignedValue, Value, ValueSlice};
use midnight_onchain_vm::cost_model::CostModel;
use midnight_onchain_vm::ops::{Key, Op};
use midnight_onchain_vm::result_mode::{GatherEvent, ResultModeGather, ResultModeVerify};

/// Compact values that can be stored in a ledger Cell. Implementations use
/// upstream FAB conversions while keeping each type's declared alignment.
pub trait CellValue: Aligned + Into<Value> + Sized {
    fn decode_cell_value(value: &ValueSlice) -> Result<Self, CompactError>;
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

primitive_cell_value!(bool, u8, u16, u32, u64, u128, Field);

impl<const MAX: u128> CellValue for BoundedUint<MAX> {
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
    StateValue::from(
        AlignedValue::new(value.into(), T::alignment())
            .expect("CellValue must match its alignment"),
    )
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

/// Read a root Cell through the ledger VM and gather its typed read event.
pub fn query_cell<T: CellValue, D: DB>(
    context: &QueryContext<D>,
    field_index: u8,
    gas_limit: Option<RunningCost>,
    cost_model: &CostModel,
) -> Result<(QueryResults<ResultModeGather, D>, T), CompactError> {
    let program = [
        Op::Dup { n: 0 },
        Op::Idx {
            cached: false,
            push_path: false,
            path: vec![Key::Value(AlignedValue::from(field_index))].into(),
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

pub fn insert_map<K: CellValue, V: CellValue, D: DB>(
    context: &QueryContext<D>,
    field_index: u8,
    key: K,
    value: V,
    gas_limit: Option<RunningCost>,
    cost_model: &CostModel,
) -> Result<QueryResults<ResultModeVerify, D>, TranscriptRejected<D>> {
    let program = [
        Op::Idx {
            cached: false,
            push_path: true,
            path: vec![Key::Value(AlignedValue::from(field_index))].into(),
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
        Op::Ins { cached: true, n: 1 },
    ];
    context.query(&program, gas_limit, cost_model)
}

pub fn member_map<K: CellValue, D: DB>(
    context: &QueryContext<D>,
    field_index: u8,
    key: K,
    gas_limit: Option<RunningCost>,
    cost_model: &CostModel,
) -> Result<(QueryResults<ResultModeGather, D>, bool), CompactError> {
    member_set(context, field_index, key, gas_limit, cost_model)
}

pub fn lookup_map<K: CellValue, V: CellValue, D: DB>(
    context: &QueryContext<D>,
    field_index: u8,
    key: K,
    gas_limit: Option<RunningCost>,
    cost_model: &CostModel,
) -> Result<(QueryResults<ResultModeGather, D>, V), CompactError> {
    let key =
        AlignedValue::new(key.into(), K::alignment()).expect("CellValue must match its alignment");
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

/// Insert a typed element into a root Set through the ledger VM.
pub fn insert_set<T: CellValue, D: DB>(
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
        Op::Ins { cached: true, n: 1 },
    ];
    context.query(&program, gas_limit, cost_model)
}

/// Test membership through a gather query and decode the ledger's Boolean Cell.
pub fn member_set<T: CellValue, D: DB>(
    context: &QueryContext<D>,
    field_index: u8,
    value: T,
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
        Op::Push {
            storage: false,
            value: constructor_cell(value),
        },
        Op::Rem { cached: false },
        Op::Ins { cached: true, n: 1 },
    ];
    context.query(&program, gas_limit, cost_model)
}

pub fn reset_set<D: DB>(
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
        Op::Pop,
        Op::Push {
            storage: true,
            value: constructor_set(),
        },
        Op::Ins { cached: true, n: 1 },
    ];
    context.query(&program, gas_limit, cost_model)
}

pub fn size_set<D: DB>(
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
    let program = [
        Op::Idx {
            cached: false,
            push_path: true,
            path: vec![Key::Value(AlignedValue::from(field_index))].into(),
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
    field_index: u8,
    amount: u16,
    gas_limit: Option<RunningCost>,
    cost_model: &CostModel,
) -> Result<QueryResults<ResultModeVerify, D>, TranscriptRejected<D>> {
    update_counter(context, field_index, amount, false, gas_limit, cost_model)
}

/// Run Compact Counter's decrement program through ledger query execution.
pub fn decrement_counter<D: DB>(
    context: &QueryContext<D>,
    field_index: u8,
    amount: u16,
    gas_limit: Option<RunningCost>,
    cost_model: &CostModel,
) -> Result<QueryResults<ResultModeVerify, D>, TranscriptRejected<D>> {
    update_counter(context, field_index, amount, true, gas_limit, cost_model)
}

fn update_counter<D: DB>(
    context: &QueryContext<D>,
    field_index: u8,
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
            path: vec![Key::Value(AlignedValue::from(field_index))].into(),
        },
        arithmetic,
        Op::Ins { cached: true, n: 1 },
    ];
    context.query(&program, gas_limit, cost_model)
}

/// The root ledger state for a contract with no public ledger fields.
pub fn empty_contract_state() -> ChargedState<DefaultDB> {
    contract_state(Vec::new())
}

/// Build a contract root from ordered ledger fields.
pub fn contract_state<D: DB>(fields: Vec<StateValue<D>>) -> ChargedState<D> {
    let root = StateValue::Array(fields.into());
    ChargedState::new(root)
}

/// A query context for a newly created empty contract.
pub fn empty_query_context() -> QueryContext<DefaultDB> {
    QueryContext::new(empty_contract_state(), ContractAddress::default())
}
