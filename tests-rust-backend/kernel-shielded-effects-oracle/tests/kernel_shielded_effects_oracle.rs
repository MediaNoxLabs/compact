// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//   http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use compact_rust_kernel_shielded_effects_oracle_fixture::ledger_contract;
use midnight_compact_runtime as runtime;
use midnight_onchain_state::state::{
    ContractMaintenanceAuthority, ContractOperation, ContractState, EntryPointBuf,
};
use midnight_storage::storage::HashMap;
use runtime::context::{ConstructorContext, WitnessContext};
use runtime::ledger::{ContractAddress, DefaultDB, StateValue};
use serde_json::{Value, json};
type Amount = runtime::BoundedUint<18446744073709551615>;
fn fixed(v: &Value) -> runtime::FixedBytes<32> {
    runtime::FixedBytes::new(serde_json::from_value(v.clone()).unwrap())
}
fn amount(v: &Value) -> Amount {
    Amount::new(v.as_str().unwrap().parse().unwrap()).unwrap()
}
fn initial() -> runtime::context::CircuitContext<Vec<String>> {
    ledger_contract::initial_state(ConstructorContext::new(vec![]))
        .unwrap()
        .into_circuit_context(ContractAddress::default())
}
fn state_hex(state: StateValue<DefaultDB>) -> String {
    let ops = [
        "mint",
        "nullifier",
        "spend",
        "claim_receive",
        "batch",
        "selected",
        "witness_order",
        "read_state",
    ]
    .into_iter()
    .fold(HashMap::new(), |acc, n| {
        acc.insert(
            EntryPointBuf(n.as_bytes().to_vec()),
            ContractOperation::new(None),
        )
    });
    let state = ContractState::new(state, ops, ContractMaintenanceAuthority::default());
    let mut bytes = vec![];
    midnight_serialize::tagged_serialize(&state, &mut bytes).unwrap();
    hex::encode(bytes)
}
struct Witnesses;
impl ledger_contract::Witnesses<Vec<String>> for Witnesses {
    fn next_domain(
        &self,
        ctx: WitnessContext<'_, Vec<String>, ledger_contract::LedgerView<'_>>,
    ) -> (Vec<String>, runtime::FixedBytes<32>) {
        let mut p = ctx.private_state.clone();
        p.push("domain".into());
        let mut bytes = [0; 32];
        bytes[0] = 3;
        (p, runtime::FixedBytes::new(bytes))
    }
    fn next_amount(
        &self,
        ctx: WitnessContext<'_, Vec<String>, ledger_contract::LedgerView<'_>>,
    ) -> (Vec<String>, Amount) {
        let mut p = ctx.private_state.clone();
        p.push("amount".into());
        (p, Amount::new(13).unwrap())
    }
}
#[test]
fn kernel_effects_match_independent_typescript_and_keep_gas_reporting_distinct() {
    let rows: Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/kernel-shielded-effects-oracle.json"
    ))
    .unwrap();
    for row in rows.as_array().unwrap() {
        let ctx = initial();
        assert_eq!(state_hex(ctx.query.state.get_ref().clone()), row["before"]);
        let args = &row["args"];
        let result = match row["name"].as_str().unwrap() {
            "mint" => ledger_contract::mint(ctx, fixed(&args[0]), amount(&args[1])),
            "nullifier" => ledger_contract::nullifier(ctx, fixed(&args[0])),
            "spend" => ledger_contract::spend(ctx, fixed(&args[0])),
            "claim_receive" => ledger_contract::claim_receive(ctx, fixed(&args[0])),
            "batch" => ledger_contract::batch(
                ctx,
                fixed(&args[0]),
                fixed(&args[1]),
                amount(&args[2]),
                amount(&args[3]),
            ),
            "selected" => {
                ledger_contract::selected(ctx, args[0].as_bool().unwrap(), fixed(&args[1]))
            }
            "witness_order" => ledger_contract::witness_order(ctx, &Witnesses),
            _ => unreachable!(),
        };
        if row["label"] == "overflow" {
            let error = result.err().unwrap();
            assert!(matches!(
                error,
                runtime::CompactError::LedgerQueryRejected(_)
            ));
            assert!(
                row["error"]
                    .as_str()
                    .unwrap()
                    .contains("arithmetic overflow")
            );
            assert_eq!(row["attempts"], 2);
            assert_eq!(row["queries"].as_array().unwrap().len(), 1);
            continue;
        }
        let out = result.unwrap();
        assert_eq!(
            state_hex(out.context.query.state.get_ref().clone()),
            row["after"]
        );
        assert_eq!(
            row["before"], row["after"],
            "Kernel does not modify public contract slots"
        );
        let mut effects = json!(out.context.query.effects);
        for value in effects["shieldedMints"]
            .as_object_mut()
            .unwrap()
            .values_mut()
        {
            *value = json!(value.as_u64().unwrap().to_string());
        }
        assert_eq!(effects, row["effects"]);
        assert_eq!(json!(out.context.private_state), row["privateState"]);
        assert_eq!(json!(out.private_transcript_outputs), row["privateOutputs"]);
        assert_eq!(
            out.context.circuit_zswap(),
            &runtime::CircuitZswapPlan::default()
        );
        assert!(
            out.context
                .query
                .call_context
                .com_indices
                .iter()
                .next()
                .is_none()
        );
        let cost = json!(out.gas_cost);
        let queries = row["queries"].as_array().unwrap();
        for dim in ["readTime", "computeTime", "bytesWritten", "bytesDeleted"] {
            let sum: u64 = queries
                .iter()
                .map(|q| q["gas"][dim].as_str().unwrap().parse::<u64>().unwrap())
                .sum();
            assert_eq!(cost[dim], sum);
            // Original TS currently exposes only the final query's aggregate gas for batch.
            let reported: u64 = row["gas"][dim].as_str().unwrap().parse().unwrap();
            let last = queries
                .last()
                .map(|q| q["gas"][dim].as_str().unwrap().parse::<u64>().unwrap())
                .unwrap_or(0);
            assert_eq!(reported, last);
        }
    }
}
