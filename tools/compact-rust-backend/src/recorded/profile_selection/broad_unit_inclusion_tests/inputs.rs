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

const RETIREMENT_SHA256: &str = "eda751f75344d765430226e2651201e947c1d1d584818c43fe024424df65df0c";

fn retirement_manifest(bytes: &[u8]) -> Result<Value, String> {
    if digest(bytes) != RETIREMENT_SHA256 {
        return Err("unreviewed legacy retirement manifest".into());
    }
    serde_json::from_slice(bytes).map_err(|error| error.to_string())
}

// #597 added only optional resource metadata to this retained oracle owner.
// Authenticate its immutable checkpoint before the original retirement projection.
const RESOURCE_METADATA_SHA256: &str =
    "a4d3559923d732989a8245bb1984f131479feba07554a2e5006dec16b89f8145";
const RESOURCE_METADATA: &[u8] =
    include_bytes!("../../../../recording_admission_inventory.resource-diagnostics-597.json");

fn resource_metadata_manifest(bytes: &[u8]) -> Result<Value, String> {
    if digest(bytes) != RESOURCE_METADATA_SHA256 {
        return Err("unreviewed resource metadata checkpoint".into());
    }
    serde_json::from_slice(bytes).map_err(|error| error.to_string())
}

// #611 replaces obsolete public refusal controls in one test-only oracle owner.
// Pin this separate checkpoint to avoid a self-referential inventory manifest.
const COMMIT_INPUTS_SHA256: &str =
    "4feeb870cede6548b17cc870c6d67f545274f9c98c73923e691acca665c45caf";
const COMMIT_INPUTS: &[u8] = include_bytes!("legacy-commit-inputs-611.json");

fn commit_inputs_manifest(bytes: &[u8]) -> Result<Value, String> {
    if digest(bytes) != COMMIT_INPUTS_SHA256 {
        return Err("unreviewed commitment test checkpoint".into());
    }
    serde_json::from_slice(bytes).map_err(|error| error.to_string())
}

// #614 preserves the complete early audited-local renderer as a test-only oracle.
// Reverse only its two authenticated cfg annotations before older projections.
const AUDITED_LOCAL_SHA256: &str =
    "3e433671c8b8c9086350ccbbe2f9898385a258a6482b5c97ac378f28014c9bcf";
const AUDITED_LOCAL: &[u8] = include_bytes!("legacy-audited-local-614.json");

fn audited_local_manifest(bytes: &[u8]) -> Result<Value, String> {
    if digest(bytes) != AUDITED_LOCAL_SHA256 {
        return Err("unreviewed audited-local checkpoint".into());
    }
    serde_json::from_slice(bytes).map_err(|error| error.to_string())
}

// #617 extracts the two live shared definitions from the frozen Plan oracle.
// Authenticate their new owner and restore the exact signed predecessor first.
const START_FUNDING_SHA256: &str =
    "adfbe698a69f9a31e0bc36bdfc0260099cdfd6daf43d222d97a49811042ebfed";
const START_FUNDING: &[u8] = include_bytes!("legacy-start-funding-617.json");

fn start_funding_manifest(bytes: &[u8]) -> Result<Value, String> {
    if digest(bytes) != START_FUNDING_SHA256 {
        return Err("unreviewed StartFunding extraction checkpoint".into());
    }
    serde_json::from_slice(bytes).map_err(|error| error.to_string())
}

fn shared_source(manifest: &Value, path: &str, bytes: &[u8]) -> Result<(), String> {
    let expected = manifest["shared_sources"]
        .get(path)
        .and_then(Value::as_str)
        .ok_or("unreviewed shared oracle source")?;
    if digest(bytes) != expected {
        return Err(format!("shared oracle source drift: {path}"));
    }
    Ok(())
}

fn projected_legacy_source(
    path: &str,
    current: &str,
    retirement: &Value,
    resource_metadata: &Value,
) -> Result<String, String> {
    let commit_inputs = commit_inputs_manifest(COMMIT_INPUTS)?;
    let audited_local = audited_local_manifest(AUDITED_LOCAL)?;
    let start_funding = start_funding_manifest(START_FUNDING)?;
    let before_start_funding = legacy_source(path, current, &start_funding)?;
    let before_audited_local = legacy_source(path, &before_start_funding, &audited_local)?;
    let before_commit = legacy_source(path, &before_audited_local, &commit_inputs)?;
    let before_metadata = legacy_source(path, &before_commit, resource_metadata)?;
    legacy_source(path, &before_metadata, retirement)
}

fn legacy_source(path: &str, current: &str, retirement: &Value) -> Result<String, String> {
    let Some(owner) = retirement["owners"].get(path) else {
        return Ok(current.to_owned());
    };
    if digest(current.as_bytes()) != owner["current_sha256"] {
        return Err(format!("retired oracle owner drift: {path}"));
    }
    let mut previous = current.to_owned();
    for edit in owner["predecessor_reconstruction"]
        .as_array()
        .ok_or("missing retirement edits")?
    {
        let current = edit["current"].as_str().ok_or("missing current edit")?;
        let old = edit["previous"].as_str().ok_or("missing previous edit")?;
        if current.is_empty() || previous.matches(current).count() != 1 {
            return Err(format!("nonunique retirement edit: {path}"));
        }
        previous = previous.replacen(current, old, 1);
    }
    if digest(previous.as_bytes()) != owner["previous_sha256"] {
        return Err(format!("signed oracle reconstruction differs: {path}"));
    }
    Ok(previous)
}

pub(super) fn legacy_authority() -> Value {
    let mut authority: Value = serde_json::from_str(include_str!("legacy-authority.json")).unwrap();
    let retirement = retirement_manifest(include_bytes!("legacy-retirement.json")).unwrap();
    let resource_metadata = resource_metadata_manifest(RESOURCE_METADATA).unwrap();
    let repository = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let start_funding = start_funding_manifest(START_FUNDING).unwrap();
    for path in start_funding["shared_sources"].as_object().unwrap().keys() {
        assert!(relative_path(path));
        shared_source(
            &start_funding,
            path,
            &std::fs::read(repository.join(path)).unwrap(),
        )
        .unwrap();
    }
    for row in authority["files"].as_array().unwrap() {
        let path = row["path"].as_str().unwrap();
        assert!(relative_path(path));
        let current = std::fs::read_to_string(repository.join(path)).unwrap();
        let previous =
            projected_legacy_source(path, &current, &retirement, &resource_metadata).unwrap();
        assert_eq!(
            digest(previous.as_bytes()),
            row["sha256"],
            "old source drift: {path}"
        );
    }
    authority["retirement_projection"] = retirement;
    authority["resource_metadata_projection"] = resource_metadata;
    authority["commit_input_test_projection"] = commit_inputs_manifest(COMMIT_INPUTS).unwrap();
    authority["audited_local_projection"] = audited_local_manifest(AUDITED_LOCAL).unwrap();
    authority["start_funding_projection"] = start_funding_manifest(START_FUNDING).unwrap();
    authority
}

#[test]
fn retired_oracle_projection_rejects_drift_and_forged_authority() {
    let bytes = include_bytes!("legacy-retirement.json");
    let retirement = retirement_manifest(bytes).unwrap();
    let resource_metadata = resource_metadata_manifest(RESOURCE_METADATA).unwrap();
    assert!(resource_metadata_manifest(br#"{"owners":{}}"#).is_err());
    assert!(retirement_manifest(br#"{"owners":{}}"#).is_err());
    let commit_inputs = commit_inputs_manifest(COMMIT_INPUTS).unwrap();
    assert!(commit_inputs_manifest(br#"{"owners":{}}"#).is_err());
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let start_funding = start_funding_manifest(START_FUNDING).unwrap();
    assert!(start_funding_manifest(br#"{"owners":{}}"#).is_err());
    let mut forged = start_funding.clone();
    forged["scope"] = Value::String("unreviewed".into());
    assert!(start_funding_manifest(&serde_json::to_vec(&forged).unwrap()).is_err());
    assert_eq!(start_funding["owners"].as_object().unwrap().len(), 1);
    assert_eq!(
        start_funding["shared_sources"].as_object().unwrap().len(),
        1
    );
    for path in start_funding["shared_sources"].as_object().unwrap().keys() {
        let bytes = std::fs::read(root.join(path)).unwrap();
        shared_source(&start_funding, path, &bytes).unwrap();
        let mut changed = bytes.clone();
        changed.extend_from_slice(b"\n// drift\n");
        assert!(shared_source(&start_funding, path, &changed).is_err());
        assert!(shared_source(&start_funding, "unreviewed.rs", &bytes).is_err());
    }

    for (path, row) in start_funding["owners"].as_object().unwrap() {
        let current = std::fs::read_to_string(root.join(path)).unwrap();
        let previous = legacy_source(path, &current, &start_funding).unwrap();
        assert_eq!(digest(previous.as_bytes()), row["previous_sha256"]);
        assert!(legacy_source(path, &(current.clone() + "\n// drift\n"), &start_funding).is_err());
        for edit in row["predecessor_reconstruction"].as_array().unwrap() {
            let partial = current.replacen(
                edit["current"].as_str().unwrap(),
                edit["previous"].as_str().unwrap(),
                1,
            );
            assert_ne!(partial, current);
            assert!(
                projected_legacy_source(path, &partial, &retirement, &resource_metadata).is_err()
            );
        }
    }
    let audited_local = audited_local_manifest(AUDITED_LOCAL).unwrap();
    assert!(audited_local_manifest(br#"{"owners":{}}"#).is_err());
    for (path, row) in audited_local["owners"].as_object().unwrap() {
        let current = std::fs::read_to_string(root.join(path)).unwrap();
        assert_eq!(
            digest(
                legacy_source(path, &current, &audited_local)
                    .unwrap()
                    .as_bytes()
            ),
            row["previous_sha256"]
        );
        assert!(
            projected_legacy_source(
                path,
                &(current.clone() + "\n// drift\n"),
                &retirement,
                &resource_metadata,
            )
            .is_err()
        );
        let edits = row["predecessor_reconstruction"].as_array().unwrap();
        assert_eq!(edits.len(), 2);
        for edit in edits {
            let partial = current.replacen(
                edit["current"].as_str().unwrap(),
                edit["previous"].as_str().unwrap(),
                1,
            );
            assert_ne!(partial, current);
            assert!(
                projected_legacy_source(path, &partial, &retirement, &resource_metadata).is_err()
            );
        }
    }
    for (path, row) in commit_inputs["owners"].as_object().unwrap() {
        let current = std::fs::read_to_string(root.join(path)).unwrap();
        assert_eq!(
            digest(
                legacy_source(path, &current, &commit_inputs)
                    .unwrap()
                    .as_bytes()
            ),
            row["previous_sha256"]
        );
        assert!(legacy_source(path, &(current.clone() + "\n// drift\n"), &commit_inputs).is_err());
        let edit = &row["predecessor_reconstruction"][0];
        let partial = current.replacen(
            edit["current"].as_str().unwrap(),
            edit["previous"].as_str().unwrap(),
            1,
        );
        assert_ne!(partial, current);
        assert!(projected_legacy_source(path, &partial, &retirement, &resource_metadata).is_err());
    }
    for (path, row) in retirement["owners"].as_object().unwrap() {
        let current = std::fs::read_to_string(root.join(path)).unwrap();
        assert_eq!(
            digest(
                projected_legacy_source(path, &current, &retirement, &resource_metadata)
                    .unwrap()
                    .as_bytes()
            ),
            row["previous_sha256"]
        );
        assert!(
            projected_legacy_source(
                path,
                &(current.clone() + "\n// drift\n"),
                &retirement,
                &resource_metadata,
            )
            .is_err()
        );
        let edit = &row["predecessor_reconstruction"][0];
        let before_start = legacy_source(path, &current, &start_funding).unwrap();
        let before_audited = legacy_source(path, &before_start, &audited_local).unwrap();
        let before_commit = legacy_source(path, &before_audited, &commit_inputs).unwrap();
        let before_metadata = legacy_source(path, &before_commit, &resource_metadata).unwrap();
        let partial = before_metadata.replacen(
            edit["current"].as_str().unwrap(),
            edit["previous"].as_str().unwrap(),
            1,
        );
        assert!(projected_legacy_source(path, &partial, &retirement, &resource_metadata).is_err());
    }
    // Every original source digest remains mandatory after all exact stages.
    let authority = legacy_authority();
    assert_eq!(authority["files"].as_array().unwrap().len(), 49);
    let path = "tools/compact-rust-backend/src/recorded/typed_plan/composition.rs";
    let current = std::fs::read_to_string(root.join(path)).unwrap();
    let edit = &resource_metadata["owners"][path]["predecessor_reconstruction"][0];
    let partial = current.replacen(
        edit["current"].as_str().unwrap(),
        edit["previous"].as_str().unwrap(),
        1,
    );
    assert_ne!(partial, current);
    assert!(projected_legacy_source(path, &partial, &retirement, &resource_metadata).is_err());
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
