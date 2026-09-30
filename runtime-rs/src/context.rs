//! Compact-level envelopes around ledger query and shielded state.
//!
//! The state, query, cost, and Zswap fields are upstream ledger types. This
//! module only ties them to a contract's private state and records ownership
//! transitions between constructor and circuit calls.

pub use midnight_base_crypto::cost_model::RunningCost;
use midnight_onchain_vm::cost_model::{CostModel, INITIAL_COST_MODEL};
use midnight_zswap::local::State as ZswapLocalState;

use crate::ledger::{CellValue, ChargedState, ContractAddress, DB, DefaultDB, QueryContext};
use crate::{CompactError, ledger};

pub struct ConstructorContext<Private, D: DB = DefaultDB> {
    pub private_state: Private,
    pub zswap_state: ZswapLocalState<D>,
}

impl<Private, D: DB> ConstructorContext<Private, D> {
    pub fn new(private_state: Private) -> Self {
        Self {
            private_state,
            zswap_state: ZswapLocalState::default(),
        }
    }
}

pub struct ConstructorResult<Private, D: DB = DefaultDB> {
    pub ledger_state: ChargedState<D>,
    pub private_state: Private,
    pub zswap_state: ZswapLocalState<D>,
}

impl<Private, D: DB> ConstructorResult<Private, D> {
    pub fn new(context: ConstructorContext<Private, D>, ledger_state: ChargedState<D>) -> Self {
        Self {
            ledger_state,
            private_state: context.private_state,
            zswap_state: context.zswap_state,
        }
    }

    pub fn into_circuit_context(self, address: ContractAddress) -> CircuitContext<Private, D> {
        CircuitContext {
            private_state: self.private_state,
            query: QueryContext::new(self.ledger_state, address),
            zswap_state: self.zswap_state,
            cost_model: INITIAL_COST_MODEL.clone(),
            gas_limit: None,
        }
    }
}

pub struct CircuitContext<Private, D: DB = DefaultDB> {
    pub private_state: Private,
    pub query: QueryContext<D>,
    pub zswap_state: ZswapLocalState<D>,
    pub cost_model: CostModel,
    pub gas_limit: Option<RunningCost>,
}

pub struct CircuitResult<Private, Output, D: DB = DefaultDB> {
    pub context: CircuitContext<Private, D>,
    pub result: Output,
    pub gas_cost: RunningCost,
}

impl<Private, D: DB> CircuitContext<Private, D> {
    pub fn insert_map<K: CellValue, V: CellValue>(
        mut self,
        field_index: u8,
        key: K,
        value: V,
    ) -> Result<CircuitResult<Private, (), D>, CompactError> {
        let result = ledger::insert_map(
            &self.query,
            field_index,
            key,
            value,
            self.gas_limit.clone(),
            &self.cost_model,
        )
        .map_err(|error| CompactError::LedgerQueryRejected(format!("{error:?}")))?;
        self.query = result.context;
        Ok(CircuitResult {
            context: self,
            result: (),
            gas_cost: result.gas_cost,
        })
    }

    pub fn member_map<K: CellValue>(
        mut self,
        field_index: u8,
        key: K,
    ) -> Result<CircuitResult<Private, bool, D>, CompactError> {
        let (result, present) = ledger::member_map(
            &self.query,
            field_index,
            key,
            self.gas_limit.clone(),
            &self.cost_model,
        )?;
        self.query = result.context;
        Ok(CircuitResult {
            context: self,
            result: present,
            gas_cost: result.gas_cost,
        })
    }

    pub fn lookup_map<K: CellValue, V: CellValue>(
        mut self,
        field_index: u8,
        key: K,
    ) -> Result<CircuitResult<Private, V, D>, CompactError> {
        let (result, value) = ledger::lookup_map(
            &self.query,
            field_index,
            key,
            self.gas_limit.clone(),
            &self.cost_model,
        )?;
        self.query = result.context;
        Ok(CircuitResult {
            context: self,
            result: value,
            gas_cost: result.gas_cost,
        })
    }
    pub fn insert_set<T: CellValue>(
        mut self,
        field_index: u8,
        value: T,
    ) -> Result<CircuitResult<Private, (), D>, CompactError> {
        let result = ledger::insert_set(
            &self.query,
            field_index,
            value,
            self.gas_limit.clone(),
            &self.cost_model,
        )
        .map_err(|error| CompactError::LedgerQueryRejected(format!("{error:?}")))?;
        self.query = result.context;
        Ok(CircuitResult {
            context: self,
            result: (),
            gas_cost: result.gas_cost,
        })
    }

    pub fn member_set<T: CellValue>(
        mut self,
        field_index: u8,
        value: T,
    ) -> Result<CircuitResult<Private, bool, D>, CompactError> {
        let (result, member) = ledger::member_set(
            &self.query,
            field_index,
            value,
            self.gas_limit.clone(),
            &self.cost_model,
        )?;
        self.query = result.context;
        Ok(CircuitResult {
            context: self,
            result: member,
            gas_cost: result.gas_cost,
        })
    }

    pub fn remove_set<T: CellValue>(
        mut self,
        field_index: u8,
        value: T,
    ) -> Result<CircuitResult<Private, (), D>, CompactError> {
        let result = ledger::remove_set(
            &self.query,
            field_index,
            value,
            self.gas_limit.clone(),
            &self.cost_model,
        )
        .map_err(|error| CompactError::LedgerQueryRejected(format!("{error:?}")))?;
        self.query = result.context;
        Ok(CircuitResult {
            context: self,
            result: (),
            gas_cost: result.gas_cost,
        })
    }

    pub fn reset_set(
        mut self,
        field_index: u8,
    ) -> Result<CircuitResult<Private, (), D>, CompactError> {
        let result = ledger::reset_set(
            &self.query,
            field_index,
            self.gas_limit.clone(),
            &self.cost_model,
        )
        .map_err(|error| CompactError::LedgerQueryRejected(format!("{error:?}")))?;
        self.query = result.context;
        Ok(CircuitResult {
            context: self,
            result: (),
            gas_cost: result.gas_cost,
        })
    }

    pub fn size_set(
        mut self,
        field_index: u8,
    ) -> Result<CircuitResult<Private, u64, D>, CompactError> {
        let (result, size) = ledger::size_set(
            &self.query,
            field_index,
            self.gas_limit.clone(),
            &self.cost_model,
        )?;
        self.query = result.context;
        Ok(CircuitResult {
            context: self,
            result: size,
            gas_cost: result.gas_cost,
        })
    }

    pub fn is_empty_set(
        mut self,
        field_index: u8,
    ) -> Result<CircuitResult<Private, bool, D>, CompactError> {
        let (result, empty) = ledger::is_empty_set(
            &self.query,
            field_index,
            self.gas_limit.clone(),
            &self.cost_model,
        )?;
        self.query = result.context;
        Ok(CircuitResult {
            context: self,
            result: empty,
            gas_cost: result.gas_cost,
        })
    }

    pub fn read_cell<T: CellValue>(
        mut self,
        field_index: u8,
    ) -> Result<CircuitResult<Private, T, D>, CompactError> {
        let (result, value) = ledger::query_cell::<T, D>(
            &self.query,
            field_index,
            self.gas_limit.clone(),
            &self.cost_model,
        )?;
        self.query = result.context;
        Ok(CircuitResult {
            context: self,
            result: value,
            gas_cost: result.gas_cost,
        })
    }

    pub fn write_cell<T: CellValue>(
        mut self,
        field_index: u8,
        value: T,
    ) -> Result<CircuitResult<Private, (), D>, CompactError> {
        let result = ledger::write_cell(
            &self.query,
            field_index,
            value,
            self.gas_limit.clone(),
            &self.cost_model,
        )
        .map_err(|error| CompactError::LedgerQueryRejected(format!("{error:?}")))?;
        self.query = result.context;
        Ok(CircuitResult {
            context: self,
            result: (),
            gas_cost: result.gas_cost,
        })
    }

    pub fn increment_counter(
        mut self,
        field_index: u8,
        amount: u16,
    ) -> Result<CircuitResult<Private, (), D>, CompactError> {
        let result = ledger::increment_counter(
            &self.query,
            field_index,
            amount,
            self.gas_limit.clone(),
            &self.cost_model,
        )
        .map_err(|error| CompactError::LedgerQueryRejected(format!("{error:?}")))?;
        self.query = result.context;
        Ok(CircuitResult {
            context: self,
            result: (),
            gas_cost: result.gas_cost,
        })
    }

    pub fn decrement_counter(
        mut self,
        field_index: u8,
        amount: u16,
    ) -> Result<CircuitResult<Private, (), D>, CompactError> {
        let result = ledger::decrement_counter(
            &self.query,
            field_index,
            amount,
            self.gas_limit.clone(),
            &self.cost_model,
        )
        .map_err(|error| CompactError::LedgerQueryRejected(format!("{error:?}")))?;
        self.query = result.context;
        Ok(CircuitResult {
            context: self,
            result: (),
            gas_cost: result.gas_cost,
        })
    }
}
