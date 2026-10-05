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

use compact_rust_shielded_send_oracle_fixture::{ledger_contract, types};
use midnight_compact_runtime as runtime;
use midnight_transient_crypto::hash;
use runtime::context::{CircuitContext, ConstructorContext};
use runtime::ledger::{ContractAddress, DefaultDB, HashOutput};
use runtime::{BoundedUint, FixedBytes};
use serde_json::{Value, json};

fn fixture() -> Value {
    serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/shielded-send-oracle.json"
    ))
    .unwrap()
}

fn bytes(value: &Value) -> [u8; 32] {
    let bytes: Vec<u8> = serde_json::from_value(value.clone()).unwrap();
    bytes.try_into().unwrap()
}

fn coin(row: &Value) -> types::QualifiedShieldedCoinInfo {
    let input = &row["input"];
    types::QualifiedShieldedCoinInfo {
        nonce: FixedBytes::new(bytes(&input["nonce"])),
        color: FixedBytes::new(bytes(&input["color"])),
        value: BoundedUint::new(input["value"].as_str().unwrap().parse().unwrap()).unwrap(),
        mt_index: BoundedUint::new(input["mt_index"].as_str().unwrap().parse().unwrap()).unwrap(),
    }
}

fn recipient(row: &Value) -> types::Either {
    let recipient = &row["recipient"];
    types::Either {
        is_left: recipient["is_left"].as_bool().unwrap(),
        left: types::ZswapCoinPublicKey {
            bytes: FixedBytes::new(bytes(&recipient["left"]["bytes"])),
        },
        right: types::ContractAddress {
            bytes: FixedBytes::new(bytes(&recipient["right"]["bytes"])),
        },
    }
}

fn initial(row: &Value) -> CircuitContext<()> {
    let address = ContractAddress(HashOutput({
        let mut bytes = [0; 32];
        bytes[0] = 9;
        bytes
    }));
    let mut context = ledger_contract::initial_state(ConstructorContext::new(()))
        .unwrap()
        .into_circuit_context(address)
        .with_coin_public_key_bytes({
            let mut bytes = [0; 32];
            bytes[0] = 7;
            bytes
        });
    context
        .set_zswap_output_start(row["start"].as_str().unwrap().parse().unwrap())
        .unwrap();
    context
}

fn amount(row: &Value) -> BoundedUint<{ u128::MAX }> {
    BoundedUint::new(row["sentValue"].as_str().unwrap().parse().unwrap()).unwrap()
}

fn effect_sets(mut effects: Value) -> Value {
    // Upstream ledger serializes claims from sets; TS preserves insertion order.
    // Public transcript order is checked separately and remains exact.
    for key in [
        "claimedNullifiers",
        "claimedShieldedReceives",
        "claimedShieldedSpends",
    ] {
        if let Some(items) = effects[key].as_array_mut() {
            items.sort_by_key(|item| item.as_str().unwrap().to_owned());
        }
    }
    effects
}

fn upstream_nonce(input: FixedBytes<32>, domain: &[u8]) -> FixedBytes<32> {
    let mut field_bytes = [0_u8; 32];
    field_bytes[..domain.len()].copy_from_slice(domain);
    let tag = runtime::Field::from_le_bytes(&field_bytes).unwrap();
    let degraded = hash::degrade_to_transient(HashOutput(input.into_array()));
    FixedBytes::new(hash::upgrade_from_transient(hash::transient_hash(&[tag, degraded])).0)
}

#[test]
fn send_oracle_matches_native_recording_and_replay() {
    let data = fixture();
    for row in data["rows"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|row| row.get("error").is_none())
    {
        let invoke = |context, input| match row["name"].as_str().unwrap() {
            "send_to_self" => ledger_contract::send_to_self(context, input, amount(row)),
            "send_to_user" => ledger_contract::send_to_user(
                context,
                input,
                types::ZswapCoinPublicKey {
                    bytes: recipient(row).left.bytes,
                },
                amount(row),
            ),
            "send_to_contract" => ledger_contract::send_to_contract(
                context,
                input,
                types::ContractAddress {
                    bytes: recipient(row).right.bytes,
                },
                amount(row),
            ),
            "send_forward" => {
                ledger_contract::send_forward(context, input, recipient(row), amount(row))
            }
            _ => unreachable!(),
        };
        let record = |context, input| match row["name"].as_str().unwrap() {
            "send_to_self" => ledger_contract::recorded::send_to_self(context, input, amount(row)),
            "send_to_user" => ledger_contract::recorded::send_to_user(
                context,
                input,
                types::ZswapCoinPublicKey {
                    bytes: recipient(row).left.bytes,
                },
                amount(row),
            ),
            "send_to_contract" => ledger_contract::recorded::send_to_contract(
                context,
                input,
                types::ContractAddress {
                    bytes: recipient(row).right.bytes,
                },
                amount(row),
            ),
            "send_forward" => {
                ledger_contract::recorded::send_forward(context, input, recipient(row), amount(row))
            }
            _ => unreachable!(),
        };
        let native = invoke(initial(row), coin(row)).unwrap();
        let recorded = record(initial(row), coin(row)).unwrap();
        assert_eq!(native.result, recorded.execution.result, "{}", row["name"]);
        assert_eq!(
            native.result.sent.value.value().to_string(),
            row["result"]["sent"]["value"]
        );
        assert_eq!(
            native.result.sent.nonce.into_array(),
            bytes(&row["result"]["sent"]["nonce"])
        );
        assert_eq!(
            native.result.sent.nonce,
            upstream_nonce(coin(row).nonce, b"midnight:kernel:nonce_evolve")
        );
        assert_eq!(
            native.result.sent.color.into_array(),
            bytes(&row["result"]["sent"]["color"])
        );
        assert_eq!(
            native.result.change.is_some,
            row["result"]["change"]["is_some"]
        );
        assert_eq!(
            native.result.change.value.value.value().to_string(),
            row["result"]["change"]["value"]["value"]
        );
        assert_eq!(
            native.result.change.value.nonce.into_array(),
            bytes(&row["result"]["change"]["value"]["nonce"])
        );
        if native.result.change.is_some {
            assert_eq!(
                native.result.change.value.nonce,
                upstream_nonce(coin(row).nonce, b"midnight:kernel:nonce_evolve/2")
            );
        }
        assert_eq!(
            native.result.change.value.color.into_array(),
            bytes(&row["result"]["change"]["value"]["color"])
        );
        assert_eq!(
            native.context.query.state,
            recorded.execution.context.query.state
        );
        assert_eq!(
            native.context.query.effects,
            recorded.execution.context.query.effects
        );
        assert_eq!(native.gas_cost, recorded.execution.gas_cost);
        assert_eq!(
            native.private_transcript_outputs,
            recorded.execution.private_transcript_outputs
        );
        assert_eq!(
            effect_sets(json!(native.context.query.effects)),
            effect_sets(row["effects"].clone())
        );
        assert_eq!(json!(recorded.public.verify_ops()), row["publicTranscript"]);
        assert_eq!(
            json!(native.private_transcript_outputs),
            row["privateOutputs"]
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
                json!(native.gas_cost)[dimension],
                sum,
                "{} {dimension}",
                row["name"]
            );
            assert_eq!(
                json!(replay.gas_cost)[dimension]
                    .as_u64()
                    .unwrap()
                    .to_string(),
                row["replayGas"][dimension]
            );
        }
        let plan = native.context.circuit_zswap();
        assert_eq!(plan, recorded.execution.context.circuit_zswap());
        assert_eq!(plan.inputs().len(), 1);
        assert_eq!(
            plan.outputs().len(),
            row["plan"]["outputs"].as_array().unwrap().len()
        );
        assert_eq!(plan.next_index().to_string(), row["plan"]["currentIndex"]);
        for (actual, expected) in plan
            .outputs()
            .iter()
            .zip(row["plan"]["outputs"].as_array().unwrap())
        {
            assert_eq!(actual.coin.nonce.0.0, bytes(&expected["coinInfo"]["nonce"]));
            assert_eq!(actual.coin.value.to_string(), expected["coinInfo"]["value"]);
            let commitment = actual.coin.commitment(&actual.recipient);
            assert_eq!(
                native
                    .context
                    .query
                    .call_context
                    .com_indices
                    .get(&commitment),
                Some(&actual.provisional_index)
            );
        }
        assert_eq!(
            hex::encode(
                plan.outputs()[0]
                    .coin
                    .commitment(&plan.outputs()[0].recipient)
                    .0
                    .0
            ),
            row["sentCommitmentHex"]
        );
        if plan.outputs().len() == 2 {
            assert_eq!(
                hex::encode(
                    plan.outputs()[1]
                        .coin
                        .commitment(&plan.outputs()[1].recipient)
                        .0
                        .0
                ),
                row["changeCommitmentHex"]
            );
        }
    }
}

#[test]
fn underflow_fails_after_input_claim_and_before_output() {
    for row in fixture()["rows"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|row| row.get("error").is_some())
    {
        let native = match row["name"].as_str().unwrap() {
            "send_to_self" => ledger_contract::send_to_self(initial(row), coin(row), amount(row)),
            "send_to_user" => ledger_contract::send_to_user(
                initial(row),
                coin(row),
                types::ZswapCoinPublicKey {
                    bytes: recipient(row).left.bytes,
                },
                amount(row),
            ),
            _ => unreachable!(),
        };
        let recorded = match row["name"].as_str().unwrap() {
            "send_to_self" => {
                ledger_contract::recorded::send_to_self(initial(row), coin(row), amount(row))
            }
            "send_to_user" => ledger_contract::recorded::send_to_user(
                initial(row),
                coin(row),
                types::ZswapCoinPublicKey {
                    bytes: recipient(row).left.bytes,
                },
                amount(row),
            ),
            _ => unreachable!(),
        };
        assert!(matches!(
            native.err().unwrap(),
            runtime::CompactError::UnsignedUnderflow
        ));
        assert!(matches!(
            recorded.err().unwrap(),
            runtime::CompactError::UnsignedUnderflow
        ));
        assert_eq!(
            row["queries"].as_array().unwrap().len(),
            if row["name"] == "send_to_self" { 3 } else { 2 }
        );
    }
}
