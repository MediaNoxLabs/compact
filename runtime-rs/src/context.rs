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

//
// Facade aggregates bundling existing upstream state types into the shapes
// the compiler emits references to. Mirror TS `CircuitContext<PS>`,
// `ConstructorContext<PS>` from @midnight-ntwrk/compact-runtime.

use crate::{
    CallProofData, CallProofDataTrace, ChargedState, ContractAddress, CostModel, DefaultDB,
    PartialProofData, QueryContext, RunningCost, ZswapLocalState, DB, INITIAL_COST_MODEL,
};

/// Context passed into each impure / provable circuit invocation.
#[derive(Clone)]
pub struct CircuitContext<PS, D = DefaultDB>
where
    D: DB,
{
    pub current_private_state: PS,
    pub current_query_context: QueryContext<D>,
    pub current_zswap_local_state: ZswapLocalState<D>,
    pub cost_model: CostModel,
    pub gas_limit: Option<RunningCost>,
    pub call_proof_data_trace: CallProofDataTrace<D>,
}

impl<PS, D> CircuitContext<PS, D>
where
    D: DB,
{
    /// Fold calls finalized by one generated nested call into its caller's
    /// active proof data, preserving the public/private transcript order at
    /// the call site. Existing records before `checkpoint` are left alone, so
    /// sequential top-level calls remain distinct. Nested input/output and
    /// metadata are dropped, matching the TypeScript emitter's one shared
    /// `PartialProofData` object per exported/root invocation. The expected
    /// parent address is captured before the nested call moves the context, so
    /// a returned foreign-contract context cannot authorize folding itself.
    pub fn with_folded_nested_call_proof_data(
        mut self,
        checkpoint: usize,
        expected_contract_address: ContractAddress,
        parent_proof_data: &mut PartialProofData<D>,
    ) -> Result<Self, crate::CompactError> {
        let nested_calls = self.call_proof_data_trace.drain_from(checkpoint)?;
        if self.current_query_context.address != expected_contract_address
            || nested_calls.iter().any(|call| {
                call.contract_address != expected_contract_address
                    || call.initial_query_context.address != expected_contract_address
                    || call.final_query_context.address != expected_contract_address
            })
        {
            return Err(crate::CompactError::ProofData(
                "cross-contract nested proof data cannot be folded into a root call".into(),
            ));
        }
        for call in nested_calls {
            parent_proof_data.fold_nested(call.proof_data);
        }
        Ok(self)
    }

    /// Finalize and append one circuit-call proof-data record to this context.
    /// Generated wrappers call this after they have applied all ledger effects
    /// and encoded the circuit output.
    pub fn with_finalized_call_proof_data(
        mut self,
        circuit_id: impl Into<String>,
        initial_query_context: QueryContext<D>,
        partial_proof_data: PartialProofData<D>,
        output: crate::AlignedValue,
    ) -> Self {
        let contract_address = initial_query_context.address;
        let final_query_context = self.current_query_context.clone();
        self.call_proof_data_trace.push(CallProofData::new(
            circuit_id,
            contract_address,
            initial_query_context,
            final_query_context,
            partial_proof_data.finalize(output),
        ));
        self
    }

    /// Build a fresh `CircuitContext` from a contract state and a private
    /// state. Mirrors the TS `createCircuitContext` helper: instantiates a
    /// `QueryContext` against the dummy contract address, an empty
    /// `ZswapLocalState`, the default `INITIAL_COST_MODEL`, and no gas limit.
    pub fn new(state: ChargedState<D>, private_state: PS) -> Self {
        Self {
            current_private_state: private_state,
            current_query_context: QueryContext::new(state, ContractAddress::default()),
            current_zswap_local_state: ZswapLocalState::default(),
            cost_model: INITIAL_COST_MODEL.clone(),
            gas_limit: None,
            call_proof_data_trace: CallProofDataTrace::new(),
        }
    }
}

/// Context passed into the contract constructor.
#[derive(Clone)]
pub struct ConstructorContext<PS, D = DefaultDB>
where
    D: DB,
{
    pub initial_private_state: PS,
    pub empty_zswap_local_state: ZswapLocalState<D>,
    pub cost_model: CostModel,
    pub gas_limit: Option<RunningCost>,
}
