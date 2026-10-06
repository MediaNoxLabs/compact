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

//! Developer-facing source compatibility checks, not authenticated provenance.
//!
//! A selected source root must declare the compiler's exact compatibility matrix.
//! Source hashes identify the inspected files; they do not establish trust in them.
//! Generated contracts also retain the independent linked runtime ABI assertion.

use std::collections::BTreeMap;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use toml_edit::{DocumentMut, Item};

/// Current minimum Rust version of generated consumers and their runtime pair.
pub const RUST_VERSION: &str = "1.88";

/// Version boundaries are independent: a milestone is not a package or ABI version.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Compatibility {
    pub schema_version: u32,
    pub runtime_abi: u32,
    pub ir_schema: u32,
    pub capability_schema: u32,
    pub ledger_version: String,
    pub rust_version: String,
    pub runtime_package: String,
    pub runtime_version: String,
    pub macros_package: String,
    pub macros_version: String,
    pub ledger_dependencies: BTreeMap<String, String>,
}

/// Compiler-owned requirements, included in both packaged and Nix backend builds.
pub fn required() -> Compatibility {
    let mut required: Compatibility = serde_json::from_str(include_str!("compatibility.json"))
        .expect("compiler compatibility record is valid");
    required.runtime_abi = crate::RUNTIME_ABI_VERSION;
    required.ir_schema = crate::ir::SCHEMA_VERSION;
    required.capability_schema = crate::RUST_CAPABILITY_SCHEMA_VERSION;
    required.rust_version = RUST_VERSION.into();
    required
}

/// An inspected root retained for the entire generation, without fallback/reselection.
#[derive(Debug)]
pub struct ValidatedRuntime {
    root: PathBuf,
    pub compatibility: Compatibility,
    pub source_sha256: BTreeMap<String, String>,
}

impl ValidatedRuntime {
    pub fn root(&self) -> &Path {
        &self.root
    }
    pub fn runtime_path(&self) -> PathBuf {
        self.root.join("runtime-rs")
    }
}

fn mismatch(
    label: &str,
    selected: impl std::fmt::Debug,
    required: impl std::fmt::Debug,
) -> Box<dyn Error> {
    format!("Rust runtime compatibility mismatch at {label}: required {required:?}, selected {selected:?}; select the compiler's matching runtime source root or regenerate with its compiler").into()
}

fn field(document: &DocumentMut, section: &str, name: &str) -> Option<String> {
    document
        .get(section)?
        .get(name)?
        .as_str()
        .map(str::to_owned)
}

fn require_field(
    document: &DocumentMut,
    section: &str,
    name: &str,
    expected: &str,
    package: &str,
) -> Result<(), Box<dyn Error>> {
    let actual = field(document, section, name);
    if actual.as_deref() != Some(expected) {
        return Err(mismatch(
            &format!("{package}.{section}.{name}"),
            actual,
            expected,
        ));
    }
    Ok(())
}

fn manifest(
    root: &Path,
    package: &str,
    name: &str,
    version: &str,
    msrv: &str,
) -> Result<DocumentMut, Box<dyn Error>> {
    let path = root.join(package).join("Cargo.toml");
    let document: DocumentMut = fs::read_to_string(&path)
        .map_err(|error| {
            format!(
                "Rust runtime compatibility manifest {}: {error}",
                path.display()
            )
        })?
        .parse()?;
    for (key, expected) in [
        ("name", name),
        ("version", version),
        ("rust-version", msrv),
        ("edition", "2024"),
    ] {
        require_field(&document, "package", key, expected, package)?;
    }
    Ok(document)
}

fn dependency(
    document: &DocumentMut,
    name: &str,
    version: &str,
    path: Option<&str>,
) -> Result<(), Box<dyn Error>> {
    let item = document
        .get("dependencies")
        .and_then(|table| table.get(name));
    let actual = item.and_then(|item| {
        item.as_str()
            .or_else(|| item.get("version").and_then(Item::as_str))
    });
    if actual != Some(version) {
        return Err(mismatch(
            &format!("dependencies.{name}.version"),
            actual,
            version,
        ));
    }
    let selected_path = item
        .and_then(|item| item.get("path"))
        .and_then(Item::as_str);
    if selected_path != path {
        return Err(mismatch(
            &format!("dependencies.{name}.path"),
            selected_path,
            path,
        ));
    }
    for key in ["git", "registry", "registry-index", "package"] {
        if item.and_then(|item| item.get(key)).is_some() {
            return Err(format!(
                "Rust runtime compatibility: dependencies.{name}.{key} override is unsupported"
            )
            .into());
        }
    }
    Ok(())
}

fn hash_file(
    root: &Path,
    relative: &Path,
    hashes: &mut BTreeMap<String, String>,
) -> Result<(), Box<dyn Error>> {
    let path = root.join(relative);
    let kind = fs::symlink_metadata(&path)?.file_type();
    if kind.is_dir() {
        for entry in fs::read_dir(path)? {
            hash_file(root, &relative.join(entry?.file_name()), hashes)?;
        }
    } else if kind.is_file() {
        hashes.insert(
            relative.to_string_lossy().replace('\\', "/"),
            format!("{:x}", Sha256::digest(fs::read(path)?)),
        );
    } else {
        return Err(format!("unsupported runtime source entry: {}", path.display()).into());
    }
    Ok(())
}

/// Validate before creating output or invoking the frontend. This is deliberately
/// exact for this source-distributed compiler; arbitrary same-version trees are
/// not interchangeable, and registry availability is not checked here.
pub fn validate_root(root: &Path) -> Result<ValidatedRuntime, Box<dyn Error>> {
    let root = fs::canonicalize(root)?;
    let record_path = root.join("runtime-rs/compatibility.json");
    let bytes = fs::read(&record_path).map_err(|error| format!("Rust runtime compatibility record {}: {error}; select a matching runtime root (no fallback)",record_path.display()))?;
    let selected: Compatibility = serde_json::from_slice(&bytes).map_err(|error| {
        format!(
            "invalid Rust runtime compatibility record {}: {error}",
            record_path.display()
        )
    })?;
    let required = required();
    let actual = serde_json::to_value(&selected)?;
    let expected = serde_json::to_value(&required)?;
    for (key, value) in expected.as_object().expect("record is an object") {
        if actual.get(key) != Some(value) {
            return Err(mismatch(key, actual.get(key), value));
        }
    }
    let runtime = manifest(
        &root,
        "runtime-rs",
        &required.runtime_package,
        &required.runtime_version,
        &required.rust_version,
    )?;
    let macros = manifest(
        &root,
        "runtime-rs-macros",
        &required.macros_package,
        &required.macros_version,
        &required.rust_version,
    )?;
    if macros
        .get("lib")
        .and_then(|lib| lib.get("proc-macro"))
        .and_then(Item::as_bool)
        != Some(true)
    {
        return Err(
            "Rust runtime compatibility: runtime-rs-macros.lib.proc-macro must be true".into(),
        );
    }
    dependency(
        &runtime,
        &required.macros_package,
        &format!("={}", required.macros_version),
        Some("../runtime-rs-macros"),
    )?;
    for (name, version) in &required.ledger_dependencies {
        dependency(&runtime, name, version, None)?;
    }
    let declared = runtime
        .get("dependencies")
        .and_then(Item::as_table_like)
        .ok_or("Rust runtime compatibility: dependencies must be a TOML table")?;
    for (name, _) in declared
        .iter()
        .filter(|(name, _)| name.starts_with("midnight-"))
    {
        if name != required.macros_package && !required.ledger_dependencies.contains_key(name) {
            return Err(format!(
                "Rust runtime compatibility: unrecorded Midnight dependency {name}"
            )
            .into());
        }
    }
    // These public literals are a useful stale-source diagnostic, not execution
    // of a selected build script or authentication of arbitrary Rust source.
    let source = syn::parse_file(&fs::read_to_string(root.join("runtime-rs/src/lib.rs"))?)?;
    for (name, expected) in [
        ("RUST_RUNTIME_ABI", required.runtime_abi.to_string()),
        ("LEDGER_VERSION", required.ledger_version.clone()),
    ] {
        let actual = source.items.iter().find_map(|item| match item {
            syn::Item::Const(item) if item.ident == name => match item.expr.as_ref() {
                syn::Expr::Lit(expr) => match &expr.lit {
                    syn::Lit::Int(value) => Some(value.base10_digits().to_owned()),
                    syn::Lit::Str(value) => Some(value.value()),
                    _ => None,
                },
                _ => None,
            },
            _ => None,
        });
        if actual.as_deref() != Some(&expected) {
            return Err(mismatch(name, actual, expected));
        }
    }
    let mut source_sha256 = BTreeMap::new();
    for package in ["runtime-rs", "runtime-rs-macros"] {
        for part in ["Cargo.toml", "src"] {
            hash_file(&root, &Path::new(package).join(part), &mut source_sha256)?;
        }
    }
    hash_file(
        &root,
        Path::new("runtime-rs/compatibility.json"),
        &mut source_sha256,
    )?;
    Ok(ValidatedRuntime {
        root,
        compatibility: selected,
        source_sha256,
    })
}
