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

//! Private exhaustive classification of pinned upstream local-helper state.
//! Adding an upstream field requires an explicit boundary decision here.
use crate::ledger::{DB, QueryContext};
use midnight_onchain_runtime::context::CallContext;
use midnight_zswap::local::State as WalletState;

pub(super) fn same_query<D: DB>(a: &QueryContext<D>, b: &QueryContext<D>) -> bool {
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
pub(super) fn same_wallet<D: DB>(a: &WalletState<D>, b: &WalletState<D>) -> bool {
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
