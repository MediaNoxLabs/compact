// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

use compact_rust_shielded_merge_oracle_fixture::{ledger_contract as c, types};
use midnight_compact_runtime as runtime;
use runtime::context::{CircuitContext, CircuitResult, ConstructorContext};
use runtime::ledger::{CoinRecipient, ContractAddress, DefaultDB, HashOutput};
use runtime::{BoundedUint, FixedBytes};
use serde_json::{Value, json};

fn fixture() -> Value {
    serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/shielded-merge-oracle.json"
    ))
    .unwrap()
}
fn bytes(value: &Value) -> FixedBytes<32> {
    FixedBytes::new(serde_json::from_value(value.clone()).unwrap())
}
fn coin(value: &Value) -> types::ShieldedCoinInfo {
    types::ShieldedCoinInfo {
        nonce: bytes(&value["nonce"]),
        color: bytes(&value["color"]),
        value: BoundedUint::new(value["value"].as_str().unwrap().parse().unwrap()).unwrap(),
    }
}
fn qualified(value: &Value) -> types::QualifiedShieldedCoinInfo {
    let coin = coin(value);
    types::QualifiedShieldedCoinInfo {
        nonce: coin.nonce,
        color: coin.color,
        value: coin.value,
        mt_index: BoundedUint::new(value["mt_index"].as_str().unwrap().parse().unwrap()).unwrap(),
    }
}
fn initial() -> CircuitContext<()> {
    let mut address = [0; 32];
    address[0] = 9;
    let mut context = c::initial_state(ConstructorContext::new(()))
        .unwrap()
        .into_circuit_context(ContractAddress(HashOutput(address)));
    context.set_zswap_output_start(2).unwrap();
    context
}
fn native(
    row: &Value,
) -> Result<CircuitResult<(), types::ShieldedCoinInfo>, runtime::CompactError> {
    match row["name"].as_str().unwrap() {
        "merge_qualified" => {
            c::merge_qualified(initial(), qualified(&row["a"]), qualified(&row["b"]))
        }
        "receive_then_merge" => {
            c::receive_then_merge(initial(), qualified(&row["a"]), coin(&row["b"]))
        }
        _ => unreachable!(),
    }
}
fn recorded(
    row: &Value,
) -> Result<
    runtime::recording::RecordedCircuitResult<(), types::ShieldedCoinInfo>,
    runtime::CompactError,
> {
    match row["name"].as_str().unwrap() {
        "merge_qualified" => {
            c::recorded::merge_qualified(initial(), qualified(&row["a"]), qualified(&row["b"]))
        }
        "receive_then_merge" => {
            c::recorded::receive_then_merge(initial(), qualified(&row["a"]), coin(&row["b"]))
        }
        _ => unreachable!(),
    }
}
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
    json!({"nonce": coin.nonce.0, "color": coin.color.0, "value": coin.value.value().to_string()})
}

#[test]
fn native_and_recorded_merge_preserve_typescript_order_bounds_and_failure_precedence() {
    let data = fixture();
    let rows = data["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 20);
    for row in rows {
        let immediate = row["name"] == "receive_then_merge";
        let result = native(row);
        let recorded = recorded(row);
        if let Some(error) = row.get("error") {
            let failure = result.err().expect("TS rejection must reject natively");
            assert_eq!(
                recorded.err().expect("recorded must reject").to_string(),
                failure.to_string()
            );
            if row["a"]["color"] != row["b"]["color"] {
                assert!(
                    matches!(failure, runtime::CompactError::AssertionFailed(ref message) if message == "Can only merge coins of the same color")
                );
                assert!(
                    error
                        .as_str()
                        .unwrap()
                        .contains("Can only merge coins of the same color")
                );
            } else {
                assert!(matches!(
                    failure,
                    runtime::CompactError::InvalidUnsignedValue
                ));
                assert!(
                    error
                        .as_str()
                        .unwrap()
                        .contains("cast from Field or Uint value to smaller Uint value failed")
                );
            }
            // Independent TS evidence exposes successful prefixes, not a
            // returned post-error context or private transcript.
            assert!(row.get("privateOutputs").is_none());
            assert_eq!(
                row["queries"].as_array().unwrap().len(),
                if immediate { 5 } else { 3 }
            );
            let events = row["events"]
                .as_array()
                .unwrap()
                .iter()
                .map(|event| event["kind"].as_str().unwrap())
                .collect::<Vec<_>>();
            assert_eq!(
                events,
                if immediate {
                    vec!["output", "input", "input"]
                } else {
                    vec!["input", "input"]
                }
            );
            continue;
        }
        let native = result.unwrap();
        let recorded = recorded.unwrap();
        assert_eq!(recorded.execution.result, native.result);
        assert_eq!(
            recorded.execution.context.query.state,
            native.context.query.state
        );
        assert_eq!(
            recorded.execution.context.query.effects,
            native.context.query.effects
        );
        assert_eq!(
            recorded.execution.context.circuit_zswap(),
            native.context.circuit_zswap()
        );
        assert_eq!(
            recorded.execution.private_transcript_outputs,
            native.private_transcript_outputs
        );
        assert_eq!(recorded.execution.gas_cost, native.gas_cost);
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
        assert_eq!(replay.context.state, native.context.query.state);
        assert_eq!(replay.context.effects, native.context.query.effects);
        for dimension in ["readTime", "computeTime", "bytesWritten", "bytesDeleted"] {
            assert_eq!(
                json!(replay.gas_cost)[dimension]
                    .as_u64()
                    .unwrap()
                    .to_string(),
                row["wholeProgramReplayGas"][dimension]
            );
        }

        assert_eq!(coin_json(&native.result), row["result"]);
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
        let plan = native.context.circuit_zswap();
        assert_eq!(plan.inputs().len(), 2);
        assert_eq!(plan.outputs().len(), if immediate { 2 } else { 1 });
        assert_eq!(plan.next_index(), if immediate { 4 } else { 3 });
        assert_eq!(plan.inputs()[0].mt_index.to_string(), row["a"]["mt_index"]);
        assert_eq!(
            plan.inputs()[1].mt_index,
            if immediate {
                0
            } else {
                row["b"]["mt_index"].as_str().unwrap().parse().unwrap()
            }
        );
        let output = plan.outputs().last().unwrap();
        assert_eq!(output.coin.value.to_string(), row["result"]["value"]);
        assert_eq!(output.provisional_index, if immediate { 3 } else { 2 });
        assert_eq!(
            output.recipient,
            CoinRecipient::Contract(native.context.query.address)
        );
        assert_eq!(
            native.private_transcript_outputs.len(),
            if immediate { 4 } else { 3 }
        );
        assert_eq!(
            row["publicTranscript"].as_array().unwrap().len(),
            if immediate { 36 } else { 27 }
        );
    }
    for start in [0, 10] {
        // Same value and color, first-input nonce changes under swapping.
        assert_eq!(
            rows[start]["result"]["value"],
            rows[start + 1]["result"]["value"]
        );
        assert_ne!(
            rows[start]["result"]["nonce"],
            rows[start + 1]["result"]["nonce"]
        );
        // Raw execution admits duplicate coin identities; strict offer
        // consumption is a distinct boundary, not normalized here.
        assert!(rows[start + 9].get("error").is_none());
    }
}
