// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//   http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use midnight_compact_runtime::context::CircuitResult;
use midnight_compact_runtime::ledger::DefaultDB;
use midnight_compact_runtime::recording::RecordedCircuitResult;
use serde_json::{Value, json};

pub fn assert_ts_trace(
    name: &str,
    native: &CircuitResult<(), (), DefaultDB>,
    recorded: &RecordedCircuitResult<(), (), DefaultDB>,
    reference: &Value,
) {
    let actual = serde_json::to_value(recorded.public.verify_ops()).unwrap();
    let shape = Value::Array(
        actual
            .as_array()
            .unwrap()
            .iter()
            .map(|operation| {
                if let Some(kind) = operation.as_str() {
                    return json!({ "kind": kind });
                }
                if let Some(idx) = operation.get("idx") {
                    json!({
                        "kind": "idx", "cached": idx["cached"], "pushPath": idx["pushPath"],
                        "pathLength": idx["path"].as_array().unwrap().len(),
                    })
                } else if let Some(push) = operation.get("push") {
                    json!({ "kind": "push", "storage": push["storage"] })
                } else if let Some(ins) = operation.get("ins") {
                    json!({ "kind": "ins", "cached": ins["cached"], "n": ins["n"] })
                } else if let Some(rem) = operation.get("rem") {
                    json!({ "kind": "rem", "cached": rem["cached"] })
                } else if let Some(dup) = operation.get("dup") {
                    json!({ "kind": "dup", "n": dup["n"] })
                } else if let Some(popeq) = operation.get("popeq") {
                    json!({
                        "kind": "popeq", "cached": popeq["cached"],
                        "resultAtoms": popeq["result"]["value"],
                    })
                } else {
                    panic!("{name}: unexpected VM operation {operation}");
                }
            })
            .collect(),
    );
    assert_eq!(
        shape, reference["publicTranscriptShape"],
        "{name}: ordered TypeScript VM"
    );
    assert_eq!(
        recorded.execution.private_transcript_outputs.len(),
        reference["privateTranscriptCount"].as_u64().unwrap() as usize,
        "{name}: TypeScript private transcript count"
    );

    // The generated TypeScript wrapper's reportedGas retains only its last
    // query. Sum the captured VM queries to compare the whole circuit.
    let gas = serde_json::to_value(native.gas_cost).unwrap();
    assert_eq!(
        recorded.execution.gas_cost, native.gas_cost,
        "{name}: native gas"
    );
    for dimension in ["readTime", "computeTime", "bytesWritten", "bytesDeleted"] {
        let expected: u64 = reference["queries"]
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
        assert_eq!(
            gas[dimension].as_u64().unwrap(),
            expected,
            "{name}: TypeScript {dimension}"
        );
    }
}
