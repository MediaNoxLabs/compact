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

use compact_rust_call_arg_declared_type_fixture::ledger_contract::{
    LedgerView, Witnesses, bridgeTupleIntoVec, bridgeVecIntoTuple, commitFieldOnly, commitSmall,
    commitU128, hashPersistentVec, hashTransientVec, impureBare, impureConst, impureInIfArm,
    initial_state, inlinedAssert, pureBodyFieldOnly, pureBodyVec, pureFromImpure, witnessBare,
    witnessConst,
};
#[path = "../../boolean_observation_assertions.rs"]
mod boolean_observation_assertions;
use midnight_compact_runtime::context::{ConstructorContext, WitnessContext};
use midnight_compact_runtime::ledger::{ContractAddress, DefaultDB, StateValue};
use midnight_compact_runtime::{Field, FixedVector};
use midnight_onchain_state::state::{
    ContractMaintenanceAuthority, ContractOperation, ContractState, EntryPointBuf,
};
use midnight_serialize::tagged_serialize;
use midnight_storage::storage::HashMap;
use std::cell::RefCell;

const CIRCUITS: &[&str] = &[
    "commitSmall",
    "commitU128",
    "commitFieldOnly",
    "pureBodyVec",
    "pureBodyFieldOnly",
    "bridgeTupleIntoVec",
    "bridgeVecIntoTuple",
    "witnessConst",
    "witnessBare",
    "pureFromImpure",
    "impureConst",
    "impureBare",
    "impureInIfArm",
    "inlinedAssert",
    "hashPersistentVec",
    "hashTransientVec",
];

struct SumWitness;

impl Witnesses<()> for SumWitness {
    fn sumWitness(
        &self,
        _context: WitnessContext<'_, (), LedgerView<'_>>,
        values: FixedVector<Field, 2>,
    ) -> ((), Field) {
        ((), values.0[0] + values.0[1])
    }
}

#[derive(Default)]
struct AdvancingWitness {
    calls: RefCell<Vec<u64>>,
}

impl Witnesses<u64> for AdvancingWitness {
    fn sumWitness(
        &self,
        context: WitnessContext<'_, u64, LedgerView<'_>>,
        values: FixedVector<Field, 2>,
    ) -> (u64, Field) {
        assert_eq!(values.0, [Field::from(0_u64), Field::from(1_u64)]);
        self.calls.borrow_mut().push(*context.private_state);
        (*context.private_state + 1, values.0[0] + values.0[1])
    }
}

fn state_hex(state: StateValue<DefaultDB>) -> String {
    let mut operations: HashMap<EntryPointBuf, ContractOperation, DefaultDB> = HashMap::new();
    for name in CIRCUITS {
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

#[test]
fn declared_call_arguments_match_typescript_state_bytes() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/call-arg-declared-type.json"
    ))
    .unwrap();
    let captured = oracle["circuits"].as_object().unwrap();
    assert_eq!(captured.len(), CIRCUITS.len());
    for name in CIRCUITS {
        assert!(captured.contains_key(*name));
    }

    let initial = initial_state(ConstructorContext::new(())).unwrap();
    assert_eq!(
        state_hex(initial.ledger_state.get_ref().clone()),
        oracle["afterInit"]["stateHex"]
    );

    for name in CIRCUITS {
        let context = initial_state(ConstructorContext::new(()))
            .unwrap()
            .into_circuit_context(ContractAddress::default());
        let result = match *name {
            "commitSmall" => commitSmall(context).unwrap().context,
            "commitU128" => commitU128(context).unwrap().context,
            "commitFieldOnly" => commitFieldOnly(context).unwrap().context,
            "pureBodyVec" => pureBodyVec(context).unwrap().context,
            "pureBodyFieldOnly" => pureBodyFieldOnly(context).unwrap().context,
            "bridgeTupleIntoVec" => bridgeTupleIntoVec(context).unwrap().context,
            "bridgeVecIntoTuple" => bridgeVecIntoTuple(context).unwrap().context,
            "witnessConst" => witnessConst(context, &SumWitness).unwrap().context,
            "witnessBare" => witnessBare(context, &SumWitness).unwrap().context,
            "pureFromImpure" => pureFromImpure(context).unwrap().context,
            "impureConst" => impureConst(context).unwrap().context,
            "impureBare" => impureBare(context).unwrap().context,
            "impureInIfArm" => impureInIfArm(context).unwrap().context,
            "inlinedAssert" => inlinedAssert(context).unwrap().context,
            "hashPersistentVec" => hashPersistentVec(context).unwrap().context,
            "hashTransientVec" => hashTransientVec(context).unwrap().context,
            _ => unreachable!(),
        };
        assert_eq!(
            state_hex(result.query.state.get_ref().clone()),
            oracle["circuits"][name]["stateHex"],
            "{name}"
        );
    }
}

#[test]
fn recorded_persistent_commitments_match_typescript_trace_and_gas() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/call-arg-declared-type.json"
    ))
    .unwrap();
    for name in ["commitSmall", "commitU128", "commitFieldOnly"] {
        let native = initial_state(ConstructorContext::new(()))
            .unwrap()
            .into_circuit_context(ContractAddress::default());
        let recording = initial_state(ConstructorContext::new(()))
            .unwrap()
            .into_circuit_context(ContractAddress::default());
        let native = match name {
            "commitSmall" => commitSmall(native).unwrap(),
            "commitU128" => commitU128(native).unwrap(),
            "commitFieldOnly" => commitFieldOnly(native).unwrap(),
            _ => unreachable!(),
        };
        let recorded = match name {
            "commitSmall" => compact_rust_call_arg_declared_type_fixture::ledger_contract::recorded::commitSmall(recording).unwrap(),
            "commitU128" => compact_rust_call_arg_declared_type_fixture::ledger_contract::recorded::commitU128(recording).unwrap(),
            "commitFieldOnly" => compact_rust_call_arg_declared_type_fixture::ledger_contract::recorded::commitFieldOnly(recording).unwrap(),
            _ => unreachable!(),
        };
        boolean_observation_assertions::assert_ts_trace(
            name,
            &native,
            &recorded,
            &oracle["circuits"][name]["trace"],
        );
        assert_eq!(
            recorded.execution.context.query.state.get_ref(),
            native.context.query.state.get_ref(),
            "{name}: recorded state"
        );
        assert_eq!(
            state_hex(recorded.execution.context.query.state.get_ref().clone()),
            oracle["circuits"][name]["stateHex"],
            "{name}: TypeScript state"
        );
    }
}

#[test]
fn recorded_closed_pure_field_call_matches_typescript_trace_and_gas() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/call-arg-declared-type.json"
    ))
    .unwrap();
    let native_context = initial_state(ConstructorContext::new(()))
        .unwrap()
        .into_circuit_context(ContractAddress::default());
    let recording_context = initial_state(ConstructorContext::new(()))
        .unwrap()
        .into_circuit_context(ContractAddress::default());
    let native = pureBodyFieldOnly(native_context).unwrap();
    let recorded =
        compact_rust_call_arg_declared_type_fixture::ledger_contract::recorded::pureBodyFieldOnly(
            recording_context,
        )
        .unwrap();
    boolean_observation_assertions::assert_ts_trace(
        "pureBodyFieldOnly",
        &native,
        &recorded,
        &oracle["circuits"]["pureBodyFieldOnly"]["trace"],
    );
    assert_eq!(
        recorded.execution.context.query.state.get_ref(),
        native.context.query.state.get_ref(),
        "recorded state"
    );
    assert_eq!(
        state_hex(recorded.execution.context.query.state.get_ref().clone()),
        oracle["circuits"]["pureBodyFieldOnly"]["stateHex"],
        "TypeScript state"
    );
}

#[test]
fn recorded_vector_witness_let_matches_typescript_and_advances_private_state_once() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/call-arg-declared-type.json"
    ))
    .unwrap();
    let expected = &oracle["witnessConstAdvanced"];
    let native_witness = AdvancingWitness::default();
    let recorded_witness = AdvancingWitness::default();
    let native = witnessConst(
        initial_state(ConstructorContext::new(7_u64))
            .unwrap()
            .into_circuit_context(ContractAddress::default()),
        &native_witness,
    )
    .unwrap();
    let recorded =
        compact_rust_call_arg_declared_type_fixture::ledger_contract::recorded::witnessConst(
            initial_state(ConstructorContext::new(7_u64))
                .unwrap()
                .into_circuit_context(ContractAddress::default()),
            &recorded_witness,
        )
        .unwrap();
    assert_eq!(&*native_witness.calls.borrow(), &[7]);
    assert_eq!(&*recorded_witness.calls.borrow(), &[7]);
    assert_eq!(
        expected["witnessCalls"],
        serde_json::json!([{"privateState": 7, "values": ["0", "1"]}])
    );
    assert_eq!(native.context.private_state, 8);
    assert_eq!(recorded.execution.context.private_state, 8);
    assert_eq!(expected["privateState"], 8);
    boolean_observation_assertions::assert_ts_trace(
        "witnessConst",
        &native,
        &recorded,
        &expected["trace"],
    );
    assert_eq!(
        recorded.execution.context.query.state.get_ref(),
        native.context.query.state.get_ref(),
    );
    assert_eq!(
        state_hex(recorded.execution.context.query.state.get_ref().clone()),
        expected["stateHex"],
    );
    for output in [
        &native.private_transcript_outputs,
        &recorded.execution.private_transcript_outputs,
    ] {
        let actual = output
            .iter()
            .map(|item| {
                serde_json::json!({
                    "valueAtoms": item.value.0.iter().map(|atom| &atom.0).collect::<Vec<_>>(),
                    "alignment": item.alignment,
                })
            })
            .collect::<Vec<_>>();
        assert_eq!(
            serde_json::to_value(actual).unwrap(),
            expected["privateTranscriptOutputs"]
        );
    }
}
