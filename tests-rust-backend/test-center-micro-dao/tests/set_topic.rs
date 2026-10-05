// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
#[path = "../support/advance.rs"]
mod support;
use compact_rust_test_center_micro_dao_fixture::{ledger_contract as c, types};
use midnight_compact_runtime as runtime;
use runtime::{BoundedUint, FixedBytes};
use serde_json::{Value, json};

fn fixture() -> Value {
    serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/micro-dao-set-topic.json"
    ))
    .unwrap()
}
fn mode(row: &Value) -> support::Mode {
    match row["options"]["mode"].as_str() {
        Some("wrong") => support::Mode::Wrong,
        Some("failure") => support::Mode::Failure,
        Some("malformed") => support::Mode::Malformed,
        _ => support::Mode::Normal,
    }
}
fn context(row: &Value) -> runtime::context::CircuitContext<support::Private> {
    let state: runtime::ledger::ContractState<runtime::ledger::DefaultDB> =
        midnight_serialize::tagged_deserialize(
            &mut hex::decode(row["before"].as_str().unwrap())
                .unwrap()
                .as_slice(),
        )
        .unwrap();
    let mut context = runtime::context::CircuitContext::from_contract_state(
        support::Private { calls: 0 },
        runtime::ledger::ContractAddress::default(),
        &state,
    );
    context.set_zswap_output_start(2).unwrap();
    context
}
fn coin(row: &Value) -> types::ShieldedCoinInfo {
    types::ShieldedCoinInfo {
        nonce: FixedBytes::new([3; 32]),
        color: FixedBytes::new([row["options"]["color"].as_u64().unwrap_or(0) as u8; 32]),
        value: BoundedUint::new(
            row["options"]["value"]
                .as_str()
                .unwrap_or("10")
                .parse()
                .unwrap(),
        )
        .unwrap(),
    }
}
fn effects(mut value: Value) -> Value {
    for key in [
        "claimedNullifiers",
        "claimedShieldedReceives",
        "claimedShieldedSpends",
    ] {
        value[key]
            .as_array_mut()
            .unwrap()
            .sort_by_key(|v| v.as_str().unwrap().to_owned());
    }
    value
}
#[test]
fn original_set_topic_native_preserves_independent_typescript_cases() {
    let data = fixture();
    let rows = data["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 16);
    for row in rows {
        let witness = support::Witness::new(mode(row));
        let result = c::set_topic(
            context(row),
            &witness,
            row["options"]["topic"]
                .as_str()
                .unwrap_or("Proposal 🗳️")
                .into(),
            types::ZswapCoinPublicKey {
                bytes: FixedBytes::new([5; 32]),
            },
            coin(row),
        );
        assert_eq!(
            json!(*witness.calls.borrow()),
            row["witnessCalls"],
            "{}",
            row["name"]
        );
        if let Some(error) = row.get("error") {
            let actual = result.err().expect("TS rejection must reject");
            if row["name"] == "mergeOverflow" {
                assert!(matches!(
                    actual,
                    runtime::CompactError::InvalidUnsignedValue
                ));
            } else if row["name"] == "witnessFailure" {
                assert!(matches!(actual, runtime::CompactError::AssertionFailed(_)));
            } else if row["name"] == "witnessMalformed" {
                assert!(matches!(
                    actual,
                    runtime::CompactError::InvalidLedgerCell(_)
                ));
            } else {
                let runtime::CompactError::AssertionFailed(message) = actual else {
                    panic!("{actual:?}")
                };
                assert!(error.as_str().unwrap().contains(&message));
            }
            assert!(row.get("privateOutputs").is_none());
            continue;
        }
        let result = result.unwrap();
        let expected: runtime::ledger::ContractState<runtime::ledger::DefaultDB> =
            midnight_serialize::tagged_deserialize(
                &mut hex::decode(row["after"].as_str().unwrap())
                    .unwrap()
                    .as_slice(),
            )
            .unwrap();
        assert_eq!(
            result.context.query.state.get_ref(),
            expected.data.get_ref()
        );
        assert_eq!(result.context.private_state.calls, 1);
        assert_eq!(
            json!(result.private_transcript_outputs),
            row["privateOutputs"]
        );
        assert_eq!(
            effects(json!(result.context.query.effects)),
            effects(row["effects"].clone())
        );
        for dimension in ["readTime", "computeTime", "bytesWritten", "bytesDeleted"] {
            assert_eq!(
                json!(result.gas_cost)[dimension]
                    .as_u64()
                    .unwrap()
                    .to_string(),
                row["queryCostSum"][dimension]
            );
        }
        let occupied = row["options"]["occupied"] == true;
        assert_eq!(
            result.context.circuit_zswap().inputs().len(),
            if occupied { 2 } else { 0 }
        );
        assert_eq!(
            result.context.circuit_zswap().outputs().len(),
            if occupied { 2 } else { 1 }
        );
        let partition = row["partition"]
            .as_array()
            .expect("actual pinned ledger partition");
        assert!(partition[usize::from(!occupied)].is_null());
        assert_eq!(
            row["publicTranscript"].as_array().unwrap().len(),
            if occupied { 71 } else { 44 }
        );
    }
}
