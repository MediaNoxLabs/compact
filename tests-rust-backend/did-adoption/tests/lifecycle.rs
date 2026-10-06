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

//! Native source-call acceptance, not proof or network acceptance. TS programs
//! are replayed as independent oracles; no Rust recording is claimed for gaps.
use compact_rust_did_adoption_fixture::{
    ledger_contract as c, ledger_slots as slots, runtime as r,
};
use midnight_compact_testkit::{ArtifactIdentity, ContractLab, Environment, LabError};
use r::context::{ConstructorContext, ConstructorResult};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
#[path = "../support/codec.rs"]
mod codec;
#[path = "../support/lifecycle_calls.rs"]
mod lifecycle_calls;
#[path = "../support/lifecycle_witness.rs"]
mod lifecycle_witness;
use lifecycle_witness::Witness;
fn template(text: &str) -> r::ledger::ContractState<r::ledger::DefaultDB> {
    midnight_serialize::tagged_deserialize(&mut hex::decode(text).unwrap().as_slice()).unwrap()
}
fn state_hex(
    state: &r::ledger::ChargedState<r::ledger::DefaultDB>,
    template: &r::ledger::ContractState<r::ledger::DefaultDB>,
) -> String {
    let mut result = template.clone();
    result.data = r::ledger::ChargedState::new(state.get_ref().clone());
    let mut bytes = vec![];
    midnight_serialize::tagged_serialize(&result, &mut bytes).unwrap();
    hex::encode(bytes)
}
// Upstream StateValue's streaming JSON decoder requires its tag before contents.
// serde_json::Value sorts object keys; restore only that wire-order requirement.
fn tag_first(v: &Value) -> String {
    match v {
        Value::Array(a) => format!(
            "[{}]",
            a.iter().map(tag_first).collect::<Vec<_>>().join(",")
        ),
        Value::Object(o) => {
            let entries = o.get("tag").map(|v| ("tag", v)).into_iter().chain(
                o.iter()
                    .filter(|(k, _)| k.as_str() != "tag")
                    .map(|(k, v)| (k.as_str(), v)),
            );
            format!(
                "{{{}}}",
                entries
                    .map(|(k, v)| format!("{}:{}", serde_json::to_string(k).unwrap(), tag_first(v)))
                    .collect::<Vec<_>>()
                    .join(",")
            )
        }
        _ => serde_json::to_string(v).unwrap(),
    }
}
// Replay each actual TS query, including the prefix before a source assertion.
// Native failure contexts are consumed; this does not attest their internal stream.
fn replay_prefix(
    before: &midnight_compact_testkit::Snapshot<u64>,
    row: &Value,
) -> r::ledger::QueryContext<r::ledger::DefaultDB> {
    let ctx = ConstructorResult::new(
        ConstructorContext::new(*before.private_state()),
        before.public_state().clone(),
    )
    .into_circuit_context(Default::default());
    let mut query = ctx.query;
    let mut index = 0;
    let mut witnesses = vec![];
    for event in row["events"].as_array().unwrap() {
        if let Some(name) = event["witness"].as_str() {
            witnesses.push(name);
            continue;
        }
        assert_eq!(event["query"], index);
        // JS omits the unit result marker; Rust Gather represents it as null.
        // No read value is supplied: the authoritative VM gathers it from state.
        let mut gather = event["program"].clone();
        for op in gather.as_array_mut().unwrap() {
            if let Some(pop) = op.get_mut("popeq") {
                assert!(pop.get("result").is_none());
                pop.as_object_mut()
                    .unwrap()
                    .insert("result".into(), Value::Null);
            }
        }
        let program: Vec<
            midnight_onchain_vm::ops::Op<
                midnight_onchain_vm::result_mode::ResultModeGather,
                r::ledger::DefaultDB,
            >,
        > = serde_json::from_str(&tag_first(&gather)).unwrap();
        let result = query.query(&program, None, &ctx.cost_model).unwrap();
        let gas = json!(result.gas_cost);
        for dim in ["readTime", "computeTime", "bytesWritten", "bytesDeleted"] {
            assert_eq!(
                gas[dim].as_u64().unwrap().to_string(),
                row["queries"][index][dim].as_str().unwrap()
            );
        }
        query = result.context;
        index += 1;
    }
    assert_eq!(index, row["queries"].as_array().unwrap().len());
    assert_eq!(
        witnesses,
        row["witnessCalls"]
            .as_array()
            .unwrap()
            .iter()
            .map(|w| w["name"].as_str().unwrap())
            .collect::<Vec<_>>()
    );
    query
}
fn error(text: &str) -> r::CompactError {
    r::CompactError::AssertionFailed(text.strip_prefix("failed assert: ").unwrap_or(text).into())
}
fn run(id: &str) {
    let capture: Value = serde_json::from_str(include_str!("../oracle/lifecycle.json")).unwrap();
    let s = capture["scenarios"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["id"] == id)
        .unwrap();
    let witness = Witness::new(s["constructorOptions"].clone());
    let init = c::initial_state(ConstructorContext::new(0u64), &witness);
    if let Some(e) = s["constructorError"].as_str() {
        assert_eq!(init.err().unwrap(), error(e), "{id}");
        assert_eq!(json!(*witness.calls.borrow()), s["witnessCalls"]);
        return;
    }
    let init = init.unwrap();
    let original = template(s["initial"]["state"].as_str().unwrap());
    assert_eq!(
        init.ledger_state.get_ref(),
        original.data.get_ref(),
        "constructor {id}"
    );
    assert_eq!(
        init.private_state,
        s["initial"]["private"].as_u64().unwrap()
    );
    assert_eq!(json!(*witness.calls.borrow()), s["initial"]["witnessCalls"]);
    assert_eq!(
        json!(*witness.calls.borrow()),
        json!([{"name":"controller"},{"name":"recovery"},{"name":"timestamp"}])
    );
    let state = init.ledger_state.get_ref();
    assert_eq!(slots::contractVersion.inspect(state).unwrap().value(), 2);
    assert_eq!(slots::version.inspect(state).unwrap(), 0);
    assert_eq!(slots::operationCount.inspect(state).unwrap(), 0);
    assert!(slots::active.inspect(state).unwrap());
    assert!(!slots::deactivated.inspect(state).unwrap());
    assert_eq!(
        slots::controllerPublicKey.inspect(state).unwrap(),
        codec::point("1")
    );
    assert_eq!(
        slots::recoveryAuthorityPublicKey.inspect(state).unwrap(),
        codec::point("3")
    );
    assert_eq!(
        slots::created.inspect(state).unwrap(),
        slots::updated.inspect(state).unwrap()
    );
    assert_eq!(
        slots::created.inspect(state).unwrap(),
        codec::version(
            s["constructorOptions"]["timestamp"]
                .as_str()
                .unwrap_or("100")
        )
    );
    // Native constructor kernel.self is the dummy address, not a deployment claim.
    assert_eq!(
        slots::id.inspect(state).unwrap().bytes,
        r::FixedBytes::new([0; 32])
    );
    let identity = ArtifactIdentity {
        source_sha256: Sha256::digest(include_bytes!(
            "../../../examples/rust_backend/did_adoption/packages/contract/src/did.compact"
        ))
        .into(),
        generated_sha256: Sha256::digest(include_bytes!("../lib.rs")).into(),
    };
    let env = Environment::new(Default::default(), Default::default(), [0x55; 32]);
    let mut lab = ContractLab::from_constructor(identity, env, init).unwrap();
    for row in s["steps"].as_array().unwrap() {
        let label = format!("{id}/{}", row["id"].as_str().unwrap());
        let before = lab.snapshot();
        assert_eq!(
            state_hex(before.public_state(), &original),
            row["before"],
            "{label}"
        );
        assert_eq!(
            *before.private_state(),
            row["privateBefore"].as_u64().unwrap()
        );
        let witness = Witness::new(row["options"].clone());
        let result = lab.native(|ctx| lifecycle_calls::invoke(ctx, &witness, row));
        assert_eq!(
            json!(*witness.calls.borrow()),
            row["witnessCalls"],
            "{label}"
        );
        let ts_prefix = replay_prefix(&before, row);
        if let Some(e) = row["error"].as_str() {
            let LabError::Execution(actual) = result.err().unwrap() else {
                panic!("expected source error: {label}")
            };
            assert_eq!(actual, error(e), "{label}");
            assert_eq!(
                lab.snapshot().public_state(),
                before.public_state(),
                "public rollback {label}"
            );
            assert_eq!(
                lab.private_state(),
                before.private_state(),
                "private rollback {label}"
            );
            if row["id"] == "late-timestamp-failure" {
                assert_ne!(
                    &ts_prefix.state,
                    before.public_state(),
                    "TS prefix already wrote before failing witness"
                );
                assert_eq!(
                    slots::version.inspect(ts_prefix.state.get_ref()).unwrap(),
                    slots::version
                        .inspect(before.public_state().get_ref())
                        .unwrap()
                        + 1
                );
                assert_eq!(
                    slots::updated.inspect(ts_prefix.state.get_ref()).unwrap(),
                    slots::updated
                        .inspect(before.public_state().get_ref())
                        .unwrap()
                );
            }
            continue;
        }
        let report = result.unwrap_or_else(|e| panic!("{label}: {e:?}"));
        assert_eq!(
            state_hex(report.public_state(), &original),
            row["after"],
            "{label}"
        );
        assert_eq!(
            *lab.private_state(),
            row["privateAfter"].as_u64().unwrap(),
            "{label}"
        );
        assert!(
            report.replay().is_none(),
            "native report must not claim Rust recording"
        );
        assert_eq!(&ts_prefix.state, report.public_state());
        assert_eq!(&ts_prefix.effects, report.effects());
        let after = report.public_state().get_ref();
        let prior = before.public_state().get_ref();
        assert_eq!(
            slots::created.inspect(after).unwrap(),
            slots::created.inspect(prior).unwrap()
        );
        assert_eq!(
            slots::id.inspect(after).unwrap(),
            slots::id.inspect(prior).unwrap()
        );
        assert_eq!(
            slots::recoveryAuthorityPublicKey.inspect(after).unwrap(),
            codec::point("3")
        );
        if row["name"] != "verifySchnorrJubjubDigestSignature" {
            assert_eq!(
                slots::version.inspect(after).unwrap(),
                slots::version.inspect(prior).unwrap() + 1,
                "{label}"
            );
            assert_eq!(
                slots::operationCount.inspect(after).unwrap(),
                slots::operationCount.inspect(prior).unwrap() + 1,
                "{label}"
            );
            assert_eq!(
                slots::updated.inspect(after).unwrap(),
                codec::version(row["options"]["timestamp"].as_str().unwrap()),
                "{label}"
            );
        }
        if row["name"] == "deactivate" {
            assert!(!slots::active.inspect(after).unwrap());
            assert!(slots::deactivated.inspect(after).unwrap());
        }
        if row["name"] == "rotateControllerKey" || row["name"] == "recoverControllerKey" {
            assert_eq!(
                slots::controllerPublicKey.inspect(after).unwrap(),
                codec::point(row["args"]["key"].as_str().unwrap())
            );
        }
        let private:Vec<_>=report.private_outputs().iter().map(|v|json!({"valueAtoms":v.value.0.iter().map(|a|&a.0).collect::<Vec<_>>(),"alignment":v.alignment})).collect();
        assert_eq!(json!(private), row["privateTranscript"], "{label}");
        let gas = json!(report.execution_gas());
        for dimension in ["readTime", "computeTime", "bytesWritten", "bytesDeleted"] {
            let total: u64 = row["queries"]
                .as_array()
                .unwrap()
                .iter()
                .map(|q| q[dimension].as_str().unwrap().parse::<u64>().unwrap())
                .sum();
            assert_eq!(gas[dimension], total, "{label}: {dimension}");
        }
        // Replay the independently captured TypeScript VM program against the Rust
        // prestate. This compares state/effects; it is not a generated Rust recorder.
        let ctx = ConstructorResult::new(
            ConstructorContext::new(*before.private_state()),
            before.public_state().clone(),
        )
        .into_circuit_context(Default::default());
        let program: Vec<
            midnight_onchain_vm::ops::Op<
                midnight_onchain_vm::result_mode::ResultModeVerify,
                r::ledger::DefaultDB,
            >,
        > = serde_json::from_str(&tag_first(&row["publicTranscript"])).unwrap();
        let replay = ctx.query.query(&program, None, &ctx.cost_model).unwrap();
        assert_eq!(
            &replay.context.state,
            report.public_state(),
            "oracle replay {label}"
        );
        assert_eq!(
            &replay.context.effects,
            report.effects(),
            "oracle effects {label}"
        );
        if row["name"] == "verifySchnorrJubjubDigestSignature" {
            assert_eq!(
                report.public_state(),
                before.public_state(),
                "read-only {label}"
            );
            assert_eq!(
                witness.calls.borrow().len(),
                1,
                "no timestamp witness on read-only verifier"
            );
        }
    }
}
macro_rules! case {
    ($name:ident,$id:literal) => {
        #[test]
        fn $name() {
            run($id);
        }
    };
}
case!(constructor, "constructor-only");
case!(
    constructor_controller_failure,
    "constructor-controller-failure"
);
case!(constructor_recovery_failure, "constructor-recovery-failure");
case!(
    constructor_timestamp_failure,
    "constructor-timestamp-failure"
);
case!(constructor_equal_keys, "constructor-equal-keys");
case!(constructor_zero_time, "constructor-zero-time");
case!(constructor_max_time, "constructor-max-time");
case!(aliases_services_and_late_rollback, "aliases-services");
case!(all_method_and_relation_domains, "methods-relations");
case!(schnorr_crud_and_readonly_verification, "schnorr");
case!(
    rotation_recovery_and_deactivation,
    "authorization-lifecycle"
);
