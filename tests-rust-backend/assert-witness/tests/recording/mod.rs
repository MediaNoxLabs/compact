// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use super::*;
use midnight_compact_runtime as r;
use midnight_onchain_state::state::{ContractOperation, EntryPointBuf};
use midnight_serialize::{Deserializable, Serializable};
use r::transaction::{
    Observation, ObservedCallError, ObservedContractState, PrepareCallError, VerifierKey,
};

fn observation(names: &[&str]) -> (ObservedContractState, VerifierKey) {
    // Empty serialized carrier only exercises preparation policy; no proof key claim.
    let mut bytes = Vec::new();
    Vec::<u8>::new().serialize(&mut bytes).unwrap();
    let key = VerifierKey::deserialize(&mut bytes.as_slice(), 0).unwrap();
    let mut operations = r::ledger::LedgerHashMap::new();
    for name in names {
        operations = operations.insert(
            EntryPointBuf(name.as_bytes().to_vec()),
            ContractOperation::new(Some(key.clone())),
        );
    }
    let initial = initial_state(ConstructorContext::new(7_u64)).unwrap();
    (
        ObservedContractState::new(
            Default::default(),
            r::ledger::ContractState::new(
                initial.ledger_state.get_ref().clone(),
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

fn parity(
    actual: &r::recording::RecordedCircuitResult<u64, Field>,
    native: &r::context::CircuitResult<u64, Field>,
    expected: &serde_json::Value,
) {
    assert_eq!(actual.execution.result, native.result);
    assert_eq!(
        actual.execution.result,
        Field::from(expected["result"].as_str().unwrap().parse::<u64>().unwrap())
    );
    assert_eq!(
        actual.execution.context.private_state,
        expected["privateState"].as_u64().unwrap()
    );
    assert_eq!(
        actual.execution.context.private_state,
        native.context.private_state
    );
    assert_eq!(
        actual.execution.private_transcript_outputs,
        native.private_transcript_outputs
    );
    assert_eq!(
        actual.execution.context.query.state,
        native.context.query.state
    );
    assert_eq!(
        actual.execution.context.query.effects,
        native.context.query.effects
    );
    assert_eq!(actual.execution.gas_cost, native.gas_cost);
    assert!(actual.public.verify_ops().is_empty());
    let outputs = actual
        .execution
        .private_transcript_outputs
        .iter()
        .map(|output| {
            serde_json::json!({
                "valueAtoms": output.value.0.iter().map(|atom| &atom.0).collect::<Vec<_>>(),
                "alignment": output.alignment,
            })
        })
        .collect::<Vec<_>>();
    assert_eq!(
        serde_json::to_value(outputs).unwrap(),
        expected["privateTranscriptOutputs"]
    );
}

fn refuses_empty(call: r::transaction::RecordedCall<'_, u64, Field>, key: VerifierKey) {
    assert!(matches!(
        call.prepare(key, Field::from(0_u64)),
        Err(ObservedCallError::Prepare(
            PrepareCallError::EmptyTranscript
        ))
    ));
}
use compact_rust_assert_witness_fixture::ledger_contract as c;

#[test]
fn recorded_checked_value_preserves_assertion_prefix_and_empty_preparation() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../runtime-rs/tests/fixtures/assert-witness.json"
    ))
    .unwrap();
    let (state, key) = observation(&["checked_value"]);
    for (name, first, second) in [
        ("pass", true, true),
        ("firstFails", false, false),
        ("secondFails", true, false),
    ] {
        let a = Echo::default();
        let b = Echo::default();
        let w = Echo::default();
        let contract = c::Contract::from(w.clone());
        let value = Field::from(42_u64);
        let native = c::checked_value(state.circuit_context(7), &a, first, second, value);
        let recorded =
            c::recorded::checked_value(state.circuit_context(7), &b, first, second, value);
        let call = contract
            .recording()
            .checked_value_call(&state, 7, first, second, value);
        for witness in [&a, &b, &w] {
            observed_matches(witness, &oracle[name]);
        }
        if name == "pass" {
            let native = native.unwrap();
            let recorded = recorded.unwrap();
            let call = call.unwrap();
            parity(&recorded, &native, &oracle[name]);
            parity(call.recorded(), &native, &oracle[name]);
            refuses_empty(call, key.clone());
        } else {
            assert_eq!(native.err().unwrap().to_string(), oracle[name]["error"]);
            assert_eq!(recorded.err().unwrap().to_string(), oracle[name]["error"]);
            assert_eq!(call.err().unwrap().to_string(), oracle[name]["error"]);
        }
    }
}
