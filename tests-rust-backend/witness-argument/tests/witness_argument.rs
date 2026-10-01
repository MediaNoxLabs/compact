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

use compact_rust_witness_argument_fixture::ledger_contract::{
    LedgerView, Witnesses, apply_offset, initial_state,
};
use midnight_compact_runtime::Field;
use midnight_compact_runtime::context::{ConstructorContext, WitnessContext};
use midnight_compact_runtime::ledger::ContractAddress;

struct PrivateOffset;

impl Witnesses<u64> for PrivateOffset {
    fn private_offset(
        &self,
        context: WitnessContext<'_, u64, LedgerView<'_>>,
        value: Field,
    ) -> (u64, Field) {
        assert_eq!(*context.private_state, 7);
        assert_eq!(value, Field::from(2_u64));
        (*context.private_state + 1, value + Field::from(40_u64))
    }
}

#[test]
fn generated_witness_receives_typed_argument_and_records_result() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/witness-argument-ts-output.json"
    ))
    .unwrap();
    let context = initial_state(ConstructorContext::new(7_u64))
        .unwrap()
        .into_circuit_context(ContractAddress::default());
    let result = apply_offset(context, &PrivateOffset, Field::from(2_u64)).unwrap();
    let oracle_result: u64 = oracle["result"].as_str().unwrap().parse().unwrap();
    assert_eq!(result.result, Field::from(oracle_result));
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
