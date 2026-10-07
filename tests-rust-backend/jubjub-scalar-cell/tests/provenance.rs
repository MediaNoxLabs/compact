// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use sha2::{Digest, Sha256};
#[test]
fn retained_independent_reference_is_bound_to_its_exact_sources() {
    let manifest: serde_json::Value =
        serde_json::from_str(include_str!("../oracle/provenance.json")).unwrap();
    assert_eq!(manifest["format"], "compact-acc-jubjub-reference/v1");
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    for (path, expected) in manifest["files_sha256"].as_object().unwrap() {
        assert_eq!(
            hex::encode(Sha256::digest(std::fs::read(root.join(path)).unwrap())),
            expected.as_str().unwrap(),
            "reference drift: {path}"
        );
    }
}
