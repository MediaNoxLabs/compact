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

//! The public Compact compiler command. Chez owns analysis and proof artifacts;
//! the Rust backend owns Rust syntax and the generated crate's package boundary.

use std::env;
use std::error::Error;
use std::ffi::{OsStr, OsString};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{self, Command};

use compact_rust_backend::{ir::Contract, render};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use toml_edit::{Array, DocumentMut, InlineTable, Item, Table, Value as TomlValue, value};

const TARGET_HELP: &str = "\n  --target <ts|rust> selects contract code. Repeat to emit both.\n    With no --target, TypeScript remains the default. Rust emits a standalone\n    contract/Cargo.toml, source, and matching runtime crates; ZKIR and keys are independent.\n  --rust-runtime-root <path> uses one shared runtime source root for generated\n    Rust crates. The root must contain runtime-rs/ and runtime-rs-macros/.\n  --rust-runtime-registry pins the compiler's matching runtime package version\n    without copying runtime sources. That version must be published to build.\n";

#[derive(Default)]
struct Targets {
    explicit: bool,
    ts: bool,
    rust: bool,
    runtime_root: Option<PathBuf>,
    runtime_registry: bool,
}

fn select_targets(args: Vec<OsString>) -> Result<(Targets, Vec<OsString>), String> {
    let mut targets = Targets::default();
    let mut legacy_rust = false;
    let mut legacy_skip_ts = false;
    let mut forwarded = Vec::with_capacity(args.len());
    let mut arguments = args.into_iter();
    while let Some(argument) = arguments.next() {
        if argument == "--rust" {
            legacy_rust = true;
            continue;
        }
        if argument == "--skip-ts" {
            legacy_skip_ts = true;
            continue;
        }
        if argument == "--rust-runtime-registry" {
            if targets.runtime_registry {
                return Err("--rust-runtime-registry may be given only once".into());
            }
            targets.runtime_registry = true;
            continue;
        }
        if argument
            .to_str()
            .is_some_and(|arg| arg.starts_with("--rust-runtime-registry="))
        {
            return Err("--rust-runtime-registry takes no value".into());
        }
        if argument == "--rust-runtime-root" {
            let root = arguments
                .next()
                .ok_or("--rust-runtime-root needs a directory")?;
            if targets.runtime_root.replace(PathBuf::from(root)).is_some() {
                return Err("--rust-runtime-root may be given only once".into());
            }
            continue;
        }
        if let Some(root) = argument
            .to_str()
            .and_then(|s| s.strip_prefix("--rust-runtime-root="))
        {
            if root.is_empty() {
                return Err("--rust-runtime-root needs a directory".into());
            }
            if targets.runtime_root.replace(PathBuf::from(root)).is_some() {
                return Err("--rust-runtime-root may be given only once".into());
            }
            continue;
        }
        let selected = if argument == "--target" {
            Some(arguments.next().ok_or("--target needs ts or rust")?)
        } else if let Some(value) = argument.to_str().and_then(|s| s.strip_prefix("--target=")) {
            Some(OsString::from(value))
        } else {
            None
        };
        if let Some(selected) = selected {
            targets.explicit = true;
            match selected.to_str() {
                Some("ts") => targets.ts = true,
                Some("rust") => targets.rust = true,
                _ => {
                    return Err(format!(
                        "unknown --target {:?}; valid targets are ts, rust",
                        selected
                    ));
                }
            }
        } else {
            forwarded.push(argument);
        }
    }
    if targets.explicit && (legacy_rust || legacy_skip_ts) {
        return Err(
            "--target cannot be combined with --rust or --skip-ts; use --target alone".into(),
        );
    }
    if legacy_skip_ts && !legacy_rust {
        return Err("--skip-ts requires --rust; use --target rust for new callers".into());
    }
    if legacy_rust {
        targets.rust = true;
        targets.ts = !legacy_skip_ts;
    } else if !targets.explicit {
        targets.ts = true;
    }
    if targets.runtime_root.is_some() && !targets.rust {
        return Err("--rust-runtime-root requires --target rust".into());
    }
    if targets.runtime_registry && !targets.rust {
        return Err("--rust-runtime-registry requires --target rust".into());
    }
    if targets.runtime_registry && targets.runtime_root.is_some() {
        return Err("--rust-runtime-registry cannot be combined with --rust-runtime-root".into());
    }
    Ok((targets, forwarded))
}

fn scheme_compiler() -> Result<PathBuf, Box<dyn Error>> {
    if let Some(path) = env::var_os("COMPACTC_SCHEME") {
        return Ok(PathBuf::from(path));
    }
    let sibling = env::current_exe()?.with_file_name("compactc-scheme");
    if sibling.exists() {
        Ok(sibling)
    } else {
        Ok(PathBuf::from("compactc-scheme"))
    }
}

fn package_name(source: &Path) -> String {
    let stem = source.file_stem().unwrap_or_else(|| OsStr::new("contract"));
    let mut name = String::from("compact-contract-");
    let mut separator = false;
    for character in stem.to_string_lossy().chars() {
        if character.is_ascii_alphanumeric() {
            if separator && !name.ends_with('-') {
                name.push('-');
            }
            name.push(character.to_ascii_lowercase());
            separator = false;
        } else {
            separator = true;
        }
    }
    name.trim_end_matches('-').to_owned()
}

enum RuntimeDependency<'a> {
    Path(&'a Path),
    Registry(&'a str),
}

fn crate_manifest(source: &Path, runtime: RuntimeDependency<'_>) -> Result<String, Box<dyn Error>> {
    let mut document = DocumentMut::new();
    let mut package = Table::new();
    package["name"] = value(package_name(source));
    package["version"] = value("0.1.0");
    package["edition"] = value("2024");
    package["publish"] = value(false);
    package["description"] = value(format!(
        "Rust contract generated from {}",
        source
            .file_name()
            .unwrap_or_else(|| OsStr::new("contract.compact"))
            .to_string_lossy()
    ));
    document["package"] = Item::Table(package);

    let mut library = Table::new();
    library["path"] = value("lib.rs");
    document["lib"] = Item::Table(library);

    let mut dependency = InlineTable::new();
    match runtime {
        RuntimeDependency::Path(path) => {
            dependency.insert(
                "path",
                TomlValue::from(
                    path.to_str()
                        .ok_or("Rust runtime path is not valid UTF-8 for Cargo.toml")?,
                ),
            );
        }
        RuntimeDependency::Registry(version) => {
            dependency.insert("version", TomlValue::from(format!("={version}")));
        }
    }
    dependency.insert("package", TomlValue::from("midnight-compact-runtime"));
    let mut dependencies = Table::new();
    dependencies["midnight-compact-runtime"] = Item::Value(TomlValue::InlineTable(dependency));
    document["dependencies"] = Item::Table(dependencies);

    let mut features = Table::new();
    let mut transaction_feature = Array::new();
    transaction_feature.push("midnight-compact-runtime/ledger-transaction");
    features["ledger-transaction"] = Item::Value(TomlValue::Array(transaction_feature));
    document["features"] = Item::Table(features);
    Ok(document.to_string())
}

fn shared_runtime_path(root: &Path) -> Result<PathBuf, Box<dyn Error>> {
    let root = fs::canonicalize(root)?;
    for package in ["runtime-rs", "runtime-rs-macros"] {
        if !root.join(package).join("Cargo.toml").is_file() {
            return Err(format!(
                "shared Rust runtime root lacks {package}/Cargo.toml: {}",
                root.display()
            )
            .into());
        }
    }
    Ok(root.join("runtime-rs"))
}

fn runtime_source_root() -> Result<PathBuf, Box<dyn Error>> {
    if let Some(path) = env::var_os("COMPACT_RUST_RUNTIME_DIR") {
        let path = PathBuf::from(path);
        if path.join("runtime-rs/Cargo.toml").is_file() {
            return Ok(path);
        }
        return Err(format!("runtime source directory is invalid: {}", path.display()).into());
    }
    let installed = env::current_exe()?
        .parent()
        .and_then(Path::parent)
        .ok_or("cannot locate installed compactc runtime sources")?
        .join("share/compactc");
    if installed.join("runtime-rs/Cargo.toml").is_file() {
        return Ok(installed);
    }
    let checkout = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    if checkout.join("runtime-rs/Cargo.toml").is_file() {
        return Ok(checkout);
    }
    Err("compactc cannot locate its Rust runtime sources; set COMPACT_RUST_RUNTIME_DIR".into())
}

fn runtime_package_version() -> Result<String, Box<dyn Error>> {
    let manifest = runtime_source_root()?.join("runtime-rs/Cargo.toml");
    let document: DocumentMut = fs::read_to_string(&manifest)?.parse()?;
    let package = document
        .get("package")
        .and_then(Item::as_table)
        .ok_or("matching Rust runtime manifest has no [package] table")?;
    if package.get("name").and_then(Item::as_str) != Some("midnight-compact-runtime") {
        return Err("matching Rust runtime manifest has an unexpected package name".into());
    }
    let version = package
        .get("version")
        .and_then(Item::as_str)
        .filter(|version| !version.is_empty())
        .ok_or("matching Rust runtime manifest has no package version")?;
    Ok(version.to_owned())
}

fn copy_source_tree(source: &Path, destination: &Path) -> Result<(), Box<dyn Error>> {
    fs::create_dir_all(destination)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let kind = entry.file_type()?;
        let target = destination.join(entry.file_name());
        if kind.is_dir() {
            copy_source_tree(&entry.path(), &target)?;
        } else if kind.is_file() {
            fs::copy(entry.path(), target)?;
        } else {
            return Err(format!(
                "unsupported runtime source entry: {}",
                entry.path().display()
            )
            .into());
        }
    }
    Ok(())
}

fn copy_runtime_sources(contract_dir: &Path) -> Result<(), Box<dyn Error>> {
    let root = runtime_source_root()?;
    for package in ["runtime-rs", "runtime-rs-macros"] {
        let source = root.join(package);
        let destination = contract_dir.join(package);
        fs::create_dir_all(&destination)?;
        for file in ["Cargo.toml", "README.md"] {
            fs::copy(source.join(file), destination.join(file))?;
        }
        copy_source_tree(&source.join("src"), &destination.join("src"))?;
    }
    Ok(())
}

fn manifest_tree(path: &Path, is_root: bool) -> Result<Value, Box<dyn Error>> {
    let mut entries = Map::new();
    entries.insert("type".into(), Value::String("directory".into()));
    let mut children = fs::read_dir(path)?.collect::<Result<Vec<_>, _>>()?;
    children.sort_by_key(|entry| entry.file_name());
    for entry in children {
        let name = entry.file_name().to_string_lossy().into_owned();
        if is_root && name == "contract-manifest.json" {
            continue;
        }
        let path = entry.path();
        let kind = entry.file_type()?;
        let value = if kind.is_dir() {
            manifest_tree(&path, false)?
        } else if kind.is_file() {
            let bytes = fs::read(&path)?;
            let mut properties = Map::new();
            properties.insert("type".into(), Value::String("file".into()));
            properties.insert("size".into(), Value::from(bytes.len() as u64));
            properties.insert(
                "hash".into(),
                Value::String(format!("{:x}", Sha256::digest(bytes))),
            );
            Value::Object(properties)
        } else {
            return Err(format!("unsupported generated output entry: {}", path.display()).into());
        };
        entries.insert(name, value);
    }
    Ok(Value::Object(entries))
}

fn refresh_manifest(output: &Path) -> Result<(), Box<dyn Error>> {
    let path = output.join("compiler/contract-manifest.json");
    let mut manifest: Value = serde_json::from_slice(&fs::read(&path)?)?;
    let fields = manifest
        .as_object_mut()
        .ok_or("invalid contract manifest")?;
    for directory in ["compiler", "contract", "zkir", "keys"] {
        let path = output.join(directory);
        if path.is_dir() {
            fields.insert(
                directory.into(),
                manifest_tree(&path, directory == "compiler")?,
            );
        } else {
            fields.remove(directory);
        }
    }
    fs::write(
        output.join("compiler/contract-manifest.json"),
        serde_json::to_vec_pretty(&manifest)?,
    )?;
    Ok(())
}

fn run() -> Result<i32, Box<dyn Error>> {
    let (targets, mut args) = select_targets(env::args_os().skip(1).collect())?;
    let compiler = scheme_compiler()?;
    if args.iter().any(|arg| arg == "--help") {
        let output = Command::new(compiler).arg("--help").output()?;
        io::Write::write_all(&mut io::stdout(), &output.stdout)?;
        io::Write::write_all(&mut io::stderr(), &output.stderr)?;
        if output.status.success() {
            print!("{TARGET_HELP}");
        }
        return Ok(output.status.code().unwrap_or(1));
    }
    if args.iter().any(|arg| {
        [
            "--version",
            "--language-version",
            "--ledger-version",
            "--runtime-version",
        ]
        .iter()
        .any(|query| arg == *query)
    }) {
        let status = Command::new(compiler).args(&args).status()?;
        return Ok(status.code().unwrap_or(1));
    }
    if !targets.rust {
        let status = Command::new(compiler).args(&args).status()?;
        return Ok(status.code().unwrap_or(1));
    }
    if args.len() < 2 {
        return Err(
            "usage: compactc [--target ts|rust] [flags] <source.compact> <output-directory>".into(),
        );
    }
    let shared_runtime = targets
        .runtime_root
        .as_deref()
        .map(shared_runtime_path)
        .transpose()?;
    let registry_version = targets
        .runtime_registry
        .then(runtime_package_version)
        .transpose()?;
    let source = PathBuf::from(args[args.len() - 2].clone());
    let output = PathBuf::from(args[args.len() - 1].clone());
    let insert_at = args.len() - 2;
    args.insert(insert_at, OsString::from("--emit-rust-ir"));
    if !targets.ts {
        args.insert(insert_at + 1, OsString::from("--skip-ts"));
    }
    let status = Command::new(compiler).args(&args).status()?;
    if !status.success() {
        return Ok(status.code().unwrap_or(1));
    }

    let contract_dir = output.join("contract");
    let ir: Contract =
        serde_json::from_slice(&fs::read(contract_dir.join("compact-rust-ir.json"))?)?;
    let source_code = render(&ir)?;
    fs::write(contract_dir.join("lib.rs"), source_code)?;
    let runtime = if let Some(version) = registry_version.as_deref() {
        RuntimeDependency::Registry(version)
    } else if let Some(runtime) = shared_runtime.as_deref() {
        RuntimeDependency::Path(runtime)
    } else {
        copy_runtime_sources(&contract_dir)?;
        RuntimeDependency::Path(Path::new("runtime-rs"))
    };
    fs::write(
        contract_dir.join("Cargo.toml"),
        crate_manifest(&source, runtime)?,
    )?;
    refresh_manifest(&output)?;
    Ok(0)
}

fn main() {
    match run() {
        Ok(code) => process::exit(code),
        Err(error) => {
            eprintln!("compactc: {error}");
            process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        RuntimeDependency, crate_manifest, package_name, runtime_package_version, select_targets,
    };
    use std::ffi::OsString;
    use std::path::Path;

    #[test]
    fn target_selection_preserves_other_compiler_arguments() {
        let args = [
            "--target",
            "rust",
            "--skip-zk",
            "--target",
            "ts",
            "in.compact",
            "out",
        ];
        let (targets, forwarded) =
            select_targets(args.into_iter().map(OsString::from).collect()).unwrap();
        assert!(targets.ts && targets.rust);
        assert_eq!(
            forwarded,
            ["--skip-zk", "in.compact", "out"].map(OsString::from)
        );
    }

    #[test]
    fn target_selection_rejects_missing_and_unknown_targets() {
        let missing = select_targets(vec![OsString::from("--target")]);
        assert!(matches!(missing, Err(message) if message.contains("needs ts or rust")));

        let unknown = select_targets(vec![OsString::from("--target=wasm")]);
        assert!(matches!(unknown, Err(message) if message.contains("valid targets are ts, rust")));

        let without_rust = select_targets(
            ["--rust-runtime-root", "/shared", "in.compact", "out"]
                .into_iter()
                .map(OsString::from)
                .collect(),
        );
        assert!(matches!(without_rust, Err(message) if message.contains("requires --target rust")));
        let missing_root = select_targets(
            ["--target", "rust", "--rust-runtime-root"]
                .into_iter()
                .map(OsString::from)
                .collect(),
        );
        assert!(matches!(missing_root, Err(message) if message.contains("needs a directory")));
    }

    #[test]
    fn legacy_rust_aliases_resolve_without_reaching_chez() {
        let parse = |args: &[&str]| {
            select_targets(args.iter().map(|arg| OsString::from(*arg)).collect()).unwrap()
        };
        let (both, forwarded) = parse(&["--rust", "--skip-zk", "in.compact", "out"]);
        assert!(both.rust && both.ts);
        assert_eq!(
            forwarded,
            ["--skip-zk", "in.compact", "out"].map(OsString::from)
        );

        for args in [
            &["--rust", "--skip-ts", "in.compact", "out"][..],
            &["--skip-ts", "--rust", "in.compact", "out"],
            &["--rust", "--rust", "--skip-ts", "in.compact", "out"],
        ] {
            let (rust, forwarded) = parse(args);
            assert!(rust.rust && !rust.ts);
            assert!(
                !forwarded
                    .iter()
                    .any(|arg| arg == "--rust" || arg == "--skip-ts")
            );
        }

        for args in [
            vec!["--target", "rust", "--rust", "in.compact", "out"],
            vec!["--skip-ts", "--target=ts", "in.compact", "out"],
        ] {
            let error = select_targets(args.into_iter().map(OsString::from).collect());
            assert!(matches!(error, Err(message) if message.contains("cannot be combined")));
        }
        let alone = select_targets(
            ["--skip-ts", "in.compact", "out"]
                .into_iter()
                .map(OsString::from)
                .collect(),
        );
        assert!(matches!(alone, Err(message) if message.contains("requires --rust")));
    }

    #[test]
    fn shared_runtime_option_is_removed_before_scheme_compilation() {
        let (targets, forwarded) = select_targets(
            [
                "--target=rust",
                "--rust-runtime-root=/shared",
                "in.compact",
                "out",
            ]
            .into_iter()
            .map(OsString::from)
            .collect(),
        )
        .unwrap();
        assert_eq!(targets.runtime_root.as_deref(), Some(Path::new("/shared")));
        assert_eq!(forwarded, ["in.compact", "out"].map(OsString::from));
    }

    #[test]
    fn registry_runtime_option_is_exclusive_and_removed_before_scheme() {
        let parse =
            |args: &[&str]| select_targets(args.iter().map(|arg| OsString::from(*arg)).collect());
        let (targets, forwarded) = parse(&[
            "--target=rust",
            "--rust-runtime-registry",
            "in.compact",
            "out",
        ])
        .unwrap();
        assert!(targets.runtime_registry);
        assert_eq!(forwarded, ["in.compact", "out"].map(OsString::from));
        for (args, expected) in [
            (
                vec!["--rust-runtime-registry", "in.compact", "out"],
                "requires --target rust",
            ),
            (
                vec![
                    "--target=rust",
                    "--rust-runtime-registry",
                    "--rust-runtime-registry",
                    "in.compact",
                    "out",
                ],
                "may be given only once",
            ),
            (
                vec![
                    "--target=rust",
                    "--rust-runtime-registry",
                    "--rust-runtime-root=/shared",
                    "in.compact",
                    "out",
                ],
                "cannot be combined",
            ),
            (
                vec![
                    "--target=rust",
                    "--rust-runtime-registry=0.1.0",
                    "in.compact",
                    "out",
                ],
                "takes no value",
            ),
        ] {
            let error = parse(&args);
            assert!(matches!(error, Err(message) if message.contains(expected)));
        }
    }

    #[test]
    fn registry_manifest_pins_the_matching_runtime_version() {
        let version = runtime_package_version().unwrap();
        let source = Path::new("counter.compact");
        let manifest = crate_manifest(source, RuntimeDependency::Registry(&version)).unwrap();
        let document: toml_edit::DocumentMut = manifest.parse().unwrap();
        let dependency = &document["dependencies"]["midnight-compact-runtime"];
        assert_eq!(
            dependency.get("version").and_then(toml_edit::Item::as_str),
            Some(format!("={version}").as_str())
        );
        assert!(dependency.get("path").is_none());
    }

    #[test]
    fn crate_name_is_stable_for_source_filename() {
        assert_eq!(
            package_name(Path::new("Passport_2.compact")),
            "compact-contract-passport-2"
        );
    }
}
