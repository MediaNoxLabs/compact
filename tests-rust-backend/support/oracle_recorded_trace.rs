// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use midnight_compact_runtime::{
    context::CircuitResult,
    ledger::{DefaultDB, StateValue},
    recording::RecordedCircuitResult,
};
use serde_json::{Value, json};

pub fn assert_trace<Private: std::fmt::Debug + PartialEq>(
    native: &CircuitResult<Private, ()>,
    recorded: &RecordedCircuitResult<Private, ()>,
    row: &Value,
    state_hex: impl Fn(StateValue<DefaultDB>) -> String,
) {
    assert_eq!(row["result"], json!([]));
    assert_eq!(
        state_hex(recorded.public.initial().state.get_ref().clone()),
        row["before"]
    );
    assert_eq!(
        state_hex(native.context.query.state.get_ref().clone()),
        row["after"]
    );
    assert_eq!(
        native.context.query.state,
        recorded.execution.context.query.state
    );
    assert_eq!(
        native.context.query.effects,
        recorded.execution.context.query.effects
    );
    assert_eq!(
        native.context.private_state,
        recorded.execution.context.private_state
    );
    assert_eq!(json!(recorded.public.verify_ops()), row["publicTranscript"]);
    assert_eq!(
        native.private_transcript_outputs,
        recorded.execution.private_transcript_outputs
    );
    let private: Vec<_> = native
        .private_transcript_outputs
        .iter()
        .map(|v| {
            let atoms: Vec<_> = v.value.0.iter().map(|a| &a.0).collect();
            json!({"valueAtoms":atoms,"alignment":v.alignment})
        })
        .collect();
    assert_eq!(json!(private), row["privateTranscript"]);
    assert_eq!(native.gas_cost, recorded.execution.gas_cost);
    let gas = json!(native.gas_cost);
    let queries = row["queries"].as_array().unwrap();
    for dimension in ["readTime", "computeTime", "bytesWritten", "bytesDeleted"] {
        let total: u64 = queries
            .iter()
            .map(|q| {
                q["gasCost"][dimension]
                    .as_str()
                    .unwrap()
                    .parse::<u64>()
                    .unwrap()
            })
            .sum();
        assert_eq!(gas[dimension], total, "{}: {dimension}", row["id"]);
        assert_eq!(
            row["reportedGas"][dimension],
            queries.last().unwrap()["gasCost"][dimension]
        );
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
    assert_eq!(replay.context.state, native.context.query.state);
    assert_eq!(replay.context.effects, native.context.query.effects);
}

pub fn cases(source: &str, count: usize) -> Vec<Value> {
    let capture: Value = serde_json::from_str(include_str!(
        "../../runtime-rs/tests/fixtures/oracle-recorded-traces.json"
    ))
    .unwrap();
    assert_eq!(capture["cases"].as_array().unwrap().len(), 9);
    let rows: Vec<_> = capture["cases"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|row| row["source"] == source)
        .cloned()
        .collect();
    assert_eq!(rows.len(), count);
    rows
}
