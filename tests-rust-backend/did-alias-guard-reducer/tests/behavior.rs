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

use compact_rust_did_alias_guard_reducer_fixture::{ledger_contract as c, runtime as r};
use midnight_compact_testkit::{ArtifactIdentity, ContractLab, Environment, LabError};
use r::context::{ConstructorContext, ConstructorResult};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
struct Witness;
fn native_call(
    ctx: r::context::CircuitContext<u64>,
    row: &Value,
    _witness: &Witness,
) -> Result<r::context::CircuitResult<u64, ()>, r::CompactError> {
    let mutation = match row["mutation"].as_u64().unwrap() {
        0 => compact_rust_did_alias_guard_reducer_fixture::types::Mutation::Undefined,
        1 => compact_rust_did_alias_guard_reducer_fixture::types::Mutation::Insert,
        2 => compact_rust_did_alias_guard_reducer_fixture::types::Mutation::Remove,
        _ => panic!("unknown mutation"),
    };
    match row["operation"].as_str().unwrap() {
        "run" => c::run(ctx, mutation),
        "local_control" => c::local_control(ctx, mutation),
        "transitive_control" => c::transitive_control(ctx, mutation),
        _ => panic!("unreviewed operation"),
    }
}

fn recorded_call(
    ctx: r::context::CircuitContext<u64>,
    row: &Value,
    _witness: &Witness,
) -> Result<r::recording::RecordedCircuitResult<u64, ()>, r::CompactError> {
    let mutation = match row["mutation"].as_u64().unwrap() {
        0 => compact_rust_did_alias_guard_reducer_fixture::types::Mutation::Undefined,
        1 => compact_rust_did_alias_guard_reducer_fixture::types::Mutation::Insert,
        2 => compact_rust_did_alias_guard_reducer_fixture::types::Mutation::Remove,
        _ => panic!("unknown mutation"),
    };
    match row["operation"].as_str().unwrap() {
        "run" => c::recorded::run(ctx, mutation),
        "local_control" => c::recorded::local_control(ctx, mutation),
        "transitive_control" => c::recorded::transitive_control(ctx, mutation),
        _ => panic!("unreviewed operation"),
    }
}

fn state(row: &Value, key: &str) -> r::ledger::ContractState<r::ledger::DefaultDB> {
    midnight_serialize::tagged_deserialize(
        &mut hex::decode(row[key].as_str().unwrap()).unwrap().as_slice(),
    )
    .unwrap()
}
#[test]
fn maintained_source_semantics_match_independent_ts_and_recorded_replay() {
    let capture: Value = serde_json::from_str(include_str!("../oracle/cases.json")).unwrap();
    assert_eq!(
        capture["format"],
        "compact-did-primitive-reducer-capture/v1"
    );
    assert_eq!(capture["kind"], "alias-guard");
    let rows = capture["cases"].as_array().unwrap();
    let expected = BTreeSet::from([
        "run-insert",
        "run-remove",
        "run-undefined",
        "local_control-insert",
        "local_control-remove",
        "local_control-undefined",
        "transitive_control-insert",
        "transitive_control-remove",
        "transitive_control-undefined",
    ]);
    assert_eq!(rows.len(), expected.len());
    assert_eq!(
        rows.iter()
            .map(|row| row["id"].as_str().unwrap())
            .collect::<BTreeSet<_>>(),
        expected
    );
    for row in rows {
        let id = row["id"].as_str().unwrap();
        let before = state(row, "before");
        let private = row["privateBefore"].as_u64().unwrap();
        let initial = c::initial_state(ConstructorContext::new(0u64)).unwrap();
        let mut derived = initial.into_circuit_context(Default::default());
        if let Some(setup) = row["setup"].as_array() {
            for step in setup {
                derived = native_call(derived, step, &Witness).unwrap().context;
            }
        }
        assert_eq!(
            derived.query.state, before.data,
            "constructor-derived prestate {id}"
        );
        assert_eq!(derived.private_state, private);
        let identity = ArtifactIdentity {
            source_sha256: Sha256::digest(include_bytes!(
                "../../../tools/compact-rust-backend/tests/set-composition/pure_unit_guard.compact"
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
        let context = ConstructorResult::new(ConstructorContext::new(private), before.data.clone())
            .into_circuit_context(Default::default());
        let nw = Witness;
        let rw = Witness;
        let native = native_call(context, row, &nw);
        let recorded = lab.recorded(|ctx| recorded_call(ctx, row, &rw));
        assert_eq!(row["witnessInputs"], json!([]));
        if let Some(message) = row["error"].as_str() {
            let expected = r::CompactError::AssertionFailed(
                message
                    .strip_prefix("failed assert: ")
                    .unwrap_or(message)
                    .into(),
            );
            assert_eq!(native.err().unwrap(), expected, "native {id}");
            assert!(
                matches!(recorded,Err(LabError::Execution(ref error)) if *error==expected),
                "recorded {id}"
            );
            assert_eq!(lab.snapshot().public_state(), &before.data, "rollback {id}");
            assert_eq!(*lab.private_state(), private, "private rollback {id}");
            continue;
        }
        let native = native.unwrap();
        let recorded = recorded.unwrap();
        let after = state(row, "after");
        let _: () = native.result;
        assert_eq!(row["result"], json!([]));
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
        assert_eq!(*lab.private_state(), row["privateAfter"].as_u64().unwrap());
        assert_eq!(
            recorded.private_outputs(),
            &native.private_transcript_outputs
        );
        let outputs:Vec<_>=recorded.private_outputs().iter().map(|value|json!({"valueAtoms":value.value.0.iter().map(|a|&a.0).collect::<Vec<_>>(),"alignment":value.alignment})).collect();
        assert_eq!(
            json!(outputs),
            row["privateTranscript"],
            "private transcript {id}"
        );
        assert_eq!(
            json!(recorded.replay().unwrap().program()),
            row["publicTranscript"],
            "ordered VM {id}"
        );
        assert_eq!(recorded.execution_gas(), native.gas_cost, "native gas {id}");
        let gas = json!(recorded.execution_gas());
        for dimension in ["readTime", "computeTime", "bytesWritten", "bytesDeleted"] {
            let sum: u64 = row["queries"]
                .as_array()
                .unwrap()
                .iter()
                .map(|q| q[dimension].as_str().unwrap().parse::<u64>().unwrap())
                .sum();
            assert_eq!(gas[dimension], sum, "{id}: {dimension}");
        }
    }
}
