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
        .parse()
        .map_err(|error| {
            format!(
                "invalid Rust runtime compatibility manifest {}: {error}",
                path.display()
            )
        })?;
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

fn dependency_entry(
    item: Option<&Item>,
    label: &str,
    version: &str,
    path: Option<&str>,
) -> Result<(), Box<dyn Error>> {
    let actual = item.and_then(|item| {
        item.as_str()
            .or_else(|| item.get("version").and_then(Item::as_str))
    });
    if actual != Some(version) {
        return Err(mismatch(&format!("{label}.version"), actual, version));
    }
    let selected_path = item
        .and_then(|item| item.get("path"))
        .map(|value| {
            value
                .as_str()
                .ok_or_else(|| format!("Rust runtime compatibility: {label}.path must be a string"))
        })
        .transpose()?;
    if selected_path != path {
        return Err(mismatch(&format!("{label}.path"), selected_path, path));
    }
    for key in ["git", "registry", "registry-index", "package", "workspace"] {
        if item.and_then(|item| item.get(key)).is_some() {
            return Err(format!(
                "Rust runtime compatibility: {label}.{key} override is unsupported"
            )
            .into());
        }
    }
    Ok(())
}

fn dependency_table(
    table: &Item,
    label: &str,
    package: &str,
    allow_macro_sibling: bool,
    required: &Compatibility,
) -> Result<(), Box<dyn Error>> {
    let table = table
        .as_table_like()
        .ok_or_else(|| format!("Rust runtime compatibility: {label} must be a TOML table"))?;
    for (name, item) in table.iter() {
        // Cargo's package alias determines the real package; the local key can
        // look unrelated to Midnight. Check both sides before classification.
        let resolved = match item.get("package") {
            Some(value) => value.as_str().ok_or_else(|| {
                format!("Rust runtime compatibility: {label}.{name}.package must be a string")
            })?,
            None => name,
        };
        if !name.starts_with("midnight-") && !resolved.starts_with("midnight-") {
            continue;
        }
        let entry = format!("{label}.{name}");
        if package != "runtime-rs" {
            return Err(format!("Rust runtime compatibility: {entry} resolves to unreviewed Midnight dependency {resolved} in {package}").into());
        }
        let (version, path) = if resolved == required.macros_package && allow_macro_sibling {
            (
                format!("={}", required.macros_version),
                Some("../runtime-rs-macros"),
            )
        } else if let Some(version) = required.ledger_dependencies.get(resolved) {
            (version.clone(), None)
        } else {
            return Err(format!("Rust runtime compatibility: {entry} resolves to unrecorded Midnight dependency {resolved}").into());
        };
        let diagnostic = if resolved == name {
            entry
        } else {
            format!("{entry} (package {resolved})")
        };
        dependency_entry(Some(item), &diagnostic, &version, path)?;
    }
    Ok(())
}

fn declared_dependencies(
    document: &DocumentMut,
    package: &str,
    required: &Compatibility,
) -> Result<(), Box<dyn Error>> {
    let contexts = ["dependencies", "dev-dependencies", "build-dependencies"];
    for context in contexts {
        if let Some(table) = document.get(context) {
            dependency_table(
                table,
                &format!("{package}.{context}"),
                package,
                context == "dependencies",
                required,
            )?;
        }
    }
    if let Some(targets) = document.get("target") {
        let targets = targets.as_table_like().ok_or_else(|| {
            format!("Rust runtime compatibility: {package}.target must be a TOML table")
        })?;
        // Inspect every target, including targets not active on this host. A
        // generated portable consumer must not change its graph on another host.
        for (target, config) in targets.iter() {
            let label = format!("{package}.target.{target}");
            let config = config.as_table_like().ok_or_else(|| {
                format!("Rust runtime compatibility: {label} must be a TOML table")
            })?;
            for context in contexts {
                if let Some(table) = config.get(context) {
                    dependency_table(
                        table,
                        &format!("{label}.{context}"),
                        package,
                        false,
                        required,
                    )?;
                }
            }
        }
    }
    Ok(())
}

fn source_error(action: &str, path: &Path, error: impl std::fmt::Display) -> Box<dyn Error> {
    format!(
        "Rust runtime compatibility {action} {}: {error}",
        path.display()
    )
    .into()
}

fn hash_file(
    root: &Path,
    relative: &Path,
    hashes: &mut BTreeMap<String, String>,
) -> Result<(), Box<dyn Error>> {
    let path = root.join(relative);
    let kind = fs::symlink_metadata(&path)
        .map_err(|e| source_error("source metadata", &path, e))?
        .file_type();
    if kind.is_dir() {
        for entry in
            fs::read_dir(&path).map_err(|e| source_error("read source directory", &path, e))?
        {
            let entry = entry.map_err(|e| source_error("read source entry", &path, e))?;
            hash_file(root, &relative.join(entry.file_name()), hashes)?;
        }
    } else if kind.is_file() {
        hashes.insert(
            relative.to_string_lossy().replace('\\', "/"),
            format!(
                "{:x}",
                Sha256::digest(fs::read(&path).map_err(|e| source_error(
                    "read source file",
                    &path,
                    e
                ))?)
            ),
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
    let root =
        fs::canonicalize(root).map_err(|e| source_error("selected runtime root", root, e))?;
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
    let normal = runtime
        .get("dependencies")
        .and_then(Item::as_table_like)
        .ok_or("Rust runtime compatibility: runtime-rs.dependencies must be a TOML table")?;
    dependency_entry(
        normal.get(&required.macros_package),
        &format!("runtime-rs.dependencies.{}", required.macros_package),
        &format!("={}", required.macros_version),
        Some("../runtime-rs-macros"),
    )?;
    for (name, version) in &required.ledger_dependencies {
        dependency_entry(
            normal.get(name),
            &format!("runtime-rs.dependencies.{name}"),
            version,
            None,
        )?;
    }
    declared_dependencies(&runtime, "runtime-rs", &required)?;
    declared_dependencies(&macros, "runtime-rs-macros", &required)?;
    // These public literals are a useful stale-source diagnostic, not execution
    // of a selected build script or authentication of arbitrary Rust source.
    let source_path = root.join("runtime-rs/src/lib.rs");
    let source_text = fs::read_to_string(&source_path)
        .map_err(|e| source_error("read source file", &source_path, e))?;
    let source = syn::parse_file(&source_text)
        .map_err(|e| source_error("parse source file", &source_path, e))?;
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
