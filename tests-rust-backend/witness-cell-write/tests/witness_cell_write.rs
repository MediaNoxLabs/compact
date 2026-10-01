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

use compact_rust_witness_cell_write_fixture::ledger_contract::{
    LedgerView, Witnesses, initial_state, read_cell, write_secret, write_twice,
};
use midnight_compact_runtime::Field;
use midnight_compact_runtime::context::{CircuitResult, ConstructorContext, WitnessContext};
use midnight_compact_runtime::ledger::ContractAddress;

struct Secret;

impl Witnesses<u64> for Secret {
    fn secret(
        &self,
        context: WitnessContext<'_, u64, LedgerView<'_>>,
        seed: Field,
    ) -> (u64, Field) {
        assert_eq!(seed, Field::from(2_u64));
        let expected_cell = if *context.private_state == 7 { 0 } else { 9 };
        assert_eq!(context.ledger.cell().unwrap(), Field::from(expected_cell));
        (
            *context.private_state + 1,
            seed + Field::from(*context.private_state),
        )
    }
}

fn assert_oracle_output(write: CircuitResult<u64, ()>, oracle: &serde_json::Value) {
    assert_eq!(
        write.context.private_state,
        oracle["privateState"].as_u64().unwrap()
    );
    let expected_outputs = oracle["privateTranscriptOutputs"].as_array().unwrap();
    assert_eq!(
        write.private_transcript_outputs.len(),
        expected_outputs.len()
    );
    for (output, expected) in write
        .private_transcript_outputs
        .iter()
        .zip(expected_outputs)
    {
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
    let read = read_cell(write.context).unwrap();
    let expected_cell: u64 = oracle["cell"].as_str().unwrap().parse().unwrap();
    assert_eq!(read.result, Field::from(expected_cell));
}

#[test]
fn witnessed_cell_writes_keep_ledger_and_private_effects_in_order() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/witness-cell-write-ts-output.json"
    ))
    .unwrap();
    let context = initial_state(ConstructorContext::new(7_u64))
        .unwrap()
        .into_circuit_context(ContractAddress::default());
    let single = write_secret(context, &Secret, Field::from(2_u64)).unwrap();
    assert_oracle_output(single, &oracle["single"]);

    let context = initial_state(ConstructorContext::new(7_u64))
        .unwrap()
        .into_circuit_context(ContractAddress::default());
    let twice = write_twice(context, &Secret, Field::from(2_u64)).unwrap();
    assert_oracle_output(twice, &oracle["twice"]);
}
