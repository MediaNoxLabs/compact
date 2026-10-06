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
        let Self {
            address: _,      // Applied when ContractLab constructs the fresh context.
            fixture_seed: _, // Snapshot provenance only; never an RNG reset.
            block,
            coin_public_key,
            cost_model,
            query_gas_limit,
        } = self;
        let BlockContext {
            tblock,
            tblock_err,
            parent_block_hash,
            last_block_time,
        } = block;
        let call = &mut context.query.call_context;
        call.tblock = *tblock;
        call.tblock_err = *tblock_err;
        call.parent_block_hash = *parent_block_hash;
        call.last_block_time = *last_block_time;
        context.cost_model = cost_model.clone();
        context.gas_limit = *query_gas_limit;
        if let Some(key) = coin_public_key {
            context = context.with_coin_public_key_bytes(*key);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::context::ConstructorContext;
    use compact_rust_counter_parameter_fixture::ledger_contract as counter;
    use midnight_base_crypto::{hash::HashOutput, time::Timestamp};

    #[test]
    fn applying_policy_does_not_reset_address_identity_or_fixture_provenance() {
        let original_address = ContractAddress(HashOutput([3; 32]));
        let context = counter::initial_state(ConstructorContext::new(17))
            .unwrap()
            .into_circuit_context(original_address)
            .with_coin_public_key_bytes([5; 32]);
        let prior_own_address = context.query.call_context.own_address;
        let mut environment = Environment::new(
            ContractAddress(HashOutput([7; 32])),
            BlockContext::default(),
            [11; 32],
        );
        environment.block.tblock = Timestamp::from_secs(100);
        environment.block.last_block_time = Timestamp::from_secs(95);
        environment.block.tblock_err = 2;
        environment.block.parent_block_hash = HashOutput([13; 32]);
        environment.query_gas_limit = Some(RunningCost::ZERO);
        let applied = environment.apply(context);
        assert_eq!(applied.query.address, original_address);
        assert_eq!(applied.query.call_context.own_address, prior_own_address);
        assert_eq!(applied.own_coin_public_key().unwrap(), [5; 32]);
        assert_eq!(applied.private_state, 17);
        assert_eq!(applied.query.call_context.tblock, environment.block.tblock);
        assert_eq!(
            applied.query.call_context.last_block_time,
            environment.block.last_block_time
        );
        assert_eq!(applied.query.call_context.tblock_err, 2);
        assert_eq!(
            applied.query.call_context.parent_block_hash,
            environment.block.parent_block_hash
        );
        assert_eq!(applied.cost_model, environment.cost_model);
        assert_eq!(applied.gas_limit, Some(RunningCost::ZERO));
        assert_eq!(environment.coin_public_key, None);
        assert_eq!(environment.fixture_seed, [11; 32]);
    }
}
