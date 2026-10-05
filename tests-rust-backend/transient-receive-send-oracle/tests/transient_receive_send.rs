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

use compact_rust_transient_receive_send_oracle_fixture::{ledger_contract, types};
use midnight_compact_runtime as runtime;
use runtime::context::{CircuitContext, ConstructorContext};
use runtime::ledger::{CoinRecipient, ContractAddress, DefaultDB, HashOutput};
use runtime::{BoundedUint, FixedBytes};
use serde_json::{Value, json};

fn bytes(first: u8) -> [u8; 32] {
    let mut value = [0; 32];
    value[0] = first;
    value
}
fn fixture() -> Value {
    serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/transient-receive-send-oracle.json"
    ))
    .unwrap()
}
fn coin(row: &Value) -> types::ShieldedCoinInfo {
    types::ShieldedCoinInfo {
        nonce: FixedBytes::new(bytes(row["nonce"].as_u64().unwrap() as u8)),
        color: FixedBytes::new(bytes(2)),
        value: BoundedUint::new(row["value"].as_str().unwrap().parse().unwrap()).unwrap(),
    }
}
fn recipient(row: &Value) -> types::Either {
    match row["recipientKind"].as_str().unwrap() {
        "user" => types::Either {
            is_left: true,
            left: types::ZswapCoinPublicKey {
                bytes: FixedBytes::new(bytes(7)),
            },
            right: types::ContractAddress::default(),
        },
        kind => types::Either {
            is_left: false,
            left: types::ZswapCoinPublicKey::default(),
            right: types::ContractAddress {
                bytes: FixedBytes::new(bytes(if kind == "self" { 9 } else { 12 })),
            },
        },
    }
}
fn initial(row: &Value) -> CircuitContext<()> {
    let mut context = ledger_contract::initial_state(ConstructorContext::new(()))
        .unwrap()
        .into_circuit_context(ContractAddress(HashOutput(bytes(9))))
        .with_coin_public_key_bytes(bytes(7));
    context
        .set_zswap_output_start(row["start"].as_str().unwrap().parse().unwrap())
        .unwrap();
    context
}
// Upstream effect sets serialize in key order, while JS preserves insertion
// order. Sort only these multiset fields; keep duplicates and raw query order.
fn effect_multisets(mut effects: Value) -> Value {
    for key in [
        "claimedNullifiers",
        "claimedShieldedReceives",
        "claimedShieldedSpends",
    ] {
        effects[key]
            .as_array_mut()
            .unwrap()
            .sort_by_key(|value| value.as_str().unwrap().to_owned());
    }
    effects
}
fn coin_json(coin: &types::ShieldedCoinInfo) -> Value {
    json!({ "nonce": coin.nonce.0, "color": coin.color.0,
        "value": coin.value.value().to_string() })
}

#[test]
fn unchanged_receive_then_full_immediate_send_matches_raw_typescript() {
    for row in fixture()["rows"].as_array().unwrap() {
        let native =
            ledger_contract::receive_then_send(initial(row), coin(row), recipient(row)).unwrap();
        assert!(!native.result.change.is_some);
        assert_eq!(coin_json(&native.result.sent), row["result"]["sent"]);
        assert_eq!(
            coin_json(&native.result.change.value),
            row["result"]["change"]["value"]
        );
        assert_eq!(
            json!(native.private_transcript_outputs),
            row["privateOutputs"]
        );
        assert_eq!(
            effect_multisets(json!(native.context.query.effects)),
            effect_multisets(row["effects"].clone())
        );
        let expected: midnight_onchain_state::state::ContractState<DefaultDB> =
            midnight_serialize::tagged_deserialize(
                &mut hex::decode(row["after"].as_str().unwrap())
                    .unwrap()
                    .as_slice(),
            )
            .unwrap();
        assert_eq!(
            native.context.query.state.get_ref(),
            expected.data.get_ref()
        );
        assert_eq!(row["before"], row["after"]);
        for dimension in ["readTime", "computeTime", "bytesWritten", "bytesDeleted"] {
            assert_eq!(
                json!(native.gas_cost)[dimension]
                    .as_u64()
                    .unwrap()
                    .to_string(),
                row["sumQueryGas"][dimension]
            );
        }
        assert_eq!(
            row["wrapperLastQueryGas"],
            row["queries"].as_array().unwrap().last().unwrap()["gas"]
        );
        assert_ne!(row["sumQueryGas"], row["wrapperLastQueryGas"]);
        assert_eq!(
            row["events"]
                .as_array()
                .unwrap()
                .iter()
                .map(|event| event["kind"].as_str().unwrap())
                .collect::<Vec<_>>(),
            ["output", "input", "output"]
        );
        let plan = native.context.circuit_zswap();
        assert_eq!(plan.inputs().len(), 1);
        assert_eq!(plan.outputs().len(), 2);
        assert_eq!(plan.inputs()[0].mt_index, 0);
        assert_eq!(plan.next_index().to_string(), row["plan"]["currentIndex"]);
        let received = &plan.outputs()[0];
        let sent = &plan.outputs()[1];
        assert_eq!(received.provisional_index.to_string(), row["start"]);
        assert_eq!(sent.provisional_index, received.provisional_index + 1);
        assert_eq!(received.coin.value.to_string(), row["value"]);
        assert_eq!(sent.coin.value, received.coin.value);
        assert_eq!(
            received.recipient,
            CoinRecipient::Contract(ContractAddress(HashOutput(bytes(9))))
        );
        assert_eq!(
            hex::encode(received.coin.commitment(&received.recipient).0.0),
            row["receivedCommitmentHex"]
        );
        assert_eq!(
            hex::encode(sent.coin.commitment(&sent.recipient).0.0),
            row["sentCommitmentHex"]
        );
        assert_eq!(plan.inputs()[0], received.coin.qualify(0));
        for output in plan.outputs() {
            assert_eq!(
                native
                    .context
                    .query
                    .call_context
                    .com_indices
                    .get(&output.coin.commitment(&output.recipient)),
                Some(&output.provisional_index)
            );
        }
        assert_eq!(
            native.context.query.call_context.com_indices.iter().count(),
            2
        );
    }
}
