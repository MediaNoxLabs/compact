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

use compact_rust_cell_parameter_fixture::ledger_contract::{Contract, initial_state, set_flag};
use midnight_compact_runtime::context::ConstructorContext;
use midnight_compact_runtime::ledger::{ContractAddress, StateValue, read_cell};

#[test]
fn generated_parameterized_cell_write_uses_supplied_value() {
    let constructor = initial_state(ConstructorContext::new(())).unwrap();
    let context = constructor.into_circuit_context(ContractAddress::default());
    let result = set_flag(context, true).unwrap();
    let StateValue::Array(fields) = result.context.query.state.get_ref() else {
        panic!("expected ledger field array")
    };
    assert!(read_cell::<bool, _>(&fields.get(0).unwrap()).unwrap());

    let result = set_flag(result.context, false).unwrap();
    let StateValue::Array(fields) = result.context.query.state.get_ref() else {
        panic!("expected ledger field array")
    };
    assert!(!read_cell::<bool, _>(&fields.get(0).unwrap()).unwrap());
}

#[test]
fn recorded_parameterized_cell_writes_replay_both_values() {
    let constructor = initial_state(ConstructorContext::new(())).unwrap();
    let context = constructor.into_circuit_context(ContractAddress::default());
    let contract = Contract::default();
    let first = contract.recording.set_flag(context, true).unwrap();
    let first_replay = first
        .public
        .initial()
        .query(
            first.public.verify_ops(),
            None,
            &first.execution.context.cost_model,
        )
        .unwrap();
    assert_eq!(
        first_replay.context.effects,
        first.execution.context.query.effects
    );

    let second = contract
        .recording
        .set_flag(first.execution.context, false)
        .unwrap();
    let second_replay = second
        .public
        .initial()
        .query(
            second.public.verify_ops(),
            None,
            &second.execution.context.cost_model,
        )
        .unwrap();
    assert_eq!(
        second_replay.context.effects,
        second.execution.context.query.effects
    );
    let StateValue::Array(fields) = second_replay.context.state.get_ref() else {
        panic!("expected ledger field array")
    };
    assert!(!read_cell::<bool, _>(fields.get(0).unwrap()).unwrap());
}
