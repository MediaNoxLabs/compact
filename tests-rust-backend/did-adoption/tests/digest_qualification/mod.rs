// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Original frozen TS cases plus provider refusal. Empty verifier carriers test preparation only.
use super::*;
use midnight_serialize::{Deserializable, Serializable};
use r::fab::AlignedValue;
use r::recording::{RecordedCircuitResult, RecordingFrame};
use r::transaction::{CallSpec, Observation, ObservedContractState, VerifierKey, prepare_call};
use std::{cell::RefCell, rc::Rc};
mod manual;
mod provider;
mod scenarios;
use provider::Provider;

fn assertion(ok: bool, message: &str) -> Result<(), r::CompactError> {
    if ok {
        Ok(())
    } else {
        Err(r::CompactError::AssertionFailed(message.into()))
    }
}
fn observation(state: r::ledger::ContractState<r::ledger::DefaultDB>) -> ObservedContractState {
    ObservedContractState::new(
        Default::default(),
        state,
        Observation {
            transaction_hash: [0; 32],
            block_hash: [0; 32],
            block_height: 0,
        },
    )
}
fn verifier() -> VerifierKey {
    let mut bytes = Vec::new();
    Vec::<u8>::new().serialize(&mut bytes).unwrap();
    VerifierKey::deserialize(&mut bytes.as_slice(), 0).unwrap()
}
fn parity(
    actual: &RecordedCircuitResult<u64, ()>,
    manual: &RecordedCircuitResult<u64, ()>,
    native: &r::context::CircuitResult<u64, ()>,
    row: &Value,
) {
    assert_eq!(
        actual.execution.context.query.state,
        native.context.query.state
    );
    assert_eq!(
        actual.execution.context.query.effects,
        native.context.query.effects
    );
    assert_eq!(
        actual.execution.context.private_state,
        row["privateAfter"].as_u64().unwrap()
    );
    assert_eq!(
        actual.execution.private_transcript_outputs,
        native.private_transcript_outputs
    );
    assert_eq!(
        actual.execution.context.private_state,
        native.context.private_state
    );
    assert_eq!(actual.execution.gas_cost, native.gas_cost);
    assert_eq!(actual.public.verify_ops(), manual.public.verify_ops());
    assert_eq!(json!(actual.public.verify_ops()), row["publicTranscript"]);
    let outputs: Vec<_> = actual.execution.private_transcript_outputs.iter().map(|v| json!({
        "valueAtoms": v.value.0.iter().map(|a| &a.0).collect::<Vec<_>>(), "alignment":v.alignment,
    })).collect();
    assert_eq!(json!(outputs), row["privateTranscript"]);
    let replay = actual
        .public
        .initial()
        .query(actual.public.verify_ops(), None, &native.context.cost_model)
        .unwrap();
    let expected = manual
        .public
        .initial()
        .query(manual.public.verify_ops(), None, &native.context.cost_model)
        .unwrap();
    assert_eq!(replay.context.state, native.context.query.state);
    assert_eq!(replay.context.effects, native.context.query.effects);
    assert_eq!(replay.gas_cost, expected.gas_cost); // Replay and execution have distinct costs.
    let gas = json!(actual.execution.gas_cost);
    for name in ["readTime", "computeTime", "bytesWritten", "bytesDeleted"] {
        let sum: u64 = row["queries"]
            .as_array()
            .unwrap()
            .iter()
            .map(|q| q[name].as_str().unwrap().parse::<u64>().unwrap())
            .sum();
        assert_eq!(gas[name], sum);
    }
}
