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

use compact_rust_did_relation_nested_reducer_fixture::{ledger_contract as c, runtime as r, types};
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
fn method(row: &Value) -> types::Method {
    types::Method {
        id: r::OpaqueString(row["key"].as_str().unwrap().into()),
        publicKeyJwk: types::Jwk {
            curve: match row["curve"].as_u64().unwrap() {
                0 => types::Curve::Ed25519,
                1 => types::Curve::X25519,
                _ => panic!("unreviewed curve"),
            },
            x: r::OpaqueString("x".into()),
        },
    }
}
fn constructor_method(row: &Value) -> types::Method {
    if let Some(seed) = row["setup"].as_array().and_then(|steps| steps.first()) {
        return method(seed);
    }
    types::Method {
        id: r::OpaqueString("constructor-control".into()),
        publicKeyJwk: types::Jwk {
            curve: types::Curve::Ed25519,
            x: r::OpaqueString("x".into()),
        },
    }
}
fn native(
    ctx: r::context::CircuitContext<u64>,
    row: &Value,
) -> Result<r::context::CircuitResult<u64, ()>, r::CompactError> {
    match row["operation"].as_str().unwrap() {
        "seed" => c::seed(ctx, method(row)),
        "check" => c::check(
            ctx,
            r::OpaqueString(row["key"].as_str().unwrap().into()),
            row["agreement"].as_bool().unwrap(),
        ),
        other => panic!("unreviewed {other}"),
    }
}
#[test]
fn independent_ts_native_recorded_lazy_lookup_program_gas_and_rollback() {
    let capture: Value = serde_json::from_str(include_str!("../oracle/cases.json")).unwrap();
    assert_eq!(capture["format"], "compact-did-relation-reducer-capture/v1");
    assert_eq!(capture["kind"], "nested");
    for row in capture["cases"].as_array().unwrap() {
        let id = row["id"].as_str().unwrap();
        let initial =
            c::initial_state(ConstructorContext::new(0u64), constructor_method(row)).unwrap();
        let ctx = initial.into_circuit_context(Default::default());
        let before = state(&row["before"]);
        assert_eq!(ctx.query.state, before.data, "derived prestate {id}");
        let private = row["privateBefore"].as_u64().unwrap();
        let identity = ArtifactIdentity {
            source_sha256: Sha256::digest(include_bytes!(
                "../../../examples/rust_backend/did_relation_nested_reducer.compact"
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
        let native = native(
            ConstructorResult::new(ConstructorContext::new(private), before.data.clone())
                .into_circuit_context(Default::default()),
            row,
        );
        let key = r::OpaqueString(row["key"].as_str().unwrap().into());
        let agreement = row["agreement"].as_bool().unwrap();
        let recorded = lab.recorded(|ctx| c::recorded::check(ctx, key, agreement));
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
