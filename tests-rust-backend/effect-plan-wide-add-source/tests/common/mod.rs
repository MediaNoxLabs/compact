// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use compact_rust_effect_plan_wide_add_source_fixture::{ledger_contract as c, runtime as r};
use midnight_onchain_state::state::{ContractOperation, EntryPointBuf};
use r::context::{CircuitResult, ConstructorContext};
use r::recording::RecordedCircuitResult;
use r::transaction::{Observation, ObservedContractState, VerifierKey};
use rand::{Rng, SeedableRng, rngs::StdRng};
pub fn bytes(high: u128, low: u128) -> Vec<u8> {
    let mut result = low.to_le_bytes().to_vec();
    result.extend(high.to_le_bytes());
    while result.last() == Some(&0) {
        result.pop();
    }
    result
}
pub fn setup() -> (ObservedContractState, VerifierKey) {
    let key: VerifierKey = StdRng::seed_from_u64(564).r#gen();
    let mut operations = midnight_storage::storage::HashMap::new();
    for name in ["add128", "mixed", "narrow128", "witnessed"] {
        operations = operations.insert(
            EntryPointBuf(name.as_bytes().to_vec()),
            ContractOperation::new(Some(key.clone())),
        );
    }
    let context = c::initial_state(ConstructorContext::new(5_u64)).unwrap();
    let context = context.into_circuit_context(Default::default());
    (
        ObservedContractState::new(
            Default::default(),
            r::ledger::ContractState::new(
                context.query.state.get_ref().clone(),
                operations,
                Default::default(),
            ),
            Observation {
                transaction_hash: [0; 32],
                block_hash: [0; 32],
                block_height: 0,
            },
        ),
        key,
    )
}
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
/// Compare the captured official TS row, including private output encoding.
/// Rust meters callback ledger reads; TS wrapper gas excludes their separate views.
/// Compare against measured query costs, preserving wrapper gas in the oracle.
pub fn assert_ts<T>(
    name: &str,
    input: &r::fab::AlignedValue,
    output: &r::fab::AlignedValue,
    execution: &CircuitResult<u64, T>,
) {
    let oracle: serde_json::Value =
        serde_json::from_str(include_str!("../../oracle/typescript.json")).unwrap();
    let input_atoms = serde_json::json!(atoms(input));
    let output_atoms = serde_json::json!(atoms(output));
    let private_atoms = serde_json::json!(
        execution
            .private_transcript_outputs
            .iter()
            .map(atoms)
            .collect::<Vec<_>>()
    );
    let row = oracle["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| {
            row["circuit"] == name
                && row["status"] == "success"
                && row["proofData"]["input"]["value"] == input_atoms
                && row["proofData"]["output"]["value"] == output_atoms
                && serde_json::Value::Array(
                    row["proofData"]["privateTranscriptOutputs"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|value| value["value"].clone())
                        .collect(),
                ) == private_atoms
        })
        .expect("matching official TS execution row");
    let gas = &execution.gas_cost;
    assert_eq!(
        gas.read_time.into_picoseconds().to_string(),
        row["queryCostSum"]["readTime"]
    );
    assert_eq!(
        gas.compute_time.into_picoseconds().to_string(),
        row["queryCostSum"]["computeTime"]
    );
    assert_eq!(
        gas.bytes_written.to_string(),
        row["queryCostSum"]["bytesWritten"]
    );
    assert_eq!(
        gas.bytes_deleted.to_string(),
        row["queryCostSum"]["bytesDeleted"]
    );
    assert_eq!(
        execution.context.private_state,
        row["privateState"].as_u64().unwrap()
    );
}
