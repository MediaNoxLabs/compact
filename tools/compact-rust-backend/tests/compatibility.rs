// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

use compact_rust_backend::compatibility::{required, validate_root};

static NEXT: AtomicU64 = AtomicU64::new(0);
struct Root(PathBuf);
impl Root {
    fn new() -> Self {
        let root = Self(std::env::temp_dir().join(format!(
            "compact-compatibility-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        )));
        let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        for package in ["runtime-rs", "runtime-rs-macros"] {
            fs::create_dir_all(root.0.join(package).join("src")).unwrap();
            fs::copy(
                repo.join(package).join("Cargo.toml"),
                root.0.join(package).join("Cargo.toml"),
            )
            .unwrap();
        }
        fs::copy(
            repo.join("runtime-rs/compatibility.json"),
            root.0.join("runtime-rs/compatibility.json"),
        )
        .unwrap();
        fs::write(
            root.0.join("runtime-rs/src/lib.rs"),
            "pub const RUST_RUNTIME_ABI:u32=50; pub const LEDGER_VERSION:&str=\"ledger-8.0.3\";",
        )
        .unwrap();
        fs::write(
            root.0.join("runtime-rs-macros/src/lib.rs"),
            "extern crate proc_macro;",
        )
        .unwrap();
        root
    }
    fn replace(&self, path: &str, before: &str, after: &str) {
        let path = self.0.join(path);
        let source = fs::read_to_string(&path).unwrap();
        assert!(source.contains(before), "missing test mutation");
        fs::write(path, source.replace(before, after)).unwrap();
    }
    fn fails(&self, expected: &str) {
        let error = validate_root(&self.0).unwrap_err().to_string();
        assert!(error.contains(expected), "{error}");
    }
}
impl Drop for Root {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

#[test]
fn current_source_pair_matches_compiler_and_is_identified() {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let result = validate_root(&repo).unwrap();
    assert_eq!(result.compatibility, required());
    assert_eq!(result.root(), fs::canonicalize(&repo).unwrap());
    let testkit: toml_edit::DocumentMut = fs::read_to_string(repo.join("testkit-rs/Cargo.toml"))
        .unwrap()
        .parse()
        .unwrap();
    assert_eq!(
        testkit["package"]["rust-version"].as_str(),
        Some(required().rust_version.as_str())
    );
    assert_eq!(
        testkit["dependencies"]["midnight-compact-runtime"]["version"].as_str(),
        Some(format!("={}", required().runtime_version).as_str())
    );
    assert!(
        result
            .source_sha256
            .contains_key("runtime-rs-macros/src/lib.rs")
    );
    assert!(
        result
            .source_sha256
            .contains_key("runtime-rs/compatibility.json")
    );
    assert!(result.source_sha256.values().all(|hash| hash.len() == 64));
    let embedded: serde_json::Value =
        serde_json::from_str(include_str!("../src/compatibility.json")).unwrap();
    assert_eq!(embedded, serde_json::to_value(required()).unwrap());
}

#[test]
fn missing_malformed_and_unknown_metadata_are_not_accepted() {
    let root = Root::new();
    fs::remove_file(root.0.join("runtime-rs/compatibility.json")).unwrap();
    root.fails("no fallback");
    fs::write(root.0.join("runtime-rs/compatibility.json"), "{").unwrap();
    root.fails("invalid Rust runtime compatibility record");
    let mut value = serde_json::to_value(required()).unwrap();
    value["unknown"] = true.into();
    fs::write(
        root.0.join("runtime-rs/compatibility.json"),
        value.to_string(),
    )
    .unwrap();
    root.fails("unknown field");
}

#[test]
fn all_declared_version_boundaries_are_exact() {
    for (key, value) in [
        ("schema_version", serde_json::json!(2)),
        ("runtime_abi", serde_json::json!(49)),
        ("ir_schema", serde_json::json!(19)),
        ("capability_schema", serde_json::json!(2)),
        ("ledger_version", serde_json::json!("ledger-9.0.0")),
        ("rust_version", serde_json::json!("1.99")),
        ("runtime_version", serde_json::json!("0.2.0")),
        ("macros_version", serde_json::json!("0.2.0")),
    ] {
        let root = Root::new();
        let mut record = serde_json::to_value(required()).unwrap();
        record[key] = value;
        fs::write(
            root.0.join("runtime-rs/compatibility.json"),
            record.to_string(),
        )
        .unwrap();
        root.fails(key);
    }
}

#[test]
fn manifest_and_source_disagreement_cannot_hide_behind_valid_record() {
    for (path, before, after, expected) in [
        (
            "runtime-rs/Cargo.toml",
            "name = \"midnight-compact-runtime\"",
            "name = \"wrong\"",
            "package.name",
        ),
        (
            "runtime-rs/Cargo.toml",
            "rust-version = \"1.88\"",
            "rust-version = \"1.99\"",
            "rust-version",
        ),
        (
            "runtime-rs-macros/Cargo.toml",
            "version = \"0.1.0\"",
            "version = \"0.2.0\"",
            "package.version",
        ),
        (
            "runtime-rs/Cargo.toml",
            "version = \"=0.1.0\"",
            "version = \"=0.2.0\"",
            "macros.version",
        ),
        (
            "runtime-rs/Cargo.toml",
            "midnight-onchain-vm = \"=3.0.0\"",
            "midnight-onchain-vm = \"=4.0.0\"",
            "midnight-onchain-vm.version",
        ),
        (
            "runtime-rs/Cargo.toml",
            "version = \"=8.0.3\", optional",
            "version = \"=9.0.0\", optional",
            "midnight-ledger.version",
        ),
        (
            "runtime-rs/src/lib.rs",
            "ABI:u32=50",
            "ABI:u32=49",
            "RUST_RUNTIME_ABI",
        ),
        (
            "runtime-rs/src/lib.rs",
            "ledger-8.0.3",
            "ledger-9.0.0",
            "LEDGER_VERSION",
        ),
    ] {
        let root = Root::new();
        root.replace(path, before, after);
        root.fails(expected);
    }
}

#[test]
fn content_fingerprints_change_without_claiming_authentication() {
    let root = Root::new();
    let before = validate_root(&root.0).unwrap();
    fs::write(
        root.0.join("runtime-rs/src/extra.rs"),
        "// local source change",
    )
    .unwrap();
    let after = validate_root(&root.0).unwrap();
    assert_eq!(before.compatibility, after.compatibility);
    assert_ne!(before.source_sha256, after.source_sha256);
}

#[test]
fn public_cli_preflight_preserves_existing_output_without_frontend_or_fallback() {
    for mode in ["shared", "bundled", "registry"] {
        let root = Root::new();
        root.replace(
            "runtime-rs/compatibility.json",
            "\"runtime_abi\": 50",
            "\"runtime_abi\": 49",
        );
        let out = root.0.join("existing");
        fs::create_dir(&out).unwrap();
        fs::write(out.join("sentinel"), b"keep exact bytes").unwrap();
        let mut command = Command::new(env!("CARGO_BIN_EXE_compactc"));
        command
            .args(["--target", "rust"])
            .env("COMPACTC_SCHEME", root.0.join("must-not-execute"));
        if mode == "shared" {
            command
                .arg("--rust-runtime-root")
                .arg(root.0.join("runtime-rs/.."));
        } else {
            command.env("COMPACT_RUST_RUNTIME_DIR", &root.0);
        }
        if mode == "registry" {
            command.arg("--rust-runtime-registry");
        }
        let result = command
            .arg("missing-source.compact")
            .arg(&out)
            .output()
            .unwrap();
        assert!(!result.status.success());
        let error = String::from_utf8(result.stderr).unwrap();
        assert!(error.contains("runtime_abi"), "{mode}: {error}");
        assert_eq!(fs::read(out.join("sentinel")).unwrap(), b"keep exact bytes");
        assert_eq!(fs::read_dir(out).unwrap().count(), 1);
        assert!(!root.0.join(".existing.compactc.lock").exists());
    }
}

#[cfg(unix)]
#[test]
fn linked_source_entries_are_refused_before_copying() {
    let root = Root::new();
    std::os::unix::fs::symlink("lib.rs", root.0.join("runtime-rs/src/alias.rs")).unwrap();
    root.fails("unsupported runtime source entry");
}

#[test]
fn extra_ledger_graph_and_missing_macro_manifest_are_clear_failures() {
    let root = Root::new();
    root.replace(
        "runtime-rs/Cargo.toml",
        "[dependencies]",
        "[dependencies]\nmidnight-other-graph = \"=9.0.0\"",
    );
    root.fails("unrecorded Midnight dependency midnight-other-graph");
    let root = Root::new();
    fs::remove_file(root.0.join("runtime-rs-macros/Cargo.toml")).unwrap();
    root.fails("runtime-rs-macros/Cargo.toml");
}

#[test]
fn inline_dependency_table_is_valid_and_malformed_container_is_an_error() {
    let root = Root::new();
    let path = root.0.join("runtime-rs/Cargo.toml");
    let mut document: toml_edit::DocumentMut = fs::read_to_string(&path).unwrap().parse().unwrap();
    let inline = document["dependencies"]
        .as_table()
        .unwrap()
        .clone()
        .into_inline_table();
    document["dependencies"] = toml_edit::Item::Value(toml_edit::Value::InlineTable(inline));
    fs::write(&path, document.to_string()).unwrap();
    assert_eq!(validate_root(&root.0).unwrap().compatibility, required());
    document["dependencies"] = toml_edit::value("invalid dependency container");
    fs::write(&path, document.to_string()).unwrap();
    root.fails("dependencies.midnight-compact-runtime-macros.version");
}

#[cfg(unix)]
#[test]
fn selected_source_changes_during_frontend_do_not_replace_output() {
    use std::os::unix::fs::PermissionsExt;
    let root = Root::new();
    let mut ir: serde_json::Value =
        serde_json::from_str(include_str!("../fixtures/identity.json")).unwrap();
    ir["schema_version"] = compact_rust_backend::ir::SCHEMA_VERSION.into();
    let ir_path = root.0.join("prepared-ir.json");
    fs::write(&ir_path, ir.to_string()).unwrap();
    let script = root.0.join("frontend");
    fs::write(&script, r#"#!/bin/sh
set -eu
for out do :; done
mkdir -p "$out/contract" "$out/compiler"
cp "$PREPARED_IR" "$out/contract/compact-rust-ir.json"
printf '%s' '{"circuits":[{"name":"identity","pure":true,"proof":false}]}' > "$out/compiler/contract-info.json"
printf '%s' '{"manifest-version":"1","compiler-version":"test","language-version":"test","runtime-version":"test"}' > "$out/compiler/contract-manifest.json"
printf '\n// ordinary source edit during generation\n' >> "$SELECTED_RUNTIME_SRC"
"#).unwrap();
    fs::set_permissions(&script, fs::Permissions::from_mode(0o700)).unwrap();
    let output = root.0.join("existing");
    fs::create_dir(&output).unwrap();
    fs::write(output.join("sentinel"), b"original output").unwrap();
    let result = Command::new(env!("CARGO_BIN_EXE_compactc"))
        .args(["--target", "rust", "--rust-runtime-root"])
        .arg(&root.0)
        .arg("identity.compact")
        .arg(&output)
        .env("COMPACTC_SCHEME", &script)
        .env("PREPARED_IR", &ir_path)
        .env("SELECTED_RUNTIME_SRC", root.0.join("runtime-rs/src/lib.rs"))
        .output()
        .unwrap();
    assert!(!result.status.success());
    let error = String::from_utf8(result.stderr).unwrap();
    assert!(
        error.contains("source changed during generation"),
        "{error}"
    );
    assert_eq!(
        fs::read(output.join("sentinel")).unwrap(),
        b"original output"
    );
    assert_eq!(fs::read_dir(output).unwrap().count(), 1);
}

#[test]
fn broken_installed_root_does_not_fall_back_to_build_checkout() {
    let root = Root::new();
    let bin = root.0.join("installed/bin");
    fs::create_dir_all(&bin).unwrap();
    fs::create_dir_all(root.0.join("installed/share/compactc")).unwrap();
    let executable = bin.join("compactc");
    fs::copy(env!("CARGO_BIN_EXE_compactc"), &executable).unwrap();
    let output = root.0.join("existing");
    fs::create_dir(&output).unwrap();
    fs::write(output.join("sentinel"), b"keep").unwrap();
    let result = Command::new(executable)
        .args(["--target", "rust", "missing.compact"])
        .arg(&output)
        .env_remove("COMPACT_RUST_RUNTIME_DIR")
        .env("COMPACTC_SCHEME", root.0.join("must-not-run"))
        .output()
        .unwrap();
    assert!(!result.status.success());
    let error = String::from_utf8(result.stderr).unwrap();
    assert!(
        error.contains("installed/share/compactc/runtime-rs/compatibility.json"),
        "{error}"
    );
    assert!(error.contains("no fallback"), "{error}");
    assert_eq!(fs::read(output.join("sentinel")).unwrap(), b"keep");
}
