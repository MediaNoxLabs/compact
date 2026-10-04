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

use compact_rust_witness_vector_action_fixture::ledger_contract::{
    LedgerView, Witnesses, discardResult, initial_state, keepResult, recorded, reuseResult,
};
use midnight_compact_runtime::context::{CircuitResult, ConstructorContext, WitnessContext};
use midnight_compact_runtime::ledger::{ContractAddress, DefaultDB, StateValue};
use midnight_compact_runtime::{Field, FixedVector};
use midnight_onchain_state::state::{
    ContractMaintenanceAuthority, ContractOperation, ContractState, EntryPointBuf,
};
use midnight_serialize::tagged_serialize;
use midnight_storage::storage::HashMap;
use std::cell::Cell;

struct SumWitness {
    calls: Cell<usize>,
}

impl Witnesses<u64> for SumWitness {
    fn sumWitness(
        &self,
        context: WitnessContext<'_, u64, LedgerView<'_>>,
        values: FixedVector<Field, 2>,
    ) -> (u64, Field) {
        self.calls.set(self.calls.get() + 1);
        assert_eq!(*context.private_state, 7);
        assert_eq!(context.ledger.stored().unwrap(), Field::from(0_u64));
        assert_eq!(
            values,
            FixedVector::new([Field::from(0_u64), Field::from(1_u64)])
        );
        (8, Field::from(8_u64))
    }
}

fn state_hex(state: StateValue<DefaultDB>) -> String {
    let mut operations: HashMap<EntryPointBuf, ContractOperation, DefaultDB> = HashMap::new();
    for name in ["keepResult", "discardResult", "reuseResult"] {
        operations = operations.insert(
            EntryPointBuf(name.as_bytes().to_vec()),
            ContractOperation::new(None),
        );
    }
    let contract_state =
        ContractState::new(state, operations, ContractMaintenanceAuthority::default());
    let mut bytes = Vec::new();
    tagged_serialize(&contract_state, &mut bytes).unwrap();
    hex::encode(bytes)
}

fn assert_oracle(result: CircuitResult<u64, ()>, calls: usize, oracle: &serde_json::Value) {
    assert_eq!(calls, oracle["calls"].as_u64().unwrap() as usize);
    assert_eq!(result.context.private_state, oracle["privateState"]);
    assert_eq!(
        state_hex(result.context.query.state.get_ref().clone()),
        oracle["stateHex"]
    );
    let expected = oracle["privateTranscriptOutputs"].as_array().unwrap();
    assert_eq!(result.private_transcript_outputs.len(), expected.len());
    for (output, expected) in result.private_transcript_outputs.iter().zip(expected) {
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
}

#[test]
fn witness_vector_actions_evaluate_once_and_preserve_transcript() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/witness-vector-action.json"
    ))
    .unwrap();

    let keep_witness = SumWitness {
        calls: Cell::new(0),
    };
    let keep_context = initial_state(ConstructorContext::new(7_u64))
        .unwrap()
        .into_circuit_context(ContractAddress::default());
    let kept = keepResult(keep_context, &keep_witness).unwrap();
    assert_oracle(kept, keep_witness.calls.get(), &oracle["keepResult"]);

    let discard_witness = SumWitness {
        calls: Cell::new(0),
    };
    let discard_context = initial_state(ConstructorContext::new(7_u64))
        .unwrap()
        .into_circuit_context(ContractAddress::default());
    let discarded = discardResult(discard_context, &discard_witness).unwrap();
    assert_oracle(
        discarded,
        discard_witness.calls.get(),
        &oracle["discardResult"],
    );

    let reuse_witness = SumWitness {
        calls: Cell::new(0),
    };
    let reuse_context = initial_state(ConstructorContext::new(7_u64))
        .unwrap()
        .into_circuit_context(ContractAddress::default());
    let reused = reuseResult(reuse_context, &reuse_witness).unwrap();
    assert_oracle(reused, reuse_witness.calls.get(), &oracle["reuseResult"]);
}

#[test]
fn recorded_vector_let_witnesses_match_typescript_and_evaluate_once() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/witness-vector-action.json"
    ))
    .unwrap();
    for name in ["keepResult", "reuseResult"] {
        let native_witness = SumWitness {
            calls: Cell::new(0),
        };
        let recording_witness = SumWitness {
            calls: Cell::new(0),
        };
        let native_context = initial_state(ConstructorContext::new(7_u64))
            .unwrap()
            .into_circuit_context(ContractAddress::default());
        let recording_context = initial_state(ConstructorContext::new(7_u64))
            .unwrap()
            .into_circuit_context(ContractAddress::default());
        let native = match name {
            "keepResult" => keepResult(native_context, &native_witness).unwrap(),
            "reuseResult" => reuseResult(native_context, &native_witness).unwrap(),
            _ => unreachable!(),
        };
        let recorded = match name {
            "keepResult" => recorded::keepResult(recording_context, &recording_witness).unwrap(),
            "reuseResult" => recorded::reuseResult(recording_context, &recording_witness).unwrap(),
            _ => unreachable!(),
        };
        assert_eq!(
            native_witness.calls.get(),
            1,
            "{name}: native witness calls"
        );
        assert_eq!(
            recording_witness.calls.get(),
            1,
            "{name}: recorded witness calls"
        );
        assert_eq!(native.context.private_state, 8);
        assert_eq!(recorded.execution.context.private_state, 8);
        assert_eq!(native.gas_cost, recorded.execution.gas_cost);
        assert_eq!(
            native.context.query.state.get_ref(),
            recorded.execution.context.query.state.get_ref(),
        );
        assert_eq!(
            state_hex(recorded.execution.context.query.state.get_ref().clone()),
            oracle[name]["stateHex"],
        );
        let expected = &oracle[name]["privateTranscriptOutputs"];
        for outputs in [
            &native.private_transcript_outputs,
            &recorded.execution.private_transcript_outputs,
        ] {
            let actual = outputs
                .iter()
                .map(|output| {
                    serde_json::json!({
                        "valueAtoms": output.value.0.iter().map(|atom| &atom.0).collect::<Vec<_>>(),
                        "alignment": output.alignment,
                    })
                })
                .collect::<Vec<_>>();
            assert_eq!(serde_json::to_value(actual).unwrap(), *expected, "{name}");
        }
    }
}
