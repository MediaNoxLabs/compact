// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Corpus identity and filesystem boundaries are separate from compiler outcomes.

use serde::Deserialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    collections::HashSet,
    path::{Component, Path},
};

pub(super) const MANIFEST_SHA256: &str =
    "36a663979756572cd6a9d10ea6bf122a7e2d1d78ed0504244bede61f8038d6b4";

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Case {
    pub id: String,
    pub file: String,
    pub root: String,
    pub rules: Vec<String>,
    pub sha256: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    scope: String,
    cases: Vec<Case>,
}

pub(super) struct Input {
    pub case: Case,
    pub bytes: Vec<u8>,
}

pub(super) fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn relative_path(value: &str) -> bool {
    !value.is_empty()
        && Path::new(value)
            .components()
            .all(|c| matches!(c, Component::Normal(_)))
}

fn validate_cases(cases: &[Case]) -> Result<(), String> {
    let mut ids = HashSet::new();
    let mut files = HashSet::new();
    for case in cases {
        if case.id.is_empty()
            || !case
                .id
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_')
        {
            return Err(format!("unsafe case identifier: {}", case.id));
        }
        if !relative_path(&case.file) || case.root.is_empty() || case.rules.is_empty() {
            return Err(format!("invalid case metadata: {}", case.id));
        }
        if !ids.insert(&case.id) || !files.insert(&case.file) {
            return Err(format!("duplicate case identity or input: {}", case.id));
        }
        if case.sha256.len() != 64
            || !case
                .sha256
                .bytes()
                .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
        {
            return Err(format!("invalid input digest: {}", case.id));
        }
    }
    Ok(())
}

fn authenticated_manifest(bytes: &[u8]) -> Result<Manifest, String> {
    if digest(bytes) != MANIFEST_SHA256 {
        return Err("original matrix manifest digest mismatch".into());
    }
    let manifest: Manifest = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
    if manifest.cases.len() != 163 || manifest.scope.is_empty() {
        return Err("original matrix shape mismatch".into());
    }
    validate_cases(&manifest.cases)?;
    Ok(manifest)
}

pub(super) fn load(path: &Path) -> Result<Vec<Input>, String> {
    let manifest = authenticated_manifest(&std::fs::read(path).map_err(|e| e.to_string())?)?;
    let base = path
        .parent()
        .ok_or("matrix has no parent")?
        .canonicalize()
        .map_err(|e| e.to_string())?;
    let mut canonical_inputs = HashSet::new();
    let mut inputs = Vec::new();
    for case in manifest.cases {
        let path = base
            .join(&case.file)
            .canonicalize()
            .map_err(|e| e.to_string())?;
        if !path.starts_with(&base) || !canonical_inputs.insert(path.clone()) {
            return Err(format!("escaped or duplicate canonical input: {}", case.id));
        }
        let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
        if digest(&bytes) != case.sha256 {
            return Err(format!("input digest mismatch: {}", case.id));
        }
        inputs.push(Input { case, bytes });
    }
    Ok(inputs)
}

pub(super) fn legacy_authority() -> Value {
    let authority: Value = serde_json::from_str(include_str!("legacy-authority.json")).unwrap();
    let repository = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    for row in authority["files"].as_array().unwrap() {
        let path = row["path"].as_str().unwrap();
        assert!(relative_path(path));
        assert_eq!(
            digest(&std::fs::read(repository.join(path)).unwrap()),
            row["sha256"],
            "old source drift: {path}"
        );
    }
    authority
}

#[test]
fn corpus_identity_rejects_duplicate_and_escaped_inputs() {
    let original = Case {
        id: "one".into(),
        file: "matrix-inputs/one.json".into(),
        root: "root".into(),
        rules: vec!["LocalUnit".into()],
        sha256: "a".repeat(64),
    };
    assert!(validate_cases(std::slice::from_ref(&original)).is_ok());
    assert!(validate_cases(&[original.clone(), original.clone()]).is_err());
    for path in [
        "../one.json",
        "/one.json",
        "matrix-inputs/../../one.json",
        "",
    ] {
        let mut changed = original.clone();
        changed.file = path.into();
        assert!(validate_cases(&[changed]).is_err(), "{path}");
    }
    let mut duplicate_file = original.clone();
    duplicate_file.id = "two".into();
    assert!(validate_cases(&[original.clone(), duplicate_file]).is_err());
    let mut malformed = original;
    malformed.sha256 = "0".repeat(63);
    assert!(validate_cases(&[malformed]).is_err());
    assert!(authenticated_manifest(br#"{"scope":"forged","cases":[]}"#).is_err());
}
