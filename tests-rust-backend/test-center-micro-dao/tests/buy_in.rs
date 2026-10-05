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
use compact_rust_test_center_micro_dao_fixture::{
    ledger_contract as c, ledger_slots as slots, types,
};
use midnight_compact_runtime as runtime;
use runtime::context::{CircuitContext, ConstructorContext};
use runtime::{BoundedUint, FixedBytes};
use serde_json::{Value, json};

fn number(row: &Value, key: &str, default: u128) -> u128 {
    row["options"][key]
        .as_str()
        .map(|s| s.parse().unwrap())
        .unwrap_or(default)
}
fn state(row: &Value, key: &str) -> runtime::ledger::ContractState<runtime::ledger::DefaultDB> {
    midnight_serialize::tagged_deserialize(
        &mut hex::decode(row[key].as_str().unwrap()).unwrap().as_slice(),
    )
    .unwrap()
}
fn context(row: &Value) -> CircuitContext<Value> {
    let mut ctx = c::initial_state(
        ConstructorContext::new(json!({"calls": 0})),
        FixedBytes::new([4; 32]),
        types::Costs {
            seed_dust: BoundedUint::new(10).unwrap(),
            buy_in_dust: BoundedUint::new(number(row, "cost", 3)).unwrap(),
        },
    )
    .unwrap()
    .into_circuit_context(runtime::ledger::ContractAddress::default());
    let occupied = row["options"]["occupied"] == true;
    ctx = slots::pot_has_coin.write(ctx, occupied).unwrap().context;
    if occupied {
        ctx = slots::pot
            .write(
                ctx,
                types::QualifiedShieldedCoinInfo {
                    nonce: FixedBytes::new([1; 32]),
                    color: FixedBytes::new(
                        [row["options"]["potColor"].as_u64().unwrap_or(0) as u8; 32],
                    ),
                    value: BoundedUint::new(number(row, "potValue", 17)).unwrap(),
                    mt_index: BoundedUint::new(0).unwrap(),
                },
            )
            .unwrap()
            .context;
    }
    assert_eq!(
        ctx.query.state.get_ref(),
        state(row, "before").data.get_ref()
    );
    if row["options"]["missingKey"] != true {
        ctx = ctx.with_coin_public_key_bytes([7; 32]);
    }
    ctx.set_zswap_output_start(2).unwrap();
    ctx
}
fn coin(row: &Value) -> types::ShieldedCoinInfo {
    types::ShieldedCoinInfo {
        nonce: FixedBytes::new([3; 32]),
        color: FixedBytes::new([row["options"]["color"].as_u64().unwrap_or(0) as u8; 32]),
        value: BoundedUint::new(number(
            row,
            "value",
            number(row, "amount", 2) * number(row, "cost", 3),
        ))
        .unwrap(),
    }
}
fn normalized_effects(mut value: Value) -> Value {
    // These upstream fields are sets. Sort without deduplicating so an extra
    // entry cannot disappear; source/query/private transcript order is exact.
    for field in [
        "claimedNullifiers",
        "claimedShieldedReceives",
        "claimedShieldedSpends",
    ] {
        value[field]
            .as_array_mut()
            .unwrap()
            .sort_by_key(Value::to_string);
    }
    // TS serializes bigint map values as decimal strings; Rust serde emits
    // the same integer as a JSON number. Keep every token-color key and exact
    // amount while comparing the shared semantic value.
    for amount in value["shieldedMints"].as_object_mut().unwrap().values_mut() {
        if let Some(number) = amount.as_u64() {
            *amount = json!(number.to_string());
        }
    }
    value
}
#[test]
fn original_buy_in_native_and_recorded_match_independent_typescript() {
    let data: Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/micro-dao-buy-in.json"
    ))
    .unwrap();
    let rows = data["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 18);
    for row in rows {
        let name = row["name"].as_str().unwrap();
        let result = c::buy_in(
            context(row),
            coin(row),
            BoundedUint::new(number(row, "amount", 2)).unwrap(),
        );
        let recorded = c::recorded::buy_in(
            context(row),
            coin(row),
            BoundedUint::new(number(row, "amount", 2)).unwrap(),
        );
        assert!(row["witnessCalls"].as_array().unwrap().is_empty());
        if let Some(error) = row.get("error") {
            let actual = result.err().expect("independent TS rejection must reject");
            assert_eq!(recorded.err().unwrap(), actual, "{name}");
            match name {
                "mergeOverflow" => assert!(matches!(
                    actual,
                    runtime::CompactError::InvalidUnsignedValue
                )),
                "missingKeyEmpty" | "missingKeyOccupied" => assert!(matches!(
                    actual,
                    runtime::CompactError::MissingCoinPublicKey
                )),
                _ => {
                    let runtime::CompactError::AssertionFailed(message) = actual else {
                        panic!("{name}: {actual:?}")
                    };
                    assert!(
                        error.as_str().unwrap().contains(&message),
                        "{name}: {message}"
                    );
                }
            }
            continue;
        }
        let out = result.unwrap_or_else(|error| panic!("{name}: {error}"));
        let recorded = recorded.unwrap_or_else(|error| panic!("{name}: {error}"));
        assert_eq!(recorded.execution.result, out.result, "{name}");
        assert_eq!(
            recorded.execution.context.query.state, out.context.query.state,
            "{name}"
        );
        assert_eq!(
            recorded.execution.context.query.effects, out.context.query.effects,
            "{name}"
        );
        assert_eq!(
            recorded.execution.context.private_state, out.context.private_state,
            "{name}"
        );
        assert_eq!(
            recorded.execution.context.circuit_zswap(),
            out.context.circuit_zswap(),
            "{name}"
        );
        assert_eq!(
            recorded.execution.private_transcript_outputs, out.private_transcript_outputs,
            "{name}"
        );
        assert_eq!(recorded.execution.gas_cost, out.gas_cost, "{name}");
        assert_eq!(
            json!(recorded.public.verify_ops()),
            row["publicTranscript"],
            "{name}"
        );
        let mut initial = recorded.public.initial().clone();
        initial.call_context.com_indices = recorded
            .execution
            .context
            .query
            .call_context
            .com_indices
            .clone();
        let replay = initial
            .query(recorded.public.verify_ops(), None, &out.context.cost_model)
            .unwrap();
        assert_eq!(replay.context.state, out.context.query.state, "{name}");
        assert_eq!(replay.context.effects, out.context.query.effects, "{name}");
        assert_eq!(
            out.context.query.state.get_ref(),
            state(row, "after").data.get_ref(),
            "{name}"
        );
        assert_eq!(out.context.private_state, row["privateState"], "{name}");
        assert_eq!(
            json!(out.private_transcript_outputs),
            row["privateOutputs"],
            "{name}"
        );
        assert_eq!(
            json!(runtime::fab::AlignedValue::from(out.result.clone())),
            row["output"],
            "{name}"
        );
        assert_eq!(
            json!({"nonce":out.result.nonce.0,"color":out.result.color.0,"value":out.result.value.value().to_string()}),
            row["result"],
            "{name}"
        );
        assert_eq!(
            normalized_effects(json!(out.context.query.effects)),
            normalized_effects(row["effects"].clone()),
            "{name}"
        );
        for dimension in ["readTime", "computeTime", "bytesWritten", "bytesDeleted"] {
            assert_eq!(
                json!(out.gas_cost)[dimension].as_u64().unwrap().to_string(),
                row["queryCostSum"][dimension],
                "{name}: {dimension}"
            );
            assert_eq!(
                json!(replay.gas_cost)[dimension]
                    .as_u64()
                    .unwrap()
                    .to_string(),
                row["replayProbe"]["gas"][dimension],
                "{name}: replay {dimension}"
            );
        }
        let plan = out.context.circuit_zswap();
        let occupied = row["options"]["occupied"] == true;
        assert_eq!(plan.inputs().len(), if occupied { 2 } else { 0 });
        assert_eq!(plan.outputs().len(), if occupied { 3 } else { 2 });
        assert_eq!(plan.next_index(), 2 + plan.outputs().len() as u64);
        assert_eq!(plan.next_index().to_string(), row["plan"]["currentIndex"]);
        for (i, input) in plan.inputs().iter().enumerate() {
            assert_eq!(
                json!({"nonce":input.nonce.0.0,"color":input.type_.0.0,"value":input.value.to_string(),"mt_index":input.mt_index.to_string()}),
                row["plan"]["inputs"][i],
                "{name}: input{i}"
            );
        }
        for (i, output) in plan.outputs().iter().enumerate() {
            let expected = &row["plan"]["outputs"][i];
            assert_eq!(output.provisional_index, 2 + i as u64);
            assert_eq!(
                json!({"nonce":output.coin.nonce.0.0,"color":output.coin.type_.0.0,"value":output.coin.value.to_string()}),
                expected["coinInfo"],
                "{name}: output{i}"
            );
            match output.recipient {
                runtime::ledger::CoinRecipient::User(key) => {
                    assert_eq!(expected["recipient"]["is_left"], true);
                    assert_eq!(json!(key.0.0), expected["recipient"]["left"]["bytes"]);
                }
                runtime::ledger::CoinRecipient::Contract(address) => {
                    assert_eq!(expected["recipient"]["is_left"], false);
                    assert_eq!(json!(address.0.0), expected["recipient"]["right"]["bytes"]);
                }
            }
            let commitment = output.coin.commitment(&output.recipient);
            let captured = row["provisionalComIndices"]
                .as_array()
                .unwrap()
                .iter()
                .find(|entry| entry[0] == hex::encode(commitment.0.0))
                .expect("each persistent output has an exact captured commitment");
            assert_eq!(
                output.provisional_index.to_string(),
                captured[1],
                "{name}: output{i}"
            );
            assert_eq!(
                out.context.query.call_context.com_indices.get(&commitment),
                Some(&output.provisional_index),
                "{name}: output{i}"
            );
        }
        let partition = row["partition"]
            .as_array()
            .expect("pinned upstream partition");
        assert!(partition[1].is_null());
        assert_eq!(
            partition[0]["program"].as_array().unwrap().len(),
            if occupied { 81 } else { 54 }
        );
    }
}
