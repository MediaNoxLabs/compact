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

use compact_rust_canonical_output_order_oracle_fixture::{ledger_contract, types};
use midnight_compact_runtime as runtime;
use midnight_onchain_state::state::{
    ContractMaintenanceAuthority, ContractOperation, ContractState, EntryPointBuf,
};
use midnight_storage::storage::HashMap;
use runtime::context::ConstructorContext;
use runtime::ledger::{CoinInfo, CoinRecipient, ContractAddress, DefaultDB, StateValue};
use serde_json::{Value, json};

fn coin() -> types::ShieldedCoinInfo {
    let mut nonce = [0; 32];
    nonce[0] = 3;
    let mut color = [0; 32];
    color[0] = 2;
    types::ShieldedCoinInfo {
        nonce: runtime::FixedBytes::new(nonce),
        color: runtime::FixedBytes::new(color),
        value: runtime::BoundedUint::new(17).unwrap(),
    }
}

fn change() -> types::ShieldedCoinInfo {
    let mut c = coin();
    c.nonce.0[0] = 4;
    c.value = runtime::BoundedUint::new(25).unwrap();
    c
}
fn recipient(left: bool) -> types::Either {
    let mut user_key = [0; 32];
    user_key[0] = 7;
    types::Either {
        is_left: left,
        left: types::ZswapCoinPublicKey {
            bytes: runtime::FixedBytes::new(if left { user_key } else { [0; 32] }),
        },
        right: types::ContractAddress {
            bytes: runtime::FixedBytes::new([0; 32]),
        },
    }
}

fn initial() -> runtime::context::CircuitContext<Vec<String>> {
    ledger_contract::initial_state(ConstructorContext::new(vec![]))
        .unwrap()
        .into_circuit_context(ContractAddress::default())
}
fn qualified() -> types::QualifiedShieldedCoinInfo {
    let c = coin();
    types::QualifiedShieldedCoinInfo {
        nonce: runtime::FixedBytes::new({
            let mut n = [0; 32];
            n[0] = 1;
            n
        }),
        color: c.color,
        value: runtime::BoundedUint::new(42).unwrap(),
        mt_index: runtime::BoundedUint::new(3).unwrap(),
    }
}
fn state_hex(state: StateValue<DefaultDB>) -> String {
    let operations = ["distribute", "read_coin"]
        .into_iter()
        .fold(HashMap::new(), |acc, name| {
            acc.insert(
                EntryPointBuf(name.as_bytes().to_vec()),
                ContractOperation::new(None),
            )
        });
    let state = ContractState::new(state, operations, ContractMaintenanceAuthority::default());
    let mut bytes = vec![];
    midnight_serialize::tagged_serialize(&state, &mut bytes).unwrap();
    hex::encode(bytes)
}
fn encoded_coin(coin: CoinInfo) -> Value {
    json!({"nonce":coin.nonce.0.0,"color":coin.type_.0.0,"value":coin.value.to_string()})
}
fn encoded_recipient(r: &CoinRecipient) -> Value {
    let (left, user, contract) = match r {
        CoinRecipient::User(key) => (true, key.0.0, [0; 32]),
        CoinRecipient::Contract(addr) => (false, [0; 32], addr.0.0),
    };
    json!({"is_left":left,"left":{"bytes":user},"right":{"bytes":contract}})
}
#[test]
fn two_provisional_outputs_match_independent_typescript() {
    let rows: Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/canonical-output-order-oracle.json"
    ))
    .unwrap();
    for row in rows["rows"].as_array().unwrap() {
        let name = row["name"].as_str().unwrap();
        let left = row["left"].as_bool().unwrap();
        let start = row["start"].as_str().unwrap().parse().unwrap();
        let mut ctx = initial();
        ctx.set_zswap_output_start(start).unwrap();
        assert_eq!(state_hex(ctx.query.state.get_ref().clone()), row["before"]);
        assert_eq!(name, "distribute");
        let bytes = |first| {
            let mut b = [0; 32];
            b[0] = first;
            runtime::FixedBytes::new(b)
        };
        let out = ledger_contract::distribute(
            ctx,
            qualified(),
            coin(),
            change(),
            recipient(left),
            recipient(false),
            bytes(5),
            bytes(6),
            bytes(8),
        )
        .unwrap();
        let mut recorded_context = initial();
        recorded_context.set_zswap_output_start(start).unwrap();
        let recorded = ledger_contract::recorded::distribute(
            recorded_context,
            qualified(),
            coin(),
            change(),
            recipient(left),
            recipient(false),
            bytes(5),
            bytes(6),
            bytes(8),
        )
        .unwrap();
        assert_eq!(
            recorded.execution.context.circuit_zswap(),
            out.context.circuit_zswap()
        );
        assert_eq!(
            recorded.execution.context.query.state.get_ref(),
            out.context.query.state.get_ref()
        );
        assert_eq!(
            recorded.execution.context.query.effects,
            out.context.query.effects
        );
        assert_eq!(
            recorded.execution.context.query.call_context.com_indices,
            out.context.query.call_context.com_indices
        );
        assert_eq!(
            recorded.execution.context.private_state,
            out.context.private_state
        );
        assert_eq!(
            recorded.execution.private_transcript_outputs,
            out.private_transcript_outputs
        );
        assert_eq!(recorded.execution.gas_cost, out.gas_cost);
        let ops: Vec<Value> = row["queries"]
            .as_array()
            .unwrap()
            .iter()
            .flat_map(|q| q["ops"].as_array().unwrap().clone())
            .collect();
        assert_eq!(
            serde_json::to_value(recorded.public.verify_ops()).unwrap(),
            json!(ops)
        );
        let plan = out.context.circuit_zswap();
        assert_eq!(plan.next_index().to_string(), row["plan"]["currentIndex"]);
        let inputs: Vec<Value> = plan
            .inputs()
            .iter()
            .map(|c| {
                let mut v = encoded_coin(CoinInfo {
                    nonce: c.nonce,
                    type_: c.type_,
                    value: c.value,
                });
                v["mt_index"] = json!(c.mt_index.to_string());
                v
            })
            .collect();
        assert_eq!(json!(inputs), row["plan"]["inputs"]);
        let outputs:Vec<Value>=plan.outputs().iter().map(|o|json!({"coinInfo":encoded_coin(o.coin),"recipient":encoded_recipient(&o.recipient)})).collect();
        assert_eq!(json!(outputs), row["plan"]["outputs"]);
        for output in plan.outputs() {
            let key = output.coin.commitment(&output.recipient);
            assert_eq!(
                *out.context
                    .query
                    .call_context
                    .com_indices
                    .get(&key)
                    .unwrap(),
                plan.outputs()
                    .iter()
                    .rfind(|o| o.coin.commitment(&o.recipient) == key)
                    .unwrap()
                    .provisional_index
            );
        }
        assert_eq!(
            state_hex(out.context.query.state.get_ref().clone()),
            row["after"]
        );
        // Claimed spends are an upstream set; serialization iteration differs.
        // Exact execution order is checked above through the public query program.
        let mut actual_effects = json!(out.context.query.effects);
        let mut expected_effects = row["effects"].clone();
        for effects in [&mut actual_effects, &mut expected_effects] {
            effects["claimedShieldedSpends"]
                .as_array_mut()
                .unwrap()
                .sort_by_key(|value| value.as_str().unwrap().to_owned());
        }
        assert_eq!(actual_effects, expected_effects);
        assert_eq!(json!(out.context.private_state), row["privateState"]);
        assert_eq!(json!(out.private_transcript_outputs), row["privateOutputs"]);
        let cost = json!(out.gas_cost);
        for dim in ["readTime", "computeTime", "bytesWritten", "bytesDeleted"] {
            // TS reports the last query cost; Rust accumulates every query.
            assert_eq!(
                row["gas"][dim],
                row["queries"].as_array().unwrap().last().unwrap()["gas"][dim]
            );
            let query_sum: u64 = row["queries"]
                .as_array()
                .unwrap()
                .iter()
                .map(|q| q["gas"][dim].as_str().unwrap().parse::<u64>().unwrap())
                .sum();
            assert_eq!(cost[dim], query_sum);
        }
        assert!(out.context.zswap_state.coins.is_empty());
        assert!(out.context.zswap_state.pending_outputs.is_empty());
        assert_eq!(out.context.zswap_state.first_free, 0);
        if name == "distribute" {
            let read = ledger_contract::read_coin(out.context).unwrap();
            assert_eq!(
                read.result.mt_index.value().to_string(),
                row["qualifiedIndex"]
            );
        }
    }
}
