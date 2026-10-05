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
#[path = "../support/advance.rs"]
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
fn number(row: &Value, name: &str, default: u64) -> u64 {
    row["options"][name]
        .as_str()
        .map(|s| s.parse().unwrap())
        .unwrap_or(default)
}
fn context(row: &Value) -> CircuitContext<Private> {
    let o = &row["options"];
    let phase = match o["phase"].as_u64().unwrap_or(3) {
        0 => types::LedgerState::setup,
        1 => types::LedgerState::commit,
        2 => types::LedgerState::reveal,
        _ => types::LedgerState::r#final,
    };
    let mut ctx = support::seeded(
        phase,
        number(row, "yes", 2),
        number(row, "no", 3),
        number(row, "round", 7),
        o["empty"] == true,
        o["pot"] != false,
    )
    .unwrap();
    if let Some(budget) = o["gasLimit"].as_object() {
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
    Witness::new(match row["options"]["mode"].as_str() {
        Some("wrong") => Mode::Wrong,
        Some("failure") => Mode::Failure,
        Some("malformed") => Mode::Malformed,
        _ => Mode::Normal,
    })
}
fn check(out: CircuitResult<Private, ()>, row: &Value) {
    assert_eq!(
        state_hex(out.context.query.state.get_ref().clone()),
        row["after"],
        "{}",
        row["name"]
    );
    assert_eq!(json!(out.context.query.effects), row["effects"]);
    assert_eq!(
        json!({"calls":out.context.private_state.calls}),
        row["privateState"]
    );
    assert_eq!(json!(out.private_transcript_outputs), row["privateOutputs"]);
    assert_eq!(out.private_transcript_outputs.len(), 1);
    assert_eq!(json!(runtime::fab::AlignedValue::from(())), row["output"]);
    assert_eq!(row["result"], json!([]));
    assert!(out.context.circuit_zswap().inputs().is_empty());
    assert!(out.context.circuit_zswap().outputs().is_empty());
    for dim in ["readTime", "computeTime", "bytesWritten", "bytesDeleted"] {
        assert_eq!(
            json!(out.gas_cost)[dim].as_u64().unwrap().to_string(),
            row["queryCostSum"][dim]
        );
        // Pinned TS reports the final query rather than accumulated gas.
        assert_eq!(
            row["reportedGas"][dim],
            row["queries"].as_array().unwrap().last().unwrap()["gas"][dim]
        );
    }
    let state = out.context.query.state.get_ref();
    assert_eq!(slots::pot.inspect(state).unwrap(), support::pot());
    assert_eq!(
        slots::pot_has_coin.inspect(state).unwrap(),
        row["options"]["pot"] != false
    );
    if row["options"]["phase"].is_null() {
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
            slots::committed_votes
                .inspect(state)
                .unwrap()
                .first_free()
                .unwrap()
                .value(),
            0
        );
        assert_eq!(slots::yes.inspect(state).unwrap(), 0);
        assert_eq!(slots::no.inspect(state).unwrap(), 0);
        assert_eq!(
            slots::round.inspect(state).unwrap(),
            number(row, "round", 7) + 1
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
}
#[test]
fn original_advance_native_recorded_and_replay_match_pinned_ts() {
    let rows: Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/micro-dao-advance.json"
    ))
    .unwrap();
    for row in rows.as_array().unwrap() {
        let w = witness(row);
        let native = c::advance(context(row), &w);
        let wr = witness(row);
        let recorded = c::recorded::advance(context(row), &wr);
        assert_eq!(*w.calls.borrow(), *wr.calls.borrow());
        match (&native, &recorded) {
            (Err(n), Err(r)) => assert_eq!(n, r),
            (Ok(_), Ok(_)) => {}
            _ => panic!("native/recorded outcome mismatch: {}", row["name"]),
        }
        assert_eq!(json!(*w.calls.borrow()), row["witnessCalls"]);
        if let Some(error) = row["error"].as_str() {
            let err = native.err().unwrap();
            let text = err.to_string();
            match row["name"].as_str().unwrap() {
                "noMax" => {
                    assert_eq!(
                        err,
                        runtime::CompactError::UnsignedOutOfRange {
                            value: 1u128 << 64,
                            max: u64::MAX as u128
                        }
                    );
                    assert!(
                        error.contains("18446744073709551616 is greater than 18446744073709551615")
                    );
                    assert_eq!(row["queries"].as_array().unwrap().len(), 4);
                }
                "roundMax" => {
                    assert!(text.contains("ArithmeticOverflow"), "{text}");
                    assert_eq!(error, "Error: arithmetic overflow");
                    assert_eq!(row["queries"].as_array().unwrap().len(), 13);
                }
                "witnessFailure" => {
                    assert_eq!(
                        err,
                        runtime::CompactError::AssertionFailed("advance witness refused".into())
                    );
                    assert_eq!(error, "advance witness refused");
                }
                "witnessMalformed" => {
                    assert!(text.contains("Bytes<32>"));
                    assert!(error.starts_with("type error:"));
                }
                "zeroGas" | "prefixGas" => {
                    assert!(text.contains("OutOfGas"));
                    assert!(error.contains("gas budget"));
                }
                _ => assert_eq!(text, error),
            }
        } else {
            check(native.unwrap(), row);
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
