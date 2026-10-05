// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
#[path = "../support/vote_commit.rs"]
mod support;
use compact_rust_test_center_micro_dao_fixture::{
    ledger_contract as c, ledger_slots as slots, types,
};
use midnight_compact_runtime as runtime;
use midnight_onchain_state::state::{
    ContractMaintenanceAuthority, ContractOperation, ContractState, EntryPointBuf,
};
use midnight_storage::storage::HashMap;
use runtime::context::{CircuitContext, CircuitResult};
use runtime::ledger::{DefaultDB, StateValue};
use serde_json::{Value, json};
use support::{Private, Witness};
fn state_hex(state: StateValue<DefaultDB>) -> String {
    let operations = [
        "vote_commit",
        "vote_reveal",
        "advance",
        "set_topic",
        "buy_in",
        "cash_out",
        "dao_voting_token",
    ]
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
fn round(row: &Value) -> u64 {
    row["options"]["round"]
        .as_str()
        .map(|s| s.parse().unwrap())
        .unwrap_or(7)
}
fn context_coin(row: &Value) -> (CircuitContext<Private>, types::ShieldedCoinInfo) {
    let o = &row["options"];
    let ctx = support::seeded(
        if o["phase"] == 0 {
            types::LedgerState::setup
        } else {
            types::LedgerState::commit
        },
        round(row),
        o["privatePhase"].as_u64().unwrap_or(0) as u8,
        o["duplicate"] == true,
        o["fullTree"] == true,
    )
    .unwrap();
    assert_eq!(
        state_hex(ctx.query.state.get_ref().clone()),
        row["before"],
        "{}",
        row["name"]
    );
    // Separate token-color native call, like the independent TS setup. Its gas is not part of vote_commit.
    let token = c::dao_voting_token(ctx).unwrap();
    let coin = types::ShieldedCoinInfo {
        nonce: runtime::FixedBytes::new([3; 32]),
        color: if o["wrongColor"] == true {
            runtime::FixedBytes::new([0; 32])
        } else {
            token.result
        },
        value: runtime::BoundedUint::new(
            o["value"].as_str().map(|s| s.parse().unwrap()).unwrap_or(1),
        )
        .unwrap(),
    };
    (token.context, coin)
}
fn check(out: CircuitResult<Private, ()>, row: &Value) {
    assert_eq!(
        state_hex(out.context.query.state.get_ref().clone()),
        row["after"]
    );
    assert_eq!(json!(out.context.query.effects), row["effects"]);
    assert_eq!(
        json!({"phase":out.context.private_state.phase,"vote":out.context.private_state.vote,"calls":out.context.private_state.calls}),
        row["privateState"]
    );
    assert_eq!(json!(out.private_transcript_outputs), row["privateOutputs"]);
    assert_eq!(out.private_transcript_outputs.len(), 7);
    assert_eq!(json!(runtime::fab::AlignedValue::from(())), row["output"]);
    assert_eq!(row["result"], json!([]));
    for dim in ["readTime", "computeTime", "bytesWritten", "bytesDeleted"] {
        assert_eq!(
            json!(out.gas_cost)[dim].as_u64().unwrap().to_string(),
            row["queryCostSum"][dim]
        );
    }
    let state = out.context.query.state.get_ref();
    assert_eq!(
        slots::state.inspect(state).unwrap(),
        types::LedgerState::commit
    );
    assert_eq!(slots::round.inspect(state).unwrap(), round(row));
    assert!(
        slots::committed_participants
            .inspect(state)
            .unwrap()
            .member(support::participant(round(row)))
    );
    let tree = slots::committed_votes.inspect(state).unwrap();
    assert_eq!(tree.first_free().unwrap().value(), 1);
    assert!(
        tree.find_path_for_leaf(support::commitment(
            row["options"]["ballot"] != false,
            round(row)
        ))
        .is_some()
    );
    let plan = out.context.circuit_zswap();
    assert_eq!(plan.inputs().len(), 1);
    assert_eq!(plan.outputs().len(), 2);
    assert_eq!(plan.next_index(), 4);
    let input = &plan.inputs()[0];
    assert_eq!(
        json!({"nonce":input.nonce.0.0,"color":input.type_.0.0,"value":input.value.to_string(),"mt_index":input.mt_index.to_string()}),
        row["plan"]["inputs"][0]
    );
    for (i, output) in plan.outputs().iter().enumerate() {
        assert_eq!(output.provisional_index, 2 + i as u64);
        assert_eq!(
            json!({"nonce":output.coin.nonce.0.0,"color":output.coin.type_.0.0,"value":output.coin.value.to_string()}),
            row["plan"]["outputs"][i]["coinInfo"]
        );
    }
    assert!(
        matches!(plan.outputs()[0].recipient, runtime::ledger::CoinRecipient::Contract(address) if address == out.context.query.address)
    );
    assert!(
        matches!(plan.outputs()[1].recipient, runtime::ledger::CoinRecipient::User(key) if key.0.0 == [0;32])
    );
    assert_eq!(
        row["events"]
            .as_array()
            .unwrap()
            .iter()
            .map(|e| e["kind"].as_str().unwrap())
            .collect::<Vec<_>>(),
        ["output", "input", "output"]
    );
}
#[test]
fn original_vote_commit_native_matches_eighteen_independent_ts_cases() {
    let rows: Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/micro-dao-vote-commit.json"
    ))
    .unwrap();
    assert_eq!(rows.as_array().unwrap().len(), 18);
    let mut successes = 0;
    for row in rows.as_array().unwrap() {
        let w = Witness::new(row["options"]["mode"].as_str().unwrap_or("normal"));
        let (ctx, coin) = context_coin(row);
        let native = c::vote_commit(ctx, &w, row["options"]["ballot"] != false, coin);
        assert_eq!(
            json!(*w.calls.borrow()),
            row["witnessCalls"],
            "{}",
            row["name"]
        );
        if let Some(expected) = row["error"].as_str() {
            let err = native.err().expect("expected source rejection");
            let text = err.to_string();
            match row["name"].as_str().unwrap() {
                "secretMalformed" => {
                    assert!(text.contains("Bytes<32>"));
                    assert!(expected.starts_with("type error:"));
                }
                "fullTree" => {
                    assert!(text.contains("Bounds"), "{text}");
                    assert_eq!(expected, "Error: exceeded structure bounds");
                }
                "stateFailure" | "secretFailure" | "recordFailure" | "advanceFailure" => {
                    assert!(text.contains(expected), "{text}")
                }
                _ => assert_eq!(text, expected),
            }
        } else {
            successes += 1;
            check(native.unwrap(), row);
        }
    }
    assert_eq!(successes, 4);
}
