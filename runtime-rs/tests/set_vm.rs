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
    ChargedState, ContractAddress, DefaultDB, QueryContext, StateValue, constructor_set,
    insert_set, is_empty_set, member_set, remove_set, reset_set, size_set,
};
use midnight_onchain_vm::cost_model::INITIAL_COST_MODEL;

#[test]
fn ledger_vm_inserts_and_checks_a_boolean_set() {
    let state = StateValue::Array(vec![constructor_set::<DefaultDB>()].into());
    let context = QueryContext::new(ChargedState::new(state), ContractAddress::default());
    let (result, empty) = is_empty_set(&context, 0, None, &INITIAL_COST_MODEL).unwrap();
    assert!(empty);
    let (result, size) = size_set(&result.context, 0, None, &INITIAL_COST_MODEL).unwrap();
    assert_eq!(size, 0);
    let (result, present) =
        member_set(&result.context, 0, true, None, &INITIAL_COST_MODEL).unwrap();
    assert!(!present);

    let result = insert_set(&result.context, 0, true, None, &INITIAL_COST_MODEL).unwrap();
    let (result, present) =
        member_set(&result.context, 0, true, None, &INITIAL_COST_MODEL).unwrap();
    assert!(present);
    let (result, present) =
        member_set(&result.context, 0, false, None, &INITIAL_COST_MODEL).unwrap();
    assert!(!present);
    let (result, size) = size_set(&result.context, 0, None, &INITIAL_COST_MODEL).unwrap();
    assert_eq!(size, 1);
    let (result, empty) = is_empty_set(&result.context, 0, None, &INITIAL_COST_MODEL).unwrap();
    assert!(!empty);
    let result = remove_set(&result.context, 0, true, None, &INITIAL_COST_MODEL).unwrap();
    let (result, empty) = is_empty_set(&result.context, 0, None, &INITIAL_COST_MODEL).unwrap();
    assert!(empty);
    let result = insert_set(&result.context, 0, false, None, &INITIAL_COST_MODEL).unwrap();
    let result = reset_set(&result.context, 0, None, &INITIAL_COST_MODEL).unwrap();
    let (_, empty) = is_empty_set(&result.context, 0, None, &INITIAL_COST_MODEL).unwrap();
    assert!(empty);
}
