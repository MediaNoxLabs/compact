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

//! Real DID constructor and generated recorded call through ContractLab.
//! This is local VM replay evidence, not proof or network acceptance.

use compact_rust_did_adoption_fixture::{ledger_contract as did, runtime as r, types};
use midnight_compact_testkit::{ArtifactIdentity, ContractLab, Environment, LabError};
use r::{BoundedUint, CompactError, Field, context::ConstructorContext};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::cell::RefCell;

#[path = "../support/witness.rs"]
mod call_witness;
#[allow(dead_code)]
#[path = "../support/codec.rs"]
mod codec;
#[path = "../support/lifecycle_witness.rs"]
mod constructor_witness;

fn capture(id: &str) -> Value {
    let document: Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/unit-composition.json"
    ))
    .unwrap();
    document["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|case| case["id"] == id)
        .unwrap()
        .clone()
}

fn template(row: &Value) -> r::ledger::ContractState<r::ledger::DefaultDB> {
    midnight_serialize::tagged_deserialize(
        &mut hex::decode(row["before"].as_str().unwrap())
            .unwrap()
            .as_slice(),
    )
    .unwrap()
}

fn state_hex(
    state: &r::ledger::ChargedState<r::ledger::DefaultDB>,
    template: &r::ledger::ContractState<r::ledger::DefaultDB>,
) -> String {
    let mut full = template.clone();
    full.data = state.clone();
    let mut bytes = vec![];
    midnight_serialize::tagged_serialize(&full, &mut bytes).unwrap();
    hex::encode(bytes)
}

fn lab(row: &Value) -> ContractLab<u64> {
    let witness = constructor_witness::Witness::new(json!({"timestamp":"100"}));
    // The retained unit-composition oracle starts with private 7. The original
    // constructor calls controller, recovery and timestamp witnesses once each.
    let initial = did::initial_state(ConstructorContext::new(4_u64), &witness).unwrap();
    assert_eq!(
        initial.private_state,
        row["privateBefore"].as_u64().unwrap()
    );
    assert_eq!(
        json!(*witness.calls.borrow()),
        json!([{"name":"controller"},{"name":"recovery"},{"name":"timestamp"}])
    );
    assert_eq!(initial.ledger_state.get_ref(), template(row).data.get_ref());
    let identity = ArtifactIdentity {
        source_sha256: Sha256::digest(include_bytes!(
            "../../../examples/rust_backend/did_adoption/packages/contract/src/did.compact"
        ))
        .into(),
        generated_sha256: Sha256::digest(include_bytes!("../lib.rs")).into(),
    };
    let environment = Environment::new(Default::default(), Default::default(), [0x55; 32]);
    ContractLab::from_constructor(identity, environment, initial).unwrap()
}

fn signature(row: &Value) -> types::SchnorrSignature {
    types::SchnorrSignature {
        announcement: r::ec_mul_generator(Field::from(2_u64)).unwrap(),
        response: Field::from_le_bytes(&hex::decode(row["responseHex"].as_str().unwrap()).unwrap())
            .unwrap(),
    }
}

fn version(row: &Value) -> BoundedUint<{ u64::MAX as u128 }> {
    BoundedUint::new(row["expected"].as_str().unwrap().parse().unwrap()).unwrap()
}

fn private_outputs(outputs: &[r::fab::AlignedValue]) -> Value {
    json!(
        outputs
            .iter()
            .map(|value| {
                let atoms: Vec<_> = value.value.0.iter().map(|atom| &atom.0).collect();
                json!({"valueAtoms":atoms,"alignment":value.alignment})
            })
            .collect::<Vec<_>>()
    )
}

#[test]
fn did_deactivate_recorded_lab_matches_original_typescript_and_native() {
    let row = capture("did/valid");
    let original = template(&row);
    let mut lab = lab(&row);
    let before = lab.snapshot();
    assert_eq!(state_hex(before.public_state(), &original), row["before"]);
    let mut native = lab.fork();
    let recorded_witness = call_witness::Witness {
        options: row["options"].clone(),
        calls: RefCell::default(),
    };
    let native_witness = call_witness::Witness {
        options: row["options"].clone(),
        calls: RefCell::default(),
    };
    let native_result = native
        .native(|ctx| did::deactivate(ctx, &native_witness, signature(&row), version(&row)))
        .unwrap();
    let report = lab
        .recorded(|ctx| {
            did::recorded::deactivate(ctx, &recorded_witness, signature(&row), version(&row))
        })
        .unwrap();
    assert_eq!(*report.output(), ());
    assert_eq!(report.before(), before.public_state());
    assert_eq!(report.public_state(), native_result.public_state());
    assert_eq!(report.effects(), native_result.effects());
    assert_eq!(report.execution_gas(), native_result.execution_gas());
    assert_eq!(state_hex(report.public_state(), &original), row["after"]);
    assert_eq!(*lab.private_state(), row["privateAfter"].as_u64().unwrap());
    assert_eq!(json!(*recorded_witness.calls.borrow()), row["witnessCalls"]);
    assert_eq!(json!(*native_witness.calls.borrow()), row["witnessCalls"]);
    assert_eq!(
        private_outputs(report.private_outputs()),
        row["privateTranscript"]
    );
    assert_eq!(report.private_outputs(), native_result.private_outputs());

    let replay = report
        .replay()
        .expect("generated DID call has sealed replay");
    assert_eq!(json!(replay.program()), row["publicTranscript"]);
    let gas = json!(report.execution_gas());
    for dimension in ["readTime", "computeTime", "bytesWritten", "bytesDeleted"] {
        let captured: u64 = row["queries"]
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
        assert_eq!(gas[dimension], captured, "query-summed {dimension}");
        assert_eq!(
            row["reportedGas"][dimension],
            row["queries"].as_array().unwrap().last().unwrap()["gasCost"][dimension]
        );
    }
    // Replay is a single sealed VM query. Its cost has its own scope; compare
    // with an independent upstream query from the exact constructor checkpoint.
    let direct =
        r::ledger::QueryContext::new(before.public_state().clone(), lab.environment().address)
            .query(replay.program(), None, &lab.environment().cost_model)
            .unwrap();
    assert_eq!(replay.gas(), direct.gas_cost);
    assert_eq!(&direct.context.state, report.public_state());
    assert_eq!(&direct.context.effects, report.effects());
}

#[test]
fn did_late_witness_failure_keeps_owned_lab_checkpoint_and_external_journal() {
    let row = capture("did/timestamp-failure");
    let mut lab = lab(&row);
    let before = lab.snapshot();
    let witness = call_witness::Witness {
        options: row["options"].clone(),
        calls: RefCell::default(),
    };
    let error = lab
        .recorded(|ctx| did::recorded::deactivate(ctx, &witness, signature(&row), version(&row)))
        .expect_err("timestamp witness rejects after earlier public reads");
    assert!(matches!(
        error,
        LabError::Execution(CompactError::AssertionFailed(ref message))
            if message == row["error"].as_str().unwrap()
    ));
    assert_eq!(lab.snapshot().public_state(), before.public_state());
    assert_eq!(lab.private_state(), before.private_state());
    // The witness is outside the owned lab checkpoint. Its calls are observable
    // even though the failed generated call exposes no Rust partial trace.
    assert_eq!(json!(*witness.calls.borrow()), row["witnessCalls"]);
    assert!(row["publicTranscript"].is_null());
}
