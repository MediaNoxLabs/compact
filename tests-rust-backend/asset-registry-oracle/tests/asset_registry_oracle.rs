#![allow(non_snake_case)]

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

use compact_rust_asset_registry_oracle_fixture::ledger_contract::{
    LedgerView, Witnesses, initial_state, removeRecord, setRecord, setWatch, tag,
};
use compact_rust_asset_registry_oracle_fixture::types::{
    AssetClass, AssetRecord, ListMutation, RecordMutation,
};
use midnight_compact_runtime as runtime;
use midnight_onchain_state::state::{
    ContractMaintenanceAuthority, ContractOperation, ContractState, EntryPointBuf,
};
use midnight_serialize::tagged_serialize;
use midnight_storage::storage::HashMap;
use runtime::context::{ConstructorContext, WitnessContext};
use runtime::ledger::{ContractAddress, DefaultDB, StateValue};

fn state_hex(state: StateValue<DefaultDB>) -> String {
    let mut operations: HashMap<EntryPointBuf, ContractOperation, DefaultDB> = HashMap::new();
    for name in [
        "setCustodian",
        "setRecord",
        "removeRecord",
        "setCustodyGrant",
        "setWatch",
        "tag",
        "assertStoredRecordFresh",
        "assertGrantEffective",
        "acceptIfFresh",
        "close",
    ] {
        operations = operations.insert(
            EntryPointBuf(name.as_bytes().to_vec()),
            ContractOperation::new(None),
        );
    }
    let contract_state =
        ContractState::new(state, operations, ContractMaintenanceAuthority::default());
    let mut bytes = Vec::new();
    tagged_serialize(&contract_state, &mut bytes).unwrap();
    hex::encode(bytes)
}

fn assert_snapshot(state: &StateValue<DefaultDB>, oracle: &serde_json::Value, index: usize) {
    assert_eq!(state_hex(state.clone()), oracle[index]["stateHex"]);
    let record_count = runtime::ledger::read_cell_at_path::<u64, _>(state, &[1, 5]).unwrap();
    let write_count = runtime::ledger::read_cell_at_path::<u64, _>(state, &[1, 9]).unwrap();
    assert_eq!(record_count.to_string(), oracle[index]["recordCount"]);
    assert_eq!(write_count.to_string(), oracle[index]["writeCount"]);
}

struct Stub;

impl Witnesses<()> for Stub {
    fn localOperatorKey(
        &self,
        _: WitnessContext<'_, (), LedgerView<'_>>,
    ) -> ((), runtime::JubjubPoint) {
        ((), runtime::hash_to_curve(runtime::Field::from(1_u64)))
    }

    fn localAuditorKey(
        &self,
        _: WitnessContext<'_, (), LedgerView<'_>>,
    ) -> ((), runtime::JubjubPoint) {
        ((), runtime::hash_to_curve(runtime::Field::from(2_u64)))
    }

    fn currentTimestamp(
        &self,
        context: WitnessContext<'_, (), LedgerView<'_>>,
    ) -> ((), runtime::BoundedUint<{ u64::MAX as u128 }>) {
        context.ledger.recordCount().unwrap();
        context.ledger.records().unwrap();
        context.ledger.watchList().unwrap();
        ((), runtime::BoundedUint::new(1_700_000_000).unwrap())
    }
}

#[test]
fn chunked_collections_and_counters_execute_through_the_asset_registry() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/asset-registry-oracle.json"
    ))
    .unwrap();
    let initial = initial_state(ConstructorContext::new(()), &Stub).unwrap();
    let initial_state = initial.ledger_state.get_ref();
    assert_snapshot(initial_state, &oracle, 0);
    assert_eq!(
        runtime::ledger::read_cell_at_path::<u64, _>(initial_state, &[1, 5]).unwrap(),
        0
    );
    assert_eq!(
        runtime::ledger::set_view_at_path::<runtime::Field, DefaultDB>(initial_state, &[1, 14])
            .unwrap()
            .size()
            .unwrap()
            .value(),
        0
    );

    let context = initial.into_circuit_context(ContractAddress::default());
    let tagged = tag(context, &Stub, runtime::Field::from(7_u64)).unwrap();
    let state = tagged.context.query.state.get_ref();
    assert_snapshot(state, &oracle, 1);
    assert!(
        runtime::ledger::set_view_at_path::<runtime::Field, DefaultDB>(state, &[1, 14])
            .unwrap()
            .member(runtime::Field::from(7_u64))
    );

    let key = runtime::OpaqueString::from("asset-1");
    let record = AssetRecord {
        kind: AssetClass::Instrument,
        ..Default::default()
    };
    let inserted = setRecord(
        tagged.context,
        &Stub,
        key.clone(),
        record.clone(),
        RecordMutation::Insert,
    )
    .unwrap();
    let state = inserted.context.query.state.get_ref();
    assert_snapshot(state, &oracle, 2);
    assert_eq!(
        runtime::ledger::read_cell_at_path::<u64, _>(state, &[1, 5]).unwrap(),
        1
    );
    let records =
        runtime::ledger::map_view_at_path::<runtime::OpaqueString, AssetRecord, _>(state, &[1, 10])
            .unwrap();
    assert!(records.member(key.clone()));
    assert_eq!(records.lookup(key.clone()).unwrap(), record);

    let watched = setWatch(inserted.context, &Stub, key.clone(), ListMutation::Add).unwrap();
    let state = watched.context.query.state.get_ref();
    assert_snapshot(state, &oracle, 3);
    assert!(
        runtime::ledger::set_view_at_path::<runtime::OpaqueString, _>(state, &[1, 13])
            .unwrap()
            .member(key.clone())
    );
    let unwatched = setWatch(watched.context, &Stub, key.clone(), ListMutation::Drop).unwrap();
    let state = unwatched.context.query.state.get_ref();
    assert_snapshot(state, &oracle, 4);
    assert!(
        !runtime::ledger::set_view_at_path::<runtime::OpaqueString, _>(state, &[1, 13])
            .unwrap()
            .member(key.clone())
    );
    let removed = removeRecord(unwatched.context, &Stub, key.clone()).unwrap();
    let state = removed.context.query.state.get_ref();
    assert_snapshot(state, &oracle, 5);
    assert_eq!(
        runtime::ledger::read_cell_at_path::<u64, _>(state, &[1, 5]).unwrap(),
        1
    );
    assert!(
        !runtime::ledger::map_view_at_path::<runtime::OpaqueString, AssetRecord, _>(
            state,
            &[1, 10]
        )
        .unwrap()
        .member(key.clone())
    );
    assert!(
        runtime::ledger::set_view_at_path::<runtime::OpaqueString, _>(state, &[1, 12])
            .unwrap()
            .member(key)
    );
}
