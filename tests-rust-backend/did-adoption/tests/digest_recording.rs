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

use compact_rust_did_adoption_fixture::{ledger_contract as c, runtime as r, types};
use midnight_compact_testkit::{ArtifactIdentity, ContractLab, Environment, LabError};
use r::context::{ConstructorContext, ConstructorResult};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
#[allow(dead_code)]
#[path = "../support/codec.rs"]
mod codec;
#[path = "../support/lifecycle_witness.rs"]
mod lifecycle_witness;
use lifecycle_witness::Witness;

fn state(row: &Value) -> r::ledger::ContractState<r::ledger::DefaultDB> {
    midnight_serialize::tagged_deserialize(
        &mut hex::decode(row["before"].as_str().unwrap())
            .unwrap()
            .as_slice(),
    )
    .unwrap()
}

#[test]
fn original_digest_five_cases_match_native_recorded_and_independent_ts() {
    let capture: Value = serde_json::from_str(include_str!("../oracle/lifecycle.json")).unwrap();
    let mut count = 0;
    for scenario in capture["scenarios"].as_array().unwrap() {
        for row in scenario["steps"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|r| r["name"] == "verifySchnorrJubjubDigestSignature")
        {
            count += 1;
            let before = state(row);
            let private_before = row["privateBefore"].as_u64().unwrap();
            let constructor = ConstructorResult::new(
                ConstructorContext::new(private_before),
                before.data.clone(),
            );
            let identity = ArtifactIdentity {
                source_sha256: Sha256::digest(include_bytes!(
                    "../../../examples/rust_backend/did_adoption/packages/contract/src/did.compact"
                ))
                .into(),
                generated_sha256: Sha256::digest(include_bytes!("../lib.rs")).into(),
            };
            let mut lab = ContractLab::from_constructor(
                identity,
                Environment::new(Default::default(), Default::default(), [0x88; 32]),
                constructor,
            )
            .unwrap();
            let id = codec::string(&row["args"]["id"]);
            let digest = r::FixedVector::new([1u64, 2, 3, 4].map(r::Field::from));
            let sig = types::SchnorrSignature {
                announcement: codec::point("2"),
                response: codec::field_hex(row["responseHex"].as_str().unwrap()),
            };
            let nw = Witness::new(row["options"].clone());
            let rw = Witness::new(row["options"].clone());
            let native = c::verifySchnorrJubjubDigestSignature(
                ConstructorResult::new(
                    ConstructorContext::new(private_before),
                    before.data.clone(),
                )
                .into_circuit_context(Default::default()),
                &nw,
                id.clone(),
                digest.clone(),
                sig.clone(),
            );
            let recorded = lab.recorded(|ctx| {
                c::recorded::verifySchnorrJubjubDigestSignature(
                    ctx,
                    &rw,
                    id.clone(),
                    digest.clone(),
                    sig.clone(),
                )
            });
            let label = row["id"].as_str().unwrap();
            assert_eq!(
                json!(*nw.calls.borrow()),
                row["witnessCalls"],
                "native {label}"
            );
            assert_eq!(
                json!(*rw.calls.borrow()),
                row["witnessCalls"],
                "recorded {label}"
            );
            if let Some(message) = row["error"].as_str() {
                let expected = r::CompactError::AssertionFailed(
                    message
                        .strip_prefix("failed assert: ")
                        .unwrap_or(message)
                        .into(),
                );
                assert_eq!(native.err().unwrap(), expected, "native {label}");
                assert!(
                    matches!(recorded, Err(LabError::Execution(ref e)) if *e == expected),
                    "recorded {label}"
                );
                assert_eq!(
                    lab.snapshot().public_state(),
                    &before.data,
                    "rollback {label}"
                );
                assert_eq!(
                    *lab.private_state(),
                    private_before,
                    "private rollback {label}"
                );
                continue;
            }
            let native = native.unwrap();
            let recorded = recorded.unwrap();
            assert_eq!(recorded.public_state(), &before.data, "read-only {label}");
            assert_eq!(
                recorded.public_state(),
                &native.context.query.state,
                "state {label}"
            );
            assert_eq!(
                recorded.effects(),
                &native.context.query.effects,
                "effects {label}"
            );
            assert_eq!(
                *lab.private_state(),
                row["privateAfter"].as_u64().unwrap(),
                "private {label}"
            );
            assert_eq!(
                recorded.private_outputs(),
                &native.private_transcript_outputs,
                "outputs {label}"
            );
            let private: Vec<_> = recorded
                .private_outputs()
                .iter()
                .map(|v| {
                    json!({
                        "valueAtoms": v.value.0.iter().map(|a| &a.0).collect::<Vec<_>>(),
                        "alignment": v.alignment,
                    })
                })
                .collect();
            assert_eq!(
                json!(private),
                row["privateTranscript"],
                "transcript {label}"
            );
            assert_eq!(
                json!(recorded.replay().unwrap().program()),
                row["publicTranscript"],
                "VM {label}"
            );
            assert_eq!(recorded.execution_gas(), native.gas_cost, "gas {label}");
            let gas = json!(recorded.execution_gas());
            for dim in ["readTime", "computeTime", "bytesWritten", "bytesDeleted"] {
                let total: u64 = row["queries"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|q| q[dim].as_str().unwrap().parse::<u64>().unwrap())
                    .sum();
                assert_eq!(gas[dim], total, "{label}: {dim}");
            }
        }
    }
    assert_eq!(count, 5);
}
