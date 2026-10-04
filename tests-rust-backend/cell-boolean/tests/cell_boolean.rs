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

use compact_rust_cell_boolean_fixture::ledger_contract::{Contract, initial_state, set_flag};
use midnight_compact_runtime::context::ConstructorContext;
use midnight_compact_runtime::ledger::{ContractAddress, StateValue, read_cell};

#[test]
fn generated_cell_contract_writes_through_ledger_vm() {
    let constructor = initial_state(ConstructorContext::new(())).unwrap();
    let StateValue::Array(initial_fields) = constructor.ledger_state.get_ref() else {
        panic!("expected ledger field array")
    };
    assert!(!read_cell::<bool, _>(initial_fields.get(0).unwrap()).unwrap());

    let context = constructor.into_circuit_context(ContractAddress::default());
    let result = set_flag(context).unwrap();
    let StateValue::Array(fields) = result.context.query.state.get_ref() else {
        panic!("expected ledger field array")
    };
    assert!(read_cell::<bool, _>(fields.get(0).unwrap()).unwrap());
}

#[test]
fn generated_cell_write_records_a_replayable_ledger_program() {
    let constructor = initial_state(ConstructorContext::new(())).unwrap();
    let context = constructor.into_circuit_context(ContractAddress::default());
    let recorded = Contract::default().recording.set_flag(context).unwrap();
    let replay = recorded
        .public
        .initial()
        .query(
            recorded.public.verify_ops(),
            None,
            &recorded.execution.context.cost_model,
        )
        .unwrap();
    assert_eq!(
        replay.context.effects,
        recorded.execution.context.query.effects
    );
    let StateValue::Array(fields) = replay.context.state.get_ref() else {
        panic!("expected ledger field array")
    };
    assert!(read_cell::<bool, _>(fields.get(0).unwrap()).unwrap());
}
