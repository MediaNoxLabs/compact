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

//! Checked-in evidence is bound to the original source, capture code and typed
//! assertions. Changes require reviewing and regenerating the explicit matrix.
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, fs, path::Path};
fn data(name: &str) -> Value {
    serde_json::from_str(
        &fs::read_to_string(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("oracle")
                .join(name),
        )
        .unwrap(),
    )
    .unwrap()
}
fn hash_rows(rows: &Value) {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    for row in rows.as_array().unwrap() {
        let path = row["path"].as_str().unwrap();
        assert!(!Path::new(path).is_absolute());
        assert!(!path.split('/').any(|p| p == ".."));
        assert_eq!(
            hex::encode(Sha256::digest(fs::read(root.join(path)).unwrap())),
            row["sha256"],
            "stale evidence: {path}"
        );
    }
}
#[test]
fn capture_source_and_reviewed_test_identities() {
    for file in [
        "lifecycle.json",
        "pure.json",
        "point-lifecycle.json",
        "alias-lifecycle.json",
        "service-lifecycle.json",
        "schnorr-method-lifecycle.json",
        "jwk-method-lifecycle.json",
    ] {
        hash_rows(&data(file)["provenance"]);
    }
    hash_rows(&data("reviewed-matrix.json")["artifacts"]);
}
#[test]
fn exact_reviewed_native_cohort_and_outcomes() {
    let capture = data("lifecycle.json");
    let matrix = data("reviewed-matrix.json");
    let scenarios = capture["scenarios"].as_array().unwrap();
    assert_eq!(scenarios.len(), 11);
    assert_eq!(
        scenarios
            .iter()
            .filter(|s| s.get("constructorError").is_some())
            .count(),
        4
    );
    let rows: Vec<_> = scenarios
        .iter()
        .flat_map(|s| {
            s["steps"].as_array().unwrap().iter().map(move |r| {
                (
                    format!(
                        "{}/{}",
                        s["id"].as_str().unwrap(),
                        r["id"].as_str().unwrap()
                    ),
                    r,
                )
            })
        })
        .collect();
    assert_eq!(rows.len(), 122);
    let ids: BTreeSet<_> = rows.iter().map(|(id, _)| id.as_str()).collect();
    assert_eq!(ids.len(), rows.len());
    assert_eq!(
        matrix["lifecycle_cases"].as_array().unwrap().len(),
        rows.len()
    );
    for (id, row) in &rows {
        let reviewed = matrix["lifecycle_cases"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["case"] == *id)
            .unwrap();
        assert_eq!(reviewed["export"], row["name"]);
        assert_eq!(reviewed["exact_error"], row["error"]);
        assert_eq!(
            reviewed["outcome"],
            if row.get("error").is_some() {
                "error"
            } else {
                "success"
            }
        );
    }
    let exports: BTreeSet<_> = rows
        .iter()
        .map(|(_, r)| r["name"].as_str().unwrap())
        .collect();
    assert_eq!(
        exports,
        BTreeSet::from([
            "rotateControllerKey",
            "recoverControllerKey",
            "setAlsoKnownAs",
            "setVerificationMethod",
            "removeVerificationMethod",
            "setSchnorrJubjubVerificationMethod",
            "removeSchnorrJubjubVerificationMethod",
            "verifySchnorrJubjubDigestSignature",
            "setVerificationMethodRelation",
            "setService",
            "removeService",
            "deactivate"
        ])
    );
    for name in exports {
        assert!(
            rows.iter()
                .any(|(_, r)| r["name"] == name && r.get("error").is_none()),
            "missing native success {name}"
        );
    }
}
#[test]
fn exact_twelve_direct_pure_exports_and_unique_cases() {
    let capture = data("pure.json");
    let matrix = data("reviewed-matrix.json");
    let rows = capture["cases"].as_array().unwrap();
    assert_eq!(rows.len(), 102);
    let exports: BTreeSet<_> = rows.iter().map(|r| r["name"].as_str().unwrap()).collect();
    assert_eq!(exports.len(), 12);
    let ids: BTreeSet<_> = rows
        .iter()
        .map(|r| (r["name"].as_str().unwrap(), r["id"].as_str().unwrap()))
        .collect();
    assert_eq!(ids.len(), 102);
    assert_eq!(matrix["pure_cases"].as_array().unwrap().len(), 102);
    for r in rows {
        assert!(
            matrix["pure_cases"]
                .as_array()
                .unwrap()
                .iter()
                .any(|m| m["export"] == r["name"] && m["case"] == r["id"])
        );
    }
}
