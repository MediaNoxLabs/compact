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
use compact_rust_witness_field_expression_fixture::ledger_contract as c;

#[derive(Clone, Default)]
struct OrderedSecret(std::rc::Rc<std::cell::RefCell<Vec<(u64, Field)>>>);
impl Witnesses<u64> for OrderedSecret {
    fn secret(
        &self,
        context: WitnessContext<'_, u64, LedgerView<'_>>,
        seed: Field,
    ) -> (u64, Field) {
        self.0.borrow_mut().push((*context.private_state, seed));
        Secret.secret(context, seed)
    }
}

#[test]
fn recorded_field_expressions_match_retained_ts_and_witness_order() {
    let add: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../runtime-rs/tests/fixtures/witness-field-expression-ts-output.json"
    ))
    .unwrap();
    let sum: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../runtime-rs/tests/fixtures/witness-field-expression-pair-ts-output.json"
    ))
    .unwrap();
    let (state, key) = observation(&["add_secret", "sum_secrets"]);
    let seed = Field::from(2_u64);
    for (name, expected) in [("add_secret", &add), ("sum_secrets", &sum)] {
        let a = OrderedSecret::default();
        let b = OrderedSecret::default();
        let w = OrderedSecret::default();
        let contract = c::Contract::from(w.clone());
        let handle = contract.recording();
        let (native, recorded, call, expected_calls) = if name == "add_secret" {
            (
                c::add_secret(state.circuit_context(7), &a, seed).unwrap(),
                c::recorded::add_secret(state.circuit_context(7), &b, seed).unwrap(),
                handle.add_secret_call(&state, 7, seed).unwrap(),
                vec![(7, seed)],
            )
        } else {
            (
                c::sum_secrets(state.circuit_context(7), &a, seed).unwrap(),
                c::recorded::sum_secrets(state.circuit_context(7), &b, seed).unwrap(),
                handle.sum_secrets_call(&state, 7, seed).unwrap(),
                vec![(7, seed), (8, seed)],
            )
        };
        for witness in [&a, &b, &w] {
            assert_eq!(*witness.0.borrow(), expected_calls, "{name}");
        }
        parity(&recorded, &native, expected);
        parity(call.recorded(), &native, expected);
        refuses_empty(call, key.clone());
    }
}
