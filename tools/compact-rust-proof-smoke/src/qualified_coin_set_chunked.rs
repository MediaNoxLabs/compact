// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//  http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Source-specific binding for the shared qualified Set funding scenario.
use super::*;
use compact_rust_qualified_coin_set_chunked_oracle_fixture::{ledger_contract as contract, types};

#[path = "qualified_coin_set/scenario.rs"]
#[allow(
    clippy::duplicate_mod,
    reason = "instantiate the shared scenario with this generated contract's distinct types"
)]
mod scenario;

pub(super) fn run(root: &Path) -> Result<(), Box<dyn Error>> {
    scenario::run(
        root,
        compact_rust_qualified_coin_set_chunked_oracle_fixture::RETAINED_IR,
    )
}

fn contains_qualified(
    state: &ContractState<DefaultDB>,
    coin: types::QualifiedShieldedCoinInfo,
) -> Result<bool, Box<dyn Error>> {
    Ok(contract::PublicStateView::from(state).pot()?.member(coin))
}
