// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

use compact_rust_stateful_struct_oracle_fixture::{ledger_contract as c, pure_circuits, types};
use midnight_base_crypto::{fab::AlignedValue, hash::HashOutput};
use midnight_compact_runtime as runtime;
use midnight_onchain_state::state::{
    ContractMaintenanceAuthority, ContractOperation, ContractState, EntryPointBuf,
};
use midnight_storage::storage::HashMap;
use runtime::context::{CircuitResult, ConstructorContext, WitnessContext};
use runtime::ledger::{ContractAddress, DefaultDB, StateValue};
use serde_json::{Value, json};

fn bytes(n: u8) -> [u8; 32] {
    let mut b = [0; 32];
    b[0] = n;
    b
}
struct Witnesses;
impl c::Witnesses<Vec<u8>> for Witnesses {
    fn next_value(
        &self,
        ctx: WitnessContext<'_, Vec<u8>, c::LedgerView<'_>>,
        tag: runtime::BoundedUint<255>,
    ) -> (Vec<u8>, runtime::BoundedUint<18446744073709551615>) {
        assert_eq!(ctx.contract_address.0.0, bytes(9));
        let value = tag.value() * 10 + ctx.private_state.len() as u128;
        let mut p = ctx.private_state.clone();
        p.push(tag.value() as u8);
        (p, runtime::BoundedUint::new(value).unwrap())
    }
}
fn state_hex(state: StateValue<DefaultDB>) -> String {
    let operations = ["snapshot", "reverse", "nested", "planned"]
        .into_iter()
        .fold(HashMap::new(), |a, n| {
            a.insert(
                EntryPointBuf(n.as_bytes().to_vec()),
                ContractOperation::new(None),
            )
        });
    let s = ContractState::new(state, operations, ContractMaintenanceAuthority::default());
    let mut out = vec![];
    midnight_serialize::tagged_serialize(&s, &mut out).unwrap();
    hex::encode(out)
}
fn snapshot(s: &types::Snapshot) -> Value {
    json!({"first":s.first.value().to_string(),"address":{"bytes":s.address.bytes.0},"second":s.second.value().to_string()})
}
fn check<T: Clone + Into<AlignedValue>>(
    out: CircuitResult<Vec<u8>, T>,
    result: Value,
    row: &Value,
) {
    assert_eq!(result, row["result"]);
    assert_eq!(
        json!(Into::<AlignedValue>::into(out.result.clone())),
        row["output"]
    );
    assert_eq!(
        state_hex(out.context.query.state.get_ref().clone()),
        row["after"]
    );
    assert_eq!(json!(out.context.private_state), row["privateState"]);
    assert_eq!(json!(out.private_transcript_outputs), row["privateOutputs"]);
    assert_eq!(json!(out.context.query.effects), row["effects"]);
    let cost = json!(out.gas_cost);
    for dim in ["readTime", "computeTime", "bytesWritten", "bytesDeleted"] {
        let sum: u64 = row["queries"]
            .as_array()
            .unwrap()
            .iter()
            .map(|q| q["gas"][dim].as_str().unwrap().parse::<u64>().unwrap())
            .sum();
        assert_eq!(cost[dim], sum);
        assert_eq!(cost[dim].as_u64().unwrap().to_string(), row["gas"][dim]);
    }
    let plan = out.context.circuit_zswap();
    assert_eq!(plan.next_index().to_string(), row["plan"]["currentIndex"]);
    assert!(plan.inputs().is_empty());
    assert_eq!(
        plan.outputs().len(),
        row["plan"]["outputs"].as_array().unwrap().len()
    );
    for o in plan.outputs() {
        assert_eq!(o.provisional_index, 7);
        assert_eq!(o.coin.nonce.0.0, bytes(7));
        assert_eq!(o.coin.type_.0.0, bytes(8));
        assert_eq!(o.coin.value, 42);
        assert_eq!(
            o.recipient,
            runtime::ledger::CoinRecipient::Contract(ContractAddress(HashOutput(bytes(9))))
        );
        assert_eq!(
            *out.context
                .query
                .call_context
                .com_indices
                .get(&o.coin.commitment(&o.recipient))
                .unwrap(),
            7
        );
    }
}
#[test]
fn ordered_struct_members_match_independent_typescript() {
    let rows: Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/stateful-struct-oracle.json"
    ))
    .unwrap();
    for row in rows.as_array().unwrap() {
        let mut ctx = c::initial_state(ConstructorContext::new(vec![]))
            .unwrap()
            .into_circuit_context(ContractAddress(HashOutput(bytes(9))));
        ctx.set_zswap_output_start(7).unwrap();
        assert_eq!(state_hex(ctx.query.state.get_ref().clone()), row["before"]);
        match row["name"].as_str().unwrap() {
            "snapshot" => {
                let out = c::snapshot(ctx, &Witnesses, row["selected"].as_bool().unwrap()).unwrap();
                let v = snapshot(&out.result);
                check(out, v, row);
            }
            "reverse" => {
                let out = c::reverse(ctx, &Witnesses).unwrap();
                let v = snapshot(&out.result);
                check(out, v, row);
            }
            "nested" => {
                let out = c::nested(ctx, &Witnesses).unwrap();
                let v = json!({"head":snapshot(&out.result.head),"tail":out.result.tail.value().to_string()});
                check(out, v, row);
            }
            "planned" => {
                let coin = types::ShieldedCoinInfo {
                    nonce: runtime::FixedBytes::new(bytes(7)),
                    color: runtime::FixedBytes::new(bytes(8)),
                    value: runtime::BoundedUint::new(42).unwrap(),
                };
                let recipient = types::Either {
                    is_left: false,
                    left: types::ZswapCoinPublicKey {
                        bytes: runtime::FixedBytes::new(bytes(0)),
                    },
                    right: types::ContractAddress {
                        bytes: runtime::FixedBytes::new(bytes(9)),
                    },
                };
                let out = c::planned(ctx, &Witnesses, coin, recipient).unwrap();
                let v = json!({"first":out.result.first.value().to_string(),"emitted":[],"address":{"bytes":out.result.address.bytes.0},"after":out.result.after.value().to_string()});
                check(out, v, row);
            }
            "pure_snapshot" => {
                let out = pure_circuits::pure_snapshot(
                    runtime::BoundedUint::new(9).unwrap(),
                    runtime::BoundedUint::new(u64::MAX as u128).unwrap(),
                );
                assert_eq!(snapshot(&out.unwrap()), row["result"]);
                assert!(row["gas"].is_null());
            }
            _ => unreachable!(),
        }
    }
}
