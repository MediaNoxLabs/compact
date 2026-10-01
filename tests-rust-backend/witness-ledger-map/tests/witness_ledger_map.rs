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

use compact_rust_witness_ledger_map_fixture::ledger_contract::{
    LedgerView, Witnesses, initial_state, private_has, put_true,
};
use midnight_compact_runtime::Field;
use midnight_compact_runtime::context::{CircuitResult, ConstructorContext, WitnessContext};
use midnight_compact_runtime::ledger::ContractAddress;

struct HasTrue;

impl Witnesses<u64> for HasTrue {
    fn has_true(&self, context: WitnessContext<'_, u64, LedgerView<'_>>) -> (u64, bool) {
        let map = context.ledger.table().unwrap();
        let present = map.member(true);
        assert_eq!(map.is_empty(), map.size().unwrap().value() == 0);
        if present {
            assert_eq!(map.lookup(true).unwrap(), Field::from(42_u64));
        } else {
            assert!(map.lookup(true).is_err());
        }
        (*context.private_state + 1, present)
    }
}

fn assert_oracle_output(result: &CircuitResult<u64, bool>, oracle: &serde_json::Value) {
    assert_eq!(result.result, oracle["result"].as_bool().unwrap());
    assert_eq!(
        result.context.private_state,
        oracle["privateState"].as_u64().unwrap()
    );
    assert_eq!(result.private_transcript_outputs.len(), 1);
    let output = &result.private_transcript_outputs[0];
    let atoms = output
        .value
        .0
        .iter()
        .map(|atom| &atom.0)
        .collect::<Vec<_>>();
    assert_eq!(
        serde_json::to_value(atoms).unwrap(),
        oracle["privateTranscriptOutputs"][0]["valueAtoms"]
    );
    assert_eq!(
        serde_json::to_value(&output.alignment).unwrap(),
        oracle["privateTranscriptOutputs"][0]["alignment"]
    );
}

#[test]
fn witness_reads_current_typed_map() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/witness-ledger-map-ts-output.json"
    ))
    .unwrap();
    let context = initial_state(ConstructorContext::new(7_u64))
        .unwrap()
        .into_circuit_context(ContractAddress::default());
    let before = private_has(context, &HasTrue).unwrap();
    assert_oracle_output(&before, &oracle["before"]);
    let write = put_true(before.context, Field::from(42_u64)).unwrap();
    assert!(write.private_transcript_outputs.is_empty());
    let after = private_has(write.context, &HasTrue).unwrap();
    assert_oracle_output(&after, &oracle["after"]);
}
