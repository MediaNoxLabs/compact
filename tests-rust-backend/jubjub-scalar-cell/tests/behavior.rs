// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use compact_rust_jubjub_scalar_cell_fixture::{ledger_contract as c, runtime as r};
use midnight_compact_testkit::{ArtifactIdentity, ContractLab, Environment, LabError};
use r::context::{ConstructorContext, ConstructorResult};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

fn state(value: &Value) -> r::ledger::ContractState<r::ledger::DefaultDB> {
    midnight_serialize::tagged_deserialize(
        &mut hex::decode(value.as_str().unwrap()).unwrap().as_slice(),
    )
    .unwrap()
}
fn field(value: &Value) -> r::Field {
    r::Field::from_le_bytes(&hex::decode(value.as_str().unwrap()).unwrap()).unwrap()
}
fn point(value: r::JubjubPoint) -> Value {
    let atoms = r::fab::Value::from(value);
    json!({"x":hex::encode(r::jubjub_point_x(value).as_le_bytes()),"y":hex::encode(r::jubjub_point_y(value).as_le_bytes()),"fab":atoms.0.iter().map(|a|hex::encode(&a.0)).collect::<Vec<_>>()})
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
// The JS capture serializes Maps as entry arrays; Rust serde uses objects.
// This profile has no asset effects, so require empty maps before normalizing.
fn empty_effects(value: &Value) -> Value {
    let mut value = value.clone();
    for field in [
        "claimedUnshieldedSpends",
        "shieldedMints",
        "unshieldedInputs",
        "unshieldedMints",
        "unshieldedOutputs",
    ] {
        assert_eq!(
            value[field],
            json!([]),
            "unexpected nonempty effect {field}"
        );
        value[field] = json!({});
    }
    value
}
fn encoded(value: &r::ledger::ContractState<r::ledger::DefaultDB>) -> String {
    let mut bytes = Vec::new();
    midnight_serialize::tagged_serialize(value, &mut bytes).unwrap();
    hex::encode(bytes)
}

#[test]
fn scalar_boundaries_match_original_oracle_ts_state_and_recorded_execution() {
    let capture: Value = serde_json::from_str(include_str!("../oracle/cases.json")).unwrap();
    assert_eq!(capture["format"], "compact-acc-jubjub-wrapper-capture/v1");
    let rows = capture["cases"].as_array().unwrap();
    assert_eq!(rows.len(), 7);
    assert_eq!(
        rows.iter()
            .map(|row| row["id"].as_str().unwrap())
            .collect::<BTreeSet<_>>(),
        BTreeSet::from([
            "zero",
            "one",
            "eight",
            "q_minus_one",
            "q",
            "q_plus_one",
            "native_max"
        ])
    );
    for row in rows {
        let id = row["id"].as_str().unwrap();
        let before = state(&row["before"]);
        let after = state(&row["after"]);
        let private = row["privateBefore"].as_u64().unwrap();
        let initial = c::initial_state(ConstructorContext::new(private)).unwrap();
        assert_eq!(
            initial.ledger_state, before.data,
            "constructor prestate {id}"
        );
        let mut reconstructed = before.clone();
        reconstructed.data = initial.ledger_state.clone();
        assert_eq!(encoded(&reconstructed), row["before"].as_str().unwrap());
        let identity = ArtifactIdentity {
            source_sha256: Sha256::digest(include_bytes!(
                "../../../examples/rust_backend/jubjub_scalar_cell.compact"
            ))
            .into(),
            generated_sha256: Sha256::digest(include_bytes!("../lib.rs")).into(),
        };
        let mut lab = ContractLab::from_constructor(
            identity,
            Environment::new(Default::default(), Default::default(), [0; 32]),
            initial,
        )
        .unwrap();
        let supplied = r::ec_mul_generator(r::Field::from(7u64)).unwrap();
        assert_eq!(point(supplied), row["point"]);
        let scalar = field(&row["scalar"]);
        let context = ConstructorResult::new(ConstructorContext::new(private), before.data.clone())
            .into_circuit_context(Default::default());
        let native = c::apply(context, supplied, scalar).unwrap();
        let recorded = lab
            .recorded(|ctx| c::recorded::apply(ctx, supplied, scalar))
            .unwrap();
        assert_eq!(
            point(native.result),
            row["expected"],
            "original oracle {id}"
        );
        assert_eq!(point(*recorded.output()), row["result"], "TS point {id}");
        assert_eq!(*recorded.output(), native.result);
        assert_eq!(recorded.public_state(), &after.data, "TS state {id}");
        assert_eq!(recorded.public_state(), &native.context.query.state);
        reconstructed.data = recorded.public_state().clone();
        assert_eq!(
            encoded(&reconstructed),
            row["after"].as_str().unwrap(),
            "complete serialized state {id}"
        );
        assert_eq!(recorded.effects(), &native.context.query.effects);
        assert_eq!(
            json!(recorded.effects()),
            empty_effects(&row["effectsAfter"])
        );
        assert_eq!(*lab.private_state(), row["privateAfter"].as_u64().unwrap());
        assert_eq!(native.context.private_state, private);
        assert_eq!(
            recorded.private_outputs(),
            &native.private_transcript_outputs
        );
        assert!(recorded.private_outputs().is_empty());
        assert_eq!(row["privateTranscript"], json!([]));
        assert_eq!(row["witnessInputs"], json!([]));
        assert_eq!(
            json!(recorded.replay().unwrap().program()),
            row["publicTranscript"]
        );
        assert_eq!(recorded.execution_gas(), native.gas_cost);
        let gas = json!(recorded.execution_gas());
        for dimension in ["readTime", "computeTime", "bytesWritten", "bytesDeleted"] {
            assert_eq!(
                gas[dimension].as_u64().unwrap().to_string(),
                row["totalGas"][dimension].as_str().unwrap()
            );
        }
        // Replay each captured query separately. This is not a whole-program
        // replay gas shortcut, even though this profile currently has one query.
        let context = ConstructorResult::new(ConstructorContext::new(private), before.data.clone())
            .into_circuit_context(Default::default());
        let mut query = context.query;
        assert_eq!(
            row["queryTrace"].as_array().unwrap().len(),
            row["queries"].as_array().unwrap().len()
        );
        for (index, trace) in row["queryTrace"].as_array().unwrap().iter().enumerate() {
            let program: Vec<
                midnight_onchain_vm::ops::Op<
                    midnight_onchain_vm::result_mode::ResultModeGather,
                    r::ledger::DefaultDB,
                >,
            > = serde_json::from_str(&tag_first(&trace["program"])).unwrap();
            let result = query.query(&program, None, &context.cost_model).unwrap();
            let gas = json!(result.gas_cost);
            for dimension in ["readTime", "computeTime", "bytesWritten", "bytesDeleted"] {
                assert_eq!(
                    gas[dimension].as_u64().unwrap().to_string(),
                    row["queries"][index][dimension].as_str().unwrap()
                );
            }
            query = result.context;
        }
        assert_eq!(query.state, after.data);
        assert_eq!(query.effects, native.context.query.effects);
        if id == "q" {
            assert_eq!(native.result, r::JubjubPoint::identity());
            assert_eq!(
                r::ec_mul(supplied, scalar),
                Err(r::CompactError::InvalidJubjubScalar)
            );
        }
    }
}

#[test]
fn generated_raw_mul_preserves_canonical_refusal_and_rollback() {
    let capture: Value = serde_json::from_str(include_str!("../oracle/cases.json")).unwrap();
    let rows = capture["rawCases"].as_array().unwrap();
    assert_eq!(rows.len(), 2);
    for row in rows {
        let id = row["id"].as_str().unwrap();
        let before = state(&row["before"]);
        let private = row["privateBefore"].as_u64().unwrap();
        let initial = c::initial_state(ConstructorContext::new(private)).unwrap();
        assert_eq!(initial.ledger_state, before.data);
        let identity = ArtifactIdentity {
            source_sha256: Sha256::digest(include_bytes!(
                "../../../examples/rust_backend/jubjub_scalar_cell.compact"
            ))
            .into(),
            generated_sha256: Sha256::digest(include_bytes!("../lib.rs")).into(),
        };
        let mut lab = ContractLab::from_constructor(
            identity,
            Environment::new(Default::default(), Default::default(), [0; 32]),
            initial,
        )
        .unwrap();
        let supplied = r::ec_mul_generator(r::Field::from(7u64)).unwrap();
        let scalar = field(&row["scalar"]);
        let context = ConstructorResult::new(ConstructorContext::new(private), before.data.clone())
            .into_circuit_context(Default::default());
        let native = c::raw(context, supplied, scalar);
        let recorded = lab.recorded(|ctx| c::recorded::raw(ctx, supplied, scalar));
        if id == "raw-q" {
            assert_eq!(native.err().unwrap(), r::CompactError::InvalidJubjubScalar);
            assert!(matches!(
                recorded,
                Err(LabError::Execution(r::CompactError::InvalidJubjubScalar))
            ));
            assert_eq!(lab.snapshot().public_state(), &before.data);
            assert_eq!(*lab.private_state(), private);
            assert_eq!(row["queries"], json!([]));
            assert_eq!(row["before"], row["after"]);
            continue;
        }
        assert_eq!(id, "raw-q_minus_one");
        let native = native.unwrap();
        let recorded = recorded.unwrap();
        assert_eq!(point(native.result), row["expected"]);
        assert_eq!(point(*recorded.output()), row["result"]);
        assert_eq!(recorded.public_state(), &state(&row["after"]).data);
        assert_eq!(recorded.public_state(), &native.context.query.state);
        assert_eq!(recorded.effects(), &native.context.query.effects);
        assert_eq!(
            json!(recorded.effects()),
            empty_effects(&row["effectsAfter"])
        );
        assert!(recorded.private_outputs().is_empty());
        assert_eq!(*lab.private_state(), private);
        assert_eq!(
            json!(recorded.replay().unwrap().program()),
            row["publicTranscript"]
        );
        assert_eq!(recorded.execution_gas(), native.gas_cost);
        let gas = json!(native.gas_cost);
        for dimension in ["readTime", "computeTime", "bytesWritten", "bytesDeleted"] {
            assert_eq!(
                gas[dimension].as_u64().unwrap().to_string(),
                row["totalGas"][dimension].as_str().unwrap()
            );
        }
    }
}
