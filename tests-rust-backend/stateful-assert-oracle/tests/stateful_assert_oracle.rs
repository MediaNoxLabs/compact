// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use compact_rust_stateful_assert_oracle_fixture::ledger_contract as c;
use midnight_compact_runtime as runtime;
use midnight_onchain_state::state::{
    ContractMaintenanceAuthority, ContractOperation, ContractState, EntryPointBuf,
};
use midnight_storage::storage::HashMap;
use runtime::context::{CircuitResult, ConstructorContext, WitnessContext};
use serde_json::{Value, json};
use std::cell::RefCell;
struct Witnesses {
    gates: Vec<bool>,
    trace: RefCell<Vec<Value>>,
}
impl c::Witnesses<Vec<u8>> for Witnesses {
    fn next_gate(
        &self,
        ctx: WitnessContext<'_, Vec<u8>, c::LedgerView<'_>>,
        tag: runtime::BoundedUint<255>,
    ) -> (Vec<u8>, bool) {
        self.trace
            .borrow_mut()
            .push(json!({"tag":tag.value(),"prior":ctx.private_state}));
        let mut private = ctx.private_state.clone();
        private.push(tag.value() as u8);
        (private, self.gates[tag.value() as usize - 1])
    }
}
fn state_hex(state: runtime::ledger::StateValue<runtime::ledger::DefaultDB>) -> String {
    let operations = ["checked", "unit_result"]
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
fn check<T: Clone + Into<runtime::fab::AlignedValue>>(
    out: Result<CircuitResult<Vec<u8>, T>, runtime::CompactError>,
    row: &Value,
) {
    if let Some(error) = row.get("error") {
        let err = match out {
            Err(e) => e,
            Ok(_) => panic!("expected failure {row}"),
        };
        if error.as_str().unwrap().contains("failed assert:") {
            let runtime::CompactError::AssertionFailed(message) = err else {
                panic!("expected assertion, got {err}")
            };
            assert!(error.as_str().unwrap().ends_with(&message));
        } else {
            assert!(matches!(err, runtime::CompactError::LedgerQueryRejected(_)));
            assert!(err.to_string().to_lowercase().contains("gas"), "{err}");
        }
        return;
    }
    let out = out.unwrap();
    assert_eq!(
        json!(Into::<runtime::fab::AlignedValue>::into(out.result)),
        row["output"]
    );
    assert_eq!(json!(out.context.private_state), row["privateState"]);
    assert_eq!(json!(out.private_transcript_outputs), row["privateOutputs"]);
    assert_eq!(json!(out.context.query.effects), row["effects"]);
    assert_eq!(
        state_hex(out.context.query.state.get_ref().clone()),
        row["after"]
    );
    let cost = json!(out.gas_cost);
    let queries = row["queries"].as_array().unwrap();
    for dim in ["readTime", "computeTime", "bytesWritten", "bytesDeleted"] {
        let sum: u64 = queries
            .iter()
            .map(|q| q["gas"][dim].as_str().unwrap().parse::<u64>().unwrap())
            .sum();
        assert_eq!(cost[dim], sum);
        // TS currently reports its last accepted query as aggregate circuit gas.
        assert_eq!(row["gas"][dim], queries.last().unwrap()["gas"][dim]);
    }
}
#[test]
fn nested_assertions_match_typescript_short_circuit_witnesses_queries_and_rejections() {
    let rows: Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/stateful-assert-oracle.json"
    ))
    .unwrap();
    for row in rows.as_array().unwrap() {
        let w = Witnesses {
            gates: serde_json::from_value(row["gates"].clone()).unwrap(),
            trace: RefCell::new(vec![]),
        };
        let mut ctx = c::initial_state(
            ConstructorContext::new(vec![]),
            row["open"].as_bool().unwrap(),
            runtime::BoundedUint::new(row["low"].as_u64().unwrap() as u128).unwrap(),
            runtime::BoundedUint::new(row["high"].as_u64().unwrap() as u128).unwrap(),
        )
        .unwrap()
        .into_circuit_context(runtime::ledger::ContractAddress::default());
        assert_eq!(state_hex(ctx.query.state.get_ref().clone()), row["before"]);
        if row["zeroGas"].as_bool().unwrap() {
            ctx.gas_limit = Some(Default::default());
        }
        if row["name"] == "checked" {
            check(c::checked(ctx, &w, row["selected"].as_bool().unwrap()), row);
        } else {
            check(
                c::unit_result(ctx, &w, row["selected"].as_bool().unwrap()),
                row,
            );
        }
        assert_eq!(json!(*w.trace.borrow()), row["trace"]);
    }
}
