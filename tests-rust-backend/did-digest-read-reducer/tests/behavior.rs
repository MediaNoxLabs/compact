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

use compact_rust_did_digest_read_reducer_fixture::{ledger_contract as c, runtime as r};
use midnight_compact_testkit::{ArtifactIdentity, ContractLab, Environment, LabError};
use r::context::{ConstructorContext, ConstructorResult, WitnessContext};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::cell::RefCell;

struct Witness {
    admit: bool,
    calls: RefCell<Vec<&'static str>>,
}
impl c::TryWitnesses<u64> for Witness {
    fn admitted(
        &self,
        context: WitnessContext<'_, u64, c::LedgerView<'_>>,
    ) -> Result<(u64, bool), r::CompactError> {
        self.calls.borrow_mut().push("admitted");
        Ok((*context.private_state + 1, self.admit))
    }
}
fn state(row: &Value) -> r::ledger::ContractState<r::ledger::DefaultDB> {
    midnight_serialize::tagged_deserialize(
        &mut hex::decode(row["before"].as_str().unwrap())
            .unwrap()
            .as_slice(),
    )
    .unwrap()
}
fn identity() -> ArtifactIdentity {
    ArtifactIdentity {
        source_sha256: Sha256::digest(include_bytes!(
            "../../../examples/rust_backend/did_digest_read_reducer/contract.compact"
        ))
        .into(),
        generated_sha256: Sha256::digest(include_bytes!("../lib.rs")).into(),
    }
}
#[test]
fn renamed_product_map_read_matches_independent_ts_for_success_and_failures() {
    let capture: Value = serde_json::from_str(include_str!("../oracle/cases.json")).unwrap();
    assert_eq!(
        capture["format"],
        "compact-did-digest-read-reducer-capture/v1"
    );
    let rows = capture["cases"].as_array().unwrap();
    assert_eq!(rows.len(), 4);
    for row in rows {
        let label = row["id"].as_str().unwrap();
        let before = state(row);
        let private = row["privateBefore"].as_u64().unwrap();
        let mut lab = ContractLab::from_constructor(
            identity(),
            Environment::new(Default::default(), Default::default(), [0x88; 32]),
            ConstructorResult::new(ConstructorContext::new(private), before.data.clone()),
        )
        .unwrap();
        let id = r::OpaqueString(row["methodId"].as_str().unwrap().to_owned());
        let point = r::ec_mul_generator(r::Field::from(
            row["expectedKey"].as_str().unwrap().parse::<u64>().unwrap(),
        ))
        .unwrap();
        let nw = Witness {
            admit: row["admit"].as_bool().unwrap(),
            calls: RefCell::new(Vec::new()),
        };
        let rw = Witness {
            admit: row["admit"].as_bool().unwrap(),
            calls: RefCell::new(Vec::new()),
        };
        let native = c::verify(
            ConstructorResult::new(ConstructorContext::new(private), before.data.clone())
                .into_circuit_context(Default::default()),
            &nw,
            id.clone(),
            point,
        );
        let recorded = lab.recorded(|ctx| c::recorded::verify(ctx, &rw, id.clone(), point));
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
                matches!(recorded, Err(LabError::Execution(ref error)) if *error == expected),
                "recorded {label}"
            );
            assert_eq!(
                lab.snapshot().public_state(),
                &before.data,
                "rollback {label}"
            );
            assert_eq!(*lab.private_state(), private, "private rollback {label}");
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
        assert_eq!(*lab.private_state(), row["privateAfter"].as_u64().unwrap());
        assert_eq!(
            recorded.private_outputs(),
            &native.private_transcript_outputs
        );
        let private: Vec<_> = recorded
            .private_outputs()
            .iter()
            .map(|value| {
                json!({
                    "valueAtoms": value.value.0.iter().map(|atom| &atom.0).collect::<Vec<_>>(),
                    "alignment": value.alignment,
                })
            })
            .collect();
        assert_eq!(json!(private), row["privateTranscript"], "private {label}");
        assert_eq!(
            json!(recorded.replay().unwrap().program()),
            row["publicTranscript"],
            "VM {label}"
        );
        assert_eq!(recorded.execution_gas(), native.gas_cost, "gas {label}");
        let gas = json!(recorded.execution_gas());
        for dimension in ["readTime", "computeTime", "bytesWritten", "bytesDeleted"] {
            let sum: u64 = row["queries"]
                .as_array()
                .unwrap()
                .iter()
                .map(|q| q[dimension].as_str().unwrap().parse::<u64>().unwrap())
                .sum();
            assert_eq!(gas[dimension], sum, "{label}: {dimension}");
        }
    }
}
