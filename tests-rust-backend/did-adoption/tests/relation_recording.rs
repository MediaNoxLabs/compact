// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Original DID relation cases: typed calls, independent TS observations and rollback.
use compact_rust_did_adoption_fixture::{ledger_contract as c, runtime as r};
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
use lifecycle_witness::Witness;

fn state(text: &str) -> r::ledger::ContractState<r::ledger::DefaultDB> {
    midnight_serialize::tagged_deserialize(&mut hex::decode(text).unwrap().as_slice()).unwrap()
}
fn error(text: &str) -> r::CompactError {
    r::CompactError::AssertionFailed(text.strip_prefix("failed assert: ").unwrap_or(text).into())
}
fn tag_first(value: &Value) -> String {
    match value {
        Value::Array(values) => format!(
            "[{}]",
            values.iter().map(tag_first).collect::<Vec<_>>().join(",")
        ),
        Value::Object(fields) => {
            let entries = fields.get("tag").map(|v| ("tag", v)).into_iter().chain(
                fields
                    .iter()
                    .filter(|(key, _)| key.as_str() != "tag")
                    .map(|(key, value)| (key.as_str(), value)),
            );
            format!(
                "{{{}}}",
                entries
                    .map(|(key, value)| format!(
                        "{}:{}",
                        serde_json::to_string(key).unwrap(),
                        tag_first(value)
                    ))
                    .collect::<Vec<_>>()
                    .join(",")
            )
        }
        _ => serde_json::to_string(value).unwrap(),
    }
}
fn replay_queries(
    before: &r::ledger::ContractState<r::ledger::DefaultDB>,
    private: u64,
    row: &Value,
) -> r::ledger::QueryContext<r::ledger::DefaultDB> {
    let context = ConstructorResult::new(ConstructorContext::new(private), before.data.clone())
        .into_circuit_context(Default::default());
    let mut query = context.query;
    let mut index = 0;
    for event in row["events"].as_array().unwrap() {
        if event.get("witness").is_some() {
            continue;
        }
        assert_eq!(event["query"], index, "{} query order", row["id"]);
        let mut gather = event["program"].clone();
        for op in gather.as_array_mut().unwrap() {
            if let Some(pop) = op.get_mut("popeq") {
                assert!(pop.get("result").is_none());
                pop.as_object_mut()
                    .unwrap()
                    .insert("result".into(), Value::Null);
            }
        }
        let program: Vec<
            midnight_onchain_vm::ops::Op<
                midnight_onchain_vm::result_mode::ResultModeGather,
                r::ledger::DefaultDB,
            >,
        > = serde_json::from_str(&tag_first(&gather)).unwrap();
        let result = query.query(&program, None, &context.cost_model).unwrap();
        let gas = json!(result.gas_cost);
        for dimension in ["readTime", "computeTime", "bytesWritten", "bytesDeleted"] {
            assert_eq!(
                gas[dimension].as_u64().unwrap().to_string(),
                row["queries"][index][dimension].as_str().unwrap(),
                "{} query {index} {dimension}",
                row["id"]
            );
        }
        query = result.context;
        index += 1;
    }
    assert_eq!(index, row["queries"].as_array().unwrap().len());
    query
}
fn recorded(
    context: r::context::CircuitContext<u64>,
    witness: &Witness,
    row: &Value,
) -> Result<r::recording::RecordedCircuitResult<u64, ()>, r::CompactError> {
    let a = &row["args"];
    let signature = compact_rust_did_adoption_fixture::types::SchnorrSignature {
        announcement: codec::point("2"),
        response: codec::field_hex(row["responseHex"].as_str().unwrap()),
    };
    c::recorded::setVerificationMethodRelation(
        context,
        witness,
        codec::relation(&a["relation"]),
        codec::string(&a["id"]),
        codec::set(&a["mutation"]),
        signature,
        codec::version(row["version"].as_str().unwrap()),
    )
}
fn check(row: &Value) {
    let label = row["id"].as_str().unwrap();
    let before = state(row["before"].as_str().unwrap());
    let private = row["privateBefore"].as_u64().unwrap();
    let identity = ArtifactIdentity {
        source_sha256: Sha256::digest(include_bytes!(
            "../../../examples/rust_backend/did_adoption/packages/contract/src/did.compact"
        ))
        .into(),
        generated_sha256: Sha256::digest(include_bytes!("../lib.rs")).into(),
    };
    let env = Environment::new(Default::default(), Default::default(), [0x71; 32]);
    let constructor = ConstructorResult::new(ConstructorContext::new(private), before.data.clone());
    let mut lab = ContractLab::from_constructor(identity, env, constructor).unwrap();
    let nw = Witness::new(row["options"].clone());
    let rw = Witness::new(row["options"].clone());
    let native = lifecycle_calls::invoke(
        ConstructorResult::new(ConstructorContext::new(private), before.data.clone())
            .into_circuit_context(Default::default()),
        &nw,
        row,
    );
    let recorded = lab.recorded(|context| recorded(context, &rw, row));
    assert_eq!(
        json!(*nw.calls.borrow()),
        row["witnessCalls"],
        "{label} native witnesses"
    );
    assert_eq!(
        json!(*rw.calls.borrow()),
        row["witnessCalls"],
        "{label} recorded witnesses"
    );
    let ts_queries = replay_queries(&before, private, row);
    if let Some(message) = row["error"].as_str() {
        let expected = error(message);
        assert_eq!(native.err().unwrap(), expected, "{label} native error");
        assert!(
            matches!(recorded,Err(LabError::Execution(ref actual)) if *actual==expected),
            "{label} recorded error: {recorded:?}"
        );
        assert_eq!(
            lab.snapshot().public_state(),
            &before.data,
            "{label} public rollback"
        );
        assert_eq!(lab.private_state(), &private, "{label} private rollback");
        return;
    }
    let native = native.unwrap_or_else(|e| panic!("{label}: native: {e:?}"));
    let recorded = recorded.unwrap_or_else(|e| panic!("{label}: recorded: {e:?}"));
    assert_eq!(
        recorded.public_state(),
        &state(row["after"].as_str().unwrap()).data,
        "{label} TS state"
    );
    assert_eq!(
        recorded.public_state(),
        &native.context.query.state,
        "{label} native state"
    );
    assert_eq!(
        recorded.effects(),
        &native.context.query.effects,
        "{label} effects"
    );
    assert_eq!(
        &ts_queries.state,
        recorded.public_state(),
        "{label} TS query state"
    );
    assert_eq!(
        &ts_queries.effects,
        recorded.effects(),
        "{label} TS query effects"
    );
    assert_eq!(
        *lab.private_state(),
        row["privateAfter"].as_u64().unwrap(),
        "{label} private"
    );
    assert_eq!(
        recorded.private_outputs(),
        &native.private_transcript_outputs,
        "{label} private outputs"
    );
    let transcript:Vec<_>=recorded.private_outputs().iter().map(|v|json!({"valueAtoms":v.value.0.iter().map(|a|&a.0).collect::<Vec<_>>(),"alignment":v.alignment})).collect();
    assert_eq!(
        json!(transcript),
        row["privateTranscript"],
        "{label} TS private transcript"
    );
    assert_eq!(
        json!(recorded.replay().unwrap().program()),
        row["publicTranscript"],
        "{label} public program"
    );
    assert_eq!(
        recorded.execution_gas(),
        native.gas_cost,
        "{label} native gas"
    );
    let gas = json!(recorded.execution_gas());
    for dimension in ["readTime", "computeTime", "bytesWritten", "bytesDeleted"] {
        let total: u64 = row["queries"]
            .as_array()
            .unwrap()
            .iter()
            .map(|q| q[dimension].as_str().unwrap().parse::<u64>().unwrap())
            .sum();
        assert_eq!(gas[dimension], total, "{label} TS {dimension}");
    }
}
#[test]
fn original_42_relation_calls_match_native_recorded_replay_and_owned_rollback() {
    let mut total = 0;
    let mut success = 0;
    let mut failure = 0;
    for capture in [
        include_str!("../oracle/lifecycle.json"),
        include_str!("../oracle/jwk-method-lifecycle.json"),
    ] {
        let data: Value = serde_json::from_str(capture).unwrap();
        for scenario in data["scenarios"].as_array().unwrap() {
            for row in scenario["steps"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|r| r["name"] == "setVerificationMethodRelation")
            {
                check(row);
                total += 1;
                if row.get("error").is_some() {
                    failure += 1
                } else {
                    success += 1
                }
            }
        }
    }
    assert_eq!((total, success, failure), (42, 23, 19));
}
