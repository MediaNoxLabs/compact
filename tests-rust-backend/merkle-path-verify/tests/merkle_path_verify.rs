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

use compact_rust_merkle_path_verify_fixture::ledger_contract::recorded;
use compact_rust_merkle_path_verify_fixture::ledger_contract::{
    LedgerView, Witnesses, append, initial_state, replace, verify,
};
use compact_rust_merkle_path_verify_fixture::types::MerkleTreePath;
use midnight_compact_runtime as runtime;
use midnight_onchain_state::state::{
    ContractMaintenanceAuthority, ContractOperation, ContractState, EntryPointBuf,
};
use midnight_serialize::tagged_serialize;
use midnight_storage::storage::HashMap;
use runtime::context::{CircuitResult, ConstructorContext, WitnessContext};
use runtime::ledger::{ContractAddress, DefaultDB, StateValue};

struct PathWitness;

impl Witnesses<()> for PathWitness {
    fn leaf_path(&self, context: WitnessContext<'_, (), LedgerView<'_>>) -> ((), MerkleTreePath) {
        let path = context
            .ledger
            .t()
            .unwrap()
            .path_for_leaf(0, runtime::BoundedUint::<255>::new(7).unwrap())
            .unwrap();
        ((), MerkleTreePath::from_ledger_path(path).unwrap())
    }
}

fn assert_oracle_output(output: &CircuitResult<(), bool>, oracle: &serde_json::Value) {
    assert_eq!(output.result, oracle["result"]);
    assert_eq!(output.private_transcript_outputs.len(), 1);
    let transcript = &output.private_transcript_outputs[0];
    let atoms = transcript
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
        serde_json::to_value(&transcript.alignment).unwrap(),
        oracle["privateTranscriptOutputs"][0]["alignment"]
    );
}

fn state_hex(state: StateValue<DefaultDB>) -> String {
    let mut operations: HashMap<EntryPointBuf, ContractOperation, DefaultDB> = HashMap::new();
    for name in ["append", "replace", "verify"] {
        operations = operations.insert(
            EntryPointBuf(name.as_bytes().to_vec()),
            ContractOperation::new(None),
        );
    }
    let contract = ContractState::new(state, operations, ContractMaintenanceAuthority::default());
    let mut bytes = Vec::new();
    tagged_serialize(&contract, &mut bytes).unwrap();
    hex::encode(bytes)
}

fn context_after_append() -> runtime::context::CircuitContext<()> {
    let initial = initial_state(ConstructorContext::new(())).unwrap();
    let context = initial.into_circuit_context(ContractAddress::default());
    append(context, runtime::BoundedUint::<255>::new(7).unwrap())
        .unwrap()
        .context
}

fn context_after_replace() -> runtime::context::CircuitContext<()> {
    replace(
        context_after_append(),
        runtime::BoundedUint::<255>::new(8).unwrap(),
    )
    .unwrap()
    .context
}

#[test]
fn witness_merkle_root_check_matches_typescript_before_and_after_replacement() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/merkle-path-verify.json"
    ))
    .unwrap();
    let initial = initial_state(ConstructorContext::new(())).unwrap();
    let context = initial.into_circuit_context(ContractAddress::default());
    let after_append = append(context, runtime::BoundedUint::<255>::new(7).unwrap()).unwrap();
    let valid = verify(after_append.context, &PathWitness).unwrap();
    assert_oracle_output(&valid, &oracle["afterAppend"]);
    let after_replace =
        replace(valid.context, runtime::BoundedUint::<255>::new(8).unwrap()).unwrap();
    let invalid = verify(after_replace.context, &PathWitness).unwrap();
    assert_oracle_output(&invalid, &oracle["afterReplace"]);
}

#[test]
fn recorded_merkle_root_check_matches_native_typescript_and_replay() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/merkle-path-verify.json"
    ))
    .unwrap();
    for (label, expected) in [("afterAppend", true), ("afterReplace", false)] {
        let context = || {
            if expected {
                context_after_append()
            } else {
                context_after_replace()
            }
        };
        let expected_oracle = &oracle[label];
        let native = verify(context(), &PathWitness).unwrap();
        let recorded = recorded::verify(context(), &PathWitness).unwrap();
        assert_eq!(native.result, expected, "{label}");
        assert_oracle_output(&native, expected_oracle);
        assert_oracle_output(&recorded.execution, expected_oracle);
        assert_eq!(recorded.execution.gas_cost, native.gas_cost, "{label}");
        assert_eq!(
            recorded.execution.context.query.effects,
            native.context.query.effects
        );
        assert_eq!(
            state_hex(recorded.execution.context.query.state.get_ref().clone()),
            expected_oracle["state"],
            "{label}: ledger state"
        );
        assert_eq!(
            state_hex(native.context.query.state.get_ref().clone()),
            expected_oracle["state"],
            "{label}: native ledger state"
        );

        let queries = expected_oracle["queries"].as_array().unwrap();
        assert_eq!(queries.len(), 1, "{label}: one public query");
        let actual_gas = serde_json::to_value(recorded.execution.gas_cost).unwrap();
        for key in ["readTime", "computeTime", "bytesWritten", "bytesDeleted"] {
            let expected: u64 = queries[0]["gasCost"][key]
                .as_str()
                .unwrap()
                .parse()
                .unwrap();
            assert_eq!(
                actual_gas[key].as_u64().unwrap(),
                expected,
                "{label}: {key}"
            );
            assert_eq!(
                expected_oracle["reportedGas"][key],
                queries[0]["gasCost"][key]
            );
        }

        let mut public = serde_json::to_value(recorded.public.verify_ops()).unwrap();
        assert_eq!(
            public.as_array().unwrap().len(),
            7,
            "{label}: VM operation count"
        );
        let observed = &public[6]["popeq"]["result"];
        assert!(
            !observed.is_null(),
            "{label}: observed read result is pinned"
        );
        public[6]["popeq"]["result"] = serde_json::Value::Null;
        assert_eq!(
            public, queries[0]["program"],
            "{label}: ordered public VM program"
        );

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
            replay.gas_cost, recorded.execution.gas_cost,
            "{label}: replay gas"
        );
        assert_eq!(
            replay.context.effects,
            recorded.execution.context.query.effects
        );
        assert_eq!(
            state_hex(replay.context.state.get_ref().clone()),
            expected_oracle["state"],
            "{label}: replay ledger state"
        );
    }
    let wrong_slot = runtime::slots::MerkleSlot::<runtime::BoundedUint<255>, 3, false>::new(&[9]);
    assert!(
        wrong_slot
            .record_check_root(
                runtime::recording::RecordingFrame::new(context_after_append()),
                runtime::Field::from(0u64),
            )
            .is_err(),
        "an invalid ledger path must reject rather than record a synthetic false result"
    );
}
