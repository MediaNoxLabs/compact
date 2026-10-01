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

use compact_rust_counter_fixture::ledger_contract::{increment, initial_state, read_round};
use midnight_compact_runtime::context::ConstructorContext;
use midnight_compact_runtime::ledger::{ContractAddress, StateValue, read_counter};
use midnight_compact_runtime::recording::RecordingFrame;

#[test]
fn generated_counter_contract_runs_through_ledger_vm() {
    let constructor = initial_state(ConstructorContext::new(())).unwrap();
    let context = constructor.into_circuit_context(ContractAddress::default());
    let read = read_round(context).unwrap();
    assert_eq!(read.result.value(), 0);
    let result = increment(read.context).unwrap();
    let StateValue::Array(fields) = result.context.query.state.get_ref() else {
        panic!("expected ledger field array")
    };
    assert_eq!(read_counter(&fields.get(0).unwrap()).unwrap(), 1);
    let read = read_round(result.context).unwrap();
    assert_eq!(read.result.value(), 1);
}

#[test]
fn generated_counter_state_can_enter_a_replayable_ledger_trace() {
    let constructor = initial_state(ConstructorContext::new(())).unwrap();
    let context = constructor.into_circuit_context(ContractAddress::default());
    let native = increment(context).unwrap();

    let constructor = initial_state(ConstructorContext::new(())).unwrap();
    let context = constructor.into_circuit_context(ContractAddress::default());
    let recorded = RecordingFrame::new(context)
        .increment_counter(0_u8, 1)
        .unwrap()
        .finish(());
    let replay = recorded
        .public
        .initial()
        .query(
            recorded.public.verify_ops(),
            None,
            &recorded.execution.context.cost_model,
        )
        .unwrap();

    for state in [
        native.context.query.state.get_ref(),
        recorded.execution.context.query.state.get_ref(),
        replay.context.state.get_ref(),
    ] {
        let StateValue::Array(fields) = state else {
            panic!("expected ledger field array")
        };
        assert_eq!(read_counter(&fields.get(0).unwrap()).unwrap(), 1);
    }
    assert_eq!(recorded.public.verify_ops().len(), 3);
}
