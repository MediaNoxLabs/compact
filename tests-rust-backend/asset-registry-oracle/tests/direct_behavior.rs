// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use compact_rust_asset_registry_oracle_fixture::{
    ledger_contract::{self, LedgerView, Witnesses},
    pure_circuits as p,
    types::*,
};
use midnight_compact_runtime as runtime;
use midnight_onchain_state::state::{
    ContractMaintenanceAuthority, ContractOperation, ContractState, EntryPointBuf,
};
use midnight_serialize::tagged_serialize;
use midnight_storage::storage::HashMap;
use runtime::context::{ConstructorContext, WitnessContext};
use runtime::ledger::{ContractAddress, DefaultDB, StateValue};
use runtime::{BoundedUint, CompactError, Field, FixedBytes, OpaqueString};
use serde_json::Value;
use std::{cell::RefCell, collections::BTreeSet};
fn capture() -> Value {
    serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/direct-call-registry.json"
    ))
    .unwrap()
}
fn u64v(v: &Value) -> BoundedUint<{ u64::MAX as u128 }> {
    BoundedUint::new(v.as_str().unwrap().parse().unwrap()).unwrap()
}
fn bytes(v: &Value) -> FixedBytes<32> {
    FixedBytes::new(
        v.as_array()
            .unwrap()
            .iter()
            .map(|n| n.as_u64().unwrap() as u8)
            .collect::<Vec<_>>()
            .try_into()
            .unwrap(),
    )
}
fn record(v: &Value) -> AssetRecord {
    AssetRecord {
        code: bytes(&v["code"]),
        note: OpaqueString::from(v["note"].as_str().unwrap()),
        provenance: Provenance {
            facility: bytes(&v["provenance"]["facility"]),
            registeredAt: u64v(&v["provenance"]["registeredAt"]),
        },
        kind: match v["kind"].as_u64().unwrap() {
            0 => AssetClass::Unspecified,
            1 => AssetClass::Instrument,
            2 => AssetClass::Container,
            3 => AssetClass::Document,
            _ => panic!("invalid enum"),
        },
        quantity: u64v(&v["quantity"]),
    }
}
#[test]
fn three_pure_registry_guards_match_direct_typescript_successes_and_errors() {
    let data = capture();
    let rows = data["cases"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r["group"] == "asset")
        .collect::<Vec<_>>();
    assert_eq!(rows.len(), 17);
    let mut names = BTreeSet::new();
    let mut ids = BTreeSet::new();
    let mut failures = 0;
    for row in rows {
        let name = row["export"].as_str().unwrap();
        names.insert(name);
        assert!(ids.insert(row["id"].as_str().unwrap()));
        let a = &row["args"];
        let result = match name {
            "assertRecordFreshEnough" => p::assertRecordFreshEnough(
                FreshnessPolicy {
                    enforceMaxAge: a[0]["enforceMaxAge"].as_bool().unwrap(),
                    maxAge: u64v(&a[0]["maxAge"]),
                },
                record(&a[1]),
                u64v(&a[2]),
            ),
            "assertRecordClassKnown" => p::assertRecordClassKnown(record(&a[0])),
            "assertGrantNotFuture" => p::assertGrantNotFuture(
                CustodyGrant {
                    code: bytes(&a[0]["code"]),
                    holder: compact_rust_asset_registry_oracle_fixture::types::ContractAddress {
                        bytes: bytes(&a[0]["holder"]["bytes"]),
                    },
                    grantedAt: u64v(&a[0]["grantedAt"]),
                },
                u64v(&a[1]),
            ),
            _ => panic!("unexpected export"),
        };
        if row["ok"] == true {
            result.unwrap();
            assert_eq!(row["result"], serde_json::json!([]));
        } else {
            failures += 1;
            let error = result.unwrap_err();
            assert!(matches!(error, CompactError::AssertionFailed(_)));
            assert_eq!(row["errorClass"], "CompactError");
            assert_eq!(error.to_string(), row["error"], "{}", row["id"]);
        }
    }
    assert_eq!(failures, 6);
    assert_eq!(
        names,
        BTreeSet::from([
            "assertRecordFreshEnough",
            "assertRecordClassKnown",
            "assertGrantNotFuture"
        ])
    );
}
#[derive(Default)]
struct Tracking {
    calls: RefCell<Vec<&'static str>>,
}
impl Witnesses<u64> for Tracking {
    fn localOperatorKey(
        &self,
        c: WitnessContext<'_, u64, LedgerView<'_>>,
    ) -> (u64, runtime::JubjubPoint) {
        self.calls.borrow_mut().push("localOperatorKey");
        (
            *c.private_state + 1,
            runtime::hash_to_curve(Field::from(1u64)),
        )
    }
    fn localAuditorKey(
        &self,
        c: WitnessContext<'_, u64, LedgerView<'_>>,
    ) -> (u64, runtime::JubjubPoint) {
        self.calls.borrow_mut().push("localAuditorKey");
        (
            *c.private_state + 1,
            runtime::hash_to_curve(Field::from(2u64)),
        )
    }
    fn currentTimestamp(
        &self,
        c: WitnessContext<'_, u64, LedgerView<'_>>,
    ) -> (u64, BoundedUint<{ u64::MAX as u128 }>) {
        self.calls.borrow_mut().push("currentTimestamp");
        (*c.private_state + 1, BoundedUint::new(1700000000).unwrap())
    }
}
fn normalize(mut program: Value) -> Value {
    for op in program.as_array_mut().unwrap() {
        if let Some(pop) = op.get_mut("popeq") {
            pop.as_object_mut().unwrap().remove("result");
        }
    }
    program
}
#[test]
fn close_matches_independent_typescript_and_repeat_refusal() {
    let data = capture();
    let expected = &data["close"]["success"];
    let nw = Tracking::default();
    let rw = Tracking::default();
    let ni = ledger_contract::initial_state(ConstructorContext::new(7u64), &nw).unwrap();
    let ri = ledger_contract::initial_state(ConstructorContext::new(7u64), &rw).unwrap();
    assert_eq!(ni.private_state, 10);
    assert_eq!(ri.private_state, 10);
    assert_eq!(
        state_hex(ni.ledger_state.get_ref().clone()),
        expected["before"]
    );
    nw.calls.borrow_mut().clear();
    rw.calls.borrow_mut().clear();
    let n =
        ledger_contract::close(ni.into_circuit_context(ContractAddress::default()), &nw).unwrap();
    let r =
        ledger_contract::recorded::close(ri.into_circuit_context(ContractAddress::default()), &rw)
            .unwrap();
    assert_eq!(expected["result"], serde_json::json!([]));
    assert_eq!(n.result, ());
    assert_eq!(r.execution.result, ());
    for output in [&n, &r.execution] {
        assert_eq!(
            state_hex(output.context.query.state.get_ref().clone()),
            expected["after"]
        );
        assert_eq!(output.context.private_state, expected["privateState"]);
        let private=output.private_transcript_outputs.iter().map(|v|serde_json::json!({"value":v.value.0.iter().map(|a|&a.0).collect::<Vec<_>>(),"alignment":v.alignment})).collect::<Vec<_>>();
        assert_eq!(serde_json::json!(private), expected["privateTranscript"]);
        let gas = serde_json::to_value(output.gas_cost).unwrap();
        for dim in ["readTime", "computeTime", "bytesWritten", "bytesDeleted"] {
            let sum: u64 = expected["queries"]
                .as_array()
                .unwrap()
                .iter()
                .map(|q| q["gasCost"][dim].as_str().unwrap().parse::<u64>().unwrap())
                .sum();
            assert_eq!(gas[dim], sum);
        }
    }
    assert_eq!(n.context.query.effects, r.execution.context.query.effects);
    assert_eq!(
        normalize(serde_json::to_value(r.public.verify_ops()).unwrap()),
        normalize(expected["publicTranscript"].clone())
    );
    assert_eq!(
        serde_json::json!(*nw.calls.borrow()),
        expected["witnessCalls"]
    );
    assert_eq!(
        serde_json::json!(*rw.calls.borrow()),
        expected["witnessCalls"]
    );
    let replay = r
        .public
        .initial()
        .query(r.public.verify_ops(), None, &r.execution.context.cost_model)
        .unwrap();
    assert_eq!(
        state_hex(replay.context.state.get_ref().clone()),
        expected["after"]
    );
    assert_eq!(replay.context.effects, r.execution.context.query.effects);
    nw.calls.borrow_mut().clear();
    rw.calls.borrow_mut().clear();
    let ne = ledger_contract::close(n.context, &nw).err().unwrap();
    let re = ledger_contract::recorded::close(r.execution.context, &rw)
        .err()
        .unwrap();
    assert_eq!(ne, re);
    assert!(matches!(ne, CompactError::AssertionFailed(_)));
    assert_eq!(ne.to_string(), data["close"]["repeat"]["error"]);
    assert_eq!(data["close"]["repeat"]["errorClass"], "CompactError");
    assert_eq!(
        data["close"]["repeat"]["queries"].as_array().unwrap().len(),
        1
    );
    assert!(nw.calls.borrow().is_empty());
    assert!(rw.calls.borrow().is_empty());
    assert_eq!(
        data["close"]["repeat"]["witnessCalls"],
        serde_json::json!([])
    );
    assert_eq!(data["close"]["repeat"]["privateState"], 11);
}

fn state_hex(state: StateValue<DefaultDB>) -> String {
    let mut operations: HashMap<EntryPointBuf, ContractOperation, DefaultDB> = HashMap::new();
    for name in [
        "setCustodian",
        "setRecord",
        "removeRecord",
        "setCustodyGrant",
        "setWatch",
        "tag",
        "assertStoredRecordFresh",
        "assertGrantEffective",
        "acceptIfFresh",
        "close",
    ] {
        operations = operations.insert(
            EntryPointBuf(name.as_bytes().to_vec()),
            ContractOperation::new(None),
        );
    }
    let contract_state =
        ContractState::new(state, operations, ContractMaintenanceAuthority::default());
    let mut bytes = Vec::new();
    tagged_serialize(&contract_state, &mut bytes).unwrap();
    hex::encode(bytes)
}
