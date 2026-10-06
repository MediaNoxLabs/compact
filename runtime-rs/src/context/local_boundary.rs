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

//! Local-helper context policy, owned beside the private context fields.
//! New context or upstream fields require an explicit boundary decision here.
use super::{CircuitContext, CircuitResult};
use crate::CompactError;
use crate::ledger::{DB, QueryContext};
use midnight_onchain_runtime::context::CallContext;
use midnight_zswap::local::State as WalletState;

impl<Private, D: DB> CircuitContext<Private, D> {
    /// Validate a local callback before the recording adopts its private effects.
    pub(crate) fn call_local_checked<Output, F>(
        self,
        call: F,
    ) -> Result<CircuitResult<Private, Output, D>, CompactError>
    where
        F: FnOnce(Self) -> Result<CircuitResult<Private, Output, D>, CompactError>,
    {
        let Self {
            private_state: _, // Local helpers may replace private state.
            query,
            zswap_state,
            circuit_zswap,
            coin_public_key,
            cost_model,
            gas_limit,
        } = &self;
        let prior_query = query.clone();
        let prior_zswap = zswap_state.clone();
        let prior_circuit_zswap = circuit_zswap.clone();
        // Same optional byte identity as own_coin_public_key().ok().
        let prior_coin_key = coin_public_key.map(|key| key.0.0);
        let prior_cost_model = cost_model.clone();
        let prior_gas_limit = *gas_limit;
        let result = call(self)?;
        let next = &result.context;
        if !same_query(&prior_query, &next.query)
            || prior_circuit_zswap != next.circuit_zswap
            || !same_wallet(&prior_zswap, &next.zswap_state)
            || prior_coin_key != next.own_coin_public_key().ok()
            || prior_cost_model != next.cost_model
            || prior_gas_limit != next.gas_limit
        {
            return Err(CompactError::InvalidLedgerCell(
                "local helper changed public or Zswap execution context".into(),
            ));
        }
        Ok(result)
    }
}

fn same_query<D: DB>(a: &QueryContext<D>, b: &QueryContext<D>) -> bool {
    let QueryContext {
        state,
        effects,
        address,
        call_context,
    } = a;
    state == &b.state
        && effects == &b.effects
        && address == &b.address
        && same_call(call_context, &b.call_context)
}
fn same_call<D: DB>(a: &CallContext<D>, b: &CallContext<D>) -> bool {
    let CallContext {
        own_address,
        tblock,
        tblock_err,
        parent_block_hash,
        caller,
        balance,
        com_indices,
        last_block_time,
    } = a;
    own_address == &b.own_address
        && tblock == &b.tblock
        && tblock_err == &b.tblock_err
        && parent_block_hash == &b.parent_block_hash
        && caller == &b.caller
        && balance == &b.balance
        && com_indices == &b.com_indices
        && last_block_time == &b.last_block_time
}
fn same_wallet<D: DB>(a: &WalletState<D>, b: &WalletState<D>) -> bool {
    let WalletState {
        coins,
        pending_spends,
        pending_outputs,
        merkle_tree,
        first_free,
    } = a;
    coins == &b.coins
        && pending_spends == &b.pending_spends
        && pending_outputs == &b.pending_outputs
        && merkle_tree == &b.merkle_tree
        && first_free == &b.first_free
}
