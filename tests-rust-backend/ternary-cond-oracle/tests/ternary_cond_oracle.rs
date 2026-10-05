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

use compact_rust_ternary_cond_oracle_fixture::ledger_contract::{
    LedgerView, Witnesses, initial_state, recorded, streamAssertEq, streamCallPure,
    streamCallWitness, streamCompareEq, streamIncrement, streamNativeArg, streamNestedIf,
    streamStructMember, streamVectorElement, streamWrite, walkerCallPure, walkerCompareEq,
    walkerConstAnnotated, walkerInlineWrite, walkerNativeArg, walkerNestedIf, walkerStructMember,
    walkerWrite, witnessArg,
};
use compact_rust_ternary_cond_oracle_fixture::ledger_slots;
#[path = "../../boolean_observation_assertions.rs"]
mod boolean_observation_assertions;
use compact_rust_ternary_cond_oracle_fixture::pure_circuits::{
    constAnnotatedBothLiteral, constUnannotatedSeqLifted, enumValued, returnTailNested,
};
use compact_rust_ternary_cond_oracle_fixture::types::Color;
use midnight_compact_runtime::context::{ConstructorContext, WitnessContext};
use midnight_compact_runtime::ledger::{ContractAddress, DefaultDB, StateValue};
use midnight_compact_runtime::{BoundedUint, Field};
use midnight_onchain_state::state::{
    ContractMaintenanceAuthority, ContractOperation, ContractState, EntryPointBuf,
};
use midnight_serialize::tagged_serialize;
use midnight_storage::storage::HashMap;
use std::cell::RefCell;

#[derive(Default)]
struct Echo {
    calls: RefCell<Vec<(u64, String)>>,
}

impl Witnesses<u64> for Echo {
    fn echoField(
        &self,
        context: WitnessContext<'_, u64, LedgerView<'_>>,
        value: Field,
    ) -> (u64, Field) {
        let decimal = if value == Field::from(1_u64) {
            "1"
        } else if value == Field::from(2_u64) {
            "2"
        } else {
            panic!("unexpected conditional Field witness argument")
        };
        self.calls
            .borrow_mut()
            .push((*context.private_state, decimal.to_owned()));
        (*context.private_state + 1, value)
    }
}

const EXPORTED: &[&str] = &[
    "walkerConstAnnotated",
    "walkerCompareEq",
    "walkerCallPure",
    "walkerStructMember",
    "walkerWrite",
    "witnessArg",
    "streamIncrement",
    "streamCompareEq",
    "streamWrite",
    "walkerVectorElement",
    "walkerNativeArg",
    "walkerNestedIf",
    "walkerInlineWrite",
    "streamCallPure",
    "streamVectorElement",
    "streamNativeArg",
    "streamStructMember",
    "streamCallWitness",
    "streamConstAnnotated",
    "streamAssertEq",
    "streamNestedIf",
];

fn state_hex(state: StateValue<DefaultDB>) -> String {
    let mut operations: HashMap<EntryPointBuf, ContractOperation, DefaultDB> = HashMap::new();
    for name in EXPORTED {
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

fn oracle() -> serde_json::Value {
    serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/ternary-cond-oracle.json"
    ))
    .unwrap()
}

fn ordered_vm_shape(operations: serde_json::Value) -> serde_json::Value {
    serde_json::Value::Array(
        operations
            .as_array()
            .unwrap()
            .iter()
            .map(|operation| {
                if let Some(kind) = operation.as_str() {
                    return serde_json::json!({"kind":kind});
                }
                if let Some(idx) = operation.get("idx") {
                    serde_json::json!({
                        "kind":"idx", "cached":idx["cached"], "pushPath":idx["pushPath"],
                        "pathLength":idx["path"].as_array().unwrap().len(),
                    })
                } else if let Some(push) = operation.get("push") {
                    serde_json::json!({"kind":"push", "storage":push["storage"]})
                } else if let Some(ins) = operation.get("ins") {
                    serde_json::json!({"kind":"ins", "cached":ins["cached"], "n":ins["n"]})
                } else if let Some(rem) = operation.get("rem") {
                    serde_json::json!({"kind":"rem", "cached":rem["cached"]})
                } else if let Some(dup) = operation.get("dup") {
                    serde_json::json!({"kind":"dup", "n":dup["n"]})
                } else if let Some(popeq) = operation.get("popeq") {
                    serde_json::json!({
                        "kind":"popeq", "cached":popeq["cached"],
                        "resultAtoms":popeq["result"]["value"],
                    })
                } else if let Some(branch) = operation.get("branch") {
                    serde_json::json!({"kind":"branch", "skip":branch["skip"]})
                } else if let Some(swap) = operation.get("swap") {
                    serde_json::json!({"kind":"swap", "n":swap["n"]})
                } else if let Some(concat) = operation.get("concat") {
                    serde_json::json!({"kind":"concat", "cached":concat["cached"], "n":concat["n"]})
                } else if let Some(jmp) = operation.get("jmp") {
                    serde_json::json!({"kind":"jmp", "skip":jmp["skip"]})
                } else if let Some(addi) = operation.get("addi") {
                    serde_json::json!({"kind":"addi", "immediate":addi["immediate"]})
                } else {
                    panic!("unexpected VM operation: {operation}");
                }
            })
            .collect(),
    )
}

#[test]
fn recorded_nested_uint4_matches_typescript_all_branches_and_replays() {
    let reference: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/ternary-recorded-nested-if.json"
    ))
    .unwrap();
    for (case, args, seed_flag) in [
        ("walkerTT", Some((true, true)), false),
        ("walkerTF", Some((true, false)), false),
        ("walkerFT", Some((false, true)), false),
        ("walkerFF", Some((false, false)), false),
        ("streamFalse", None, false),
        ("streamTrue", None, true),
    ] {
        let expected = &reference[case];
        let native_initial = initial(true, true, 111);
        let recorded_initial = initial(true, true, 111);
        let native_context = native_initial.into_circuit_context(ContractAddress::default());
        let recorded_context = recorded_initial.into_circuit_context(ContractAddress::default());
        let (native_context, recorded_context) = if seed_flag {
            (
                ledger_slots::flag
                    .write(native_context, true)
                    .unwrap()
                    .context,
                ledger_slots::flag
                    .write(recorded_context, true)
                    .unwrap()
                    .context,
            )
        } else {
            (native_context, recorded_context)
        };
        assert_eq!(
            state_hex(native_context.query.state.get_ref().clone()),
            expected["initialStateHex"],
            "{case}: initial state",
        );
        let (native, recorded) = if let Some((c, d)) = args {
            (
                walkerNestedIf(native_context, c, d).unwrap(),
                recorded::walkerNestedIf(recorded_context, c, d).unwrap(),
            )
        } else {
            (
                streamNestedIf(native_context).unwrap(),
                recorded::streamNestedIf(recorded_context).unwrap(),
            )
        };
        let _: () = native.result;
        let _: () = recorded.execution.result;
        assert_eq!(expected["result"], "", "{case}: TypeScript result");
        assert_eq!(native.gas_cost, recorded.execution.gas_cost, "{case}: gas");
        assert_eq!(
            native.context.query.effects, recorded.execution.context.query.effects,
            "{case}: effects"
        );
        assert_eq!(
            native.context.query.state.get_ref(),
            recorded.execution.context.query.state.get_ref(),
            "{case}: state"
        );
        assert_eq!(
            state_hex(recorded.execution.context.query.state.get_ref().clone()),
            expected["afterStateHex"],
            "{case}: TypeScript state"
        );
        assert!(
            native.private_transcript_outputs.is_empty(),
            "{case}: native private transcript"
        );
        assert!(
            recorded.execution.private_transcript_outputs.is_empty(),
            "{case}: recorded private transcript"
        );
        assert_eq!(
            expected["privateTranscriptCount"], 0,
            "{case}: TypeScript private transcript"
        );
        assert_eq!(
            ordered_vm_shape(serde_json::to_value(recorded.public.verify_ops()).unwrap()),
            expected["publicTranscriptShape"],
            "{case}: ordered VM",
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
            replay.context.state.get_ref(),
            native.context.query.state.get_ref(),
            "{case}: replay state"
        );
        assert_eq!(
            replay.context.effects, native.context.query.effects,
            "{case}: replay effects"
        );
        let actual = serde_json::to_value(native.gas_cost).unwrap();
        for dimension in ["readTime", "computeTime", "bytesWritten", "bytesDeleted"] {
            let expected_gas: u64 = expected["queries"]
                .as_array()
                .unwrap()
                .iter()
                .map(|query| {
                    query["gasCost"][dimension]
                        .as_str()
                        .unwrap()
                        .parse::<u64>()
                        .unwrap()
                })
                .sum();
            assert_eq!(actual[dimension], expected_gas, "{case}: {dimension}");
            assert_eq!(
                expected["queries"].as_array().unwrap().last().unwrap()["gasCost"][dimension],
                expected["reportedGas"][dimension],
                "{case}: final TypeScript query {dimension}"
            );
        }
    }
}

#[test]
fn recorded_conditional_field_vectors_match_typescript_and_replay() {
    let reference: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/ternary-recorded-vector-element.json"
    ))
    .unwrap();
    for (case, seed_flag) in [("streamFalse", false), ("streamTrue", true)] {
        let expected = &reference[case];
        let native_initial = initial(true, true, 111);
        let recorded_initial = initial(true, true, 111);
        let native_context = native_initial.into_circuit_context(ContractAddress::default());
        let recorded_context = recorded_initial.into_circuit_context(ContractAddress::default());
        let (native_context, recorded_context) = if seed_flag {
            (
                ledger_slots::flag
                    .write(native_context, true)
                    .unwrap()
                    .context,
                ledger_slots::flag
                    .write(recorded_context, true)
                    .unwrap()
                    .context,
            )
        } else {
            (native_context, recorded_context)
        };
        assert_eq!(
            state_hex(native_context.query.state.get_ref().clone()),
            expected["initialStateHex"],
            "{case}: initial state",
        );
        let native = streamVectorElement(native_context).unwrap();
        let recorded = recorded::streamVectorElement(recorded_context).unwrap();
        let _: () = native.result;
        let _: () = recorded.execution.result;
        assert_eq!(expected["result"], "", "{case}: TypeScript result");
        assert_eq!(native.gas_cost, recorded.execution.gas_cost, "{case}: gas");
        assert_eq!(
            native.context.query.effects, recorded.execution.context.query.effects,
            "{case}: effects"
        );
        assert_eq!(
            native.context.query.state.get_ref(),
            recorded.execution.context.query.state.get_ref(),
            "{case}: state"
        );
        assert_eq!(
            state_hex(recorded.execution.context.query.state.get_ref().clone()),
            expected["afterStateHex"],
            "{case}: TypeScript state"
        );
        assert!(native.private_transcript_outputs.is_empty());
        assert!(recorded.execution.private_transcript_outputs.is_empty());
        assert_eq!(expected["privateTranscriptCount"], 0);
        assert_eq!(
            ordered_vm_shape(serde_json::to_value(recorded.public.verify_ops()).unwrap()),
            expected["publicTranscriptShape"],
            "{case}: ordered VM",
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
            replay.context.state.get_ref(),
            native.context.query.state.get_ref()
        );
        assert_eq!(replay.context.effects, native.context.query.effects);
        let actual = serde_json::to_value(native.gas_cost).unwrap();
        for dimension in ["readTime", "computeTime", "bytesWritten", "bytesDeleted"] {
            let expected_gas: u64 = expected["queries"]
                .as_array()
                .unwrap()
                .iter()
                .map(|query| {
                    query["gasCost"][dimension]
                        .as_str()
                        .unwrap()
                        .parse::<u64>()
                        .unwrap()
                })
                .sum();
            assert_eq!(actual[dimension], expected_gas, "{case}: {dimension}");
            assert_eq!(
                expected["queries"].as_array().unwrap().last().unwrap()["gasCost"][dimension],
                expected["reportedGas"][dimension],
                "{case}: final TypeScript query {dimension}"
            );
        }
    }
}

#[test]
fn recorded_annotated_uint8_matches_typescript_and_replay() {
    let reference: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/ternary-recorded-annotated-uint8.json"
    ))
    .unwrap();
    for (case, c) in [("walkerTrue", true), ("walkerFalse", false)] {
        let expected = &reference[case];
        let native_initial = initial(true, true, 111);
        let recorded_initial = initial(true, true, 111);
        assert_eq!(
            state_hex(native_initial.ledger_state.get_ref().clone()),
            expected["initialStateHex"],
            "{case}: initial state",
        );
        let native = walkerConstAnnotated(
            native_initial.into_circuit_context(ContractAddress::default()),
            c,
        )
        .unwrap();
        let recorded = recorded::walkerConstAnnotated(
            recorded_initial.into_circuit_context(ContractAddress::default()),
            c,
        )
        .unwrap();
        let _: () = native.result;
        let _: () = recorded.execution.result;
        assert_eq!(expected["result"], "", "{case}: TypeScript result");
        assert_eq!(native.gas_cost, recorded.execution.gas_cost, "{case}: gas");
        assert_eq!(
            native.context.query.effects, recorded.execution.context.query.effects,
            "{case}: effects"
        );
        assert_eq!(
            native.context.query.state.get_ref(),
            recorded.execution.context.query.state.get_ref(),
            "{case}: state"
        );
        assert_eq!(
            state_hex(recorded.execution.context.query.state.get_ref().clone()),
            expected["afterStateHex"],
            "{case}: TypeScript state"
        );
        assert!(native.private_transcript_outputs.is_empty());
        assert!(recorded.execution.private_transcript_outputs.is_empty());
        assert_eq!(expected["privateTranscriptCount"], 0);
        assert_eq!(
            ordered_vm_shape(serde_json::to_value(recorded.public.verify_ops()).unwrap()),
            expected["publicTranscriptShape"],
            "{case}: ordered VM",
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
            replay.context.state.get_ref(),
            native.context.query.state.get_ref()
        );
        assert_eq!(replay.context.effects, native.context.query.effects);
        let actual = serde_json::to_value(native.gas_cost).unwrap();
        for dimension in ["readTime", "computeTime", "bytesWritten", "bytesDeleted"] {
            let expected_gas: u64 = expected["queries"]
                .as_array()
                .unwrap()
                .iter()
                .map(|query| {
                    query["gasCost"][dimension]
                        .as_str()
                        .unwrap()
                        .parse::<u64>()
                        .unwrap()
                })
                .sum();
            assert_eq!(actual[dimension], expected_gas, "{case}: {dimension}");
            assert_eq!(
                expected["queries"].as_array().unwrap().last().unwrap()["gasCost"][dimension],
                expected["reportedGas"][dimension],
                "{case}: final TypeScript query {dimension}"
            );
        }
    }
}

#[test]
fn recorded_closed_curve_arguments_match_typescript_and_replay() {
    let reference: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/ternary-recorded-native-arg.json"
    ))
    .unwrap();
    for (case, args, seed_flag) in [
        ("walkerTrue", Some(true), false),
        ("walkerFalse", Some(false), false),
        ("streamFalse", None, false),
        ("streamTrue", None, true),
    ] {
        let expected = &reference[case];
        let native_initial = initial(true, true, 111);
        let recorded_initial = initial(true, true, 111);
        let native_context = native_initial.into_circuit_context(ContractAddress::default());
        let recorded_context = recorded_initial.into_circuit_context(ContractAddress::default());
        let (native_context, recorded_context) = if seed_flag {
            (
                ledger_slots::flag
                    .write(native_context, true)
                    .unwrap()
                    .context,
                ledger_slots::flag
                    .write(recorded_context, true)
                    .unwrap()
                    .context,
            )
        } else {
            (native_context, recorded_context)
        };
        assert_eq!(
            state_hex(native_context.query.state.get_ref().clone()),
            expected["initialStateHex"],
            "{case}: initial state",
        );
        let (native, recorded) = if let Some(c) = args {
            (
                walkerNativeArg(native_context, c).unwrap(),
                recorded::walkerNativeArg(recorded_context, c).unwrap(),
            )
        } else {
            (
                streamNativeArg(native_context).unwrap(),
                recorded::streamNativeArg(recorded_context).unwrap(),
            )
        };
        let _: () = native.result;
        let _: () = recorded.execution.result;
        assert_eq!(expected["result"], "", "{case}: TypeScript result");
        assert_eq!(native.gas_cost, recorded.execution.gas_cost, "{case}: gas");
        assert_eq!(
            native.context.query.effects, recorded.execution.context.query.effects,
            "{case}: effects"
        );
        assert_eq!(
            native.context.query.state.get_ref(),
            recorded.execution.context.query.state.get_ref(),
            "{case}: state"
        );
        assert_eq!(
            state_hex(recorded.execution.context.query.state.get_ref().clone()),
            expected["afterStateHex"],
            "{case}: TypeScript state"
        );
        assert!(
            native.private_transcript_outputs.is_empty(),
            "{case}: native private transcript"
        );
        assert!(
            recorded.execution.private_transcript_outputs.is_empty(),
            "{case}: recorded private transcript"
        );
        assert_eq!(
            expected["privateTranscriptCount"], 0,
            "{case}: TypeScript private transcript"
        );
        assert_eq!(
            ordered_vm_shape(serde_json::to_value(recorded.public.verify_ops()).unwrap()),
            expected["publicTranscriptShape"],
            "{case}: ordered VM",
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
            replay.context.state.get_ref(),
            native.context.query.state.get_ref(),
            "{case}: replay state"
        );
        assert_eq!(
            replay.context.effects, native.context.query.effects,
            "{case}: replay effects"
        );
        let actual = serde_json::to_value(native.gas_cost).unwrap();
        for dimension in ["readTime", "computeTime", "bytesWritten", "bytesDeleted"] {
            let expected_gas: u64 = expected["queries"]
                .as_array()
                .unwrap()
                .iter()
                .map(|query| {
                    query["gasCost"][dimension]
                        .as_str()
                        .unwrap()
                        .parse::<u64>()
                        .unwrap()
                })
                .sum();
            assert_eq!(actual[dimension], expected_gas, "{case}: {dimension}");
            assert_eq!(
                expected["queries"].as_array().unwrap().last().unwrap()["gasCost"][dimension],
                expected["reportedGas"][dimension],
                "{case}: final TypeScript query {dimension}"
            );
        }
    }
}

#[test]
fn recorded_closed_unsigned_ternary_comparisons_match_typescript() {
    let reference: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/ternary-recorded-comparison.json"
    ))
    .unwrap();
    for case in ["walkerTrue", "walkerFalse", "streamFalse"] {
        let expected = &reference[case];
        let native_initial = initial(true, true, 111);
        let recorded_initial = initial(true, true, 111);
        assert_eq!(
            state_hex(native_initial.ledger_state.get_ref().clone()),
            expected["initialStateHex"],
            "{case}: initial state",
        );
        let native_context = native_initial.into_circuit_context(ContractAddress::default());
        let recorded_context = recorded_initial.into_circuit_context(ContractAddress::default());
        let (native, recorded) = match case {
            "walkerTrue" | "walkerFalse" => {
                let condition = case == "walkerTrue";
                let x = BoundedUint::<255>::new(1).unwrap();
                (
                    walkerCompareEq(native_context, condition, x).unwrap(),
                    recorded::walkerCompareEq(recorded_context, condition, x).unwrap(),
                )
            }
            "streamFalse" => (
                streamCompareEq(native_context).unwrap(),
                recorded::streamCompareEq(recorded_context).unwrap(),
            ),
            _ => unreachable!(),
        };
        let _: () = native.result;
        let _: () = recorded.execution.result;
        assert_eq!(expected["result"], "", "{case}: TypeScript result");
        assert_eq!(native.gas_cost, recorded.execution.gas_cost, "{case}: gas");
        assert_eq!(
            native.context.query.effects, recorded.execution.context.query.effects,
            "{case}: effects",
        );
        assert_eq!(
            native.context.query.state.get_ref(),
            recorded.execution.context.query.state.get_ref(),
            "{case}: state",
        );
        assert_eq!(
            state_hex(recorded.execution.context.query.state.get_ref().clone()),
            expected["afterStateHex"],
            "{case}: TypeScript state",
        );
        assert_eq!(native.private_transcript_outputs.len(), 0);
        assert_eq!(recorded.execution.private_transcript_outputs.len(), 0);
        assert_eq!(expected["privateTranscriptCount"], 0);
        assert_eq!(
            ordered_vm_shape(serde_json::to_value(recorded.public.verify_ops()).unwrap()),
            expected["publicTranscriptShape"],
            "{case}: ordered VM",
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
            replay.context.state.get_ref(),
            native.context.query.state.get_ref(),
            "{case}: replay state",
        );
        assert_eq!(replay.context.effects, native.context.query.effects);
        let actual = serde_json::to_value(native.gas_cost).unwrap();
        for dimension in ["readTime", "computeTime", "bytesWritten", "bytesDeleted"] {
            let expected_gas: u64 = expected["queries"]
                .as_array()
                .unwrap()
                .iter()
                .map(|query| {
                    query["gasCost"][dimension]
                        .as_str()
                        .unwrap()
                        .parse::<u64>()
                        .unwrap()
                })
                .sum();
            assert_eq!(actual[dimension], expected_gas, "{case}: {dimension}");
            assert_eq!(
                expected["queries"].as_array().unwrap().last().unwrap()["gasCost"][dimension],
                expected["reportedGas"][dimension],
                "{case}: TypeScript reported {dimension}",
            );
        }
    }
}

#[test]
fn recorded_streaming_comparison_and_struct_true_paths_match_typescript() {
    let capture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/adr221-ternary-true.json"
    ))
    .unwrap();
    assert_eq!(capture["mode"], "ternary");
    let rows = capture["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 2);
    for row in rows {
        let name = row["name"].as_str().unwrap();
        let native = ledger_slots::flag
            .write(
                initial(true, true, 111).into_circuit_context(ContractAddress::default()),
                true,
            )
            .unwrap()
            .context;
        let recorded = ledger_slots::flag
            .write(
                initial(true, true, 111).into_circuit_context(ContractAddress::default()),
                true,
            )
            .unwrap()
            .context;
        assert_eq!(
            state_hex(native.query.state.get_ref().clone()),
            row["before"],
            "{name}: seeded state"
        );
        let (native, recorded) = match name {
            "streamCompareEq" => (
                streamCompareEq(native).unwrap(),
                recorded::streamCompareEq(recorded).unwrap(),
            ),
            "streamStructMember" => (
                streamStructMember(native).unwrap(),
                recorded::streamStructMember(recorded).unwrap(),
            ),
            _ => panic!("unexpected ADR221 export {name}"),
        };
        assert_eq!(row["result"], "");
        let _: () = native.result;
        let _: () = recorded.execution.result;
        assert_eq!(native.gas_cost, recorded.execution.gas_cost, "{name}: gas");
        assert_eq!(
            native.context.query.state, recorded.execution.context.query.state,
            "{name}: state"
        );
        assert_eq!(
            native.context.query.effects, recorded.execution.context.query.effects,
            "{name}: effects"
        );
        assert_eq!(
            serde_json::to_value(&native.context.query.effects).unwrap(),
            row["effects"],
            "{name}: TypeScript effects"
        );
        assert_eq!(
            state_hex(native.context.query.state.get_ref().clone()),
            row["after"],
            "{name}: TS state"
        );
        assert_eq!(
            ledger_slots::fieldCell
                .inspect(native.context.query.state.get_ref())
                .unwrap(),
            Field::from(1_u64),
            "{name}: selected true arm"
        );
        assert!(native.private_transcript_outputs.is_empty());
        assert!(recorded.execution.private_transcript_outputs.is_empty());
        assert_eq!(row["privateTranscriptCount"], 0);
        assert_eq!(
            ordered_vm_shape(serde_json::to_value(recorded.public.verify_ops()).unwrap()),
            row["publicTranscriptShape"],
            "{name}: ordered TS VM"
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
            replay.context.state, native.context.query.state,
            "{name}: replay state"
        );
        assert_eq!(
            replay.context.effects, native.context.query.effects,
            "{name}: replay effects"
        );
        let actual = serde_json::to_value(native.gas_cost).unwrap();
        for dimension in ["readTime", "computeTime", "bytesWritten", "bytesDeleted"] {
            let expected = row["queryCostSum"][dimension]
                .as_str()
                .unwrap()
                .parse::<u64>()
                .unwrap();
            assert_eq!(actual[dimension], expected, "{name}: {dimension}");
            assert_eq!(
                row["queries"].as_array().unwrap().last().unwrap()["gasCost"][dimension],
                row["reportedGas"][dimension],
                "{name}: last reported query"
            );
        }
    }
}

#[test]
fn recorded_closed_ternary_struct_members_match_typescript() {
    let reference: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/ternary-recorded-struct-member.json"
    ))
    .unwrap();
    for case in ["walkerTrue", "walkerFalse", "streamFalse"] {
        let expected = &reference[case];
        let native_initial = initial(true, true, 111);
        let recorded_initial = initial(true, true, 111);
        assert_eq!(
            state_hex(native_initial.ledger_state.get_ref().clone()),
            expected["initialStateHex"],
            "{case}: initial state",
        );
        let native_context = native_initial.into_circuit_context(ContractAddress::default());
        let recorded_context = recorded_initial.into_circuit_context(ContractAddress::default());
        let (native, recorded) = match case {
            "walkerTrue" | "walkerFalse" => {
                let condition = case == "walkerTrue";
                (
                    walkerStructMember(native_context, condition).unwrap(),
                    recorded::walkerStructMember(recorded_context, condition).unwrap(),
                )
            }
            "streamFalse" => (
                streamStructMember(native_context).unwrap(),
                recorded::streamStructMember(recorded_context).unwrap(),
            ),
            _ => unreachable!(),
        };
        let _: () = native.result;
        let _: () = recorded.execution.result;
        assert_eq!(expected["result"], "", "{case}: TypeScript result");
        assert_eq!(native.gas_cost, recorded.execution.gas_cost, "{case}: gas");
        assert_eq!(
            native.context.query.effects, recorded.execution.context.query.effects,
            "{case}: effects",
        );
        assert_eq!(
            native.context.query.state.get_ref(),
            recorded.execution.context.query.state.get_ref(),
            "{case}: state",
        );
        assert_eq!(
            state_hex(recorded.execution.context.query.state.get_ref().clone()),
            expected["afterStateHex"],
            "{case}: TypeScript state",
        );
        assert_eq!(native.private_transcript_outputs.len(), 0);
        assert_eq!(recorded.execution.private_transcript_outputs.len(), 0);
        assert_eq!(expected["privateTranscriptCount"], 0);
        assert_eq!(
            ordered_vm_shape(serde_json::to_value(recorded.public.verify_ops()).unwrap()),
            expected["publicTranscriptShape"],
            "{case}: ordered VM",
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
            replay.context.state.get_ref(),
            native.context.query.state.get_ref(),
            "{case}: replay state",
        );
        assert_eq!(replay.context.effects, native.context.query.effects);
        let actual = serde_json::to_value(native.gas_cost).unwrap();
        for dimension in ["readTime", "computeTime", "bytesWritten", "bytesDeleted"] {
            let expected_gas: u64 = expected["queries"]
                .as_array()
                .unwrap()
                .iter()
                .map(|query| {
                    query["gasCost"][dimension]
                        .as_str()
                        .unwrap()
                        .parse::<u64>()
                        .unwrap()
                })
                .sum();
            assert_eq!(actual[dimension], expected_gas, "{case}: {dimension}");
            assert_eq!(
                expected["queries"].as_array().unwrap().last().unwrap()["gasCost"][dimension],
                expected["reportedGas"][dimension],
                "{case}: TypeScript reported {dimension}",
            );
        }
    }
}

#[test]
fn recorded_scalar_pure_and_witness_conditional_arguments_match_typescript() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/conditional-counter-oracle.json"
    ))
    .unwrap();
    for (label, conditional, seed_flag) in [
        ("walkerCallPureFalse", false, false),
        ("walkerCallPureTrue", true, false),
        ("streamCallPureFalse", false, false),
        ("streamCallPureTrue", false, true),
        ("witnessArgFalse", false, false),
        ("witnessArgTrue", true, false),
        ("streamCallWitnessFalse", false, false),
        ("streamCallWitnessTrue", false, true),
    ] {
        let make_context = || {
            let context = initial_state(
                ConstructorContext::new(7_u64),
                true,
                true,
                Field::from(111_u64),
            )
            .unwrap()
            .into_circuit_context(ContractAddress::default());
            if seed_flag {
                context.write_cell(0, true).unwrap().context
            } else {
                context
            }
        };
        let native_witnesses = Echo::default();
        let recording_witnesses = Echo::default();
        let native = match label {
            "walkerCallPureFalse" | "walkerCallPureTrue" => {
                walkerCallPure(make_context(), conditional).unwrap()
            }
            "streamCallPureFalse" | "streamCallPureTrue" => streamCallPure(make_context()).unwrap(),
            "witnessArgFalse" | "witnessArgTrue" => {
                witnessArg(make_context(), &native_witnesses, conditional).unwrap()
            }
            "streamCallWitnessFalse" | "streamCallWitnessTrue" => {
                streamCallWitness(make_context(), &native_witnesses).unwrap()
            }
            _ => unreachable!(),
        };
        let recorded = match label {
            "walkerCallPureFalse" | "walkerCallPureTrue" => {
                recorded::walkerCallPure(make_context(), conditional).unwrap()
            }
            "streamCallPureFalse" | "streamCallPureTrue" => {
                recorded::streamCallPure(make_context()).unwrap()
            }
            "witnessArgFalse" | "witnessArgTrue" => {
                recorded::witnessArg(make_context(), &recording_witnesses, conditional).unwrap()
            }
            "streamCallWitnessFalse" | "streamCallWitnessTrue" => {
                recorded::streamCallWitness(make_context(), &recording_witnesses).unwrap()
            }
            _ => unreachable!(),
        };
        let expected = &oracle[label];
        boolean_observation_assertions::assert_ts_trace(label, &native, &recorded, expected);
        assert_eq!(native.context.private_state, expected["privateState"]);
        assert_eq!(
            recorded.execution.context.private_state,
            expected["privateState"]
        );
        assert_eq!(
            native.context.query.state.get_ref(),
            recorded.execution.context.query.state.get_ref(),
        );
        assert_eq!(
            state_hex(recorded.execution.context.query.state.get_ref().clone()),
            expected["stateHex"],
        );
        for calls in [&native_witnesses.calls, &recording_witnesses.calls] {
            let actual = calls
                .borrow()
                .iter()
                .map(|(private_state, value)| {
                    serde_json::json!({"privateState": private_state, "value": value})
                })
                .collect::<Vec<_>>();
            assert_eq!(
                serde_json::to_value(actual).unwrap(),
                expected["witnessCalls"]
            );
        }
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
            assert_eq!(
                serde_json::to_value(actual).unwrap(),
                expected["privateTranscriptOutputs"],
            );
        }
    }
}

fn initial(c: bool, d: bool, x: u64) -> midnight_compact_runtime::context::ConstructorResult<()> {
    initial_state(ConstructorContext::new(()), c, d, Field::from(x)).unwrap()
}

#[test]
fn conditional_scalar_recordings_match_both_typescript_branches_and_replay() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/conditional-counter-oracle.json"
    ))
    .unwrap();
    for (name, conditional, seed_flag) in [
        ("walkerWriteFalse", false, false),
        ("walkerWriteTrue", true, false),
        ("streamWriteFalse", false, false),
        ("streamWriteTrue", true, false),
        ("walkerInlineWriteFalse", false, false),
        ("walkerInlineWriteTrue", true, false),
        ("streamConstAnnotatedFalse", false, false),
        ("streamConstAnnotatedTrue", true, true),
        ("streamIncrementFalse", false, false),
        ("streamIncrementTrue", true, true),
        ("streamAssertEqFalse", false, false),
        ("streamAssertEqTrue", true, true),
    ] {
        let native = initial(true, true, 111).into_circuit_context(ContractAddress::default());
        let recording = initial(true, true, 111).into_circuit_context(ContractAddress::default());
        let native = if seed_flag {
            native.write_cell(0, true).unwrap().context
        } else {
            native
        };
        let recording = if seed_flag {
            recording.write_cell(0, true).unwrap().context
        } else {
            recording
        };
        let (native, recorded) = match name {
            "walkerWriteFalse" | "walkerWriteTrue" => (
                walkerWrite(native, conditional, Field::from(777_u64)).unwrap(),
                recorded::walkerWrite(recording, conditional, Field::from(777_u64)).unwrap(),
            ),
            "streamWriteFalse" | "streamWriteTrue" => (
                streamWrite(native, conditional, Field::from(777_u64)).unwrap(),
                recorded::streamWrite(recording, conditional, Field::from(777_u64)).unwrap(),
            ),
            "walkerInlineWriteFalse" | "walkerInlineWriteTrue" => (
                walkerInlineWrite(native, conditional).unwrap(),
                recorded::walkerInlineWrite(recording, conditional).unwrap(),
            ),
            "streamConstAnnotatedFalse" | "streamConstAnnotatedTrue" => (
                compact_rust_ternary_cond_oracle_fixture::ledger_contract::streamConstAnnotated(
                    native,
                )
                .unwrap(),
                recorded::streamConstAnnotated(recording).unwrap(),
            ),
            "streamIncrementFalse" | "streamIncrementTrue" => (
                streamIncrement(native).unwrap(),
                recorded::streamIncrement(recording).unwrap(),
            ),
            "streamAssertEqFalse" | "streamAssertEqTrue" => (
                streamAssertEq(native).unwrap(),
                recorded::streamAssertEq(recording).unwrap(),
            ),
            _ => unreachable!(),
        };
        boolean_observation_assertions::assert_ts_trace(name, &native, &recorded, &oracle[name]);
        assert_eq!(
            oracle[name]["privateStateNull"], true,
            "{name}: TypeScript private state"
        );
        assert_eq!(
            recorded.execution.context.query.effects, native.context.query.effects,
            "{name}: effects"
        );
        assert_eq!(
            recorded.execution.context.query.state.get_ref(),
            native.context.query.state.get_ref(),
            "{name}: state"
        );
        assert_eq!(
            state_hex(recorded.execution.context.query.state.get_ref().clone()),
            oracle[name]["stateHex"],
            "{name}: TypeScript state"
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
            replay.context.state.get_ref(),
            native.context.query.state.get_ref(),
            "{name}: replay state"
        );
        assert_eq!(
            replay.context.effects, native.context.query.effects,
            "{name}: replay effects"
        );
    }
}

#[test]
fn constructor_and_stateful_ternary_routes_match_typescript_bytes() {
    let reference = oracle();
    assert_eq!(
        state_hex(initial(true, true, 111).ledger_state.get_ref().clone()),
        reference["afterInit"]["stateHex"]
    );
    assert_eq!(
        state_hex(initial(false, false, 222).ledger_state.get_ref().clone()),
        reference["afterInitFalse"]["stateHex"]
    );

    for label in [
        "afterWalkerConstAnnotated",
        "afterWalkerCompareEq",
        "afterWalkerCallPure",
        "afterWalkerStructMember",
        "afterWalkerWrite",
        "afterStreamIncrement",
        "afterStreamCompareEq",
        "afterStreamWrite",
    ] {
        let context = initial(true, true, 111).into_circuit_context(ContractAddress::default());
        let next = match label {
            "afterWalkerConstAnnotated" => walkerConstAnnotated(context, true).unwrap().context,
            "afterWalkerCompareEq" => {
                walkerCompareEq(context, true, BoundedUint::<255>::new(1).unwrap())
                    .unwrap()
                    .context
            }
            "afterWalkerCallPure" => walkerCallPure(context, true).unwrap().context,
            "afterWalkerStructMember" => walkerStructMember(context, true).unwrap().context,
            "afterWalkerWrite" => {
                walkerWrite(context, true, Field::from(555_u64))
                    .unwrap()
                    .context
            }
            "afterStreamIncrement" => streamIncrement(context).unwrap().context,
            "afterStreamCompareEq" => streamCompareEq(context).unwrap().context,
            "afterStreamWrite" => {
                streamWrite(context, false, Field::from(777_u64))
                    .unwrap()
                    .context
            }
            _ => unreachable!(),
        };
        assert_eq!(
            state_hex(next.query.state.get_ref().clone()),
            reference[label]["stateHex"],
            "{label}"
        );
    }
}

#[test]
fn pure_ternary_arms_and_subtraction_guard_execute() {
    assert_eq!(constAnnotatedBothLiteral(true).unwrap().value(), 1);
    assert_eq!(constAnnotatedBothLiteral(false).unwrap().value(), 2);
    for (c, d, expected) in [
        (true, true, 1),
        (true, false, 2),
        (false, true, 3),
        (false, false, 4),
    ] {
        assert_eq!(returnTailNested(c, d).unwrap().value(), expected);
    }
    assert_eq!(enumValued(true).unwrap(), Color::red);
    assert_eq!(enumValued(false).unwrap(), Color::green);
    let zero = BoundedUint::<255>::new(0).unwrap();
    let five = BoundedUint::<255>::new(5).unwrap();
    assert_eq!(constUnannotatedSeqLifted(false, zero).unwrap().value(), 0);
    assert_eq!(constUnannotatedSeqLifted(true, five).unwrap().value(), 4);
    assert!(constUnannotatedSeqLifted(true, zero).is_err());
}

fn assert_direct_trace(
    native: &midnight_compact_runtime::context::CircuitResult<(), ()>,
    recorded: &midnight_compact_runtime::recording::RecordedCircuitResult<(), ()>,
    row: &serde_json::Value,
) {
    assert_eq!(native.result, ());
    assert_eq!(recorded.execution.result, ());
    assert_eq!(row["result"], serde_json::json!([]));
    assert_eq!(
        state_hex(native.context.query.state.get_ref().clone()),
        row["after"]
    );
    assert_eq!(
        native.context.query.state,
        recorded.execution.context.query.state
    );
    assert_eq!(
        native.context.query.effects,
        recorded.execution.context.query.effects
    );
    assert_eq!(
        serde_json::json!(recorded.public.verify_ops()),
        row["publicTranscript"]
    );
    assert!(native.private_transcript_outputs.is_empty());
    assert!(recorded.execution.private_transcript_outputs.is_empty());
    assert_eq!(row["privateTranscript"], serde_json::json!([]));
    assert_eq!(native.gas_cost, recorded.execution.gas_cost);
    let gas = serde_json::json!(native.gas_cost);
    for dimension in ["readTime", "computeTime", "bytesWritten", "bytesDeleted"] {
        let queries = row["queries"].as_array().unwrap();
        let sum: u64 = queries
            .iter()
            .map(|q| {
                q["gasCost"][dimension]
                    .as_str()
                    .unwrap()
                    .parse::<u64>()
                    .unwrap()
            })
            .sum();
        assert_eq!(gas[dimension], sum);
        assert_eq!(
            row["reportedGas"][dimension],
            queries.last().unwrap()["gasCost"][dimension]
        );
    }
    let replay = recorded
        .public
        .initial()
        .query(
            recorded.public.verify_ops(),
            None,
            &recorded.execution.context.cost_model,
        )
        .unwrap();
    assert_eq!(replay.context.state, native.context.query.state);
    assert_eq!(replay.context.effects, native.context.query.effects);
}

#[test]
fn walker_vector_element_executes_both_branches_against_typescript() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/oracle-direct-behavior.json"
    ))
    .unwrap();
    let rows = oracle["stateful"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|row| row["export"] == "walkerVectorElement")
        .collect::<Vec<_>>();
    assert_eq!(rows.len(), 2);
    let mut conditions = std::collections::BTreeSet::new();
    for row in rows {
        let condition = row["args"][0].as_bool().unwrap();
        assert!(conditions.insert(condition));
        let fresh = || {
            initial_state(ConstructorContext::new(()), true, true, Field::from(111u64))
                .unwrap()
                .into_circuit_context(ContractAddress::default())
        };
        assert_eq!(
            state_hex(fresh().query.state.get_ref().clone()),
            row["before"]
        );
        let native =
            compact_rust_ternary_cond_oracle_fixture::ledger_contract::walkerVectorElement(
                fresh(),
                condition,
            )
            .unwrap();
        let recorded = recorded::walkerVectorElement(fresh(), condition).unwrap();
        assert_direct_trace(&native, &recorded, row);
        let vector = ledger_slots::vecCell
            .inspect(native.context.query.state.get_ref())
            .unwrap();
        assert_eq!(
            serde_json::json!(
                vector
                    .0
                    .into_iter()
                    .map(|v| hex::encode(v.as_le_bytes()))
                    .collect::<Vec<_>>()
            ),
            row["ledgerVector"]
        );
        assert_eq!(
            vector.0,
            if condition {
                [Field::from(1u64), Field::from(3u64)]
            } else {
                [Field::from(2u64), Field::from(4u64)]
            }
        );
    }
    assert_eq!(conditions, std::collections::BTreeSet::from([false, true]));
}
