// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//   http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::cell::RefCell;

use compact_rust_asset_registry_oracle_fixture::ledger_contract::{
    LedgerView, Witnesses, acceptIfFresh, initial_state, recorded, setCustodian, setCustodyGrant,
    setRecord, tag,
};
use compact_rust_asset_registry_oracle_fixture::ledger_slots;
use compact_rust_asset_registry_oracle_fixture::types::{
    AssetClass, AssetRecord, ContractAddress as Holder, CustodyGrant, FreshnessPolicy, Provenance,
    RecordMutation,
};
use midnight_compact_runtime as runtime;
use midnight_onchain_state::state::{
    ContractMaintenanceAuthority, ContractOperation, ContractState, EntryPointBuf,
};
use midnight_serialize::tagged_serialize;
use midnight_storage::storage::HashMap;
use runtime::context::{CircuitContext, CircuitResult, ConstructorContext, WitnessContext};
use runtime::ledger::{ContractAddress, DefaultDB, StateValue};
use runtime::recording::{RecordedCircuitResult, RecordingFrame};
use runtime::{BoundedUint, Field, FixedBytes, JubjubPoint};
use serde_json::{Value, json};

#[derive(Default)]
struct TrackingWitness(RefCell<Vec<&'static str>>);

impl TrackingWitness {
    fn calls(&self) -> Vec<&'static str> {
        self.0.borrow().clone()
    }

    fn clear(&self) {
        self.0.borrow_mut().clear();
    }
}

impl Witnesses<u64> for TrackingWitness {
    fn localOperatorKey(
        &self,
        context: WitnessContext<'_, u64, LedgerView<'_>>,
    ) -> (u64, JubjubPoint) {
        self.0.borrow_mut().push("localOperatorKey");
        (
            *context.private_state + 1,
            runtime::hash_to_curve(Field::from(1_u64)),
        )
    }

    fn localAuditorKey(
        &self,
        context: WitnessContext<'_, u64, LedgerView<'_>>,
    ) -> (u64, JubjubPoint) {
        self.0.borrow_mut().push("localAuditorKey");
        (
            *context.private_state + 1,
            runtime::hash_to_curve(Field::from(2_u64)),
        )
    }

    fn currentTimestamp(
        &self,
        context: WitnessContext<'_, u64, LedgerView<'_>>,
    ) -> (u64, BoundedUint<{ u64::MAX as u128 }>) {
        self.0.borrow_mut().push("currentTimestamp");
        context.ledger.recordCount().unwrap();
        context.ledger.records().unwrap();
        context.ledger.watchList().unwrap();
        (
            *context.private_state + 1,
            BoundedUint::new(1_700_000_000).unwrap(),
        )
    }
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
    let contract = ContractState::new(state, operations, ContractMaintenanceAuthority::default());
    let mut bytes = Vec::new();
    tagged_serialize(&contract, &mut bytes).unwrap();
    hex::encode(bytes)
}

fn oracle() -> Value {
    serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/asset-writable-oracle.json"
    ))
    .unwrap()
}

fn initial(mode: &str, witnesses: &TrackingWitness) -> CircuitContext<u64> {
    let initial = initial_state(ConstructorContext::new(7_u64), witnesses).unwrap();
    assert_eq!(initial.private_state, 10);
    assert_eq!(
        witnesses.calls(),
        ["localOperatorKey", "localAuditorKey", "currentTimestamp"]
    );
    witnesses.clear();
    let context = initial.into_circuit_context(ContractAddress::default());
    match mode {
        "success" => context,
        "closed" => context.write_cell_at_path(&[1, 6], false).unwrap().context,
        "frozen" => context.write_cell_at_path(&[1, 7], true).unwrap().context,
        _ => unreachable!(),
    }
}

fn holder() -> Holder {
    Holder {
        bytes: FixedBytes::new([7; 32]),
    }
}

fn shape(serialized: Value) -> Value {
    Value::Array(
        serialized
            .as_array()
            .unwrap()
            .iter()
            .map(|op| {
                if let Some(kind) = op.as_str() {
                    return json!({ "kind": kind });
                }
                if let Some(idx) = op.get("idx") {
                    json!({ "kind": "idx", "cached": idx["cached"], "pushPath": idx["pushPath"],
                "pathLength": idx["path"].as_array().unwrap().len() })
                } else if let Some(push) = op.get("push") {
                    json!({ "kind": "push", "storage": push["storage"] })
                } else if let Some(ins) = op.get("ins") {
                    json!({ "kind": "ins", "cached": ins["cached"], "n": ins["n"] })
                } else if let Some(dup) = op.get("dup") {
                    json!({ "kind": "dup", "n": dup["n"] })
                } else if let Some(addi) = op.get("addi") {
                    json!({ "kind": "addi", "immediate": addi["immediate"] })
                } else if let Some(rem) = op.get("rem") {
                    json!({ "kind": "rem", "cached": rem["cached"] })
                } else if let Some(popeq) = op.get("popeq") {
                    json!({ "kind": "popeq", "cached": popeq["cached"] })
                } else {
                    panic!("unexpected VM operation: {op}");
                }
            })
            .collect(),
    )
}

fn check_fab(output: &[runtime::fab::AlignedValue], expected: &Value) {
    let expected = expected.as_array().unwrap();
    assert_eq!(output.len(), expected.len());
    for (actual, expected) in output.iter().zip(expected) {
        let atoms = actual
            .value
            .0
            .iter()
            .map(|atom| &atom.0)
            .collect::<Vec<_>>();
        assert_eq!(serde_json::to_value(atoms).unwrap(), expected["valueAtoms"]);
        assert_eq!(
            serde_json::to_value(&actual.alignment).unwrap(),
            expected["alignment"]
        );
    }
}

fn check_success(
    native: &CircuitResult<u64, ()>,
    recorded: &RecordedCircuitResult<u64, ()>,
    expected: &Value,
    name: &str,
) {
    assert_eq!(
        native.context.private_state,
        expected["privateState"].as_u64().unwrap()
    );
    assert_eq!(
        recorded.execution.context.private_state,
        native.context.private_state
    );
    check_fab(
        &native.private_transcript_outputs,
        &expected["privateTranscriptOutputs"],
    );
    check_fab(
        &recorded.execution.private_transcript_outputs,
        &expected["privateTranscriptOutputs"],
    );
    assert_eq!(
        state_hex(native.context.query.state.get_ref().clone()),
        expected["stateHex"],
        "{name}: TypeScript native state"
    );
    assert_eq!(
        state_hex(recorded.execution.context.query.state.get_ref().clone()),
        expected["stateHex"],
        "{name}: TypeScript recorded state"
    );
    assert_eq!(
        recorded.execution.context.query.effects, native.context.query.effects,
        "{name}: effects"
    );
    assert_eq!(
        recorded.execution.gas_cost, native.gas_cost,
        "{name}: native gas"
    );
    let gas = serde_json::to_value(native.gas_cost).unwrap();
    for dimension in ["readTime", "computeTime", "bytesWritten", "bytesDeleted"] {
        let expected_total: u64 = expected["queries"]
            .as_array()
            .unwrap()
            .iter()
            .map(|query| {
                query["gasCost"][dimension]
                    .as_str()
                    .unwrap()
                    .parse::<u64>()
                    .unwrap()
            })
            .sum();
        assert_eq!(
            gas[dimension].as_u64().unwrap(),
            expected_total,
            "{name}: {dimension}"
        );
    }
    assert_eq!(
        shape(serde_json::to_value(recorded.public.verify_ops()).unwrap()),
        expected["publicTranscriptShape"],
        "{name}: ordered VM"
    );
    if let Some(expected_program) = expected.get("vmProgram") {
        let mut actual = serde_json::to_value(recorded.public.verify_ops()).unwrap();
        for operation in actual.as_array_mut().unwrap() {
            if let Some(popeq) = operation.get_mut("popeq") {
                popeq.as_object_mut().unwrap().remove("result");
            }
        }
        let mut expected_program = expected_program.clone();
        for operation in expected_program.as_array_mut().unwrap() {
            if let Some(popeq) = operation.get_mut("popeq") {
                popeq.as_object_mut().unwrap().remove("result");
            }
        }
        let actual_ops = actual.as_array().unwrap();
        let expected_ops = expected_program.as_array().unwrap();
        if actual_ops.len() != expected_ops.len() {
            let tags = |ops: &[Value]| {
                ops.iter()
                    .map(|op| {
                        op.as_str().map(str::to_owned).unwrap_or_else(|| {
                            op.as_object().unwrap().keys().next().unwrap().to_owned()
                        })
                    })
                    .collect::<Vec<_>>()
            };
            panic!(
                "{name}: VM length {} vs {}; Rust {:?}; TS {:?}",
                actual_ops.len(),
                expected_ops.len(),
                tags(actual_ops),
                tags(expected_ops)
            );
        }
        for (index, (actual, expected)) in actual_ops.iter().zip(expected_ops).enumerate() {
            assert_eq!(actual, expected, "{name}: VM operation {index}");
        }
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
    assert_eq!(
        replay.context.state.get_ref(),
        native.context.query.state.get_ref(),
        "{name}: replay state"
    );
    assert_eq!(
        replay.context.effects, native.context.query.effects,
        "{name}: replay effects"
    );
    if let Some(expected_gas) = expected.get("replayGas") {
        let actual = serde_json::to_value(replay.gas_cost).unwrap();
        for dimension in ["readTime", "computeTime", "bytesWritten", "bytesDeleted"] {
            let target: u64 = expected_gas[dimension].as_str().unwrap().parse().unwrap();
            assert_eq!(actual[dimension], target, "{name}: replay {dimension}");
        }
    }
}

fn check_failure_prefix(mode: &str, expected: &Value) {
    // A failed call returns only its error. Replay the two declared guard
    // observations separately to compare the observable prefix with TS.
    let witnesses = TrackingWitness::default();
    let context = initial(mode, &witnesses);
    let (frame, open) = ledger_slots::open
        .record_read(RecordingFrame::new(context))
        .unwrap();
    let frame = if open {
        let (frame, frozen) = ledger_slots::frozen.record_read(frame).unwrap();
        assert!(frozen);
        frame
    } else {
        assert_eq!(mode, "closed");
        frame
    };
    let prefix = frame.finish(());
    let op_tags = serde_json::to_value(prefix.public.verify_ops())
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .map(|op| {
            if let Some(kind) = op.as_str() {
                kind.to_owned()
            } else {
                op.as_object().unwrap().keys().next().unwrap().to_owned()
            }
        })
        .collect::<Vec<_>>();
    let ts_tags = expected["queries"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|query| query["opTags"].as_array().unwrap().iter())
        .map(|tag| tag.as_str().unwrap().to_owned())
        .collect::<Vec<_>>();
    assert_eq!(op_tags, ts_tags);
    let gas = serde_json::to_value(prefix.execution.gas_cost).unwrap();
    for dimension in ["readTime", "computeTime", "bytesWritten", "bytesDeleted"] {
        let expected_total: u64 = expected["queries"]
            .as_array()
            .unwrap()
            .iter()
            .map(|query| {
                query["gasCost"][dimension]
                    .as_str()
                    .unwrap()
                    .parse::<u64>()
                    .unwrap()
            })
            .sum();
        assert_eq!(gas[dimension].as_u64().unwrap(), expected_total);
    }
    assert_eq!(
        state_hex(prefix.execution.context.query.state.get_ref().clone()),
        expected["initialStateHex"]
    );
    assert_eq!(prefix.execution.context.private_state, 10);
    assert!(prefix.execution.private_transcript_outputs.is_empty());
    assert!(witnesses.calls().is_empty());
}

#[test]
fn asset_writable_guard_records_two_successes_and_rejects_closed_or_frozen() {
    let reference = oracle();
    for circuit in ["setCustodian", "tag"] {
        for mode in ["success", "closed", "frozen"] {
            let name = format!("{circuit}_{mode}");
            let expected = &reference[&name];
            let native_witnesses = TrackingWitness::default();
            let recorded_witnesses = TrackingWitness::default();
            let native = initial(mode, &native_witnesses);
            let recording = initial(mode, &recorded_witnesses);
            assert_eq!(
                state_hex(native.query.state.get_ref().clone()),
                expected["initialStateHex"]
            );
            assert_eq!(
                state_hex(recording.query.state.get_ref().clone()),
                expected["initialStateHex"]
            );
            if mode == "success" {
                let (native, recorded) = if circuit == "setCustodian" {
                    (
                        setCustodian(native, &native_witnesses, holder()).unwrap(),
                        recorded::setCustodian(recording, &recorded_witnesses, holder()).unwrap(),
                    )
                } else {
                    (
                        tag(native, &native_witnesses, Field::from(7_u64)).unwrap(),
                        recorded::tag(recording, &recorded_witnesses, Field::from(7_u64)).unwrap(),
                    )
                };
                check_success(&native, &recorded, expected, &name);
                assert_eq!(native_witnesses.calls(), ["currentTimestamp"]);
                assert_eq!(recorded_witnesses.calls(), ["currentTimestamp"]);
                assert_eq!(expected["witnessCalls"], json!(["currentTimestamp"]));
            } else {
                let (native_error, recorded_error) = if circuit == "setCustodian" {
                    (
                        setCustodian(native, &native_witnesses, holder())
                            .err()
                            .expect("native guard must fail"),
                        recorded::setCustodian(recording, &recorded_witnesses, holder())
                            .err()
                            .expect("recorded guard must fail"),
                    )
                } else {
                    (
                        tag(native, &native_witnesses, Field::from(7_u64))
                            .err()
                            .expect("native guard must fail"),
                        recorded::tag(recording, &recorded_witnesses, Field::from(7_u64))
                            .err()
                            .expect("recorded guard must fail"),
                    )
                };
                let message = if mode == "closed" {
                    "registry is closed"
                } else {
                    "registry is frozen"
                };
                assert!(
                    native_error.to_string().ends_with(message),
                    "{name}: {native_error}"
                );
                assert_eq!(recorded_error, native_error, "{name}: Rust errors");
                assert!(expected["error"].as_str().unwrap().ends_with(message));
                assert_eq!(expected["stateHex"], expected["initialStateHex"]);
                assert_eq!(expected["privateState"], 10);
                assert!(native_witnesses.calls().is_empty());
                assert!(recorded_witnesses.calls().is_empty());
                assert_eq!(expected["witnessCalls"], json!([]));
                check_failure_prefix(mode, expected);
                let query_tags = expected["queries"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|query| query["opTags"].clone())
                    .collect::<Vec<_>>();
                assert_eq!(
                    query_tags,
                    vec![json!(["dup", "idx", "popeq"]); if mode == "closed" { 1 } else { 2 }]
                );
            }
        }
    }
}

fn freshness_oracle() -> Value {
    serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/asset-freshness-oracle.json"
    ))
    .unwrap()
}

fn freshness_args(
    case: &str,
) -> (
    FreshnessPolicy,
    AssetRecord,
    BoundedUint<{ u64::MAX as u128 }>,
) {
    let (enforce, max_age, registered, current) = match case {
        "fresh" => (true, 50, 100, 120),
        "unchecked_age" => (false, 0, 100, 500),
        "future" => (true, 50, 130, 120),
        "expired" => (true, 10, 100, 120),
        _ => unreachable!(),
    };
    (
        FreshnessPolicy {
            enforceMaxAge: enforce,
            maxAge: BoundedUint::new(max_age).unwrap(),
        },
        AssetRecord {
            code: FixedBytes::new([3; 32]),
            note: runtime::OpaqueString::from("valid note"),
            provenance: Provenance {
                facility: FixedBytes::new([4; 32]),
                registeredAt: BoundedUint::new(registered).unwrap(),
            },
            kind: AssetClass::Instrument,
            quantity: BoundedUint::new(5).unwrap(),
        },
        BoundedUint::new(current).unwrap(),
    )
}

#[test]
fn asset_freshness_pure_guard_records_only_successful_calls() {
    let reference = freshness_oracle();
    for case in ["fresh", "unchecked_age", "future", "expired"] {
        let expected = &reference[case];
        let native_witnesses = TrackingWitness::default();
        let recorded_witnesses = TrackingWitness::default();
        let native = initial("success", &native_witnesses);
        let recording = initial("success", &recorded_witnesses);
        assert_eq!(
            state_hex(native.query.state.get_ref().clone()),
            expected["initialStateHex"],
            "{case}: constructor state",
        );
        let (policy, record, current) = freshness_args(case);
        let native = acceptIfFresh(
            native,
            &native_witnesses,
            policy.clone(),
            record.clone(),
            current,
        );
        let recorded =
            recorded::acceptIfFresh(recording, &recorded_witnesses, policy, record, current);
        if case == "fresh" || case == "unchecked_age" {
            let native = native.unwrap();
            let recorded = recorded.unwrap();
            assert_eq!(expected["result"], json!([]));
            assert_eq!(native.result, ());
            assert_eq!(recorded.execution.result, ());
            check_success(&native, &recorded, expected, case);
            assert_eq!(native_witnesses.calls(), ["currentTimestamp"]);
            assert_eq!(recorded_witnesses.calls(), ["currentTimestamp"]);
            assert_eq!(expected["witnessCalls"], json!(["currentTimestamp"]));
        } else {
            let native_error = native.err().expect("guard must reject native call");
            let recorded_error = recorded.err().expect("guard must reject recorded call");
            assert_eq!(recorded_error, native_error, "{case}: error parity");
            assert_eq!(
                native_error.to_string(),
                expected["error"].as_str().unwrap()
            );
            assert_eq!(expected["stateHex"], expected["initialStateHex"]);
            assert_eq!(expected["privateState"], 10);
            assert_eq!(expected["queries"], json!([]));
            assert_eq!(expected["witnessCalls"], json!([]));
            assert!(native_witnesses.calls().is_empty());
            assert!(recorded_witnesses.calls().is_empty());
        }
    }
}

fn grant_write_oracle() -> Value {
    serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/asset-custody-grant-write.json"
    ))
    .unwrap()
}

fn record_write_oracle() -> Value {
    serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/asset-record-write.json"
    ))
    .unwrap()
}

fn asset_record(revised: bool, invalid_class: bool) -> AssetRecord {
    AssetRecord {
        code: FixedBytes::new([if revised { 3 } else { 0 }; 32]),
        note: runtime::OpaqueString::from(if revised {
            "更新済み café"
        } else {
            "検査資料 🔒"
        }),
        provenance: Provenance {
            facility: FixedBytes::new([if revised { 6 } else { 4 }; 32]),
            registeredAt: BoundedUint::new(if revised { 101 } else { 91 }).unwrap(),
        },
        kind: if invalid_class {
            AssetClass::Unspecified
        } else if revised {
            AssetClass::Container
        } else {
            AssetClass::Instrument
        },
        quantity: BoundedUint::new(if revised { 9 } else { 5 }).unwrap(),
    }
}

#[test]
fn typed_asset_record_insert_update_match_typescript_native_recorded_and_replay() {
    let reference = record_write_oracle();
    let native_witnesses = TrackingWitness::default();
    let recorded_witnesses = TrackingWitness::default();
    let native_context = initial("success", &native_witnesses);
    let recorded_context = initial("success", &recorded_witnesses);
    let key = runtime::OpaqueString::from("record-α-1");
    assert_eq!(
        state_hex(native_context.query.state.get_ref().clone()),
        reference["insert"]["initialStateHex"]
    );
    let native_insert = setRecord(
        native_context,
        &native_witnesses,
        key.clone(),
        asset_record(false, false),
        RecordMutation::Insert,
    )
    .unwrap();
    let recorded_insert = recorded::setRecord(
        recorded_context,
        &recorded_witnesses,
        key.clone(),
        asset_record(false, false),
        RecordMutation::Insert,
    )
    .unwrap();
    check_success(
        &native_insert,
        &recorded_insert,
        &reference["insert"],
        "record insert",
    );
    assert_eq!(native_witnesses.calls(), ["currentTimestamp"]);
    assert_eq!(recorded_witnesses.calls(), ["currentTimestamp"]);
    native_witnesses.clear();
    recorded_witnesses.clear();

    assert_eq!(
        state_hex(native_insert.context.query.state.get_ref().clone()),
        reference["update"]["initialStateHex"]
    );
    let native_update = setRecord(
        native_insert.context,
        &native_witnesses,
        key.clone(),
        asset_record(true, false),
        RecordMutation::Update,
    )
    .unwrap();
    let recorded_update = recorded::setRecord(
        recorded_insert.execution.context,
        &recorded_witnesses,
        key,
        asset_record(true, false),
        RecordMutation::Update,
    )
    .unwrap();
    check_success(
        &native_update,
        &recorded_update,
        &reference["update"],
        "record update",
    );
    assert_eq!(native_witnesses.calls(), ["currentTimestamp"]);
    assert_eq!(recorded_witnesses.calls(), ["currentTimestamp"]);
}

#[test]
fn typed_asset_record_rejects_class_missing_duplicate_invalid_closed_and_frozen() {
    let reference = record_write_oracle();
    let key = runtime::OpaqueString::from("record-α-1");
    for (case, mode, mutation, invalid_class) in [
        ("invalidClass", "success", RecordMutation::Insert, true),
        ("missingUpdate", "success", RecordMutation::Update, false),
        (
            "invalidMutation",
            "success",
            RecordMutation::Unspecified,
            false,
        ),
        ("closed", "closed", RecordMutation::Insert, false),
        ("frozen", "frozen", RecordMutation::Insert, false),
    ] {
        let native_witnesses = TrackingWitness::default();
        let recorded_witnesses = TrackingWitness::default();
        let native_context = initial(mode, &native_witnesses);
        let recorded_context = initial(mode, &recorded_witnesses);
        assert_eq!(
            state_hex(native_context.query.state.get_ref().clone()),
            reference[case]["initialStateHex"]
        );
        let native_error = setRecord(
            native_context,
            &native_witnesses,
            key.clone(),
            asset_record(false, invalid_class),
            mutation,
        )
        .err()
        .unwrap();
        let recorded_error = recorded::setRecord(
            recorded_context,
            &recorded_witnesses,
            key.clone(),
            asset_record(false, invalid_class),
            mutation,
        )
        .err()
        .unwrap();
        assert_eq!(native_error, recorded_error, "{case}: Rust guard");
        assert_eq!(
            native_error.to_string(),
            reference[case]["error"],
            "{case}: TypeScript guard"
        );
        assert!(native_witnesses.calls().is_empty());
        assert!(recorded_witnesses.calls().is_empty());
        assert_eq!(
            reference[case]["stateHex"],
            reference[case]["initialStateHex"]
        );
    }
    let native_witnesses = TrackingWitness::default();
    let recorded_witnesses = TrackingWitness::default();
    let native_seed = setRecord(
        initial("success", &native_witnesses),
        &native_witnesses,
        key.clone(),
        asset_record(false, false),
        RecordMutation::Insert,
    )
    .unwrap();
    let recorded_seed = setRecord(
        initial("success", &recorded_witnesses),
        &recorded_witnesses,
        key.clone(),
        asset_record(false, false),
        RecordMutation::Insert,
    )
    .unwrap();
    assert_eq!(
        state_hex(native_seed.context.query.state.get_ref().clone()),
        reference["duplicateInsert"]["initialStateHex"]
    );
    native_witnesses.clear();
    recorded_witnesses.clear();
    let native_error = setRecord(
        native_seed.context,
        &native_witnesses,
        key.clone(),
        asset_record(false, false),
        RecordMutation::Insert,
    )
    .err()
    .unwrap();
    let recorded_error = recorded::setRecord(
        recorded_seed.context,
        &recorded_witnesses,
        key,
        asset_record(false, false),
        RecordMutation::Insert,
    )
    .err()
    .unwrap();
    assert_eq!(native_error, recorded_error);
    assert_eq!(
        native_error.to_string(),
        reference["duplicateInsert"]["error"]
    );
    assert!(native_witnesses.calls().is_empty());
    assert!(recorded_witnesses.calls().is_empty());
}

fn custody_grant(revised: bool) -> CustodyGrant {
    CustodyGrant {
        code: FixedBytes::new([if revised { 3 } else { 0 }; 32]),
        holder: Holder {
            bytes: FixedBytes::new([if revised { 8 } else { 7 }; 32]),
        },
        grantedAt: BoundedUint::new(if revised { 110 } else { 100 }).unwrap(),
    }
}

#[test]
fn typed_custody_grant_insert_update_match_typescript_native_recorded_and_replay() {
    let reference = grant_write_oracle();
    let native_witnesses = TrackingWitness::default();
    let recorded_witnesses = TrackingWitness::default();
    let native_context = initial("success", &native_witnesses);
    let recorded_context = initial("success", &recorded_witnesses);
    let key = runtime::OpaqueString::from("grant-note-1");
    assert_eq!(
        state_hex(native_context.query.state.get_ref().clone()),
        reference["insert"]["initialStateHex"]
    );
    let native_insert = setCustodyGrant(
        native_context,
        &native_witnesses,
        key.clone(),
        custody_grant(false),
        RecordMutation::Insert,
    )
    .unwrap();
    let recorded_insert = recorded::setCustodyGrant(
        recorded_context,
        &recorded_witnesses,
        key.clone(),
        custody_grant(false),
        RecordMutation::Insert,
    )
    .unwrap();
    check_success(
        &native_insert,
        &recorded_insert,
        &reference["insert"],
        "insert",
    );
    assert_eq!(native_witnesses.calls(), ["currentTimestamp"]);
    assert_eq!(recorded_witnesses.calls(), ["currentTimestamp"]);
    assert_eq!(
        reference["insert"]["witnessCalls"],
        json!(["currentTimestamp"])
    );
    native_witnesses.clear();
    recorded_witnesses.clear();

    assert_eq!(
        state_hex(native_insert.context.query.state.get_ref().clone()),
        reference["update"]["initialStateHex"]
    );
    let native_update = setCustodyGrant(
        native_insert.context,
        &native_witnesses,
        key.clone(),
        custody_grant(true),
        RecordMutation::Update,
    )
    .unwrap();
    let recorded_update = recorded::setCustodyGrant(
        recorded_insert.execution.context,
        &recorded_witnesses,
        key,
        custody_grant(true),
        RecordMutation::Update,
    )
    .unwrap();
    check_success(
        &native_update,
        &recorded_update,
        &reference["update"],
        "update",
    );
    assert_eq!(native_witnesses.calls(), ["currentTimestamp"]);
    assert_eq!(recorded_witnesses.calls(), ["currentTimestamp"]);
    assert_eq!(
        reference["update"]["witnessCalls"],
        json!(["currentTimestamp"])
    );
}

#[test]
fn typed_custody_grant_write_rejects_missing_duplicate_invalid_closed_and_frozen() {
    let reference = grant_write_oracle();
    let key = runtime::OpaqueString::from("grant-note-1");
    for (case, mode, mutation) in [
        ("missingUpdate", "success", RecordMutation::Update),
        ("invalidMutation", "success", RecordMutation::Unspecified),
        ("closed", "closed", RecordMutation::Insert),
        ("frozen", "frozen", RecordMutation::Insert),
    ] {
        let native_witnesses = TrackingWitness::default();
        let recorded_witnesses = TrackingWitness::default();
        let native_context = initial(mode, &native_witnesses);
        let recorded_context = initial(mode, &recorded_witnesses);
        assert_eq!(
            state_hex(native_context.query.state.get_ref().clone()),
            reference[case]["initialStateHex"],
            "{case}: seeded state"
        );
        let native_error = setCustodyGrant(
            native_context,
            &native_witnesses,
            key.clone(),
            custody_grant(false),
            mutation,
        )
        .err()
        .unwrap();
        let recorded_error = recorded::setCustodyGrant(
            recorded_context,
            &recorded_witnesses,
            key.clone(),
            custody_grant(false),
            mutation,
        )
        .err()
        .unwrap();
        assert_eq!(native_error, recorded_error, "{case}: Rust guard");
        assert_eq!(
            native_error.to_string(),
            reference[case]["error"],
            "{case}: TypeScript guard"
        );
        assert!(native_witnesses.calls().is_empty());
        assert!(recorded_witnesses.calls().is_empty());
        assert_eq!(reference[case]["witnessCalls"], json!([]));
        assert_eq!(
            reference[case]["stateHex"],
            reference[case]["initialStateHex"]
        );
    }
    let native_witnesses = TrackingWitness::default();
    let recorded_witnesses = TrackingWitness::default();
    let native_seed = setCustodyGrant(
        initial("success", &native_witnesses),
        &native_witnesses,
        key.clone(),
        custody_grant(false),
        RecordMutation::Insert,
    )
    .unwrap();
    let recorded_seed = setCustodyGrant(
        initial("success", &recorded_witnesses),
        &recorded_witnesses,
        key.clone(),
        custody_grant(false),
        RecordMutation::Insert,
    )
    .unwrap();
    assert_eq!(
        state_hex(native_seed.context.query.state.get_ref().clone()),
        reference["duplicateInsert"]["initialStateHex"]
    );
    native_witnesses.clear();
    recorded_witnesses.clear();
    let native_error = setCustodyGrant(
        native_seed.context,
        &native_witnesses,
        key.clone(),
        custody_grant(false),
        RecordMutation::Insert,
    )
    .err()
    .unwrap();
    let recorded_error = recorded::setCustodyGrant(
        recorded_seed.context,
        &recorded_witnesses,
        key,
        custody_grant(false),
        RecordMutation::Insert,
    )
    .err()
    .unwrap();
    assert_eq!(native_error, recorded_error);
    assert_eq!(
        native_error.to_string(),
        reference["duplicateInsert"]["error"]
    );
    assert!(native_witnesses.calls().is_empty());
    assert!(recorded_witnesses.calls().is_empty());
}
