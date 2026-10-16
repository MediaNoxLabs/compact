// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//  	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Finite native conformance cases. These do not qualify recording or proofs.

use compact_rust_boolean_logic_fixture::{ledger_contract as contract, pure_circuits};
use midnight_compact_runtime::{
    CompactError,
    context::{CircuitContext, CircuitResult, ConstructorContext, WitnessContext},
    ledger::ContractAddress,
};
use serde_json::json;
use std::cell::RefCell;

const REFUSAL: &str = "Boolean conformance witness refusal";

struct TruthCase {
    left: bool,
    right: bool,
    and_result: bool,
    or_result: bool,
    and_calls: &'static [bool],
    or_calls: &'static [bool],
}

// Literal specification truth tables and selected callback arguments.
const CASES: [TruthCase; 4] = [
    TruthCase {
        left: false,
        right: false,
        and_result: false,
        or_result: false,
        and_calls: &[false],
        or_calls: &[false, false],
    },
    TruthCase {
        left: false,
        right: true,
        and_result: false,
        or_result: true,
        and_calls: &[false],
        or_calls: &[false, true],
    },
    TruthCase {
        left: true,
        right: false,
        and_result: false,
        or_result: true,
        and_calls: &[true, false],
        or_calls: &[true],
    },
    TruthCase {
        left: true,
        right: true,
        and_result: true,
        or_result: true,
        and_calls: &[true, true],
        or_calls: &[true],
    },
];

struct TracedEcho {
    refuse_call: Option<usize>,
    invert_answer: bool,
    calls: RefCell<Vec<(u64, bool)>>,
}

impl TracedEcho {
    fn new(refuse_call: Option<usize>) -> Self {
        Self {
            refuse_call,
            invert_answer: false,
            calls: RefCell::new(Vec::new()),
        }
    }
}

impl contract::TryWitnesses<u64> for TracedEcho {
    fn echo(
        &self,
        context: WitnessContext<'_, u64, contract::LedgerView<'_>>,
        value: bool,
    ) -> Result<(u64, bool), CompactError> {
        let mut calls = self.calls.borrow_mut();
        calls.push((*context.private_state, value));
        if self.refuse_call == Some(calls.len()) {
            return Err(CompactError::AssertionFailed(REFUSAL.into()));
        }
        Ok((
            *context.private_state + 1,
            if self.invert_answer { !value } else { value },
        ))
    }
}

fn context() -> CircuitContext<u64> {
    contract::initial_state(ConstructorContext::new(7_u64))
        .unwrap()
        .into_circuit_context(ContractAddress::default())
}

fn assert_calls(witness: &TracedEcho, expected: &[bool]) {
    let expected: Vec<_> = expected
        .iter()
        .enumerate()
        .map(|(index, value)| (7 + index as u64, *value))
        .collect();
    assert_eq!(*witness.calls.borrow(), expected);
}

fn assert_success(
    result: CircuitResult<u64, bool>,
    witness: &TracedEcho,
    expected_value: bool,
    expected_calls: &[bool],
    expected_outputs: &[bool],
) {
    assert_eq!(result.result, expected_value);
    assert_calls(witness, expected_calls);
    assert_eq!(
        result.context.private_state,
        7 + expected_calls.len() as u64
    );
    assert_eq!(
        result.private_transcript_outputs.len(),
        expected_outputs.len()
    );
    for (output, value) in result
        .private_transcript_outputs
        .iter()
        .zip(expected_outputs)
    {
        assert_eq!(output.value.0.len(), 1, "one atom per Boolean witness");
        // Canonical ledger Boolean atoms: zero is empty, one is [1].
        let expected_bytes: &[u8] = if *value { &[1] } else { &[] };
        assert_eq!(output.value.0[0].0.as_slice(), expected_bytes);
        assert_eq!(
            serde_json::to_value(&output.alignment).unwrap(),
            json!([{ "tag": "atom", "value": { "tag": "bytes", "length": 1 } }])
        );
    }
}

#[test]
fn pure_boolean_truth_tables_are_complete() {
    for case in &CASES {
        assert_eq!(
            pure_circuits::both(case.left, case.right).unwrap(),
            case.and_result
        );
        assert_eq!(
            pure_circuits::either(case.left, case.right).unwrap(),
            case.or_result
        );
    }
    assert!(pure_circuits::invert(false).unwrap());
    assert!(!pure_circuits::invert(true).unwrap());
}

#[test]
fn witnessed_truth_tables_preserve_selected_values_and_private_state() {
    for case in &CASES {
        let witness = TracedEcho::new(None);
        assert_success(
            contract::witnessed_both(context(), &witness, case.left, case.right).unwrap(),
            &witness,
            case.and_result,
            case.and_calls,
            case.and_calls,
        );
        let witness = TracedEcho::new(None);
        assert_success(
            contract::witnessed_either(context(), &witness, case.left, case.right).unwrap(),
            &witness,
            case.or_result,
            case.or_calls,
            case.or_calls,
        );
    }
}

type BinaryCircuit = fn(
    CircuitContext<u64>,
    &TracedEcho,
    bool,
    bool,
) -> Result<CircuitResult<u64, bool>, CompactError>;

#[test]
fn binary_witness_refusals_halt_selected_evaluation_and_skip_unselected_rhs() {
    for case in &CASES {
        for (circuit, expected_value, expected_calls) in [
            (
                contract::witnessed_both::<u64, TracedEcho> as BinaryCircuit,
                case.and_result,
                case.and_calls,
            ),
            (
                contract::witnessed_either::<u64, TracedEcho> as BinaryCircuit,
                case.or_result,
                case.or_calls,
            ),
        ] {
            for refuse_call in [1, 2] {
                let witness = TracedEcho::new(Some(refuse_call));
                let result = circuit(context(), &witness, case.left, case.right);
                if refuse_call <= expected_calls.len() {
                    let error = result
                        .err()
                        .expect("a selected witness refusal must be returned");
                    assert!(
                        matches!(error, CompactError::AssertionFailed(message) if message == REFUSAL)
                    );
                    assert_calls(&witness, &expected_calls[..refuse_call]);
                } else {
                    assert_success(
                        result.unwrap(),
                        &witness,
                        expected_value,
                        expected_calls,
                        expected_calls,
                    );
                }
            }
        }
    }
}

#[test]
fn negation_evaluates_each_operand_once_and_preserves_refusal() {
    for (input, expected) in [(false, true), (true, false)] {
        let witness = TracedEcho::new(None);
        assert_success(
            contract::witnessed_not(context(), &witness, input).unwrap(),
            &witness,
            expected,
            &[input],
            &[input],
        );
        let witness = TracedEcho::new(Some(1));
        let error = contract::witnessed_not(context(), &witness, input)
            .err()
            .expect("negation must return operand refusal");
        assert!(matches!(error, CompactError::AssertionFailed(message) if message == REFUSAL));
        assert_calls(&witness, &[input]);
    }
}

#[test]
fn branching_and_negation_use_witness_answers_not_arguments() {
    let witness = || TracedEcho {
        invert_answer: true,
        ..TracedEcho::new(None)
    };
    let w = witness();
    assert_success(
        contract::witnessed_both(context(), &w, false, true).unwrap(),
        &w,
        false,
        &[false, true],
        &[true, false],
    );
    let w = witness();
    assert_success(
        contract::witnessed_both(context(), &w, true, false).unwrap(),
        &w,
        false,
        &[true],
        &[false],
    );
    let w = witness();
    assert_success(
        contract::witnessed_either(context(), &w, false, true).unwrap(),
        &w,
        true,
        &[false],
        &[true],
    );
    let w = witness();
    assert_success(
        contract::witnessed_either(context(), &w, true, false).unwrap(),
        &w,
        true,
        &[true, false],
        &[false, true],
    );
    for (input, expected, output) in [(false, false, true), (true, true, false)] {
        let w = witness();
        assert_success(
            contract::witnessed_not(context(), &w, input).unwrap(),
            &w,
            expected,
            &[input],
            &[output],
        );
    }
}
