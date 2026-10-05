// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
#[path = "../support/reveal.rs"]
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
use support::{Mode, Private, Witness};
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
fn context(row: &Value) -> CircuitContext<Private> {
    let round = row["round"].as_str().unwrap().parse::<u64>().unwrap();
    let vote = row["vote"].as_bool().unwrap();
    let options = &row["options"];
    let tree = if options["noTree"] == true {
        None
    } else {
        Some(if options["crossRound"] == true {
            round - 1
        } else {
            round
        })
    };
    let mut ctx = support::seeded(round, vote, tree).unwrap();
    if options["wrongPhase"] == true {
        ctx = slots::state
            .write(ctx, types::LedgerState::commit)
            .unwrap()
            .context;
    }
    if options["wrongPrivate"] == true {
        ctx.private_state.phase = 0;
    }
    if options["noVote"] == true {
        ctx.private_state.vote = None;
    }
    if options["repeat"] == true || options["duplicate"] == true {
        ctx = c::vote_reveal(ctx, &Witness::new(round, Mode::Normal))
            .unwrap()
            .context;
        if options["duplicate"] == true {
            ctx.private_state = Private {
                phase: 1,
                vote: Some(vote),
                calls: 0,
            };
        }
    }
    if let Some(budget) = options["gasLimit"].as_object() {
        let budget: serde_json::Map<String, Value> = budget
            .iter()
            .map(|(k, v)| {
                (
                    k.clone(),
                    json!(v.as_str().unwrap().parse::<u64>().unwrap()),
                )
            })
            .collect();
        ctx.gas_limit = Some(serde_json::from_value(Value::Object(budget)).unwrap());
    }
    assert_eq!(
        state_hex(ctx.query.state.get_ref().clone()),
        row["before"],
        "{}",
        row["name"]
    );
    ctx
}
fn witness(row: &Value) -> Witness {
    let mode = match row["options"]["mode"].as_str() {
        Some("wrong_root") => Mode::WrongRoot,
        Some("wrong_leaf") => Mode::WrongLeaf,
        Some("malformed") => Mode::Malformed,
        _ => Mode::Normal,
    };
    Witness::new(row["round"].as_str().unwrap().parse().unwrap(), mode)
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
    assert_eq!(json!(runtime::fab::AlignedValue::from(())), row["output"]);
    assert_eq!(row["result"], json!([]));
    assert_eq!(out.private_transcript_outputs.len(), 5);
    assert!(out.context.circuit_zswap().inputs().is_empty());
    assert!(out.context.circuit_zswap().outputs().is_empty());
    let gas = json!(out.gas_cost);
    for dim in ["readTime", "computeTime", "bytesWritten", "bytesDeleted"] {
        let sum: u64 = row["queries"]
            .as_array()
            .unwrap()
            .iter()
            .map(|q| q["gas"][dim].as_str().unwrap().parse::<u64>().unwrap())
            .sum();
        assert_eq!(gas[dim], sum);
        assert_eq!(sum.to_string(), row["queryCostSum"][dim]);
        // The pinned TS wrapper overwrites gasCost with the final query; retain
        // and assert that discrepancy instead of claiming wrapper-gas equality.
        assert_eq!(
            row["reportedGas"][dim],
            row["queries"].as_array().unwrap().last().unwrap()["gas"][dim]
        );
    }
    let state = out.context.query.state.get_ref();
    let yes = slots::yes.inspect(state).unwrap();
    let no = slots::no.inspect(state).unwrap();
    assert_eq!((yes, no), if row["vote"] == true { (1, 0) } else { (0, 1) });
}
#[test]
fn original_reveal_matches_ts_success_and_rejection_prefixes() {
    let rows: Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/micro-dao-reveal.json"
    ))
    .unwrap();
    for row in rows.as_array().unwrap() {
        let wn = witness(row);
        let wr = witness(row);
        let native = c::vote_reveal(context(row), &wn);
        let recorded = c::recorded::vote_reveal(context(row), &wr);
        assert_eq!(*wn.calls.borrow(), *wr.calls.borrow());
        assert_eq!(
            json!(*wr.calls.borrow()),
            row["witnessCalls"],
            "{}",
            row["name"]
        );
        assert_eq!(*wn.arguments.borrow(), *wr.arguments.borrow());
        assert_eq!(json!(*wr.arguments.borrow()), row["pathArgs"]);
        if let Some(error) = row["error"].as_str() {
            let n = native.err().unwrap().to_string();
            let r = recorded.err().unwrap().to_string();
            assert_eq!(n, r);
            if row["name"] == "malformed" {
                assert!(r.contains("Merkle path depth"));
                assert!(error.starts_with("type error:"));
            } else if row["name"] == "zeroGas" || row["name"] == "prefixGas" {
                assert!(r.contains("OutOfGas"), "{r}");
                assert!(error.contains("gas budget"));
            } else {
                assert_eq!(r, error);
            }
            continue;
        }
        check(native.unwrap(), row);
        let recorded = recorded.unwrap();
        assert_eq!(row["queries"].as_array().unwrap().len(), 7);
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
