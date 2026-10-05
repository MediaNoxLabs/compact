// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
#[path = "../support/cash_out.rs"]
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
use support::{Private, Settings};
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
fn number(row: &Value, name: &str, default: u64) -> u64 {
    row["options"][name]
        .as_str()
        .map(|s| s.parse().unwrap())
        .unwrap_or(default)
}
fn context(row: &Value) -> CircuitContext<Private> {
    let o = &row["options"];
    let s = Settings {
        phase: match o["phase"].as_u64().unwrap_or(3) {
            0 => types::LedgerState::setup,
            1 => types::LedgerState::commit,
            2 => types::LedgerState::reveal,
            _ => types::LedgerState::r#final,
        },
        yes: number(row, "yes", 4),
        no: number(row, "no", 3),
        round: number(row, "round", 7),
        value: o["value"]
            .as_str()
            .map(|s| s.parse().unwrap())
            .unwrap_or(99),
        empty_collections: o["emptyCollections"] == true,
        topic: if o["emptyTopic"] == true {
            String::new()
        } else if o["longTopic"] == true {
            "x".repeat(4096)
        } else {
            "Proposal 🗳️".into()
        },
        absent: o["absent"] == true,
        wrong_recipient: o["wrong"] == true,
        pot_flag: o["potFlag"] != false,
        missing_key: o["missingKey"] == true,
    };
    let mut ctx = support::seeded(&s).unwrap();
    if !o["gasLimit"].is_null() {
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
fn check(out: CircuitResult<Private, types::ShieldedCoinInfo>, row: &Value) {
    assert_eq!(
        state_hex(out.context.query.state.get_ref().clone()),
        row["after"],
        "{}",
        row["name"]
    );
    assert_eq!(
        json!(runtime::fab::AlignedValue::from(out.result.clone())),
        row["output"]
    );
    assert_eq!(json!(out.private_transcript_outputs), row["privateOutputs"]);
    assert_eq!(out.private_transcript_outputs.len(), 3);
    assert_eq!(out.context.private_state.calls, 0);
    assert_eq!(row["declaredWitnessCalls"], json!([]));
    assert_eq!(json!(out.context.query.effects), row["effects"]);
    assert_eq!(
        json!({"nonce":out.result.nonce.0,"color":out.result.color.0,"value":out.result.value.value().to_string()}),
        row["result"]
    );
    for dim in ["readTime", "computeTime", "bytesWritten", "bytesDeleted"] {
        assert_eq!(
            json!(out.gas_cost)[dim].as_u64().unwrap().to_string(),
            row["queryCostSum"][dim]
        );
    }
    let plan = out.context.circuit_zswap();
    assert_eq!(plan.inputs().len(), 1);
    assert_eq!(plan.outputs().len(), 1);
    assert_eq!(plan.next_index(), 3);
    let input = &plan.inputs()[0];
    assert_eq!(
        json!({"nonce":input.nonce.0.0,"color":input.type_.0.0,"value":input.value.to_string(),"mt_index":input.mt_index.to_string()}),
        row["plan"]["inputs"][0]
    );
    let output = &plan.outputs()[0];
    assert_eq!(output.provisional_index, 2);
    assert_eq!(
        json!({"nonce":output.coin.nonce.0.0,"color":output.coin.type_.0.0,"value":output.coin.value.to_string()}),
        row["plan"]["outputs"][0]["coinInfo"]
    );
    let runtime::ledger::CoinRecipient::User(key) = output.recipient else {
        panic!("expected user recipient")
    };
    assert_eq!(
        json!(key.0.0),
        row["plan"]["outputs"][0]["recipient"]["left"]["bytes"]
    );
    let state = out.context.query.state.get_ref();
    assert_eq!(
        slots::state.inspect(state).unwrap(),
        types::LedgerState::setup
    );
    assert_eq!(
        slots::topic.inspect(state).unwrap(),
        types::Maybe::default()
    );
    assert_eq!(
        slots::beneficiary.inspect(state).unwrap(),
        types::MaybeCompact1::default()
    );
    assert_eq!(
        slots::pot.inspect(state).unwrap(),
        types::QualifiedShieldedCoinInfo::default()
    );
    assert!(!slots::pot_has_coin.inspect(state).unwrap());
    assert_eq!(slots::yes.inspect(state).unwrap(), 0);
    assert_eq!(slots::no.inspect(state).unwrap(), 0);
    assert_eq!(
        slots::round.inspect(state).unwrap(),
        number(row, "round", 7) + 1
    );
    assert_eq!(
        slots::committed_votes
            .inspect(state)
            .unwrap()
            .first_free()
            .unwrap()
            .value(),
        0
    );
    assert!(
        slots::committed_participants
            .inspect(state)
            .unwrap()
            .is_empty()
    );
    assert!(
        slots::revealed_participants
            .inspect(state)
            .unwrap()
            .is_empty()
    );
}
#[test]
fn original_cash_out_native_matches_twenty_one_pinned_ts_cases() {
    let rows: Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/micro-dao-cash-out.json"
    ))
    .unwrap();
    assert_eq!(rows.as_array().unwrap().len(), 21);
    let mut successes = 0;
    for row in rows.as_array().unwrap() {
        let native = c::cash_out(context(row));
        if let Some(expected) = row["error"].as_str() {
            let err = native.err().expect("expected source rejection");
            let name = row["name"].as_str().unwrap();
            let prefix = row["queries"].as_array().unwrap().len();
            match name {
                "missingKey" | "missingKeyWrongPhase" => {
                    assert_eq!(err, runtime::CompactError::MissingCoinPublicKey);
                    assert_eq!(prefix, 0);
                    assert!(expected.contains("undefined"));
                }
                "zeroGas" => {
                    assert!(err.to_string().contains("OutOfGas"));
                    assert_eq!(prefix, 0);
                }
                "roundMax" => {
                    assert!(err.to_string().contains("ArithmeticOverflow"));
                    assert_eq!(expected, "Error: arithmetic overflow");
                    assert_eq!(prefix, 18);
                    assert_eq!(row["prefixPlan"]["inputs"].as_array().unwrap().len(), 1);
                    assert_eq!(row["prefixPlan"]["outputs"].as_array().unwrap().len(), 1);
                }
                _ => {
                    assert_eq!(err.to_string(), expected);
                    let expected_prefix = match name {
                        "phase0" | "phase1" | "phase2" => 1,
                        "absent" => 2,
                        "wrongBeneficiary" => 3,
                        _ => 5,
                    };
                    assert_eq!(prefix, expected_prefix);
                }
            }
        } else {
            successes += 1;
            check(native.unwrap(), row);
        }
    }
    assert_eq!(successes, 9);
}
