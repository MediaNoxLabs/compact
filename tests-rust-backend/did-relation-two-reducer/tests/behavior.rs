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

use compact_rust_did_relation_two_reducer_fixture::{ledger_contract as c, runtime as r, types};
use midnight_compact_testkit::{ArtifactIdentity, ContractLab, Environment, LabError};
use r::context::{ConstructorContext, ConstructorResult};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

fn state(text: &Value) -> r::ledger::ContractState<r::ledger::DefaultDB> {
    midnight_serialize::tagged_deserialize(
        &mut hex::decode(text.as_str().unwrap()).unwrap().as_slice(),
    )
    .unwrap()
}
fn relation(n: u64) -> types::Relation {
    match n {
        0 => types::Relation::Undefined,
        1 => types::Relation::Authentication,
        2 => types::Relation::KeyAgreement,
        _ => panic!("unknown relation"),
    }
}
fn call(
    ctx: r::context::CircuitContext<u64>,
    row: &Value,
) -> Result<r::context::CircuitResult<u64, ()>, r::CompactError> {
    c::update(
        ctx,
        relation(row["relation"].as_u64().unwrap()),
        r::OpaqueString(row["key"].as_str().unwrap().into()),
        row["add"].as_bool().unwrap(),
    )
}
#[test]
fn independent_ts_native_recorded_program_gas_and_rollback() {
    let capture: Value = serde_json::from_str(include_str!("../oracle/cases.json")).unwrap();
    assert_eq!(capture["format"], "compact-did-relation-reducer-capture/v1");
    assert_eq!(capture["kind"], "two");
    for row in capture["cases"].as_array().unwrap() {
        let id = row["id"].as_str().unwrap();
        let initial = c::initial_state(ConstructorContext::new(0u64)).unwrap();
        let mut context = initial.into_circuit_context(Default::default());
        if let Some(setup) = row["setup"].as_array() {
            for step in setup {
                context = call(context, step).unwrap().context;
            }
        }
        let before = state(&row["before"]);
        assert_eq!(context.query.state, before.data, "derived prestate {id}");
        let private = row["privateBefore"].as_u64().unwrap();
        assert_eq!(context.private_state, private);
        let identity = ArtifactIdentity {
            source_sha256: Sha256::digest(include_bytes!(
                "../../../examples/rust_backend/did_relation_two_reducer.compact"
            ))
            .into(),
            generated_sha256: Sha256::digest(include_bytes!("../lib.rs")).into(),
        };
        let mut lab = ContractLab::from_constructor(
            identity,
            Environment::new(Default::default(), Default::default(), [0; 32]),
            ConstructorResult::new(ConstructorContext::new(private), before.data.clone()),
        )
        .unwrap();
        let native = call(
            ConstructorResult::new(ConstructorContext::new(private), before.data.clone())
                .into_circuit_context(Default::default()),
            row,
        );
        let recorded = lab.recorded(|ctx| {
            c::recorded::update(
                ctx,
                relation(row["relation"].as_u64().unwrap()),
                r::OpaqueString(row["key"].as_str().unwrap().into()),
                row["add"].as_bool().unwrap(),
            )
        });
        if let Some(message) = row["error"].as_str() {
            let expected = r::CompactError::AssertionFailed(
                message
                    .strip_prefix("failed assert: ")
                    .unwrap_or(message)
                    .into(),
            );
            assert_eq!(native.err().unwrap(), expected, "native {id}");
            assert!(
                matches!(recorded,Err(LabError::Execution(ref e)) if *e==expected),
                "recorded {id}"
            );
            assert_eq!(lab.snapshot().public_state(), &before.data, "rollback {id}");
            assert_eq!(*lab.private_state(), private, "private rollback {id}");
            continue;
        }
        let native = native.unwrap();
        let recorded = recorded.unwrap();
        let after = state(&row["after"]);
        assert_eq!(recorded.public_state(), &after.data, "TS state {id}");
        assert_eq!(
            recorded.public_state(),
            &native.context.query.state,
            "native state {id}"
        );
        assert_eq!(
            recorded.effects(),
            &native.context.query.effects,
            "effects {id}"
        );
        assert_eq!(
            recorded.private_outputs(),
            &native.private_transcript_outputs,
            "private outputs {id}"
        );
        assert_eq!(row["privateTranscript"], json!([]), "TS private {id}");
        assert_eq!(
            json!(recorded.replay().unwrap().program()),
            row["publicTranscript"],
            "ordered VM {id}"
        );
        assert_eq!(recorded.execution_gas(), native.gas_cost, "native gas {id}");
        let gas = json!(recorded.execution_gas());
        for dim in ["readTime", "computeTime", "bytesWritten", "bytesDeleted"] {
            let sum: u64 = row["queries"]
                .as_array()
                .unwrap()
                .iter()
                .map(|q| q[dim].as_str().unwrap().parse::<u64>().unwrap())
                .sum();
            assert_eq!(gas[dim], sum, "gas {id} {dim}");
        }
    }
}
