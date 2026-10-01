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

use compact_rust_assert_witness_fixture::ledger_contract::{
    LedgerView, Witnesses, checked_value, checked_write, initial_state, read_cell,
};
use midnight_compact_runtime::Field;
use midnight_compact_runtime::context::{ConstructorContext, WitnessContext};
use midnight_compact_runtime::ledger::ContractAddress;
use std::cell::RefCell;

#[derive(Default)]
struct Echo {
    observed: RefCell<Vec<(u64, bool)>>,
}

impl Witnesses<u64> for Echo {
    fn echo(&self, context: WitnessContext<'_, u64, LedgerView<'_>>, value: bool) -> (u64, bool) {
        self.observed
            .borrow_mut()
            .push((*context.private_state, value));
        (*context.private_state + 1, value)
    }
}

fn observed_matches(witnesses: &Echo, oracle: &serde_json::Value) {
    let actual = witnesses
        .observed
        .borrow()
        .iter()
        .map(|(private_state, value)| {
            serde_json::json!({
                "privateState": private_state,
                "value": value,
            })
        })
        .collect::<Vec<_>>();
    assert_eq!(serde_json::to_value(actual).unwrap(), oracle["observed"]);
}

#[test]
fn witnessed_assertions_short_circuit_in_typescript_order() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/assert-witness.json"
    ))
    .unwrap();
    for (name, first, second) in [
        ("pass", true, true),
        ("firstFails", false, false),
        ("secondFails", true, false),
    ] {
        let witnesses = Echo::default();
        let context = initial_state(ConstructorContext::new(7_u64))
            .unwrap()
            .into_circuit_context(ContractAddress::default());
        let result = checked_value(context, &witnesses, first, second, Field::from(42_u64));
        let expected = &oracle[name];
        observed_matches(&witnesses, expected);
        if name == "pass" {
            let result = result.unwrap();
            assert_eq!(result.result, Field::from(42_u64));
            assert_eq!(result.context.private_state, expected["privateState"]);
            let outputs = expected["privateTranscriptOutputs"].as_array().unwrap();
            assert_eq!(result.private_transcript_outputs.len(), outputs.len());
            for (output, expected) in result.private_transcript_outputs.iter().zip(outputs) {
                let atoms = output
                    .value
                    .0
                    .iter()
                    .map(|atom| &atom.0)
                    .collect::<Vec<_>>();
                assert_eq!(serde_json::to_value(atoms).unwrap(), expected["valueAtoms"]);
                assert_eq!(
                    serde_json::to_value(&output.alignment).unwrap(),
                    expected["alignment"]
                );
            }
        } else {
            assert_eq!(result.err().unwrap().to_string(), expected["error"]);
        }
    }
}

#[test]
fn witnessed_assertion_guards_ledger_write() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/assert-witness.json"
    ))
    .unwrap();
    for (name, flag) in [("writePass", true), ("writeFails", false)] {
        let witnesses = Echo::default();
        let context = initial_state(ConstructorContext::new(7_u64))
            .unwrap()
            .into_circuit_context(ContractAddress::default());
        let result = checked_write(context, &witnesses, flag, Field::from(42_u64));
        let expected = &oracle[name];
        observed_matches(&witnesses, expected);
        if flag {
            let written = result.unwrap();
            assert_eq!(written.context.private_state, expected["privateState"]);
            let read = read_cell(written.context).unwrap();
            assert_eq!(read.result, Field::from(42_u64));
            assert_eq!(
                read.result,
                Field::from(expected["value"].as_str().unwrap().parse::<u64>().unwrap())
            );
        } else {
            assert_eq!(result.err().unwrap().to_string(), expected["error"]);
        }
    }
}
