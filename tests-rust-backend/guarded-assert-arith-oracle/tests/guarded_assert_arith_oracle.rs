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

use compact_rust_guarded_assert_arith_oracle_fixture::ledger_contract::{
    initial_state, recordFreshEnough, recorded,
};
use compact_rust_guarded_assert_arith_oracle_fixture::pure_circuits::{
    ageGap, assertAgeWithin, assertFreshEnough,
};
use compact_rust_guarded_assert_arith_oracle_fixture::types::{
    Attestation, StatusProof, VerifierPolicy,
};
use midnight_compact_runtime as runtime;
use midnight_onchain_state::state::{
    ContractMaintenanceAuthority, ContractOperation, ContractState, EntryPointBuf,
};
use midnight_serialize::tagged_serialize;
use midnight_storage::storage::HashMap;
use runtime::context::ConstructorContext;
use runtime::ledger::{ContractAddress, DefaultDB, StateValue};
use serde_json::{Value, json};

fn uint(value: u128) -> runtime::BoundedUint<{ u64::MAX as u128 }> {
    runtime::BoundedUint::new(value).unwrap()
}

fn attestation(created_at: u128) -> Attestation {
    Attestation {
        proof: StatusProof {
            createdAt: uint(created_at),
            issuer: uint(1),
        },
        hasExpiration: false,
        expiresAt: uint(0),
    }
}

fn policy(enforce_max_age: bool) -> VerifierPolicy {
    VerifierPolicy {
        enforceMaxAge: enforce_max_age,
        maxAge: uint(20),
    }
}

fn state_hex(state: StateValue<DefaultDB>) -> String {
    let operations: HashMap<EntryPointBuf, ContractOperation, DefaultDB> = HashMap::new().insert(
        EntryPointBuf(b"recordFreshEnough".to_vec()),
        ContractOperation::new(None),
    );
    let contract_state =
        ContractState::new(state, operations, ContractMaintenanceAuthority::default());
    let mut bytes = Vec::new();
    tagged_serialize(&contract_state, &mut bytes).unwrap();
    hex::encode(bytes)
}

fn assertion_error<T>(result: Result<T, runtime::CompactError>, oracle: &serde_json::Value) {
    let error = result.err().expect("assertion should fail");
    assert!(matches!(error, runtime::CompactError::AssertionFailed(_)));
    assert_eq!(error.to_string(), oracle["error"]);
    assert_eq!(oracle["compactError"], true);
}

#[test]
fn exact_guarded_nested_arithmetic_matches_typescript() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/guarded-assert-arith-oracle.json"
    ))
    .unwrap();
    let att = attestation(100);
    assert!(assertFreshEnough(policy(true), att.clone(), uint(110)).is_ok());
    assert_eq!(oracle["fresh"]["ok"], true);
    assertion_error(
        assertFreshEnough(policy(true), att.clone(), uint(130)),
        &oracle["tooOld"],
    );
    assertion_error(
        assertFreshEnough(policy(true), att.clone(), uint(90)),
        &oracle["future"],
    );
    assert!(assertFreshEnough(policy(false), att.clone(), uint(130)).is_ok());
    assert_eq!(oracle["withoutMaxAge"]["ok"], true);
    assert!(assertAgeWithin(att.clone(), uint(110), uint(10)).is_ok());
    assert_eq!(oracle["withinAge"]["ok"], true);
    assertion_error(
        assertAgeWithin(att.clone(), uint(111), uint(10)),
        &oracle["outsideAge"],
    );
    assert_eq!(
        ageGap(attestation(120), att.clone())
            .unwrap()
            .value()
            .to_string(),
        oracle["ageGap"]
    );
    assertion_error(ageGap(att.clone(), attestation(120)), &oracle["reverseGap"]);

    let initial = initial_state(ConstructorContext::new(())).unwrap();
    assert_eq!(
        state_hex(initial.ledger_state.get_ref().clone()),
        oracle["afterInit"]
    );
    let accepted = recordFreshEnough(
        initial.into_circuit_context(ContractAddress::default()),
        policy(true),
        att.clone(),
        uint(110),
    )
    .unwrap();
    assert_eq!(
        state_hex(accepted.context.query.state.get_ref().clone()),
        oracle["afterRecordFresh"]
    );
    let StateValue::Array(fields) = accepted.context.query.state.get_ref() else {
        panic!("expected ledger field array")
    };
    let count = runtime::ledger::read_counter(fields.get(0).unwrap()).unwrap();
    assert_eq!(count.to_string(), oracle["countAfterRecordFresh"]);
    assertion_error(
        recordFreshEnough(accepted.context, policy(true), att, uint(130)),
        &oracle["recordTooOld"],
    );
    assert_eq!(oracle["afterRejectedRecord"], oracle["afterRecordFresh"]);
}

fn vm_shape(operations: Value) -> Value {
    Value::Array(
        operations.as_array().unwrap().iter().map(|operation| {
            if let Some(kind) = operation.as_str() {
                return json!({ "kind": kind });
            }
            if let Some(index) = operation.get("idx") {
                return json!({ "kind": "idx", "cached": index["cached"],
                    "pushPath": index["pushPath"], "pathLength": index["path"].as_array().unwrap().len() });
            }
            if let Some(push) = operation.get("push") {
                return json!({ "kind": "push", "storage": push["storage"] });
            }
            if let Some(insert) = operation.get("ins") {
                return json!({ "kind": "ins", "cached": insert["cached"], "n": insert["n"] });
            }
            if let Some(duplicate) = operation.get("dup") {
                return json!({ "kind": "dup", "n": duplicate["n"] });
            }
            if let Some(add) = operation.get("addi") {
                return json!({ "kind": "addi", "immediate": add["immediate"] });
            }
            if let Some(pop) = operation.get("popeq") {
                return json!({ "kind": "popeq", "cached": pop["cached"] });
            }
            panic!("unexpected VM operation: {operation}");
        }).collect(),
    )
}

#[test]
fn guarded_counter_recording_matches_typescript_success_and_failure() {
    let oracle: Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/guarded-recording-oracle.json"
    ))
    .unwrap();
    for case in ["fresh", "unchecked_age", "future", "expired"] {
        let expected = &oracle[case];
        let initial = initial_state(ConstructorContext::new(())).unwrap();
        let initial_hex = state_hex(initial.ledger_state.get_ref().clone());
        assert_eq!(
            initial_hex, expected["initialStateHex"],
            "{case}: constructor"
        );
        let native_context = initial.into_circuit_context(ContractAddress::default());
        let initial = initial_state(ConstructorContext::new(())).unwrap();
        let recorded_context = initial.into_circuit_context(ContractAddress::default());
        let (enforce, max_age, now) = match case {
            "fresh" => (true, 20, 110),
            "unchecked_age" => (false, 0, 130),
            "future" => (true, 20, 90),
            "expired" => (true, 20, 130),
            _ => unreachable!(),
        };
        let selected_policy = VerifierPolicy {
            enforceMaxAge: enforce,
            maxAge: uint(max_age),
        };
        let selected_attestation = attestation(100);
        let native = recordFreshEnough(
            native_context,
            selected_policy.clone(),
            selected_attestation.clone(),
            uint(now),
        );
        let recorded = recorded::recordFreshEnough(
            recorded_context,
            selected_policy,
            selected_attestation,
            uint(now),
        );
        if case == "future" || case == "expired" {
            let native_error = native.err().expect("native assertion must fail");
            let recorded_error = recorded.err().expect("recorded assertion must fail");
            assert_eq!(native_error, recorded_error, "{case}: Rust errors");
            assert_eq!(native_error.to_string(), expected["error"]);
            assert_eq!(expected["stateHex"], expected["initialStateHex"]);
            assert_eq!(expected["queries"], json!([]));
            continue;
        }
        let native = native.unwrap();
        let recorded = recorded.unwrap();
        assert_eq!(expected["result"], json!([]));
        assert_eq!(native.result, ());
        assert_eq!(recorded.execution.result, ());
        assert_eq!(
            state_hex(native.context.query.state.get_ref().clone()),
            expected["stateHex"]
        );
        assert_eq!(
            state_hex(recorded.execution.context.query.state.get_ref().clone()),
            expected["stateHex"]
        );
        assert_eq!(native.gas_cost, recorded.execution.gas_cost);
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
                "{case}: {dimension}"
            );
        }
        assert!(native.private_transcript_outputs.is_empty());
        assert!(recorded.execution.private_transcript_outputs.is_empty());
        assert_eq!(expected["privateTranscriptOutputs"], json!([]));
        assert_eq!(
            native.context.query.effects,
            recorded.execution.context.query.effects
        );
        assert_eq!(
            vm_shape(serde_json::to_value(recorded.public.verify_ops()).unwrap()),
            expected["publicTranscriptShape"],
            "{case}: ordered VM"
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
            native.context.query.state.get_ref()
        );
        assert_eq!(replay.context.effects, native.context.query.effects);
    }
}
