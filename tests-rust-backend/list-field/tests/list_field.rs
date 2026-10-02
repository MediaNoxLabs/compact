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

use compact_rust_list_field_fixture::ledger_contract::{
    Contract, clear_items, drop_first, first_item, initial_state, item_count, items_empty, prepend,
};
use midnight_compact_runtime::Field;
use midnight_compact_runtime::context::ConstructorContext;
use midnight_compact_runtime::ledger::{ContractAddress, StateValue};

#[test]
fn generated_empty_list_has_upstream_shape_and_zero_length() {
    let constructor = initial_state(ConstructorContext::new(())).unwrap();
    let StateValue::Array(fields) = constructor.ledger_state.get_ref() else {
        panic!("expected ledger root array")
    };
    let StateValue::Array(list) = fields.get(0).unwrap() else {
        panic!("expected List array")
    };
    assert!(matches!(list.get(0), Some(StateValue::Null)));
    assert!(matches!(list.get(1), Some(StateValue::Null)));
    assert_eq!(list.len(), 3);
    let context = constructor.into_circuit_context(ContractAddress::default());
    let result = item_count(context).unwrap();
    assert_eq!(result.result.value(), 0);
    let result = items_empty(result.context).unwrap();
    assert!(result.result);
    let result = first_item(result.context).unwrap();
    assert!(!result.result.is_some);
    assert_eq!(result.result.value, Field::from(0_u64));
    let result = prepend(result.context, Field::from(11_u64)).unwrap();
    let result = item_count(result.context).unwrap();
    assert_eq!(result.result.value(), 1);
    let result = first_item(result.context).unwrap();
    assert!(result.result.is_some);
    assert_eq!(result.result.value, Field::from(11_u64));
    let result = items_empty(result.context).unwrap();
    assert!(!result.result);
    let result = prepend(result.context, Field::from(22_u64)).unwrap();
    let result = item_count(result.context).unwrap();
    assert_eq!(result.result.value(), 2);
    let result = first_item(result.context).unwrap();
    assert!(result.result.is_some);
    assert_eq!(result.result.value, Field::from(22_u64));
    let result = drop_first(result.context).unwrap();
    let result = item_count(result.context).unwrap();
    assert_eq!(result.result.value(), 1);
    let result = first_item(result.context).unwrap();
    assert!(result.result.is_some);
    assert_eq!(result.result.value, Field::from(11_u64));
    let result = clear_items(result.context).unwrap();
    let result = item_count(result.context).unwrap();
    assert_eq!(result.result.value(), 0);
    let result = items_empty(result.context).unwrap();
    assert!(result.result);
    let result = first_item(result.context).unwrap();
    assert!(!result.result.is_some);
    assert_eq!(result.result.value, Field::from(0_u64));
    let result = prepend(result.context, Field::from(33_u64)).unwrap();
    let result = item_count(result.context).unwrap();
    assert_eq!(result.result.value(), 1);
}

#[test]
fn generated_list_recording_replays_and_matches_native_state() {
    let initial = initial_state(ConstructorContext::new(())).unwrap();
    let context = initial.into_circuit_context(ContractAddress::default());
    let native = prepend(
        initial_state(ConstructorContext::new(()))
            .unwrap()
            .into_circuit_context(ContractAddress::default()),
        Field::from(11_u64),
    )
    .unwrap();
    let recorded = Contract::default()
        .recording
        .prepend(context, Field::from(11_u64))
        .unwrap();
    assert_eq!(
        native.context.query.state.get_ref(),
        recorded.execution.context.query.state.get_ref()
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
    assert_eq!(
        replay.context.effects,
        recorded.execution.context.query.effects
    );
    let head = Contract::default()
        .recording
        .first_item(recorded.execution.context)
        .unwrap();
    assert!(head.execution.result.is_some);
    assert_eq!(head.execution.result.value, Field::from(11_u64));
    let replay = head
        .public
        .initial()
        .query(
            head.public.verify_ops(),
            None,
            &head.execution.context.cost_model,
        )
        .unwrap();
    assert_eq!(replay.context.effects, head.execution.context.query.effects);
}
