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

use midnight_compact_runtime::ledger::{
    ChargedState, ContractAddress, DefaultDB, QueryContext, StateValue, constructor_cell,
    constructor_counter, decrement_counter, increment_counter, read_counter, write_cell,
};
use midnight_onchain_vm::cost_model::INITIAL_COST_MODEL;

fn counter_field(context: &QueryContext<DefaultDB>) -> u64 {
    let StateValue::Array(fields) = context.state.get_ref() else {
        panic!("expected contract field array")
    };
    read_counter(fields.get(0).unwrap()).unwrap()
}

#[test]
fn ledger_vm_increments_and_decrements_compact_counter() {
    let state = StateValue::Array(vec![constructor_counter::<DefaultDB>()].into());
    let context = QueryContext::new(ChargedState::new(state), ContractAddress::default());
    let result = increment_counter(&context, 0, 3, None, &INITIAL_COST_MODEL).unwrap();
    assert_eq!(counter_field(&result.context), 3);

    let result = decrement_counter(&result.context, 0, 2, None, &INITIAL_COST_MODEL).unwrap();
    assert_eq!(counter_field(&result.context), 1);
    assert!(decrement_counter(&result.context, 0, 2, None, &INITIAL_COST_MODEL).is_err());
}

#[test]
fn ledger_vm_rejects_counter_overflow() {
    let state = StateValue::Array(vec![constructor_cell::<_, DefaultDB>(u64::MAX)].into());
    let context = QueryContext::new(ChargedState::new(state), ContractAddress::default());
    assert!(increment_counter(&context, 0, 1, None, &INITIAL_COST_MODEL).is_err());
}

#[test]
fn ledger_vm_replaces_a_compact_cell() {
    let state = StateValue::Array(vec![constructor_cell::<_, DefaultDB>(false)].into());
    let context = QueryContext::new(ChargedState::new(state), ContractAddress::default());
    let result = write_cell(&context, 0, true, None, &INITIAL_COST_MODEL).unwrap();
    let StateValue::Array(fields) = result.context.state.get_ref() else {
        panic!("expected field array")
    };
    assert!(
        midnight_compact_runtime::ledger::read_cell::<bool, _>(fields.get(0).unwrap()).unwrap()
    );
}
