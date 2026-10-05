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

use compact_rust_shielded_receive_oracle_fixture::{ledger_contract, types};
use midnight_compact_runtime as runtime;
use runtime::context::{CircuitContext, ConstructorContext};
use runtime::ledger::{CoinRecipient, ContractAddress, DefaultDB, HashOutput};
use runtime::{BoundedUint, FixedBytes};
use serde_json::{Value, json};

fn bytes(first: u8) -> [u8; 32] {
    let mut bytes = [0; 32];
    bytes[0] = first;
    bytes
}
fn coin(row: &Value) -> types::ShieldedCoinInfo {
    types::ShieldedCoinInfo {
        nonce: FixedBytes::new(bytes(row["nonce"].as_u64().unwrap() as u8)),
        color: FixedBytes::new(bytes(row["color"].as_u64().unwrap() as u8)),
        value: BoundedUint::new(row["value"].as_str().unwrap().parse().unwrap()).unwrap(),
    }
}
fn initial(row: &Value) -> CircuitContext<()> {
    let address = ContractAddress(HashOutput(
        bytes(row["addressByte"].as_u64().unwrap() as u8),
    ));
    let mut context = ledger_contract::initial_state(ConstructorContext::new(()))
        .unwrap()
        .into_circuit_context(address)
        .with_coin_public_key_bytes(bytes(7));
    context
        .set_zswap_output_start(row["start"].as_str().unwrap().parse().unwrap())
        .unwrap();
    context
}
fn fixture() -> Value {
    serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/shielded-receive-oracle.json"
    ))
    .unwrap()
}

#[test]
fn unchanged_receive_matches_typescript_native_recorded_and_replay() {
    for row in fixture()["rows"].as_array().unwrap() {
        let invoke = |context, coin| match row["name"].as_str().unwrap() {
            "accept" => ledger_contract::accept(context, coin),
            "accept_renamed" => ledger_contract::accept_renamed(context, coin),
            _ => unreachable!(),
        };
        let record = |context, coin| match row["name"].as_str().unwrap() {
            "accept" => ledger_contract::recorded::accept(context, coin),
            "accept_renamed" => ledger_contract::recorded::accept_renamed(context, coin),
            _ => unreachable!(),
        };
        let native = invoke(initial(row), coin(row)).unwrap();
        let recorded = record(initial(row), coin(row)).unwrap();
        assert_eq!(native.result, ());
        assert_eq!(recorded.execution.result, ());
        assert_eq!(row["result"], json!([]));
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
            json!(native.private_transcript_outputs),
            row["privateOutputs"]
        );
        assert_eq!(json!(native.context.query.effects), row["effects"]);
        assert_eq!(json!(recorded.public.verify_ops()), row["publicTranscript"]);
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
        assert_eq!(row["queries"].as_array().unwrap().len(), 2);
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
            assert_eq!(json!(native.gas_cost)[dimension], sum);
        }
        // The current generated TypeScript wrapper reports only its last
        // nested query cost. That omission is preserved as an oracle boundary.
        assert_eq!(row["gas"], row["queries"][1]["gas"]);
        assert_ne!(
            row["gas"]["computeTime"]
                .as_str()
                .unwrap()
                .parse::<u64>()
                .unwrap(),
            json!(native.gas_cost)["computeTime"].as_u64().unwrap(),
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
        assert!(plan.inputs().is_empty());
        assert_eq!(plan.outputs().len(), 1);
        assert_eq!(plan.next_index().to_string(), row["plan"]["currentIndex"]);
        let output = &plan.outputs()[0];
        assert_eq!(output.provisional_index.to_string(), row["start"]);
        assert_eq!(
            output.coin.nonce.0.0,
            bytes(row["nonce"].as_u64().unwrap() as u8)
        );
        assert_eq!(
            output.coin.type_.0.0,
            bytes(row["color"].as_u64().unwrap() as u8)
        );
        assert_eq!(output.coin.value.to_string(), row["value"]);
        let recipient = CoinRecipient::Contract(ContractAddress(HashOutput(bytes(
            row["addressByte"].as_u64().unwrap() as u8,
        ))));
        assert_eq!(output.recipient, recipient);
        let commitment = output.coin.commitment(&recipient);
        assert_eq!(hex::encode(commitment.0.0), row["commitmentHex"]);
        assert_eq!(
            native
                .context
                .query
                .call_context
                .com_indices
                .get(&commitment),
            Some(&output.provisional_index)
        );
        assert_eq!(
            native.context.query.call_context.com_indices.iter().count(),
            row["comIndices"].as_array().unwrap().len()
        );
    }
}

#[test]
fn wide_coin_values_preserve_ledger_u128_despite_typescript_intent_limit() {
    let data = fixture();
    for row in data["overRuntimeLimit"].as_array().unwrap() {
        assert_eq!(row["rejected"], true);
        assert!(row["message"].as_str().unwrap().contains("b32b32b8"));
        let mut source = data["rows"][2].clone();
        source["value"] = row["value"].clone();
        let native = ledger_contract::accept(initial(&source), coin(&source)).unwrap();
        let recorded = ledger_contract::recorded::accept(initial(&source), coin(&source)).unwrap();
        assert_eq!(
            native.context.query.state,
            recorded.execution.context.query.state
        );
        assert_eq!(
            native.context.query.effects,
            recorded.execution.context.query.effects
        );
        let output = &native.context.circuit_zswap().outputs()[0];
        let commitment = output.coin.commitment(&output.recipient);
        assert_eq!(hex::encode(commitment.0.0), row["pureCommitmentHex"]);
        assert_eq!(
            recorded
                .execution
                .context
                .query
                .call_context
                .com_indices
                .get(&commitment),
            Some(&13)
        );
    }
}
