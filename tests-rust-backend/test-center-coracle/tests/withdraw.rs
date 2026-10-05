// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
#[path = "../support/withdraw.rs"]
mod support;
use compact_rust_test_center_coracle_fixture::{ledger_contract as c, types};
use midnight_compact_runtime as runtime;
use midnight_onchain_state::state::{
    ContractMaintenanceAuthority, ContractOperation, ContractState, EntryPointBuf,
};
use midnight_storage::storage::HashMap;

use runtime::context::{CircuitContext, CircuitResult};
use runtime::ledger::{DefaultDB, StateValue};
use serde_json::{Value, json};
use support::{Private, Settings, Witness};
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
    let red = row["color"] == "red";
    let phase = match o["phase"].as_u64().unwrap_or(if red { 5 } else { 6 }) {
        0 => types::State::no_game,
        1 => types::State::red_started,
        2 => types::State::blue_started,
        3 => types::State::red_turn,
        4 => types::State::blue_turn,
        5 => types::State::red_wins,
        6 => types::State::blue_wins,
        _ => unreachable!(),
    };
    Settings {
        red,
        both_players: o["bothPlayers"] == true,
        impostor: o["impostor"] == true,
        changed_secret: o["changedSecret"] == true,
        witness_failure: o["witnessFailure"] == true,
        phase,
        key: if o["missingKey"] == true {
            None
        } else {
            Some([o["keyByte"].as_u64().unwrap_or(11) as u8; 32])
        },
    }
}
fn context(row: &Value) -> CircuitContext<Private> {
    let mut ctx = support::seeded(&settings(row)).unwrap();
    if row["options"]["zeroGas"] == true {
        ctx.gas_limit = Some(runtime::context::RunningCost::ZERO);
    }
    assert_eq!(
        state_hex(ctx.query.state.get_ref().clone()),
        row["before"],
        "{}",
        row["name"]
    );
    ctx
}
fn check(out: CircuitResult<Private, types::WithdrawnCoins>, row: &Value) {
    assert_eq!(
        state_hex(out.context.query.state.get_ref().clone()),
        row["before"]
    );
    assert_eq!(row["before"], row["after"]);
    assert_eq!(json!(out.private_transcript_outputs), row["privateOutputs"]);
    assert_eq!(
        json!(runtime::fab::AlignedValue::from(out.result.clone())),
        row["output"]
    );
    assert_eq!(
        json!({"calls":out.context.private_state.calls}),
        row["privateState"]
    );
    let mut actual = json!(out.context.query.effects);
    let mut expected = row["effects"].clone();
    for field in [
        "claimedShieldedSpends",
        "claimedShieldedReceives",
        "claimedNullifiers",
    ] {
        if let Some(a) = actual[field].as_array_mut() {
            a.sort_by_key(Value::to_string);
        }
        if let Some(a) = expected[field].as_array_mut() {
            a.sort_by_key(Value::to_string);
        }
    }
    assert_eq!(actual, expected);
    for dim in ["readTime", "computeTime", "bytesWritten", "bytesDeleted"] {
        let sum: u64 = row["queries"]
            .as_array()
            .unwrap()
            .iter()
            .map(|q| q["gas"][dim].as_str().unwrap().parse::<u64>().unwrap())
            .sum();
        assert_eq!(json!(out.gas_cost)[dim], sum);
    }
    let plan = out.context.circuit_zswap();
    assert_eq!(plan.inputs().len(), 2);
    assert_eq!(plan.outputs().len(), 2);
    assert_eq!(plan.next_index(), 5);
    for (i, coin) in plan.inputs().iter().enumerate() {
        let expected = &row["plan"]["inputs"][i];
        assert_eq!(
            json!({"nonce":coin.nonce.0.0,"color":coin.type_.0.0,"value":coin.value.to_string(),"mt_index":coin.mt_index.to_string()}),
            *expected
        );
    }
    for (i, output) in plan.outputs().iter().enumerate() {
        assert_eq!(output.provisional_index, 3 + i as u64);
        let expected = &row["plan"]["outputs"][i];
        assert_eq!(
            json!({"nonce":output.coin.nonce.0.0,"color":output.coin.type_.0.0,"value":output.coin.value.to_string()}),
            expected["coinInfo"]
        );
        let runtime::ledger::CoinRecipient::User(key) = output.recipient else {
            panic!("user recipient expected")
        };
        assert_eq!(json!(key.0.0), expected["recipient"]["left"]["bytes"]);
    }
}
#[test]
fn original_withdraw_native_recorded_matches_independent_ts() {
    withdraw_parity();
}
fn withdraw_parity() {
    let rows: Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/coracle-withdraw.json"
    ))
    .unwrap();
    for row in rows.as_array().unwrap() {
        let witness = Witness::new(settings(row));
        let out = c::withdraw(context(row), &witness);
        let recorded_witness = Witness::new(settings(row));
        let recorded = c::recorded::withdraw(context(row), &recorded_witness);
        assert_eq!(*witness.calls.borrow(), *recorded_witness.calls.borrow());
        assert_eq!(
            json!(*witness.calls.borrow()),
            row["witnessCalls"],
            "{}",
            row["name"]
        );
        if let Some(expected) = row["error"].as_str() {
            let error = out.err().expect("expected rejection");
            assert_eq!(
                error.to_string(),
                recorded.err().expect("recorded rejection").to_string()
            );
            if row["options"]["missingKey"] == true {
                assert!(matches!(error, runtime::CompactError::MissingCoinPublicKey));
                assert!(expected.contains("undefined"));
            } else if row["options"]["zeroGas"] == true {
                assert!(error.to_string().contains("OutOfGas"));
            } else if row["options"]["witnessFailure"] == true {
                assert!(error.to_string().contains(expected));
            } else {
                assert_eq!(error.to_string(), expected);
            }
        } else {
            check(out.unwrap(), row);
            let recorded = recorded.unwrap();
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
            assert_eq!(replay.context.state, recorded.execution.context.query.state);
            assert_eq!(
                replay.context.effects,
                recorded.execution.context.query.effects
            );
            check(recorded.execution, row);
        }
    }
}
