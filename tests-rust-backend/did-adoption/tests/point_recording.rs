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

//! Original constructor-driven Point transitions; VM replay is not proof acceptance.
use compact_rust_did_adoption_fixture::{
    ledger_contract as c, ledger_slots as slots, runtime as r,
};
use midnight_compact_testkit::{ArtifactIdentity, ContractLab, Environment, LabError};
use r::context::{ConstructorContext, ConstructorResult};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
#[path = "../support/codec.rs"]
mod codec;
#[path = "../support/lifecycle_calls.rs"]
mod lifecycle_calls;
#[path = "../support/lifecycle_witness.rs"]
mod lifecycle_witness;
#[path = "../support/point_calls.rs"]
mod point_calls;
use lifecycle_witness::Witness;
fn state(row: &Value, key: &str) -> r::ledger::ContractState<r::ledger::DefaultDB> {
    midnight_serialize::tagged_deserialize(
        &mut hex::decode(row[key].as_str().unwrap()).unwrap().as_slice(),
    )
    .unwrap()
}
fn run(capture: &str, scenario_id: &str, expected_count: usize) {
    let capture: Value = serde_json::from_str(capture).unwrap();
    let scenario = capture["scenarios"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["id"] == scenario_id)
        .unwrap();
    let constructor =
        c::initial_state(ConstructorContext::new(0u64), &Witness::new(json!({}))).unwrap();
    let initial: r::ledger::ContractState<r::ledger::DefaultDB> =
        midnight_serialize::tagged_deserialize(
            &mut hex::decode(scenario["initial"]["state"].as_str().unwrap())
                .unwrap()
                .as_slice(),
        )
        .unwrap();
    assert_eq!(constructor.ledger_state, initial.data);
    let identity = ArtifactIdentity {
        source_sha256: Sha256::digest(include_bytes!(
            "../../../examples/rust_backend/did_adoption/packages/contract/src/did.compact"
        ))
        .into(),
        generated_sha256: Sha256::digest(include_bytes!("../lib.rs")).into(),
    };
    let mut lab = ContractLab::from_constructor(
        identity,
        Environment::new(Default::default(), Default::default(), [0x59; 32]),
        constructor,
    )
    .unwrap();
    let mut count = 0;
    for row in scenario["steps"].as_array().unwrap().iter().filter(|r| {
        matches!(
            r["name"].as_str(),
            Some("rotateControllerKey" | "recoverControllerKey" | "deactivate")
        )
    }) {
        count += 1;
        let before = lab.snapshot();
        assert_eq!(
            before.public_state(),
            &state(row, "before").data,
            "{}",
            row["id"]
        );
        assert_eq!(
            *before.private_state(),
            row["privateBefore"].as_u64().unwrap()
        );
        let nw = Witness::new(row["options"].clone());
        let rw = Witness::new(row["options"].clone());
        let native = lifecycle_calls::invoke(
            ConstructorResult::new(
                ConstructorContext::new(*before.private_state()),
                before.public_state().clone(),
            )
            .into_circuit_context(Default::default()),
            &nw,
            row,
        );
        let recorded = lab.recorded(|ctx| point_calls::invoke_recorded(ctx, &rw, row));
        assert_eq!(json!(*nw.calls.borrow()), row["witnessCalls"]);
        assert_eq!(json!(*rw.calls.borrow()), row["witnessCalls"]);
        if let Some(message) = row["error"].as_str() {
            let expected = r::CompactError::AssertionFailed(
                message
                    .strip_prefix("failed assert: ")
                    .unwrap_or(message)
                    .into(),
            );
            assert_eq!(native.err().unwrap(), expected);
            assert!(matches!(recorded,Err(LabError::Execution(ref error)) if *error==expected));
            assert_eq!(lab.snapshot().public_state(), before.public_state());
            assert_eq!(lab.private_state(), before.private_state());
            continue;
        }
        let native = native.unwrap();
        let recorded = recorded.unwrap();
        assert_eq!(recorded.public_state(), &state(row, "after").data);
        assert_eq!(recorded.public_state(), &native.context.query.state);
        assert_eq!(recorded.effects(), &native.context.query.effects);
        assert_eq!(lab.private_state(), &native.context.private_state);
        assert_eq!(*lab.private_state(), row["privateAfter"].as_u64().unwrap());
        assert_eq!(
            recorded.private_outputs(),
            &native.private_transcript_outputs
        );
        let private:Vec<_>=recorded.private_outputs().iter().map(|v|json!({"valueAtoms":v.value.0.iter().map(|a|&a.0).collect::<Vec<_>>(),"alignment":v.alignment})).collect();
        assert_eq!(json!(private), row["privateTranscript"]);
        assert_eq!(
            json!(recorded.replay().unwrap().program()),
            row["publicTranscript"]
        );
        assert_eq!(recorded.execution_gas(), native.gas_cost);
        let gas = json!(recorded.execution_gas());
        for dim in ["readTime", "computeTime", "bytesWritten", "bytesDeleted"] {
            let total: u64 = row["queries"]
                .as_array()
                .unwrap()
                .iter()
                .map(|q| q[dim].as_str().unwrap().parse::<u64>().unwrap())
                .sum();
            assert_eq!(gas[dim], total, "{} {dim}", row["id"]);
        }
        assert_eq!(
            slots::id
                .inspect(recorded.public_state().get_ref())
                .unwrap()
                .bytes,
            r::FixedBytes::new([0; 32])
        );
    }
    assert_eq!(count, expected_count);
    assert!(
        !slots::active
            .inspect(lab.snapshot().public_state().get_ref())
            .unwrap()
    );
}

#[test]
fn constructor_rotate_recover_deactivate_matches_native_ts_and_recorded_replay() {
    run(
        include_str!("../oracle/lifecycle.json"),
        "authorization-lifecycle",
        9,
    );
}
#[test]
fn point_specific_authority_version_witness_and_inactive_failures_preserve_rollback() {
    run(
        include_str!("../oracle/point-lifecycle.json"),
        "point-failures",
        12,
    );
}
