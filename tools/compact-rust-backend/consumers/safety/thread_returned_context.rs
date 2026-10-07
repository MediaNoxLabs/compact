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

//! Returned contexts support two sequential generated calls with observable state.
use compact_contract_counter as counter;
use counter::runtime::{context::ConstructorContext, ledger::ContractAddress};

#[test]
fn returned_context_threads_two_real_calls() {
    let state = counter::ledger_contract::initial_state(ConstructorContext::new(())).unwrap();
    let context = state.into_circuit_context(ContractAddress::default());
    let first = counter::ledger_contract::increment(context).unwrap();
    let second = counter::ledger_contract::increment(first.context).unwrap();
    let observed = counter::ledger_contract::read_round(second.context).unwrap();
    assert_eq!(observed.result.value(), 2);
}
