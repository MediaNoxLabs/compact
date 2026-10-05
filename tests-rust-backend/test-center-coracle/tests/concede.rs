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
//! The complete original Coracle source is seeded at a prior game state.
#[path = "../support/guess.rs"]
mod support;

use compact_rust_test_center_coracle_fixture::{
    ledger_contract as c, ledger_slots as slots, types,
};
use midnight_compact_runtime as runtime;
use midnight_onchain_state::state::{
    ContractMaintenanceAuthority, ContractOperation, ContractState, EntryPointBuf,
};
use midnight_storage::storage::HashMap;
use runtime::context::CircuitContext;
use runtime::context::WitnessContext;
use runtime::ledger::{CoinInfo, CoinRecipient, DefaultDB, StateValue};
use runtime::{BoundedUint, FixedBytes};
use runtime::{CompactError, Field};
use serde_json::{Value, json};
use support::{Private, Settings};

struct Witness {
    inner: support::Witness,
    secret_failure: bool,
    board_failure: bool,
}

impl Witness {
    fn new(row: &Value) -> Self {
        Self {
            inner: support::Witness::new(settings(row)),
            secret_failure: row["options"]["secretFailure"] == true,
            board_failure: row["options"]["boardFailure"] == true,
        }
    }

    fn calls(&self) -> Value {
        json!(*self.inner.calls.borrow())
    }
}

impl c::TryWitnesses<Private> for Witness {
    fn local_secret_key(
        &self,
        ctx: WitnessContext<'_, Private, c::LedgerView<'_>>,
    ) -> Result<(Private, FixedBytes<32>), CompactError> {
        let result = self.inner.local_secret_key(ctx);
        if self.secret_failure && self.inner.calls.borrow().len() == 2 {
            return Err(CompactError::AssertionFailed(
                "selected secret witness failed".into(),
            ));
        }
        result
    }

    fn local_board(
        &self,
        ctx: WitnessContext<'_, Private, c::LedgerView<'_>>,
    ) -> Result<(Private, types::Committable), CompactError> {
        let result = self.inner.local_board(ctx);
        if self.board_failure {
            return Err(CompactError::AssertionFailed(
                "selected board witness failed".into(),
            ));
        }
        result
    }

    fn local_set_board(
        &self,
        ctx: WitnessContext<'_, Private, c::LedgerView<'_>>,
        value: types::Committable,
    ) -> Result<(Private, ()), CompactError> {
        self.inner.local_set_board(ctx, value)
    }

    fn fresh_nonce(
        &self,
        ctx: WitnessContext<'_, Private, c::LedgerView<'_>>,
    ) -> Result<(Private, Field), CompactError> {
        self.inner.fresh_nonce(ctx)
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

fn settings(row: &Value) -> Settings {
    let o = &row["options"];
    Settings {
        red: row["color"] == "red",
        board: o["board"].as_str().map_or(1, |v| v.parse().unwrap()),
        empty: o["empty"] == true,
        both_players: o["bothPlayers"] == true,
        first_blue: false,
        wrong_turn: o["wrongTurn"] == true,
        dead: o["alive"] != true,
        impostor: o["impostor"] == true,
        changed_secret: o["changedSecret"] == true,
        wrong_nonce: o["wrongNonce"] == true,
        invalid_board: o["invalidBoard"] == true,
    }
}

fn coin(nonce: u8, value: u128, index: u64) -> types::QualifiedShieldedCoinInfo {
    types::QualifiedShieldedCoinInfo {
        nonce: FixedBytes::new([nonce; 32]),
        color: FixedBytes::new([2; 32]),
        value: BoundedUint::new(value).unwrap(),
        mt_index: BoundedUint::new(index.into()).unwrap(),
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

fn context(row: &Value) -> CircuitContext<Private> {
    let mut ctx = support::seeded(&settings(row)).unwrap();
    ctx = slots::pot.write(ctx, coin(3, 42, 0)).unwrap().context;
    ctx = slots::red_deposit
        .write(ctx, coin(4, 17, 1))
        .unwrap()
        .context;
    ctx = slots::blue_deposit
        .write(ctx, coin(5, 19, 2))
        .unwrap()
        .context;
    if row["options"]["missingKey"] != true {
        let key = row["options"]["keyByte"].as_u64().unwrap_or(11) as u8;
        ctx = ctx.with_coin_public_key_bytes([key; 32]);
    }
    ctx.set_zswap_output_start(3).unwrap();
    if row["options"]["zeroGas"] == true {
        ctx.gas_limit = Some(
            serde_json::from_value(json!({
                "readTime":0,"computeTime":0,"bytesWritten":0,"bytesDeleted":0
            }))
            .unwrap(),
        );
    }
    assert_eq!(
        state_hex(ctx.query.state.get_ref().clone()),
        row["before"],
        "{}",
        row["name"]
    );
    ctx
}

#[test]
fn original_concede_native_matches_corrected_ts_on_seeded_game() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/coracle-concede.json"
    ))
    .unwrap();
    for row in fixture["rows"].as_array().unwrap() {
        let witness = Witness::new(row);
        let result = c::concede(context(row), &witness);
        let recorded_witness = Witness::new(row);
        let recorded = c::recorded::concede(context(row), &recorded_witness);
        if row.get("error").is_some() {
            let error = result.err().unwrap().to_string();
            assert_eq!(
                recorded.err().unwrap().to_string(),
                error,
                "{}",
                row["name"]
            );
            assert_eq!(witness.calls(), row["witnessCalls"], "{}", row["name"]);
            assert_eq!(recorded_witness.calls(), row["witnessCalls"]);
            if row["options"]["missingKey"] == true {
                assert!(
                    error.contains("coin public key"),
                    "{}: {error}",
                    row["name"]
                );
            } else if row["options"]["zeroGas"] == true {
                assert!(error.contains("OutOfGas"), "{}: {error}", row["name"]);
            } else if row["options"]["secretFailure"] == true
                || row["options"]["boardFailure"] == true
            {
                assert_eq!(error.strip_prefix("failed assert: "), row["error"].as_str());
            } else {
                assert_eq!(error, row["error"], "{}", row["name"]);
            }
            continue;
        }
        let out = result.unwrap();
        let recorded = recorded.unwrap();
        assert_eq!(recorded_witness.calls(), row["witnessCalls"]);
        assert_eq!(recorded.execution.result, out.result);
        assert_eq!(
            recorded.execution.context.query.state,
            out.context.query.state
        );
        assert_eq!(
            recorded.execution.context.query.effects,
            out.context.query.effects
        );
        assert_eq!(
            recorded.execution.context.circuit_zswap(),
            out.context.circuit_zswap()
        );
        assert_eq!(
            recorded.execution.context.query.call_context.com_indices,
            out.context.query.call_context.com_indices
        );
        assert_eq!(
            recorded.execution.private_transcript_outputs,
            out.private_transcript_outputs
        );
        assert_eq!(recorded.execution.gas_cost, out.gas_cost);
        assert_eq!(json!(recorded.public.verify_ops()), row["publicTranscript"]);
        let replay = recorded
            .public
            .initial()
            .query(
                recorded.public.verify_ops(),
                None,
                &recorded.execution.context.cost_model,
            )
            .unwrap();
        assert_eq!(replay.context.state, out.context.query.state);
        assert_eq!(replay.context.effects, out.context.query.effects);
        for dimension in ["readTime", "computeTime", "bytesWritten", "bytesDeleted"] {
            assert_eq!(
                json!(replay.gas_cost)[dimension]
                    .as_u64()
                    .unwrap()
                    .to_string(),
                row["replayGas"][dimension]
            );
        }
        assert_eq!(
            state_hex(out.context.query.state.get_ref().clone()),
            row["after"],
            "{}",
            row["name"]
        );
        assert_eq!(
            json!(out.context.query.effects),
            row["effects"],
            "{}",
            row["name"]
        );
        assert_eq!(witness.calls(), row["witnessCalls"], "{}", row["name"]);
        assert_eq!(
            json!({"calls":out.context.private_state.calls}),
            row["privateState"]
        );
        assert_eq!(json!(out.private_transcript_outputs), row["privateOutputs"]);
        assert_eq!(
            json!(runtime::fab::AlignedValue::from(out.result.clone())),
            row["output"]
        );
        assert_eq!(json!(out.result.nonce.0), row["result"]["nonce"]);
        assert_eq!(json!(out.result.color.0), row["result"]["color"]);
        assert_eq!(out.result.value.value().to_string(), row["result"]["value"]);
        assert_eq!(out.context.circuit_zswap().inputs().len(), 1);
        assert_eq!(out.context.circuit_zswap().outputs().len(), 1);
        let plan = out.context.circuit_zswap();
        assert_eq!(plan.next_index().to_string(), row["plan"]["currentIndex"]);
        let mut input = encoded_coin(CoinInfo {
            nonce: plan.inputs()[0].nonce,
            type_: plan.inputs()[0].type_,
            value: plan.inputs()[0].value,
        });
        input["mt_index"] = json!(plan.inputs()[0].mt_index.to_string());
        assert_eq!(input, row["plan"]["inputs"][0], "{}", row["name"]);
        let output = &plan.outputs()[0];
        assert_eq!(
            encoded_coin(output.coin),
            row["plan"]["outputs"][0]["coinInfo"]
        );
        assert_eq!(
            encoded_recipient(&output.recipient),
            row["plan"]["outputs"][0]["recipient"]
        );
        assert_eq!(
            output.provisional_index.to_string(),
            row["comIndices"][0][1]
        );
        let commitment = output.coin.commitment(&output.recipient);
        assert_eq!(hex::encode(commitment.0.0), row["comIndices"][0][0]);
        assert_eq!(
            out.context.query.call_context.com_indices.get(&commitment),
            Some(&output.provisional_index)
        );
        for dimension in ["readTime", "computeTime", "bytesWritten", "bytesDeleted"] {
            let sum: u64 = row["queries"]
                .as_array()
                .unwrap()
                .iter()
                .map(|query| {
                    query["gas"][dimension]
                        .as_str()
                        .unwrap()
                        .parse::<u64>()
                        .unwrap()
                })
                .sum();
            assert_eq!(
                json!(out.gas_cost)[dimension],
                sum,
                "{} {dimension}",
                row["name"]
            );
            assert_eq!(sum.to_string(), row["queryCostSum"][dimension]);
        }
        assert_eq!(row["partition"][0]["programLength"], 42);
        assert!(row["partition"][1].is_null());
    }
}
