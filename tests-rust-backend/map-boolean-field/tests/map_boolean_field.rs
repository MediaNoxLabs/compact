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

use compact_rust_map_boolean_field_fixture::ledger_contract::{
    Contract, PublicStateView, get, has, initial_state, put, put_default, remove_key, reset_table,
    table_is_empty, table_size,
};
use midnight_compact_runtime::Field;
use midnight_compact_runtime::context::ConstructorContext;
use midnight_compact_runtime::ledger::{ContractAddress, StateValue, map_view_at_path};

#[test]
fn generated_map_contract_inserts_and_looks_up_field_values() {
    let constructor = initial_state(ConstructorContext::new(())).unwrap();
    assert!(
        PublicStateView::from(&constructor)
            .table()
            .unwrap()
            .is_empty()
    );
    let StateValue::Array(fields) = constructor.ledger_state.get_ref() else {
        panic!("expected ledger field array")
    };
    let StateValue::Map(map) = fields.get(0).unwrap() else {
        panic!("expected Map")
    };
    assert_eq!(map.size(), 0);

    let context = constructor.into_circuit_context(ContractAddress::default());
    let result = table_is_empty(context).unwrap();
    assert!(result.result);
    let result = table_size(result.context).unwrap();
    assert_eq!(result.result.value(), 0);
    let result = has(result.context, true).unwrap();
    assert!(!result.result);
    let result = put_default(result.context, true).unwrap();
    let result = get(result.context, true).unwrap();
    assert_eq!(result.result, Field::from(0_u64));
    let result = put(result.context, true, Field::from(42_u64)).unwrap();
    let result = table_size(result.context).unwrap();
    assert_eq!(result.result.value(), 1);
    let result = table_is_empty(result.context).unwrap();
    assert!(!result.result);
    let result = has(result.context, true).unwrap();
    assert!(result.result);
    let result = get(result.context, true).unwrap();
    assert_eq!(result.result, Field::from(42_u64));
    let view = PublicStateView::from(&result).table().unwrap();
    let raw =
        map_view_at_path::<bool, Field, _>(result.context.query.state.get_ref(), &[0]).unwrap();
    assert_eq!(view.lookup(true).unwrap(), raw.lookup(true).unwrap());
    assert_eq!(view.size().unwrap(), raw.size().unwrap());
    assert!(!view.member(false));
    let result = put(result.context, true, Field::from(7_u64)).unwrap();
    let result = get(result.context, true).unwrap();
    assert_eq!(result.result, Field::from(7_u64));
    let result = put_default(result.context, true).unwrap();
    let result = get(result.context, true).unwrap();
    assert_eq!(result.result, Field::from(0_u64));
    let result = has(result.context, false).unwrap();
    assert!(!result.result);
    let StateValue::Array(fields) = result.context.query.state.get_ref() else {
        panic!("expected ledger field array")
    };
    let StateValue::Map(map) = fields.get(0).unwrap() else {
        panic!("expected Map")
    };
    assert_eq!(map.size(), 1);

    let result = remove_key(result.context, true).unwrap();
    let result = table_size(result.context).unwrap();
    assert_eq!(result.result.value(), 0);
    let result = table_is_empty(result.context).unwrap();
    assert!(result.result);
    let result = put(result.context, false, Field::from(11_u64)).unwrap();
    let result = reset_table(result.context).unwrap();
    let result = table_size(result.context).unwrap();
    assert_eq!(result.result.value(), 0);
    let result = table_is_empty(result.context).unwrap();
    assert!(result.result);
    assert!(PublicStateView::from(&result).table().unwrap().is_empty());
}

#[test]
fn public_map_view_preserves_raw_errors() {
    use midnight_compact_runtime::ledger::{constructor_cell, contract_state};

    let empty = contract_state::<midnight_compact_runtime::ledger::DefaultDB>(vec![]);
    assert_eq!(
        PublicStateView::from(empty.get_ref())
            .table()
            .err()
            .unwrap(),
        map_view_at_path::<bool, Field, _>(empty.get_ref(), &[0])
            .err()
            .unwrap(),
    );
    let wrong_shape =
        contract_state::<midnight_compact_runtime::ledger::DefaultDB>(vec![constructor_cell(
            false,
        )]);
    assert_eq!(
        PublicStateView::from(wrong_shape.get_ref())
            .table()
            .err()
            .unwrap(),
        map_view_at_path::<bool, Field, _>(wrong_shape.get_ref(), &[0])
            .err()
            .unwrap(),
    );

    let initial = initial_state(ConstructorContext::new(())).unwrap();
    let context = initial.into_circuit_context(ContractAddress::default());
    let wrong_value = context.insert_map(&[0][..], true, false).unwrap();
    let typed = PublicStateView::from(&wrong_value).table().unwrap();
    let raw = map_view_at_path::<bool, Field, _>(wrong_value.context.query.state.get_ref(), &[0])
        .unwrap();
    assert_eq!(
        typed.lookup(false).unwrap_err(),
        raw.lookup(false).unwrap_err()
    );
    assert_eq!(
        typed.lookup(true).unwrap_err(),
        raw.lookup(true).unwrap_err()
    );
}

#[test]
fn generated_map_recording_replays_native_calls() {
    let contract = Contract::default();
    let context = initial_state(ConstructorContext::new(()))
        .unwrap()
        .into_circuit_context(ContractAddress::default());
    let native_context = initial_state(ConstructorContext::new(()))
        .unwrap()
        .into_circuit_context(ContractAddress::default());
    let native = put(native_context, true, Field::from(42_u64)).unwrap();
    let recorded = contract
        .recording
        .put(context, true, Field::from(42_u64))
        .unwrap();
    assert_eq!(recorded.execution.gas_cost, native.gas_cost);
    assert_eq!(
        PublicStateView::from(&recorded)
            .table()
            .unwrap()
            .lookup(true)
            .unwrap(),
        Field::from(42_u64),
    );
    assert_eq!(
        recorded.execution.context.query.state.get_ref(),
        native.context.query.state.get_ref()
    );
    let replay = recorded
        .public
        .initial()
        .query(
            recorded.public.verify_ops(),
            None,
            &recorded.execution.context.cost_model,
        )
        .unwrap();
    assert_eq!(replay.context.effects, native.context.query.effects);
    assert_eq!(
        PublicStateView::from(&replay)
            .table()
            .unwrap()
            .lookup(true)
            .unwrap(),
        Field::from(42_u64),
    );

    let native = get(native.context, true).unwrap();
    let recorded = contract
        .recording
        .get(recorded.execution.context, true)
        .unwrap();
    assert_eq!(recorded.execution.result, Field::from(42_u64));
    assert_eq!(recorded.execution.gas_cost, native.gas_cost);
    let replay = recorded
        .public
        .initial()
        .query(
            recorded.public.verify_ops(),
            None,
            &recorded.execution.context.cost_model,
        )
        .unwrap();
    assert_eq!(replay.context.effects, native.context.query.effects);
}
