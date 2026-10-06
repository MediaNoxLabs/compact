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

use crate::runtime::{
    context::{CircuitContext, RunningCost},
    ledger::{ContractAddress, DefaultDB, QueryContext},
};
use midnight_onchain_runtime::context::{BlockContext, CallContext};
use midnight_onchain_vm::cost_model::{CostModel, INITIAL_COST_MODEL};

/// Explicit call configuration; no wall clock, network lookup, or implicit RNG.
#[derive(Clone)]
pub struct Environment {
    pub address: ContractAddress,
    pub block: BlockContext,
    pub coin_public_key: Option<[u8; 32]>,
    pub cost_model: CostModel,
    /// Per-query generated execution limit, not an aggregate call/replay budget.
    /// Trusted adapters must preserve it while executing, not merely on return.
    pub query_gas_limit: Option<RunningCost>,
    /// Provenance for caller-generated deterministic fixtures, not a production key seed.
    /// The lab does not instantiate an RNG from this label.
    pub fixture_seed: [u8; 32],
}
impl Environment {
    pub fn new(address: ContractAddress, block: BlockContext, fixture_seed: [u8; 32]) -> Self {
        Self {
            address,
            block,
            fixture_seed,
            coin_public_key: None,
            cost_model: INITIAL_COST_MODEL.clone(),
            query_gas_limit: None,
        }
    }
    pub(crate) fn apply<P>(&self, mut context: CircuitContext<P>) -> CircuitContext<P> {
        let call = &mut context.query.call_context;
        call.tblock = self.block.tblock;
        call.tblock_err = self.block.tblock_err;
        call.parent_block_hash = self.block.parent_block_hash;
        call.last_block_time = self.block.last_block_time;
        context.cost_model = self.cost_model.clone();
        context.gas_limit = self.query_gas_limit;
        if let Some(key) = self.coin_public_key {
            context = context.with_coin_public_key_bytes(key);
        }
        context
    }
    pub(crate) fn matches(&self, other: &Self) -> bool {
        let Self {
            address,
            block,
            coin_public_key,
            cost_model,
            query_gas_limit,
            fixture_seed,
        } = self;
        address == &other.address
            && same_block(block, &other.block)
            && coin_public_key == &other.coin_public_key
            && cost_model == &other.cost_model
            && query_gas_limit == &other.query_gas_limit
            && fixture_seed == &other.fixture_seed
    }
}

pub(crate) fn same_call(a: &CallContext<DefaultDB>, b: &CallContext<DefaultDB>) -> bool {
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
pub(crate) fn same_initial(a: &QueryContext<DefaultDB>, b: &QueryContext<DefaultDB>) -> bool {
    let QueryContext {
        state,
        effects,
        address,
        call_context,
    } = a;
    address == &b.address
        && state == &b.state
        && effects == &b.effects
        && same_call(call_context, &b.call_context)
}

// Exhaustive patterns deliberately fail compilation on new upstream fields.
fn same_block(a: &BlockContext, b: &BlockContext) -> bool {
    let BlockContext {
        tblock,
        tblock_err,
        parent_block_hash,
        last_block_time,
    } = a;
    tblock == &b.tblock
        && tblock_err == &b.tblock_err
        && parent_block_hash == &b.parent_block_hash
        && last_block_time == &b.last_block_time
}
