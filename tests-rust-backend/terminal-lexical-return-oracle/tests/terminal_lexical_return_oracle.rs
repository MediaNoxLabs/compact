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
use compact_rust_terminal_lexical_return_oracle_fixture::ledger_contract as c;
use midnight_compact_runtime as runtime;
use midnight_onchain_state::state::{
    ContractMaintenanceAuthority, ContractOperation, ContractState, EntryPointBuf,
};
use midnight_storage::storage::HashMap;
#[path = "../support/terminal.rs"]
mod support;
use serde_json::{Value, json};
use support::Witnesses;

fn state_hex(state: runtime::ledger::StateValue<runtime::ledger::DefaultDB>) -> String {
    let operations = ["two", "three", "echo", "observed", "nested"]
        .into_iter()
        .fold(HashMap::new(), |a, n| {
            a.insert(
                EntryPointBuf(n.as_bytes().to_vec()),
                ContractOperation::new(None),
            )
        });
    let state = ContractState::new(state, operations, ContractMaintenanceAuthority::default());
    let mut bytes = vec![];
    midnight_serialize::tagged_serialize(&state, &mut bytes).unwrap();
    hex::encode(bytes)
}
#[test]
fn terminal_bindings_match_typescript_results_state_gas_and_private_order() {
    let rows: Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/terminal-lexical-return-oracle.json"
    ))
    .unwrap();
    assert_eq!(rows.as_array().unwrap().len(), 13);
    for row in rows.as_array().unwrap() {
        let w = Witnesses::new(
            row["seed"].as_u64().unwrap(),
            row["delta"].as_u64().unwrap(),
        );
        let mut ctx = support::seeded(row["seed"].as_u64().unwrap());
        assert_eq!(state_hex(ctx.query.state.get_ref().clone()), row["before"]);
        if row["zeroGas"] == true {
            ctx.gas_limit = Some(Default::default());
        }
        let result = match row["name"].as_str().unwrap() {
            "two" => c::two(ctx),
            "three" => c::three(ctx),
            "echo" => c::echo(ctx, runtime::Field::from(91)),
            "nested" => c::nested(ctx),
            "observed" => c::observed(ctx, &w, row["reject"].as_bool().unwrap()),
            _ => unreachable!(),
        };
        let wr = Witnesses::new(
            row["seed"].as_u64().unwrap(),
            row["delta"].as_u64().unwrap(),
        );
        let mut ctx = support::seeded(row["seed"].as_u64().unwrap());
        if row["zeroGas"] == true {
            ctx.gas_limit = Some(Default::default());
        }
        let recorded = match row["name"].as_str().unwrap() {
            "two" => c::recorded::two(ctx),
            "three" => c::recorded::three(ctx),
            "echo" => c::recorded::echo(ctx, runtime::Field::from(91)),
            "nested" => c::recorded::nested(ctx),
            "observed" => c::recorded::observed(ctx, &wr, row["reject"].as_bool().unwrap()),
            _ => unreachable!(),
        };
        assert_eq!(*w.trace.borrow(), *wr.trace.borrow());
        match (&result, &recorded) {
            (Err(n), Err(r)) => assert_eq!(n, r),
            (Ok(n), Ok(r)) => {
                assert_eq!(n.result, r.execution.result);
                assert_eq!(n.context.query.state, r.execution.context.query.state);
                assert_eq!(n.context.query.effects, r.execution.context.query.effects);
                assert_eq!(n.context.private_state, r.execution.context.private_state);
                assert_eq!(
                    n.private_transcript_outputs,
                    r.execution.private_transcript_outputs
                );
                assert_eq!(n.gas_cost, r.execution.gas_cost);
                assert_eq!(json!(r.public.verify_ops()), row["publicTranscript"]);
                let replay = r
                    .public
                    .initial()
                    .query(r.public.verify_ops(), None, &r.execution.context.cost_model)
                    .unwrap();
                assert_eq!(replay.context.state, r.execution.context.query.state);
                assert_eq!(replay.context.effects, r.execution.context.query.effects);
            }
            _ => panic!("native/recorded mismatch: {row}"),
        }
        assert_eq!(json!(*w.trace.borrow()), row["trace"]);
        if let Some(error) = row.get("error") {
            let err = match result {
                Err(e) => e,
                Ok(_) => panic!("expected error {row}"),
            };
            if row["zeroGas"] == true {
                assert!(matches!(err, runtime::CompactError::LedgerQueryRejected(_)));
                assert!(err.to_string().to_lowercase().contains("gas"));
                assert!(row["trace"].as_array().unwrap().is_empty());
                assert_eq!(row["events"], json!([{"query":0}]));
            } else {
                assert!(
                    matches!(err,runtime::CompactError::AssertionFailed(ref s) if s=="rejected after witness")
                );
                assert!(error.as_str().unwrap().ends_with("rejected after witness"));
                assert_eq!(row["events"], json!([{"query":0},{"seed":"4","prior":[]}]));
            }
            continue;
        }
        let out = result.unwrap();
        assert_eq!(
            out.result,
            runtime::Field::from(row["result"].as_str().unwrap().parse::<u64>().unwrap())
        );
        assert_eq!(
            json!(runtime::fab::AlignedValue::from(out.result)),
            row["output"]
        );
        assert_eq!(json!(out.context.private_state), row["privateState"]);
        assert_eq!(json!(out.private_transcript_outputs), row["privateOutputs"]);
        assert_eq!(json!(out.context.query.effects), row["effects"]);
        assert_eq!(
            state_hex(out.context.query.state.get_ref().clone()),
            row["after"]
        );
        let queries = row["queries"].as_array().unwrap();
        assert_eq!(queries.len(), 2);
        let expected_events = if row["name"] == "observed" {
            json!([{"query":0},{"seed":row["seed"].as_u64().unwrap().to_string(),"prior":[]},{"query":1}])
        } else {
            json!([{"query":0},{"query":1}])
        };
        assert_eq!(row["events"], expected_events);
        for dim in ["readTime", "computeTime", "bytesWritten", "bytesDeleted"] {
            let sum: u64 = queries
                .iter()
                .map(|q| q["gas"][dim].as_str().unwrap().parse::<u64>().unwrap())
                .sum();
            assert_eq!(json!(out.gas_cost)[dim], sum, "{row}: {dim}");
        }
    }
}
