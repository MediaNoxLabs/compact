// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//  http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::path::PathBuf;

#[test]
fn pinned_source_closure_matches_all_seventeen_hashes() {
    let manifest: serde_json::Value = serde_json::from_str(include_str!(
        "../../../examples/rust_backend/vc_passport_adoption/source-manifest.json"
    ))
    .unwrap();
    let files = manifest["files"].as_array().unwrap();
    assert_eq!(files.len(), 17);
    let source = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples/rust_backend/vc_passport_adoption");
    let mut seen = BTreeSet::new();
    for file in files {
        let path = file["path"].as_str().unwrap();
        assert!(path.ends_with(".compact"));
        assert!(!path.starts_with('/') && !path.contains(".."));
        assert!(seen.insert(path));
        let bytes = std::fs::read(source.join(path)).unwrap();
        assert_eq!(
            format!("{:x}", Sha256::digest(bytes)),
            file["sha256"],
            "{path}"
        );
    }
}

#[test]
fn dual_typescript_captures_and_export_inventory_remain_exact() {
    let branch: serde_json::Value =
        serde_json::from_str(include_str!("../oracle/branch-ts-capture.json")).unwrap();
    let upstream: serde_json::Value =
        serde_json::from_str(include_str!("../oracle/upstream-ts-capture.json")).unwrap();
    let branch_request: serde_json::Value =
        serde_json::from_str(include_str!("../oracle/branch-request-capture.json")).unwrap();
    let upstream_request: serde_json::Value =
        serde_json::from_str(include_str!("../oracle/upstream-request-capture.json")).unwrap();
    let branch_age: serde_json::Value =
        serde_json::from_str(include_str!("../oracle/branch-age-capture.json")).unwrap();
    let upstream_age: serde_json::Value =
        serde_json::from_str(include_str!("../oracle/upstream-age-capture.json")).unwrap();
    let branch_private: serde_json::Value =
        serde_json::from_str(include_str!("../oracle/branch-private-parts-capture.json")).unwrap();
    let upstream_private: serde_json::Value = serde_json::from_str(include_str!(
        "../oracle/upstream-private-parts-capture.json"
    ))
    .unwrap();
    let branch_protocol: serde_json::Value =
        serde_json::from_str(include_str!("../oracle/branch-protocol-capture.json")).unwrap();
    let upstream_protocol: serde_json::Value =
        serde_json::from_str(include_str!("../oracle/upstream-protocol-capture.json")).unwrap();
    let branch_roots: serde_json::Value =
        serde_json::from_str(include_str!("../oracle/branch-roots-capture.json")).unwrap();
    let upstream_roots: serde_json::Value =
        serde_json::from_str(include_str!("../oracle/upstream-roots-capture.json")).unwrap();
    let branch_bindings: serde_json::Value =
        serde_json::from_str(include_str!("../oracle/branch-bindings-capture.json")).unwrap();
    let upstream_bindings: serde_json::Value =
        serde_json::from_str(include_str!("../oracle/upstream-bindings-capture.json")).unwrap();
    assert_ne!(branch["profile"], upstream["profile"]);
    assert_ne!(branch_age["profile"], upstream_age["profile"]);
    assert_ne!(branch_request["profile"], upstream_request["profile"]);
    assert_eq!(branch["rows"], upstream["rows"]);
    assert_eq!(branch_age["rows"], upstream_age["rows"]);
    assert_eq!(branch_request["rows"], upstream_request["rows"]);
    assert_eq!(branch_private["rows"], upstream_private["rows"]);
    assert_eq!(branch_protocol["rows"], upstream_protocol["rows"]);
    assert_eq!(branch_roots["rows"], upstream_roots["rows"]);
    assert_eq!(branch_bindings["rows"], upstream_bindings["rows"]);
    assert_eq!(branch["rows"].as_array().unwrap().len(), 24);
    assert_eq!(branch_age["rows"].as_array().unwrap().len(), 25);
    assert_eq!(branch_request["rows"].as_array().unwrap().len(), 16);
    assert_eq!(branch_private["rows"].as_array().unwrap().len(), 11);
    assert_eq!(branch_protocol["rows"].as_array().unwrap().len(), 20);
    assert_eq!(branch_roots["rows"].as_array().unwrap().len(), 10);
    assert_eq!(branch_bindings["rows"].as_array().unwrap().len(), 23);

    let coverage: serde_json::Value =
        serde_json::from_str(include_str!("../oracle/coverage.json")).unwrap();
    let exports = coverage["exports"].as_array().unwrap();
    assert_eq!(exports.len(), 75);
    let measured = exports
        .iter()
        .filter(|export| export["evidence"] == "direct_rust_and_dual_typescript")
        .collect::<Vec<_>>();
    assert_eq!(measured.len(), 48);
    let captured_cases = branch["rows"]
        .as_array()
        .unwrap()
        .iter()
        .chain(branch_age["rows"].as_array().unwrap())
        .chain(branch_request["rows"].as_array().unwrap())
        .chain(branch_private["rows"].as_array().unwrap())
        .chain(branch_protocol["rows"].as_array().unwrap())
        .chain(branch_roots["rows"].as_array().unwrap())
        .chain(branch_bindings["rows"].as_array().unwrap())
        .map(|row| row["name"].as_str().unwrap())
        .collect::<BTreeSet<_>>();
    let covered_cases = measured
        .iter()
        .flat_map(|export| export["cases"].as_array().unwrap())
        .map(|case| case.as_str().unwrap())
        .collect::<BTreeSet<_>>();
    assert_eq!(captured_cases, covered_cases);
    assert_eq!(captured_cases.len(), 129);
    assert!(
        exports
            .iter()
            .filter(|export| export["evidence"] == "compile_only")
            .all(|export| export["cases"].as_array().unwrap().is_empty())
    );
}

#[test]
fn oracle_programs_and_captures_match_recorded_provenance() {
    let provenance: serde_json::Value =
        serde_json::from_str(include_str!("../oracle/provenance.json")).unwrap();
    let oracle = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("oracle");
    let files = provenance["oracle_files"].as_object().unwrap();
    assert_eq!(files.len(), 24);
    for (name, expected) in files {
        assert!(!name.contains('/') && !name.contains(".."));
        let bytes = std::fs::read(oracle.join(name)).unwrap();
        assert_eq!(
            format!("{:x}", Sha256::digest(bytes)),
            expected.as_str().unwrap(),
            "{name}"
        );
    }
    let lib = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("lib.rs");
    assert_eq!(
        format!("{:x}", Sha256::digest(std::fs::read(lib).unwrap())),
        provenance["branch_typescript_and_rust"]["checked_in_generated_rust_sha256"]
    );
}
