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

use compact_rust_counter_fixture::ledger_contract::{
    Contract, PublicStateView, increment, initial_state, read_round, recorded,
};
use midnight_compact_runtime::context::ConstructorContext;
use midnight_compact_runtime::ledger::{ContractAddress, StateValue, read_counter};

#[test]
fn generated_counter_contract_runs_through_ledger_vm() {
    let constructor = initial_state(ConstructorContext::new(())).unwrap();
    assert_eq!(
        PublicStateView::from(&constructor).round().unwrap().value(),
        0
    );
    let context = constructor.into_circuit_context(ContractAddress::default());
    let read = read_round(context).unwrap();
    assert_eq!(read.result.value(), 0);
    let result = increment(read.context).unwrap();
    let StateValue::Array(fields) = result.context.query.state.get_ref() else {
        panic!("expected ledger field array")
    };
    assert_eq!(read_counter(fields.get(0).unwrap()).unwrap(), 1);
    assert_eq!(PublicStateView::from(&result).round().unwrap().value(), 1);
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
    let recorded = Contract::default().recording.increment(context).unwrap();
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
        assert_eq!(read_counter(fields.get(0).unwrap()).unwrap(), 1);
        assert_eq!(PublicStateView::from(state).round().unwrap().value(), 1);
    }
    assert_eq!(PublicStateView::from(&recorded).round().unwrap().value(), 1);
    assert_eq!(recorded.public.verify_ops().len(), 3);
}

#[test]
fn public_counter_view_rejects_wrong_shape_and_alignment() {
    use midnight_compact_runtime::ledger::{constructor_cell, contract_state, read_cell_at_path};

    let empty = contract_state::<midnight_compact_runtime::ledger::DefaultDB>(vec![]);
    assert_eq!(
        PublicStateView::from(empty.get_ref()).round().unwrap_err(),
        read_cell_at_path::<u64, _>(empty.get_ref(), &[0]).unwrap_err(),
    );
    let wrong_type =
        contract_state::<midnight_compact_runtime::ledger::DefaultDB>(vec![constructor_cell(
            false,
        )]);
    assert_eq!(
        PublicStateView::from(wrong_type.get_ref())
            .round()
            .unwrap_err(),
        read_cell_at_path::<u64, _>(wrong_type.get_ref(), &[0]).unwrap_err(),
    );
}

#[test]
fn generated_counter_contract_supports_both_recording_access_forms() {
    let constructor = initial_state(ConstructorContext::new(())).unwrap();
    let context = constructor.into_circuit_context(ContractAddress::default());
    let via_field = Contract::default().recording.increment(context).unwrap();

    let constructor = initial_state(ConstructorContext::new(())).unwrap();
    let context = constructor.into_circuit_context(ContractAddress::default());
    let via_method = Contract::default().recording().increment(context).unwrap();

    assert_eq!(
        via_method.public.verify_ops(),
        via_field.public.verify_ops()
    );
    assert_eq!(
        via_method.execution.context.query.state.get_ref(),
        via_field.execution.context.query.state.get_ref()
    );
    assert_eq!(
        via_method.execution.context.query.effects,
        via_field.execution.context.query.effects
    );
    assert_eq!(via_method.execution.gas_cost, via_field.execution.gas_cost);
    assert_eq!(
        via_method.execution.private_transcript_outputs,
        via_field.execution.private_transcript_outputs
    );
}

#[test]
fn generated_counter_read_has_a_replayable_observation() {
    let constructor = initial_state(ConstructorContext::new(())).unwrap();
    let context = constructor.into_circuit_context(ContractAddress::default());
    let incremented = recorded::increment(context).unwrap();
    let read = recorded::read_round(incremented.execution.context).unwrap();
    assert_eq!(read.execution.result.value(), 1);
    let replay = read
        .public
        .initial()
        .query(
            read.public.verify_ops(),
            None,
            &read.execution.context.cost_model,
        )
        .unwrap();
    assert_eq!(read.execution.context.query.effects, replay.context.effects);
    assert!(!read.public.verify_ops().is_empty());
}
