//! Compact-level envelopes around ledger query and shielded state.
//!
//! The state, query, cost, and Zswap fields are upstream ledger types. This
//! module only ties them to a contract's private state and records ownership
//! transitions between constructor and circuit calls.

use midnight_base_crypto::cost_model::RunningCost;
use midnight_onchain_vm::cost_model::{CostModel, INITIAL_COST_MODEL};
use midnight_zswap::local::State as ZswapLocalState;

use crate::ledger::{ChargedState, ContractAddress, DB, DefaultDB, QueryContext};

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
