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
//! The unchanged original Coracle start is exercised from explicit red/blue
//! prestates. This native check does not imply a funded two-call lifecycle.

use compact_rust_test_center_coracle_fixture::{
    ledger_contract as c, ledger_slots as slots, types,
};
use midnight_compact_runtime as runtime;
use midnight_onchain_state::state::{
    ContractMaintenanceAuthority, ContractOperation, ContractState, EntryPointBuf,
};
use midnight_storage::storage::HashMap;
use runtime::context::{CircuitContext, ConstructorContext, WitnessContext};
use runtime::ledger::{CoinInfo, CoinRecipient, DefaultDB, StateValue};
use runtime::{BoundedUint, CompactError, Field, FixedBytes};
use serde_json::{Value, json};
use std::cell::RefCell;

#[derive(Clone, Debug, Default)]
struct Private {
    calls: u64,
    board: Option<types::Committable>,
}

struct Witness {
    mode: String,
    expected_pos: u64,
    calls: RefCell<Vec<Value>>,
}

impl Witness {
    fn new(row: &Value) -> Self {
        Self {
            mode: row["options"]["mode"].as_str().unwrap_or("normal").into(),
            expected_pos: row["options"]["pos"]
                .as_str()
                .unwrap_or("4")
                .parse()
                .unwrap(),
            calls: RefCell::new(Vec::new()),
        }
    }

    fn calls(&self) -> Value {
        Value::Array(self.calls.borrow().clone())
    }
}

impl c::TryWitnesses<Private> for Witness {
    fn local_secret_key(
        &self,
        ctx: WitnessContext<'_, Private, c::LedgerView<'_>>,
    ) -> Result<(Private, FixedBytes<32>), CompactError> {
        self.calls.borrow_mut().push(json!("secret"));
        if self.mode == "secretFailure" {
            return Err(CompactError::AssertionFailed(
                "secret witness refused".into(),
            ));
        }
        let mut next = ctx.private_state.clone();
        next.calls += 1;
        Ok((next, FixedBytes::new([4; 32])))
    }

    fn fresh_nonce(
        &self,
        ctx: WitnessContext<'_, Private, c::LedgerView<'_>>,
    ) -> Result<(Private, Field), CompactError> {
        self.calls.borrow_mut().push(json!("nonce"));
        if self.mode == "nonceFailure" {
            return Err(CompactError::AssertionFailed(
                "nonce witness refused".into(),
            ));
        }
        if self.mode == "nonceMalformed" {
            return Err(CompactError::AssertionFailed(
                "negative nonce cannot inhabit typed Field".into(),
            ));
        }
        let mut next = ctx.private_state.clone();
        next.calls += 1;
        Ok((next, Field::from(9u64)))
    }

    fn local_set_board(
        &self,
        ctx: WitnessContext<'_, Private, c::LedgerView<'_>>,
        board: types::Committable,
    ) -> Result<(Private, ()), CompactError> {
        assert_eq!(board.nonce, Field::from(9u64));
        assert_eq!(board.contents.position, Field::from(self.expected_pos));
        self.calls.borrow_mut().push(json!(["set", {
            "nonce":"9",
            "contents":{"position":self.expected_pos.to_string()}
        }]));
        if self.mode == "setFailure" {
            return Err(CompactError::AssertionFailed("set witness refused".into()));
        }
        let mut next = ctx.private_state.clone();
        next.calls += 1;
        next.board = Some(board);
        Ok((next, ()))
    }

    fn local_board(
        &self,
        _: WitnessContext<'_, Private, c::LedgerView<'_>>,
    ) -> Result<(Private, types::Committable), CompactError> {
        panic!("start must not call local_board")
    }
}

fn state_hex(state: StateValue<DefaultDB>) -> String {
    let operations =
        ["start", "guess", "concede", "withdraw"]
            .into_iter()
            .fold(HashMap::new(), |m, n| {
                m.insert(
                    EntryPointBuf(n.as_bytes().to_vec()),
                    ContractOperation::new(None),
                )
            });
    let state = ContractState::new(state, operations, ContractMaintenanceAuthority::default());
    let mut out = vec![];
    midnight_serialize::tagged_serialize(&state, &mut out).unwrap();
    hex::encode(out)
}

fn selected_u128(row: &Value, name: &str, default: u128) -> u128 {
    row["options"][name]
        .as_str()
        .map_or(default, |v| v.parse().unwrap())
}

fn selected_u8(row: &Value, name: &str, default: u8) -> u8 {
    row["options"][name].as_u64().map_or(default, |v| v as u8)
}

fn context(row: &Value) -> CircuitContext<Private> {
    let mut ctx = c::initial_state(ConstructorContext::new(Private::default()))
        .unwrap()
        .into_circuit_context(runtime::ledger::ContractAddress::default());
    let phase = row["options"]["phase"].as_u64().unwrap_or(0);
    let state = match phase {
        0 => types::State::no_game,
        1 => types::State::red_started,
        2 => types::State::blue_started,
        3 => types::State::red_turn,
        4 => types::State::blue_turn,
        5 => types::State::red_wins,
        6 => types::State::blue_wins,
        _ => unreachable!(),
    };
    ctx = slots::state.write(ctx, state).unwrap().context;
    if phase == 1 {
        let pot = types::QualifiedShieldedCoinInfo {
            nonce: FixedBytes::new([1; 32]),
            color: FixedBytes::new([selected_u8(row, "potColor", 5); 32]),
            value: BoundedUint::new(selected_u128(row, "potValue", 17)).unwrap(),
            mt_index: BoundedUint::new(0).unwrap(),
        };
        ctx = slots::pot.write(ctx, pot).unwrap().context;
    }
    ctx.set_zswap_output_start(2).unwrap();
    assert_eq!(
        state_hex(ctx.query.state.get_ref().clone()),
        row["before"],
        "{}",
        row["name"]
    );
    ctx
}

fn coin(row: &Value, kind: &str, nonce: u8, color: u8, default: u128) -> types::ShieldedCoinInfo {
    let color_name = format!("{kind}Color");
    let value_name = format!("{kind}Value");
    types::ShieldedCoinInfo {
        nonce: FixedBytes::new([nonce; 32]),
        color: FixedBytes::new([selected_u8(row, &color_name, color); 32]),
        value: BoundedUint::new(selected_u128(row, &value_name, default)).unwrap(),
    }
}

fn encoded_coin(coin: CoinInfo) -> Value {
    json!({"nonce":coin.nonce.0.0,"color":coin.type_.0.0,"value":coin.value.to_string()})
}

fn encoded_recipient(recipient: &CoinRecipient) -> Value {
    let (left, user, contract) = match recipient {
        CoinRecipient::User(key) => (true, key.0.0, [0; 32]),
        CoinRecipient::Contract(address) => (false, [0; 32], address.0.0),
    };
    json!({"is_left":left,"left":{"bytes":user},"right":{"bytes":contract}})
}

fn normalized_effects(mut effects: Value) -> Value {
    // The upstream claim collection is set-like; JS and Rust serialize its
    // insertion order differently. Preserve every value and duplicate while
    // comparing this collection. Public VM/query order stays exact below.
    for key in ["claimedShieldedReceives", "claimedNullifiers"] {
        if let Some(claims) = effects[key].as_array_mut() {
            claims.sort_by(|a, b| a.as_str().cmp(&b.as_str()));
        }
    }
    effects
}

#[test]
fn original_start_native_matches_corrected_ts_from_seeded_prestates() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/coracle-start.json"
    ))
    .unwrap();
    assert_eq!(fixture["rows"].as_array().unwrap().len(), 27);
    for row in fixture["rows"].as_array().unwrap() {
        let witness = Witness::new(row);
        let pos = row["options"]["pos"]
            .as_str()
            .unwrap_or("4")
            .parse::<u64>()
            .unwrap();
        let result = c::start(
            context(row),
            &witness,
            Field::from(pos),
            coin(row, "wager", 3, 5, 17),
            coin(row, "deposit", 2, 0, 100000),
        );
        let recorded_witness = Witness::new(row);
        let recorded = c::recorded::start(
            context(row),
            &recorded_witness,
            Field::from(pos),
            coin(row, "wager", 3, 5, 17),
            coin(row, "deposit", 2, 0, 100000),
        );
        if let Some(expected) = row["error"].as_str() {
            let actual = result.err().unwrap().to_string();
            assert_eq!(
                recorded.err().unwrap().to_string(),
                actual,
                "{}",
                row["name"]
            );
            assert_eq!(recorded_witness.calls(), witness.calls(), "{}", row["name"]);
            if row["name"] == "nonceMalformed" {
                // TS accepts -1n from a dynamically typed witness and rejects
                // at the later Field descriptor. Rust rejects at the typed
                // witness boundary before any malformed Field can be formed.
                assert!(actual.contains("negative nonce"));
                assert!(expected.contains("expected value of type Field"));
                assert_eq!(witness.calls(), row["witnessCalls"], "{}", row["name"]);
                continue;
            }
            if row["name"] == "blueMergeOverflow" {
                assert!(actual.contains("invalid Compact unsigned value"));
                assert!(expected.contains("cast from Field or Uint"));
            } else if [
                "secretFailure",
                "nonceFailure",
                "setFailureRed",
                "setFailureBlue",
            ]
            .contains(&row["name"].as_str().unwrap())
            {
                assert!(actual.contains(expected), "{}: {actual}", row["name"]);
            } else {
                assert_eq!(actual, expected, "{}", row["name"]);
            }
            assert_eq!(witness.calls(), row["witnessCalls"], "{}", row["name"]);
            continue;
        }
        let out = result.unwrap();
        let recorded = recorded.unwrap();
        assert_eq!(
            recorded_witness.calls(),
            row["witnessCalls"],
            "{}",
            row["name"]
        );
        assert_eq!(recorded.execution.result, out.result, "{}", row["name"]);
        assert_eq!(
            recorded.execution.context.query.state, out.context.query.state,
            "{}",
            row["name"]
        );
        assert_eq!(
            normalized_effects(json!(recorded.execution.context.query.effects)),
            normalized_effects(json!(out.context.query.effects)),
            "{}",
            row["name"]
        );
        assert_eq!(
            recorded.execution.context.circuit_zswap(),
            out.context.circuit_zswap(),
            "{}",
            row["name"]
        );
        assert_eq!(
            recorded.execution.context.query.call_context.com_indices,
            out.context.query.call_context.com_indices,
            "{}",
            row["name"]
        );
        assert_eq!(
            recorded.execution.private_transcript_outputs, out.private_transcript_outputs,
            "{}",
            row["name"]
        );
        assert_eq!(recorded.execution.gas_cost, out.gas_cost, "{}", row["name"]);
        assert_eq!(
            json!(recorded.public.verify_ops()),
            row["publicTranscript"],
            "{}",
            row["name"]
        );
        let mut replay_initial = recorded.public.initial().clone();
        replay_initial.call_context.com_indices =
            out.context.query.call_context.com_indices.clone();
        let replay = replay_initial
            .query(
                recorded.public.verify_ops(),
                None,
                &recorded.execution.context.cost_model,
            )
            .unwrap_or_else(|error| panic!("{} replay: {error:?}", row["name"]));
        assert_eq!(
            replay.context.state, out.context.query.state,
            "{}",
            row["name"]
        );
        assert_eq!(
            normalized_effects(json!(replay.context.effects)),
            normalized_effects(json!(out.context.query.effects)),
            "{}",
            row["name"]
        );
        for dim in ["readTime", "computeTime", "bytesWritten", "bytesDeleted"] {
            assert_eq!(
                json!(replay.gas_cost)[dim].as_u64().unwrap().to_string(),
                row["replayProbe"]["gas"][dim],
                "{} {dim}",
                row["name"]
            );
        }
        assert_eq!(witness.calls(), row["witnessCalls"], "{}", row["name"]);
        assert_eq!(
            state_hex(out.context.query.state.get_ref().clone()),
            row["after"],
            "{}",
            row["name"]
        );
        assert_eq!(
            normalized_effects(json!(out.context.query.effects)),
            normalized_effects(row["effects"].clone()),
            "{}",
            row["name"]
        );
        assert_eq!(
            json!(runtime::fab::AlignedValue::from(out.result)),
            row["output"],
            "{}",
            row["name"]
        );
        assert_eq!(out.result as u8, row["result"].as_u64().unwrap() as u8);
        assert_eq!(
            json!(out.private_transcript_outputs),
            row["privateOutputs"],
            "{}",
            row["name"]
        );
        let plan = out.context.circuit_zswap();
        assert_eq!(plan.next_index().to_string(), row["plan"]["currentIndex"]);
        assert_eq!(
            plan.inputs().len(),
            row["plan"]["inputs"].as_array().unwrap().len()
        );
        assert_eq!(
            plan.outputs().len(),
            row["plan"]["outputs"].as_array().unwrap().len()
        );
        for (index, input) in plan.inputs().iter().enumerate() {
            let mut encoded = encoded_coin(CoinInfo {
                nonce: input.nonce,
                type_: input.type_,
                value: input.value,
            });
            encoded["mt_index"] = json!(input.mt_index.to_string());
            assert_eq!(encoded, row["plan"]["inputs"][index], "{}", row["name"]);
        }
        for (index, output) in plan.outputs().iter().enumerate() {
            assert_eq!(
                encoded_coin(output.coin),
                row["plan"]["outputs"][index]["coinInfo"]
            );
            assert_eq!(
                encoded_recipient(&output.recipient),
                row["plan"]["outputs"][index]["recipient"]
            );
            let commitment = output.coin.commitment(&output.recipient);
            let commitment_hex = hex::encode(commitment.0.0);
            let captured = row["provisionalComIndices"]
                .as_array()
                .unwrap()
                .iter()
                .find(|entry| entry[0] == commitment_hex)
                .unwrap();
            assert_eq!(output.provisional_index.to_string(), captured[1]);
            assert_eq!(
                out.context.query.call_context.com_indices.get(&commitment),
                Some(&output.provisional_index)
            );
        }
        for dim in ["readTime", "computeTime", "bytesWritten", "bytesDeleted"] {
            let sum: u64 = row["queries"]
                .as_array()
                .unwrap()
                .iter()
                .map(|query| query["gas"][dim].as_str().unwrap().parse::<u64>().unwrap())
                .sum();
            assert_eq!(json!(out.gas_cost)[dim], sum, "{} {dim}", row["name"]);
            assert_eq!(sum.to_string(), row["queryCostSum"][dim]);
        }
        assert_eq!(
            row["partition"][if row["name"].as_str().unwrap().starts_with("blue") {
                1
            } else {
                0
            }]["programLength"],
            if row["name"].as_str().unwrap().starts_with("blue") {
                97
            } else {
                58
            }
        );
    }
}
