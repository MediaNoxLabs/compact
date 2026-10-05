// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
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
use runtime::Field;
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
    Settings {
        red: row["color"] == "red",
        board: o["board"].as_str().unwrap().parse().unwrap(),
        empty: o["empty"] == true,
        both_players: o["bothPlayers"] == true,
        first_blue: o["firstBlue"] == true,
        wrong_turn: o["wrongTurn"] == true,
        dead: o["dead"] == true,
        impostor: o["impostor"] == true,
        changed_secret: o["changedSecret"] == true,
        wrong_nonce: o["wrongNonce"] == true,
        invalid_board: o["invalidBoard"] == true,
    }
}
fn context(row: &Value) -> CircuitContext<Private> {
    let s = settings(row);
    let mut ctx = support::seeded(&s).unwrap();
    if row["options"]["repeat"] == true {
        ctx = c::guess(
            ctx,
            &Witness::new(s),
            Field::from(row["guess"].as_str().unwrap().parse::<u64>().unwrap()),
        )
        .unwrap()
        .context;
    }
    if let Some(b) = row["options"]["gasLimit"].as_object() {
        let b: serde_json::Map<String, Value> = b
            .iter()
            .map(|(k, v)| {
                (
                    k.clone(),
                    json!(v.as_str().unwrap().parse::<u64>().unwrap()),
                )
            })
            .collect();
        ctx.gas_limit = Some(serde_json::from_value(Value::Object(b)).unwrap());
    }
    assert_eq!(
        state_hex(ctx.query.state.get_ref().clone()),
        row["before"],
        "{}",
        row["name"]
    );
    ctx
}
fn check(out: CircuitResult<Private, ()>, row: &Value) {
    assert_eq!(
        state_hex(out.context.query.state.get_ref().clone()),
        row["after"]
    );
    assert_eq!(json!(out.context.query.effects), row["effects"]);
    assert_eq!(
        json!({"calls":out.context.private_state.calls}),
        row["privateState"]
    );
    assert_eq!(json!(out.private_transcript_outputs), row["privateOutputs"]);
    assert_eq!(json!(runtime::fab::AlignedValue::from(())), row["output"]);
    assert_eq!(row["result"], json!([]));
    assert_eq!(out.private_transcript_outputs.len(), 3);
    assert!(out.context.circuit_zswap().inputs().is_empty());
    assert!(out.context.circuit_zswap().outputs().is_empty());
    for d in ["readTime", "computeTime", "bytesWritten", "bytesDeleted"] {
        let sum: u64 = row["queries"]
            .as_array()
            .unwrap()
            .iter()
            .map(|q| q["gas"][d].as_str().unwrap().parse::<u64>().unwrap())
            .sum();
        assert_eq!(json!(out.gas_cost)[d], sum);
        assert_eq!(sum.to_string(), row["queryCostSum"][d]);
        assert_eq!(
            row["reportedGas"][d],
            row["queries"].as_array().unwrap().last().unwrap()["gas"][d]
        );
    }
    let state = out.context.query.state.get_ref();
    let guess = slots::last_guess.inspect(state).unwrap();
    assert!(guess.is_some);
    assert_eq!(
        guess.value,
        Field::from(row["guess"].as_str().unwrap().parse::<u64>().unwrap())
    );
    assert_eq!(
        slots::state.inspect(state).unwrap(),
        if row["color"] == "red" {
            types::State::blue_turn
        } else {
            types::State::red_turn
        }
    );
}
#[test]
fn original_guess_native_recorded_matches_ts() {
    let rows: Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/coracle-guess.json"
    ))
    .unwrap();
    for row in rows.as_array().unwrap() {
        let witness = Witness::new(settings(row));
        let out = c::guess(
            context(row),
            &witness,
            Field::from(row["guess"].as_str().unwrap().parse::<u64>().unwrap()),
        );
        let recorded_witness = Witness::new(settings(row));
        let recorded = c::recorded::guess(
            context(row),
            &recorded_witness,
            Field::from(row["guess"].as_str().unwrap().parse::<u64>().unwrap()),
        );
        assert_eq!(*witness.calls.borrow(), *recorded_witness.calls.borrow());
        assert_eq!(
            json!(*witness.calls.borrow()),
            row["witnessCalls"],
            "{}",
            row["name"]
        );
        if let Some(error) = row["error"].as_str() {
            let err = out.err().unwrap().to_string();
            assert_eq!(err, recorded.err().unwrap().to_string());
            if row["name"] == "zeroGas" {
                assert!(err.contains("OutOfGas"));
                assert!(error.contains("gas budget"));
            } else {
                assert_eq!(err, error);
            }
        } else {
            assert_eq!(row["queries"].as_array().unwrap().len(), 8);
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
            for dim in ["readTime", "computeTime", "bytesWritten", "bytesDeleted"] {
                assert_eq!(
                    json!(replay.gas_cost)[dim].as_u64().unwrap().to_string(),
                    row["replayGas"][dim]
                );
            }
            check(recorded.execution, row);
        }
    }
}
