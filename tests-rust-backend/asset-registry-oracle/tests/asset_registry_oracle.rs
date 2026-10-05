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
    LedgerView, Witnesses, assertGrantEffective, close, initial_state, recorded, removeRecord,
    setCustodyGrant, setRecord, setWatch, tag,
};
use compact_rust_asset_registry_oracle_fixture::types::{
    AssetClass, AssetRecord, ContractAddress as Holder, CustodyGrant, ListMutation, RecordMutation,
};
use midnight_compact_runtime as runtime;
use midnight_onchain_state::state::{
    ContractMaintenanceAuthority, ContractOperation, ContractState, EntryPointBuf,
};
use midnight_serialize::tagged_serialize;
use midnight_storage::storage::HashMap;
use runtime::context::{ConstructorContext, WitnessContext};
use runtime::ledger::{ContractAddress, DefaultDB, StateValue};

#[test]
fn recorded_close_matches_native_assertion_and_replay() {
    let native = initial_state(ConstructorContext::new(()), &Stub).unwrap();
    let recording = initial_state(ConstructorContext::new(()), &Stub).unwrap();
    let native = close(
        native.into_circuit_context(ContractAddress::default()),
        &Stub,
    )
    .unwrap();
    let recorded = recorded::close(
        recording.into_circuit_context(ContractAddress::default()),
        &Stub,
    )
    .unwrap();
    let replay = recorded
        .public
        .initial()
        .query(
            recorded.public.verify_ops(),
            None,
            &recorded.execution.context.cost_model,
        )
        .unwrap();
    assert_eq!(recorded.execution.gas_cost, native.gas_cost);
    assert_eq!(
        recorded.execution.context.query.effects,
        native.context.query.effects
    );
    assert_eq!(replay.context.effects, native.context.query.effects);
    assert_eq!(
        recorded.execution.context.query.state.get_ref(),
        native.context.query.state.get_ref()
    );
    assert_eq!(
        replay.context.state.get_ref(),
        native.context.query.state.get_ref()
    );
    assert!(close(native.context, &Stub).is_err());
    assert!(recorded::close(recorded.execution.context, &Stub).is_err());
}

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

struct ParityStub;

impl Witnesses<()> for ParityStub {
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
        _: WitnessContext<'_, (), LedgerView<'_>>,
    ) -> ((), runtime::BoundedUint<{ u64::MAX as u128 }>) {
        ((), runtime::BoundedUint::new(1_700_000_000).unwrap())
    }
}

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

fn context_with_record() -> runtime::context::CircuitContext<()> {
    let initial = initial_state(ConstructorContext::new(()), &ParityStub).unwrap();
    let key = runtime::OpaqueString::from("asset-1");
    let record = AssetRecord {
        kind: AssetClass::Instrument,
        ..Default::default()
    };
    setRecord(
        initial.into_circuit_context(ContractAddress::default()),
        &ParityStub,
        key,
        record,
        RecordMutation::Insert,
    )
    .unwrap()
    .context
}

#[test]
fn recorded_asset_removal_matches_typescript_native_and_replay() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/asset-remove-record.json"
    ))
    .unwrap();
    let expected = &oracle["remove"];
    let key = runtime::OpaqueString::from("asset-1");
    let native_context = context_with_record();
    let recorded_context = context_with_record();
    assert_eq!(
        state_hex(native_context.query.state.get_ref().clone()),
        oracle["preRemoveHex"]
    );
    let native = removeRecord(native_context, &ParityStub, key.clone()).unwrap();
    let recorded = recorded::removeRecord(recorded_context, &ParityStub, key).unwrap();
    assert_eq!(expected["result"], serde_json::json!([]));
    let _: () = native.result;
    let _: () = recorded.execution.result;
    assert_eq!(recorded.execution.gas_cost, native.gas_cost);
    for output in [&native, &recorded.execution] {
        assert_eq!(
            state_hex(output.context.query.state.get_ref().clone()),
            expected["stateHex"]
        );
        let gas = serde_json::to_value(output.gas_cost).unwrap();
        for key in ["readTime", "computeTime", "bytesWritten", "bytesDeleted"] {
            let amount: u64 = expected["queries"]
                .as_array()
                .unwrap()
                .iter()
                .map(|query| {
                    query["gasCost"][key]
                        .as_str()
                        .unwrap()
                        .parse::<u64>()
                        .unwrap()
                })
                .sum();
            assert_eq!(gas[key], amount, "{key}");
        }
        assert_eq!(output.private_transcript_outputs.len(), 1);
        let transcript = &output.private_transcript_outputs[0];
        let atoms = transcript
            .value
            .0
            .iter()
            .map(|atom| &atom.0)
            .collect::<Vec<_>>();
        assert_eq!(
            serde_json::to_value(atoms).unwrap(),
            expected["privateTranscriptOutputs"][0]["value"]
        );
        assert_eq!(
            serde_json::to_value(&transcript.alignment).unwrap(),
            expected["privateTranscriptOutputs"][0]["alignment"]
        );
    }
    assert_eq!(
        recorded.execution.context.query.effects,
        native.context.query.effects
    );
    let _: () = recorded.execution.context.private_state;
    assert!(expected["privateState"].is_null());

    let mut program = serde_json::to_value(recorded.public.verify_ops()).unwrap();
    let expected_program = expected["queries"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|query| query["program"].as_array().unwrap().iter().cloned())
        .collect::<Vec<_>>();
    assert_eq!(program.as_array().unwrap().len(), expected_program.len());
    for operation in program.as_array_mut().unwrap() {
        if let Some(popeq) = operation.get_mut("popeq") {
            popeq.as_object_mut().unwrap().remove("result");
        }
    }
    for (index, (actual, expected_op)) in program
        .as_array()
        .unwrap()
        .iter()
        .zip(&expected_program)
        .enumerate()
    {
        assert_eq!(actual, expected_op, "operation {index}");
    }

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
        state_hex(replay.context.state.get_ref().clone()),
        expected["stateHex"]
    );
    let replay_gas = serde_json::to_value(replay.gas_cost).unwrap();
    for key in ["readTime", "computeTime", "bytesWritten", "bytesDeleted"] {
        let expected_gas: u64 = oracle["replayGas"][key].as_str().unwrap().parse().unwrap();
        assert_eq!(replay_gas[key], expected_gas, "replay {key}");
    }
}

#[test]
fn recorded_asset_removal_rejects_missing_and_watched_records() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/asset-remove-record.json"
    ))
    .unwrap();
    let initial = initial_state(ConstructorContext::new(()), &ParityStub).unwrap();
    let missing = recorded::removeRecord(
        initial.into_circuit_context(ContractAddress::default()),
        &ParityStub,
        runtime::OpaqueString::from("missing"),
    )
    .err()
    .unwrap();
    assert_eq!(missing.to_string(), oracle["missingError"]);
    let watched = setWatch(
        context_with_record(),
        &ParityStub,
        runtime::OpaqueString::from("asset-1"),
        ListMutation::Add,
    )
    .unwrap();
    let watched = recorded::removeRecord(
        watched.context,
        &ParityStub,
        runtime::OpaqueString::from("asset-1"),
    )
    .err()
    .unwrap();
    assert_eq!(watched.to_string(), oracle["watchedError"]);
}

fn context_with_grant() -> runtime::context::CircuitContext<()> {
    let initial = initial_state(ConstructorContext::new(()), &ParityStub).unwrap();
    let grant = CustodyGrant {
        code: runtime::FixedBytes::new([0; 32]),
        holder: Holder {
            bytes: runtime::FixedBytes::new([7; 32]),
        },
        grantedAt: runtime::BoundedUint::new(100).unwrap(),
    };
    setCustodyGrant(
        initial.into_circuit_context(ContractAddress::default()),
        &ParityStub,
        runtime::OpaqueString::from("grant-1"),
        grant,
        RecordMutation::Insert,
    )
    .unwrap()
    .context
}

#[test]
fn recorded_grant_effectiveness_matches_typescript_native_and_replay() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/asset-grant-effective.json"
    ))
    .unwrap();
    let expected = &oracle["effective"];
    let key = runtime::OpaqueString::from("grant-1");
    let time = runtime::BoundedUint::new(120).unwrap();
    let native_context = context_with_grant();
    let recorded_context = context_with_grant();
    assert_eq!(
        state_hex(native_context.query.state.get_ref().clone()),
        oracle["preReadHex"]
    );
    let native = assertGrantEffective(native_context, key.clone(), time).unwrap();
    let recorded = recorded::assertGrantEffective(recorded_context, key, time).unwrap();
    let _: () = native.result;
    let _: () = recorded.execution.result;
    assert_eq!(expected["result"], serde_json::json!([]));
    assert_eq!(native.gas_cost, recorded.execution.gas_cost);
    for output in [&native, &recorded.execution] {
        assert_eq!(
            state_hex(output.context.query.state.get_ref().clone()),
            expected["stateHex"]
        );
        assert_eq!(expected["stateHex"], oracle["preReadHex"]);
        assert!(output.private_transcript_outputs.is_empty());
        let gas = serde_json::to_value(output.gas_cost).unwrap();
        for key in ["readTime", "computeTime", "bytesWritten", "bytesDeleted"] {
            let amount: u64 = expected["queries"]
                .as_array()
                .unwrap()
                .iter()
                .map(|query| {
                    query["gasCost"][key]
                        .as_str()
                        .unwrap()
                        .parse::<u64>()
                        .unwrap()
                })
                .sum();
            assert_eq!(gas[key], amount, "{key}");
        }
    }
    assert_eq!(
        native.context.query.effects,
        recorded.execution.context.query.effects
    );
    assert_eq!(expected["privateTranscriptOutputs"], serde_json::json!([]));
    let _: () = recorded.execution.context.private_state;
    assert!(expected["privateState"].is_null());

    let mut program = serde_json::to_value(recorded.public.verify_ops()).unwrap();
    let expected_program = expected["queries"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|query| query["program"].as_array().unwrap().iter().cloned())
        .collect::<Vec<_>>();
    assert_eq!(program.as_array().unwrap().len(), expected_program.len());
    for operation in program.as_array_mut().unwrap() {
        if let Some(popeq) = operation.get_mut("popeq") {
            popeq.as_object_mut().unwrap().remove("result");
        }
    }
    assert_eq!(program, serde_json::Value::Array(expected_program));

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
        state_hex(replay.context.state.get_ref().clone()),
        expected["stateHex"]
    );
    let replay_gas = serde_json::to_value(replay.gas_cost).unwrap();
    for key in ["readTime", "computeTime", "bytesWritten", "bytesDeleted"] {
        let expected_gas: u64 = oracle["replayGas"][key].as_str().unwrap().parse().unwrap();
        assert_eq!(replay_gas[key], expected_gas, "replay {key}");
    }
}

#[test]
fn recorded_grant_effectiveness_rejects_missing_and_future_grants() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/asset-grant-effective.json"
    ))
    .unwrap();
    let initial = initial_state(ConstructorContext::new(()), &ParityStub).unwrap();
    let missing = recorded::assertGrantEffective(
        initial.into_circuit_context(ContractAddress::default()),
        runtime::OpaqueString::from("missing"),
        runtime::BoundedUint::new(120).unwrap(),
    )
    .err()
    .unwrap();
    assert_eq!(missing.to_string(), oracle["missingError"]);
    let future = recorded::assertGrantEffective(
        context_with_grant(),
        runtime::OpaqueString::from("grant-1"),
        runtime::BoundedUint::new(99).unwrap(),
    )
    .err()
    .unwrap();
    assert_eq!(future.to_string(), oracle["futureError"]);
}
