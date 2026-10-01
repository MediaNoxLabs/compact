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
use midnight_compact_runtime::ledger::{ContractAddress, read_root_cell};
use midnight_compact_runtime::recording::RecordingFrame;

struct Secret;

impl Witnesses<u64> for Secret {
    fn secret(
        &self,
        context: WitnessContext<'_, u64, LedgerView<'_>>,
        seed: Field,
    ) -> (u64, Field) {
        assert_eq!(seed, Field::from(2_u64));
        secret_logic(*context.private_state, context.ledger.cell().unwrap(), seed)
    }
}

fn secret_logic(private_state: u64, current_cell: Field, seed: Field) -> (u64, Field) {
    let expected_cell = if private_state == 7 { 0 } else { 9 };
    assert_eq!(current_cell, Field::from(expected_cell));
    (private_state + 1, seed + Field::from(private_state))
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

#[test]
fn witnessed_writes_record_private_values_and_public_operations_in_order() {
    let seed = Field::from(2_u64);
    let native = write_twice(
        initial_state(ConstructorContext::new(7_u64))
            .unwrap()
            .into_circuit_context(ContractAddress::default()),
        &Secret,
        seed,
    )
    .unwrap();
    let context = initial_state(ConstructorContext::new(7_u64))
        .unwrap()
        .into_circuit_context(ContractAddress::default());
    let frame = RecordingFrame::new(context);
    let (frame, first) = frame.witness(|context| {
        let cell = read_root_cell::<Field, _>(context.query.state.get_ref(), 0).unwrap();
        secret_logic(context.private_state, cell, seed)
    });
    let frame = frame.write_cell(0_u8, first).unwrap();
    let (frame, second) = frame.witness(|context| {
        let cell = read_root_cell::<Field, _>(context.query.state.get_ref(), 0).unwrap();
        secret_logic(context.private_state, cell, seed)
    });
    let recorded = frame.write_cell(0_u8, second).unwrap().finish(());

    assert_eq!(
        recorded.execution.context.private_state,
        native.context.private_state
    );
    assert_eq!(
        recorded.execution.private_transcript_outputs,
        native.private_transcript_outputs
    );
    assert_eq!(recorded.execution.private_transcript_outputs.len(), 2);
    assert_eq!(recorded.public.verify_ops().len(), 8);
    let replay = recorded
        .public
        .initial()
        .query(
            recorded.public.verify_ops(),
            None,
            &recorded.execution.context.cost_model,
        )
        .unwrap();
    assert_eq!(
        read_root_cell::<Field, _>(native.context.query.state.get_ref(), 0).unwrap(),
        read_root_cell::<Field, _>(replay.context.state.get_ref(), 0).unwrap()
    );
    assert_eq!(
        recorded.execution.context.query.effects,
        replay.context.effects
    );
}
