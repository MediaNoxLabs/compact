// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Keep original TS evidence immutable while reviewing a new Rust consumer.
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{fs, path::Path};

fn digest(path: &Path) -> String {
    hex::encode(Sha256::digest(fs::read(path).unwrap()))
}

fn relative(path: &str) -> &Path {
    let path = Path::new(path);
    assert!(
        path.components()
            .all(|part| matches!(part, std::path::Component::Normal(_)))
    );
    path
}

pub fn verify(manifest: &str, capture_name: &str) {
    let package = Path::new(manifest);
    let root = package.join("../..");
    let binding: Value = serde_json::from_slice(
        &fs::read(package.join("oracle/reviewed-rust-binding.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(binding["format"], "compact-reviewed-rust-binding/v1");
    let captures = binding["captures"].as_array().unwrap();
    let matches: Vec<_> = captures
        .iter()
        .filter(|row| row["file"] == capture_name)
        .collect();
    assert_eq!(
        matches.len(),
        1,
        "capture must have exactly one reviewed identity"
    );
    let capture_path = package.join("oracle").join(relative(capture_name));
    assert_eq!(
        digest(&capture_path),
        matches[0]["sha256"],
        "immutable TS capture changed"
    );
    for row in binding["reviewed_artifacts"].as_array().unwrap() {
        let path = row["path"].as_str().unwrap();
        assert_eq!(
            digest(&root.join(relative(path))),
            row["sha256"],
            "stale reviewed artifact: {path}"
        );
    }
    let capture: Value = serde_json::from_slice(&fs::read(capture_path).unwrap()).unwrap();
    let replacements = binding["replacements"].as_array().unwrap();
    let package_name = package.file_name().unwrap().to_str().unwrap();
    let allowed = [
        format!("tests-rust-backend/{package_name}/lib.rs"),
        format!("tests-rust-backend/{package_name}/tests/provenance.rs"),
    ];
    for replacement in replacements {
        let path = replacement["path"].as_str().unwrap();
        assert!(
            allowed.iter().any(|allowed| allowed == path),
            "unreviewed replacement kind"
        );
        assert_eq!(
            replacements
                .iter()
                .filter(|row| row["path"] == path)
                .count(),
            1
        );
        assert_eq!(
            digest(&root.join(relative(path))),
            replacement["current_sha256"]
        );
    }
    for row in capture["provenance"].as_array().unwrap() {
        let path = row["path"].as_str().unwrap();
        let current = digest(&root.join(relative(path)));
        if let Some(replacement) = replacements.iter().find(|entry| entry["path"] == path) {
            assert_eq!(
                row["sha256"], replacement["captured_sha256"],
                "historical binding changed"
            );
            assert_eq!(current, replacement["current_sha256"]);
        } else {
            assert_eq!(current, row["sha256"], "stale unchanged provenance: {path}");
        }
    }
}
