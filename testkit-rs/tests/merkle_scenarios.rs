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

use compact_rust_merkle_tree_oracle_fixture::{
    ledger_contract::{self as merkle, recorded},
    types::MerkleTreeDigest,
};
use midnight_compact_testkit::{ArtifactIdentity, ContractLab, Environment, LabError, runtime};
use runtime::{BoundedUint, CompactError, context::ConstructorContext};
use serde_json::{Value, json};

fn identity() -> ArtifactIdentity {
    ArtifactIdentity {
        source_sha256: midnight_base_crypto::hash::persistent_hash(include_bytes!(
            "../../examples/rust_backend/merkle_tree_oracle.compact"
        ))
        .0,
        generated_sha256: midnight_base_crypto::hash::persistent_hash(include_bytes!(
            "../../tests-rust-backend/merkle-tree-oracle/lib.rs"
        ))
        .0,
    }
}

fn lab_with_environment(environment: Environment) -> ContractLab<()> {
    ContractLab::from_constructor(
        identity(),
        environment,
        merkle::initial_state(ConstructorContext::new(())).unwrap(),
    )
    .unwrap()
}

fn lab() -> ContractLab<()> {
    lab_with_environment(Environment::new(
        Default::default(),
        Default::default(),
        [7; 32],
    ))
}

fn root(state: &runtime::ledger::ChargedState<runtime::ledger::DefaultDB>) -> MerkleTreeDigest {
    let tree = runtime::ledger::merkle_tree_view_at_path(state.get_ref(), &[0]).unwrap();
    MerkleTreeDigest {
        field: tree.root().unwrap().0,
    }
}

fn first_free(state: &runtime::ledger::ChargedState<runtime::ledger::DefaultDB>) -> u128 {
    runtime::ledger::merkle_tree_view_at_path(state.get_ref(), &[0])
        .unwrap()
        .first_free()
        .unwrap()
        .value()
}

fn assert_capture<O>(
    label: &str,
    report: &midnight_compact_testkit::CallReport<O>,
    oracle: &Value,
    known_result_marker: bool,
    matching_prestate: bool,
) {
    let capture = &oracle["nativeQueries"][label];
    let queries = capture["queries"].as_array().unwrap();
    assert_eq!(queries.len(), 1, "{label}: expected one upstream query");
    let replay = report.replay().expect("recorded call must replay");
    let mut program = json!(replay.program());
    if known_result_marker {
        // TS Gather omits the cached boolean result. Rust Verify seals it in the
        // final PopEq; only this exact marker differs, never the op order.
        assert_eq!(program.as_array().unwrap().len(), 7);
        assert!(!program[6]["popeq"]["result"].is_null());
        program[6]["popeq"]["result"] = Value::Null;
    }
    assert_eq!(program, queries[0]["program"], "{label}: VM program");
    assert_eq!(report.private_outputs().len(), 0, "{label}: private output");
    assert_eq!(capture["privateOutputs"], 0);

    let execution_gas = json!(report.execution_gas());
    let replay_gas = json!(replay.gas());
    for dimension in ["readTime", "computeTime", "bytesWritten", "bytesDeleted"] {
        let captured: u64 = queries
            .iter()
            .map(|query| {
                query["gasCost"][dimension]
                    .as_str()
                    .unwrap()
                    .parse::<u64>()
                    .unwrap()
            })
            .sum();
        assert_eq!(
            replay_gas[dimension], execution_gas[dimension],
            "{label}: replay/execution gas {dimension}"
        );
        if matching_prestate {
            assert_eq!(
                execution_gas[dimension], captured,
                "{label}: execution gas {dimension}"
            );
        }
    }
}

#[test]
fn merkle_append_root_checks_and_owned_reset_match_typescript_and_vm() {
    let oracle: Value = serde_json::from_str(include_str!(
        "../../runtime-rs/tests/fixtures/merkle-tree-oracle.json"
    ))
    .unwrap();
    let mut lab = lab();
    let initial = lab.snapshot();
    let initial_root = root(initial.public_state());
    assert_eq!(first_free(initial.public_state()), 0);

    let mut native = lab.fork();
    let appended = lab
        .recorded(|c| recorded::append(c, BoundedUint::new(7)?))
        .unwrap();
    let native_append = native
        .native(|c| merkle::append(c, BoundedUint::new(7)?))
        .unwrap();
    assert_eq!(appended.public_state(), native_append.public_state());
    assert_eq!(appended.effects(), native_append.effects());
    assert_capture("append7", &appended, &oracle, false, true);
    let after_append = lab.snapshot();
    assert_eq!(first_free(after_append.public_state()), 1);
    let appended_root = root(after_append.public_state());
    assert_ne!(appended_root, initial_root);

    for (label, checked_root, expected) in [
        ("knownCurrentAfterAppend", appended_root, true),
        ("knownInitialAfterAppend", initial_root.clone(), false),
    ] {
        let before = lab.snapshot();
        let mut native = lab.fork();
        let report = lab
            .recorded(|c| recorded::known(c, checked_root.clone()))
            .unwrap();
        let native_report = native.native(|c| merkle::known(c, checked_root)).unwrap();
        assert_eq!(*report.output(), expected);
        assert_eq!(*report.output(), *native_report.output());
        assert_eq!(report.public_state(), before.public_state());
        assert_eq!(report.public_state(), native_report.public_state());
        assert_eq!(report.effects(), native_report.effects());
        assert_eq!(oracle["nativeQueries"][label]["result"], expected);
        assert_capture(label, &report, &oracle, true, true);
    }

    let mut fork = lab.fork();
    let mut native_reset = fork.fork();
    let reset = fork.recorded(recorded::reset_tree).unwrap();
    let native_reset = native_reset.native(merkle::reset_tree).unwrap();
    assert_eq!(reset.public_state(), native_reset.public_state());
    assert_eq!(reset.effects(), native_reset.effects());
    // The oracle resetTree prestate contains more entries than this one-append
    // scenario. Its ordered program is exact; meter against native execution
    // from the same prestate instead of the fuller oracle prestate.
    assert_capture("resetTree", &reset, &oracle, false, false);
    assert_eq!(reset.execution_gas(), native_reset.execution_gas());
    assert_eq!(root(reset.public_state()), initial_root);
    assert_eq!(first_free(reset.public_state()), 0);
    assert_eq!(lab.snapshot().public_state(), after_append.public_state());
    lab.restore(&initial).unwrap();
    assert_eq!(lab.snapshot().public_state(), initial.public_state());
    assert_eq!(lab.environment().fixture_seed, [7; 32]);

    let mut empty = self::lab();
    let reset_empty = empty.recorded(recorded::reset_tree).unwrap();
    assert_capture("resetEmpty", &reset_empty, &oracle, false, true);
}

#[test]
fn merkle_calls_are_repeatable_and_failed_query_preserves_checkpoint() {
    let mut a = lab();
    let mut b = lab();
    let x = a
        .recorded(|c| recorded::append(c, BoundedUint::new(7)?))
        .unwrap();
    let y = b
        .recorded(|c| recorded::append(c, BoundedUint::new(7)?))
        .unwrap();
    assert_eq!(x.public_state(), y.public_state());
    assert_eq!(x.effects(), y.effects());
    assert_eq!(x.execution_gas(), y.execution_gas());
    assert_eq!(
        json!(x.replay().unwrap().program()),
        json!(y.replay().unwrap().program())
    );
    assert_eq!(x.replay().unwrap().gas(), y.replay().unwrap().gas());

    let mut env = Environment::new(Default::default(), Default::default(), [7; 32]);
    env.query_gas_limit = Some(runtime::context::RunningCost::ZERO);
    let mut limited = lab_with_environment(env);
    let before = limited.snapshot();
    assert!(matches!(
        limited.recorded(|c| recorded::append(c, BoundedUint::new(7)?)),
        Err(LabError::Execution(CompactError::LedgerQueryRejected(_)))
    ));
    assert_eq!(limited.snapshot().public_state(), before.public_state());
    assert_eq!(limited.private_state(), before.private_state());
}
