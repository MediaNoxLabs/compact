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

//! Prove both selected branches of the original effectful return source.
use super::effectful_scenario::ChooseScenario;
use super::*;
use compact_rust_effectful_return_oracle_fixture::ledger_contract as contract;
use midnight_compact_runtime::CompactError;
use midnight_compact_runtime::context::{CircuitContext, CircuitResult};
use midnight_compact_runtime::ledger::ContractAddress;

struct Mark;
impl contract::Witnesses<u64> for Mark {
    fn mark(&self, context: WitnessContext<'_, u64, contract::LedgerView<'_>>) -> (u64, Field) {
        (*context.private_state + 1, Field::from(77_u64))
    }
}

struct Scenario;
impl ChooseScenario for Scenario {
    fn initial_context() -> Result<CircuitContext<u64>, CompactError> {
        Ok(contract::initial_state(ConstructorContext::new(0_u64))?
            .into_circuit_context(ContractAddress::default()))
    }
    fn native(
        context: CircuitContext<u64>,
        next: Field,
    ) -> Result<CircuitResult<u64, Field>, CompactError> {
        contract::choose(context, &Mark, next)
    }
    fn recorded(
        context: CircuitContext<u64>,
        next: Field,
    ) -> Result<RecordedCircuitResult<u64, Field>, CompactError> {
        contract::recorded::choose(context, &Mark, next)
    }
    fn prepared(
        observed: &ObservedContractState,
        next: Field,
        verifier: VerifierKey,
    ) -> Result<ContractCallPrototype<DefaultDB>, Box<dyn Error>> {
        let contract = contract::Contract::from(Mark);
        let call = contract.recording().choose_call(observed, 0_u64, next)?;
        Ok(call.prepare(verifier, Fr::from(0_u64))?)
    }
}

pub(super) fn run(root: &Path) -> Result<(), Box<dyn Error>> {
    effectful_scenario::run::<Scenario>(root, "effectful return")
}
