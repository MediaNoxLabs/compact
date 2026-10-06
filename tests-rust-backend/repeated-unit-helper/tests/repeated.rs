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

use compact_rust_repeated_unit_helper_fixture::ledger_contract as c;
use midnight_compact_runtime as r;
use r::context::{ConstructorContext, WitnessContext};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

#[allow(dead_code)]
#[path = "../../support/oracle_recorded_trace.rs"]
mod trace;

type Uint = r::BoundedUint<18446744073709551615>;
struct Witness;
fn witness(
    name: &str,
    ctx: WitnessContext<'_, Vec<Value>, c::LedgerView<'_>>,
    argument: Uint,
) -> (Vec<Value>, Uint) {
    let count = ctx.ledger.count().unwrap().value();
    let last = ctx.ledger.last().unwrap().value();
    let receipt = ctx.ledger.receipt().unwrap().value();
    let ordinal = ctx.private_state.len() as u128;
    let result = argument.value() + (if name == "argument" { 100 } else { 1000 }) * count + ordinal;
    let mut events = ctx.private_state.clone();
    events.push(json!({"name":name,"argument":argument.value().to_string(),"count":count.to_string(),"last":last.to_string(),"receipt":receipt.to_string(),"result":result.to_string()}));
    (events, Uint::new(result).unwrap())
}
impl c::Witnesses<Vec<Value>> for Witness {
    fn argument(
        &self,
        context: WitnessContext<'_, Vec<Value>, c::LedgerView<'_>>,
        value: Uint,
    ) -> (Vec<Value>, Uint) {
        witness("argument", context, value)
    }
    fn observe(
        &self,
        context: WitnessContext<'_, Vec<Value>, c::LedgerView<'_>>,
        value: Uint,
    ) -> (Vec<Value>, Uint) {
        witness("observe", context, value)
    }
}
fn captured() -> Value {
    serde_json::from_str(include_str!("../oracle/cases.json")).unwrap()
}
fn scenario(id: &str, expected_outputs: [u128; 4]) {
    let captured = captured();
    assert_eq!(captured["cases"].as_array().unwrap().len(), 3);
    let row = captured["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["id"] == id)
        .unwrap();
    let state: r::ledger::ContractState<r::ledger::DefaultDB> =
        midnight_serialize::tagged_deserialize(
            &mut hex::decode(row["before"].as_str().unwrap())
                .unwrap()
                .as_slice(),
        )
        .unwrap();
    let context = || {
        c::initial_state(ConstructorContext::new(Vec::<Value>::new()))
            .unwrap()
            .into_circuit_context(r::ledger::ContractAddress::default())
    };
    assert_eq!(context().query.state, state.data);
    let number = |key: &str| Uint::new(row[key].as_str().unwrap().parse().unwrap()).unwrap();
    let native = c::twice(context(), &Witness, number("first"), number("second")).unwrap();
    let recorded =
        c::recorded::twice(context(), &Witness, number("first"), number("second")).unwrap();
    trace::assert_trace(&native, &recorded, row, |data| {
        let mut copy = state.clone();
        copy.data = r::ledger::ChargedState::new(data);
        let mut bytes = Vec::new();
        midnight_serialize::tagged_serialize(&copy, &mut bytes).unwrap();
        hex::encode(bytes)
    });
    assert_eq!(json!(native.context.private_state), row["privateAfter"]);
    let events = &native.context.private_state;
    assert_eq!(
        events
            .iter()
            .map(|e| e["name"].as_str().unwrap())
            .collect::<Vec<_>>(),
        ["argument", "observe", "argument", "observe"]
    );
    assert_eq!(
        events
            .iter()
            .map(|e| e["count"].as_str().unwrap())
            .collect::<Vec<_>>(),
        ["0", "1", "1", "2"]
    );
    assert_eq!(events[0]["argument"], row["first"]);
    assert_eq!(events[2]["argument"], row["second"]);
    for (event, value) in events.iter().zip(expected_outputs) {
        assert_eq!(event["result"], value.to_string());
    }
    assert_eq!(native.private_transcript_outputs.len(), 4);
    let final_view = c::PublicStateView::from(&native.context);
    assert_eq!(final_view.count().unwrap().value(), 2);
    assert_eq!(final_view.last().unwrap().value(), expected_outputs[2]);
    assert_eq!(final_view.receipt().unwrap().value(), expected_outputs[3]);
}
#[test]
fn repeated_helper_preserves_arguments_and_each_witness_effect() {
    scenario("distinct", [3, 1004, 109, 2112]);
}
#[test]
fn reversed_arguments_remain_in_source_order() {
    scenario("reversed", [7, 1008, 105, 2108]);
}
#[test]
fn identical_labels_do_not_cache_argument_results_or_helper_execution() {
    scenario("same-label-zero", [0, 1001, 102, 2105]);
}
#[test]
fn capture_is_bound_to_reviewed_source_and_capture_program() {
    let capture = captured();
    assert_eq!(capture["compiler"], "0.31.133");
    assert_eq!(capture["runtime"], "0.16.101");
    for (role, bytes) in [
        (
            "source",
            include_bytes!("../../../examples/rust_backend/repeated_unit_helper.compact")
                .as_slice(),
        ),
        (
            "capture",
            include_bytes!("../oracle/capture.mjs").as_slice(),
        ),
    ] {
        let entry = capture["provenance"]
            .as_array()
            .unwrap()
            .iter()
            .find(|e| e["role"] == role)
            .unwrap();
        assert_eq!(entry["sha256"], format!("{:x}", Sha256::digest(bytes)));
    }
}
