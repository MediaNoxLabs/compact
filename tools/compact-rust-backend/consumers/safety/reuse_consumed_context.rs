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

//! Reusing a consumed generated-call context must fail Rust ownership checking.
use compact_contract_counter as counter;
use counter::runtime::{context::ConstructorContext, ledger::ContractAddress};

fn main() {
    let state = counter::ledger_contract::initial_state(ConstructorContext::new(())).unwrap();
    let context = state.into_circuit_context(ContractAddress::default());
    let _first = counter::ledger_contract::increment(context).unwrap();
    let _second = counter::ledger_contract::increment(context).unwrap();
}
