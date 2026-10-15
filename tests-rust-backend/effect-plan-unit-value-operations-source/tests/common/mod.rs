// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use compact_rust_effect_plan_unit_value_operations_source_fixture::runtime as r;
use r::context::CircuitResult;
use r::recording::RecordedCircuitResult;
pub fn atoms(value: &r::fab::AlignedValue) -> Vec<Vec<u8>> {
    value.value.0.iter().map(|atom| atom.0.clone()).collect()
}
pub fn compare<T: std::fmt::Debug + PartialEq>(
    native: &CircuitResult<u64, T>,
    recorded: &RecordedCircuitResult<u64, T>,
    manual: &RecordedCircuitResult<u64, T>,
) {
    for actual in [&recorded.execution, &manual.execution] {
        assert_eq!(actual.result, native.result);
        assert_eq!(actual.context.query.state, native.context.query.state);
        assert_eq!(actual.context.query.effects, native.context.query.effects);
        assert_eq!(actual.context.private_state, native.context.private_state);
        assert_eq!(actual.gas_cost, native.gas_cost);
        assert_eq!(
            actual.private_transcript_outputs,
            native.private_transcript_outputs
        );
    }
    assert_eq!(
        serde_json::to_value(recorded.public.verify_ops()).unwrap(),
        serde_json::to_value(manual.public.verify_ops()).unwrap()
    );
    let replay = recorded
        .public
        .initial()
        .query(
            recorded.public.verify_ops(),
            None,
            &native.context.cost_model,
        )
        .unwrap();
    let reference = manual
        .public
        .initial()
        .query(manual.public.verify_ops(), None, &native.context.cost_model)
        .unwrap();
    assert_eq!(replay.context.state, native.context.query.state);
    assert_eq!(replay.context.effects, native.context.query.effects);
    assert_eq!(replay.gas_cost, reference.gas_cost);
}
pub fn row(id: &str) -> serde_json::Value {
    let oracle: serde_json::Value =
        serde_json::from_str(include_str!("../../oracle/typescript.json")).unwrap();
    let rows = oracle["rows"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|row| row["id"] == id)
        .collect::<Vec<_>>();
    assert_eq!(rows.len(), 1, "unique TS case {id}");
    rows[0].clone()
}
pub fn assert_ts<T>(
    row: &serde_json::Value,
    input: &r::fab::AlignedValue,
    output: &r::fab::AlignedValue,
    execution: &CircuitResult<u64, T>,
) {
    assert_eq!(row["status"], "success");
    assert_eq!(
        row["proofData"]["input"]["value"],
        serde_json::json!(atoms(input))
    );
    assert_eq!(
        row["proofData"]["input"]["alignment"],
        serde_json::to_value(&input.alignment).unwrap()
    );
    assert_ts_execution(row, output, execution);
}
pub fn assert_ts_execution<T>(
    row: &serde_json::Value,
    output: &r::fab::AlignedValue,
    execution: &CircuitResult<u64, T>,
) {
    assert_eq!(row["status"], "success");
    assert_eq!(
        row["proofData"]["output"]["value"],
        serde_json::json!(atoms(output))
    );
    assert_eq!(
        row["proofData"]["output"]["alignment"],
        serde_json::to_value(&output.alignment).unwrap()
    );
    assert_ts_context(row, execution);
}
/// Compare effects-related evidence without claiming equal hash output.
pub fn assert_ts_context<T>(row: &serde_json::Value, execution: &CircuitResult<u64, T>) {
    let private = row["proofData"]["privateTranscriptOutputs"]
        .as_array()
        .unwrap();
    assert_eq!(private.len(), execution.private_transcript_outputs.len());
    for (expected, actual) in private.iter().zip(&execution.private_transcript_outputs) {
        assert_eq!(expected["value"], serde_json::json!(atoms(actual)));
        assert_eq!(
            expected["alignment"],
            serde_json::to_value(&actual.alignment).unwrap()
        );
    }
    // TS wrapper cost omits separately metered witness ledger views; the capture
    // sums exactly the circuit queries and callback reads, before final inspection.
    let gas = &execution.gas_cost;
    assert_eq!(
        row["queryCostSum"]["readTime"],
        gas.read_time.into_picoseconds().to_string()
    );
    assert_eq!(
        row["queryCostSum"]["computeTime"],
        gas.compute_time.into_picoseconds().to_string()
    );
    assert_eq!(
        row["queryCostSum"]["bytesWritten"],
        gas.bytes_written.to_string()
    );
    assert_eq!(
        row["queryCostSum"]["bytesDeleted"],
        gas.bytes_deleted.to_string()
    );
    assert_eq!(row["privateState"], execution.context.private_state);
}
pub mod state;
pub mod values;
pub mod witness;
