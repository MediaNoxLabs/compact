// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use compact_rust_wide_add_oracle_fixture::{ledger_contract as c, pure_circuits as p};
use midnight_compact_runtime as runtime;
use midnight_onchain_state::state::{ContractMaintenanceAuthority, ContractState};
use midnight_storage::storage::HashMap;
use runtime::context::{ConstructorContext, WitnessContext};
use runtime::{BoundedUint, WideUint};
use serde_json::{Value, json};
fn pad(bytes: &[u8]) -> Vec<u8> {
    let mut v = bytes.to_vec();
    v.resize(32, 0);
    v
}
struct Witnesses(u128, u128);
impl c::Witnesses<Vec<u8>> for Witnesses {
    fn next_value(
        &self,
        ctx: WitnessContext<'_, Vec<u8>, c::LedgerView<'_>>,
        tag: BoundedUint<255>,
    ) -> (Vec<u8>, BoundedUint<{ u128::MAX }>) {
        let mut private = ctx.private_state.clone();
        private.push(tag.value() as u8);
        (
            private,
            BoundedUint::new(if tag.value() == 1 { self.0 } else { self.1 }).unwrap(),
        )
    }
}
fn state_hex(state: runtime::ledger::StateValue<runtime::ledger::DefaultDB>) -> String {
    let s = ContractState::new(
        state,
        HashMap::new(),
        ContractMaintenanceAuthority::default(),
    );
    let mut out = vec![];
    midnight_serialize::tagged_serialize(&s, &mut out).unwrap();
    hex::encode(out)
}
#[test]
fn native_wide_add_matches_independent_typescript_values_and_checked_errors() {
    let rows: Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/wide-add-oracle.json"
    ))
    .unwrap();
    for row in rows.as_array().unwrap() {
        let name = row["name"].as_str().unwrap();
        if name == "witnessed" {
            let a = row["a"].as_str().unwrap().parse().unwrap();
            let b = row["b"].as_str().unwrap().parse().unwrap();
            let ctx = c::initial_state(ConstructorContext::new(vec![]))
                .unwrap()
                .into_circuit_context(runtime::ledger::ContractAddress::default());
            assert_eq!(state_hex(ctx.query.state.get_ref().clone()), row["before"]);
            let out =
                c::witnessed(ctx, &Witnesses(a, b), row["selected"].as_bool().unwrap()).unwrap();
            assert_eq!(json!(pad(out.result.as_le_bytes())), row["bytes"]);
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
            let gas = json!(out.gas_cost);
            for dim in ["readTime", "computeTime", "bytesWritten", "bytesDeleted"] {
                assert_eq!(gas[dim], 0);
                assert_eq!(row["gas"][dim], "0");
            }
            continue;
        }
        let b = row["b"].as_str().unwrap().parse::<u128>().unwrap();
        let result = match name {
            "add128" => p::add128(
                BoundedUint::new(row["a"].as_str().unwrap().parse().unwrap()).unwrap(),
                BoundedUint::new(b).unwrap(),
            )
            .map(|x| pad(x.as_le_bytes())),
            "merge128" => p::merge128(
                BoundedUint::new(row["a"].as_str().unwrap().parse().unwrap()).unwrap(),
                BoundedUint::new(b).unwrap(),
            )
            .map(|x| pad(&x.value().to_le_bytes())),
            "mixed" | "narrow129" => {
                let bytes: Vec<u8> = serde_json::from_value(row["aBytes"].clone()).unwrap();
                let a = WideUint::<1, { u128::MAX }>::from_le_bytes(&bytes).unwrap();
                if name == "mixed" {
                    p::mixed(a, BoundedUint::new(b).unwrap()).map(|x| pad(x.as_le_bytes()))
                } else {
                    p::narrow129(a, BoundedUint::new(b).unwrap()).map(|x| pad(x.as_le_bytes()))
                }
            }
            _ => unreachable!(),
        };
        if row.get("error").is_some() {
            assert!(result.is_err(), "{row}");
        } else {
            assert_eq!(json!(result.unwrap()), row["bytes"], "{row}");
        }
    }
}
