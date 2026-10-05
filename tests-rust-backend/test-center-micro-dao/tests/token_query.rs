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
use compact_rust_test_center_micro_dao_fixture::{ledger_contract as c, types};
use midnight_base_crypto::{fab::AlignedValue, hash::HashOutput};
use midnight_compact_runtime as runtime;
use midnight_onchain_state::state::{
    ContractMaintenanceAuthority, ContractOperation, ContractState, EntryPointBuf,
};
use midnight_storage::storage::HashMap;
use runtime::context::{CircuitResult, ConstructorContext};
use runtime::ledger::{ContractAddress, DefaultDB, StateValue};
use serde_json::{Value, json};
fn bytes(n: u8) -> [u8; 32] {
    let mut b = [0; 32];
    b[0] = n;
    b
}
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
    .fold(HashMap::new(), |a, n| {
        a.insert(
            EntryPointBuf(n.as_bytes().to_vec()),
            ContractOperation::new(None),
        )
    });
    let state = ContractState::new(state, operations, ContractMaintenanceAuthority::default());
    let mut out = vec![];
    midnight_serialize::tagged_serialize(&state, &mut out).unwrap();
    hex::encode(out)
}
fn check(out: CircuitResult<Vec<u8>, runtime::FixedBytes<32>>, row: &Value) {
    assert_eq!(json!(out.result.0), row["result"]);
    assert_eq!(json!(AlignedValue::from(out.result)), row["output"]);
    assert_eq!(
        state_hex(out.context.query.state.get_ref().clone()),
        row["after"]
    );
    assert_eq!(json!(out.context.query.effects), row["effects"]);
    assert_eq!(json!(out.context.private_state), row["privateState"]);
    assert_eq!(json!(out.private_transcript_outputs), row["privateOutputs"]);
    assert!(out.private_transcript_outputs.is_empty());
    assert!(out.context.circuit_zswap().inputs().is_empty());
    assert!(out.context.circuit_zswap().outputs().is_empty());
    let cost = json!(out.gas_cost);
    for dim in ["readTime", "computeTime", "bytesWritten", "bytesDeleted"] {
        let sum: u64 = row["queries"]
            .as_array()
            .unwrap()
            .iter()
            .map(|q| q["gas"][dim].as_str().unwrap().parse::<u64>().unwrap())
            .sum();
        assert_eq!(cost[dim], sum);
        assert_eq!(sum.to_string(), row["gas"][dim]);
    }
}
#[test]
fn original_micro_dao_token_query_matches_ts_native_recorded_and_upstream() {
    let rows: Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/micro-dao-token-query.json"
    ))
    .unwrap();
    let mut tokens = std::collections::HashSet::new();
    for row in rows.as_array().unwrap() {
        let address = ContractAddress(HashOutput(bytes(row["address"].as_u64().unwrap() as u8)));
        let make_context = || {
            c::initial_state(
                ConstructorContext::new(vec![77]),
                runtime::FixedBytes::new(bytes(4)),
                types::Costs {
                    seed_dust: runtime::BoundedUint::new(10).unwrap(),
                    buy_in_dust: runtime::BoundedUint::new(3).unwrap(),
                },
            )
            .unwrap()
            .into_circuit_context(address)
        };
        let ctx = make_context();
        assert_eq!(state_hex(ctx.query.state.get_ref().clone()), row["before"]);
        let native = c::dao_voting_token(ctx).unwrap();
        let mut domain = [0; 32];
        domain[..16].copy_from_slice(b"dao_voting_token");
        assert_eq!(
            native.result.0,
            address.custom_shielded_token_type(HashOutput(domain)).0.0
        );
        assert!(tokens.insert(native.result.0));
        check(native, row);
        let recorded = c::recorded::dao_voting_token(make_context()).unwrap();
        assert!(!recorded.public.verify_ops().is_empty());
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
