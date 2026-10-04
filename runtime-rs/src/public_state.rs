// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//   http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Borrow upstream ledger-8 state for local, read-only generated projections.

use crate::context::{CircuitContext, CircuitResult, ConstructorResult};
use crate::ledger::QueryContext;
use crate::ledger::{ContractState, DB, QueryResults, StateValue};
use crate::recording::RecordedCircuitResult;
use midnight_onchain_vm::result_mode::ResultMode;

/// A local holder of public state; this does not authenticate or query it.
pub trait PublicStateSource {
    type Database: DB;

    fn public_state(&self) -> &StateValue<Self::Database>;
}

impl<D: DB> PublicStateSource for StateValue<D> {
    type Database = D;

    fn public_state(&self) -> &StateValue<D> {
        self
    }
}

impl<D: DB> PublicStateSource for ContractState<D> {
    type Database = D;

    fn public_state(&self) -> &StateValue<D> {
        self.data.get_ref()
    }
}

impl<Private, D: DB> PublicStateSource for ConstructorResult<Private, D> {
    type Database = D;

    fn public_state(&self) -> &StateValue<D> {
        self.ledger_state.get_ref()
    }
}

impl<D: DB> PublicStateSource for QueryContext<D> {
    type Database = D;

    fn public_state(&self) -> &StateValue<D> {
        self.state.get_ref()
    }
}

impl<M: ResultMode<D>, D: DB> PublicStateSource for QueryResults<M, D> {
    type Database = D;

    fn public_state(&self) -> &StateValue<D> {
        self.context.state.get_ref()
    }
}

impl<Private, D: DB> PublicStateSource for CircuitContext<Private, D> {
    type Database = D;

    fn public_state(&self) -> &StateValue<D> {
        self.query.state.get_ref()
    }
}

impl<Private, Output, D: DB> PublicStateSource for CircuitResult<Private, Output, D> {
    type Database = D;

    fn public_state(&self) -> &StateValue<D> {
        self.context.query.state.get_ref()
    }
}

impl<Private, Output, D: DB> PublicStateSource for RecordedCircuitResult<Private, Output, D> {
    type Database = D;

    fn public_state(&self) -> &StateValue<D> {
        self.execution.context.query.state.get_ref()
    }
}

#[cfg(feature = "ledger-transaction")]
impl<D: DB> PublicStateSource for crate::transaction::ObservedContractState<D> {
    type Database = D;

    fn public_state(&self) -> &StateValue<D> {
        self.contract().data.get_ref()
    }
}
