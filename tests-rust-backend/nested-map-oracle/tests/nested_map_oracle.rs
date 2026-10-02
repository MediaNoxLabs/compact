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

use compact_rust_nested_map_oracle_fixture::ledger_contract::{initial_state, ping};
use compact_rust_nested_map_oracle_fixture::ledger_slots::users_by_org;
use midnight_compact_runtime::Field;
use midnight_compact_runtime::context::ConstructorContext;
use midnight_compact_runtime::ledger::{self, ContractAddress, DefaultDB, StateValue};
use midnight_compact_runtime::recording::RecordingFrame;
use midnight_compact_runtime::slots::{MapNode, MapSlot};
use midnight_onchain_state::state::{
    ContractMaintenanceAuthority, ContractOperation, ContractState, EntryPointBuf,
};
use midnight_serialize::tagged_serialize;
use midnight_storage::storage::HashMap;

fn state_hex(state: StateValue<DefaultDB>) -> String {
    let operations = HashMap::new().insert(
        EntryPointBuf(b"ping".to_vec()),
        ContractOperation::new(None),
    );
    let state = ContractState::new(state, operations, ContractMaintenanceAuthority::default());
    let mut bytes = Vec::new();
    tagged_serialize(&state, &mut bytes).unwrap();
    hex::encode(bytes)
}

fn assert_empty_nested_map(state: &StateValue<DefaultDB>) {
    let StateValue::Array(fields) = state else {
        panic!("expected public ledger array");
    };
    let StateValue::Map(map) = fields.get(1).unwrap() else {
        panic!("expected outer ledger Map");
    };
    assert_eq!(map.size(), 0);
}

#[test]
fn nested_map_initial_shape_and_cell_write_match_typescript() {
    let reference: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/nested-map-oracle.json"
    ))
    .unwrap();
    let initial = initial_state(ConstructorContext::new(())).unwrap();
    let state = initial.ledger_state.get_ref().clone();
    assert_empty_nested_map(&state);
    assert_eq!(state_hex(state.clone()), reference["afterInit"]["stateHex"]);
    assert_eq!(
        ledger::read_root_cell::<bool, _>(&state, 0).unwrap(),
        reference["afterInit"]["flag"]
    );

    let next = ping(initial.into_circuit_context(ContractAddress::default())).unwrap();
    let state = next.context.query.state.get_ref().clone();
    assert_empty_nested_map(&state);
    assert_eq!(state_hex(state.clone()), reference["afterPing"]["stateHex"]);
    assert_eq!(
        ledger::read_root_cell::<bool, _>(&state, 0).unwrap(),
        reference["afterPing"]["flag"]
    );
}

#[test]
fn nested_map_slot_shape_reads_match_native_gas_and_replay() {
    let _: MapSlot<
        Field,
        MapNode<Field, midnight_compact_runtime::BoundedUint<{ u64::MAX as u128 }>>,
    > = users_by_org;
    assert_eq!(users_by_org.path(), &[1]);

    let initial = initial_state(ConstructorContext::new(())).unwrap();
    let context = initial.into_circuit_context(ContractAddress::default());
    let empty = users_by_org.is_empty(context).unwrap();
    assert!(empty.result);
    let size = users_by_org.size(empty.context).unwrap();
    assert_eq!(size.result, 0);
    let native_member = users_by_org
        .member(size.context, Field::from(7_u64))
        .unwrap();
    assert!(!native_member.result);
    let gas = empty.gas_cost + size.gas_cost + native_member.gas_cost;

    let initial = initial_state(ConstructorContext::new(())).unwrap();
    let context = initial.into_circuit_context(ContractAddress::default());
    let (frame, empty) = users_by_org
        .record_is_empty(RecordingFrame::new(context))
        .unwrap();
    assert!(empty);
    let (frame, size) = users_by_org.record_size(frame).unwrap();
    assert_eq!(size, 0);
    let (frame, recorded_member) = users_by_org
        .record_member(frame, Field::from(7_u64))
        .unwrap();
    assert!(!recorded_member);
    let recorded = frame.finish(());
    assert_eq!(recorded.execution.gas_cost, gas);
    assert_eq!(
        recorded.execution.context.query.state.get_ref(),
        native_member.context.query.state.get_ref()
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
        replay.context.state.get_ref(),
        native_member.context.query.state.get_ref()
    );
    assert_eq!(replay.context.effects, native_member.context.query.effects);
}
