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

use crate::{BoundedUint, CompactError, Field};
use midnight_base_crypto::cost_model::RunningCost;
use midnight_base_crypto::fab::{Aligned, AlignedValue, Value, ValueSlice};
use midnight_onchain_vm::cost_model::CostModel;
use midnight_onchain_vm::ops::{Key, Op};
use midnight_onchain_vm::result_mode::ResultModeVerify;

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

/// Compact Counter initializes as a ledger Cell of fixed-width Uint64.
pub fn constructor_counter<D: DB>() -> StateValue<D> {
    constructor_cell::<u64, D>(0)
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
    let root = StateValue::Array(Vec::<StateValue<DefaultDB>>::new().into());
    ChargedState::new(root)
}

/// A query context for a newly created empty contract.
pub fn empty_query_context() -> QueryContext<DefaultDB> {
    QueryContext::new(empty_contract_state(), ContractAddress::default())
}
