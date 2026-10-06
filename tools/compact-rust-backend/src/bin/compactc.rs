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
use std::path::{Component, Path, PathBuf};
use std::process::{self, Command};
use std::time::{SystemTime, UNIX_EPOCH};

use compact_rust_backend::{compatibility, ir::Contract, render_with_proof_capabilities};
use fs2::FileExt;
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use toml_edit::{Array, DocumentMut, InlineTable, Item, Table, Value as TomlValue, value};

const TARGET_HELP: &str = "\n  --target <ts|rust> selects contract code. Repeat to emit both.\n    With no --target, TypeScript remains the default. Rust emits a standalone\n    contract/Cargo.toml, source, capability report, and matching runtime crates.\n    ZKIR and keys are independent.\n  --rust-require-recording rejects proof-required exported circuits without both\n    replayable recording and typed observed-call APIs.\n  --rust-runtime-root <path> uses one shared runtime source root for generated\n    Rust crates. The root must contain runtime-rs/ and runtime-rs-macros/.\n  --rust-runtime-registry pins the compiler's matching runtime package version\n    without copying runtime sources. That version must be published to build.\n";

#[derive(Default)]
struct Targets {
    explicit: bool,
    ts: bool,
    rust: bool,
    runtime_root: Option<PathBuf>,
    runtime_registry: bool,
    require_recording: bool,
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
        if argument == "--rust-require-recording" {
            if targets.require_recording {
                return Err("--rust-require-recording may be given only once".into());
            }
            targets.require_recording = true;
            continue;
        }
        if argument
            .to_str()
            .is_some_and(|arg| arg.starts_with("--rust-require-recording="))
        {
            return Err("--rust-require-recording takes no value".into());
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
            if root.is_empty() {
                return Err("--rust-runtime-root needs a directory".into());
            }
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
        } else {
            argument
                .to_str()
                .and_then(|s| s.strip_prefix("--target="))
                .map(OsString::from)
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
    if targets.require_recording && !targets.rust {
        return Err("--rust-require-recording requires --target rust".into());
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
    package["rust-version"] = value(compatibility::RUST_VERSION);
    package["publish"] = value(false);
    package["description"] = value(format!(
        "Rust contract generated from {}",
        source
            .file_name()
            .unwrap_or_else(|| OsStr::new("contract.compact"))
            .to_string_lossy()
    ));
    document["package"] = Item::Table(package);

    // Compact retains explicit Boolean comparisons in source assertions.
    // This stylistic lint should not make an otherwise valid generated crate
    // fail a consumer's `clippy -D warnings` gate.
    let mut clippy = Table::new();
    clippy["bool_comparison"] = value("allow");
    let mut lints = Table::new();
    lints["clippy"] = Item::Table(clippy);
    document["lints"] = Item::Table(lints);

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

fn runtime_source_root() -> Result<PathBuf, Box<dyn Error>> {
    if let Some(path) = env::var_os("COMPACT_RUST_RUNTIME_DIR") {
        let path = PathBuf::from(path);
        if path.join("runtime-rs/Cargo.toml").is_file() {
            return Ok(path);
        }
        return Err(format!("runtime source directory is invalid: {}", path.display()).into());
    }
    let executable = env::current_exe()?;
    let directory = executable
        .parent()
        .ok_or("cannot locate installed compactc runtime sources")?;
    // Release archives keep executable names at their root for the compact
    // installer; Nix packages keep binaries under bin/. Both preserve share/.
    for prefix in [Some(directory), directory.parent()].into_iter().flatten() {
        let installed = prefix.join("share/compactc");
        if installed.try_exists().map_err(|error| {
            format!(
                "cannot inspect installed runtime source root {}: {error}",
                installed.display()
            )
        })? {
            return Ok(installed);
        }
    }
    let checkout = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    if checkout.join("runtime-rs/Cargo.toml").is_file() {
        return Ok(checkout);
    }
    Err("compactc cannot locate its Rust runtime sources; set COMPACT_RUST_RUNTIME_DIR".into())
}

#[cfg(test)]
fn runtime_package_version() -> Result<String, Box<dyn Error>> {
    Ok(compatibility::validate_root(&runtime_source_root()?)?
        .compatibility
        .runtime_version)
}

fn write_source_file(source: &Path, destination: &Path) -> Result<(), Box<dyn Error>> {
    // fs::copy preserves archive/Nix timestamps on macOS. Cargo can then reuse
    // an older runtime in a shared target directory despite changed sources.
    // These are generated source files, so write their bytes with fresh times.
    let bytes = fs::read(source).map_err(|error| {
        format!(
            "cannot read selected runtime source {}: {error}",
            source.display()
        )
    })?;
    fs::write(destination, bytes).map_err(|error| {
        format!(
            "cannot copy selected runtime source {} to {}: {error}",
            source.display(),
            destination.display()
        )
    })?;
    Ok(())
}

fn copy_source_tree(source: &Path, destination: &Path) -> Result<(), Box<dyn Error>> {
    fs::create_dir_all(destination).map_err(|error| {
        format!(
            "cannot create runtime copy directory {} from {}: {error}",
            destination.display(),
            source.display()
        )
    })?;
    for entry in fs::read_dir(source).map_err(|error| {
        format!(
            "cannot read selected runtime source directory {}: {error}",
            source.display()
        )
    })? {
        let entry = entry.map_err(|error| {
            format!(
                "cannot read selected runtime source entry in {}: {error}",
                source.display()
            )
        })?;
        let kind = entry.file_type().map_err(|error| {
            format!(
                "cannot inspect selected runtime source {}: {error}",
                entry.path().display()
            )
        })?;
        let target = destination.join(entry.file_name());
        if kind.is_dir() {
            copy_source_tree(&entry.path(), &target)?;
        } else if kind.is_file() {
            write_source_file(&entry.path(), &target)?;
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

fn copy_runtime_sources(root: &Path, contract_dir: &Path) -> Result<(), Box<dyn Error>> {
    for package in ["runtime-rs", "runtime-rs-macros"] {
        let source = root.join(package);
        let destination = contract_dir.join(package);
        fs::create_dir_all(&destination).map_err(|error| {
            format!(
                "cannot create runtime copy directory {} from {}: {error}",
                destination.display(),
                source.display()
            )
        })?;
        for file in ["Cargo.toml", "README.md", "LICENSE"] {
            write_source_file(&source.join(file), &destination.join(file))?;
        }
        if package == "runtime-rs" {
            write_source_file(
                &source.join("compatibility.json"),
                &destination.join("compatibility.json"),
            )?;
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

/// Keep failed Rust-target runs out of the requested output directory.
struct StagedOutput {
    path: PathBuf,
    published: bool,
    _lock: fs::File,
}

fn interrupted_backup(name: &str, candidate: &OsStr) -> bool {
    let prefix = format!(".{name}.compactc-stage-");
    let Some(parts) = candidate
        .to_str()
        .and_then(|candidate| candidate.strip_prefix(&prefix))
        .and_then(|candidate| candidate.strip_suffix("-previous"))
    else {
        return false;
    };
    let numbers = parts.split('-').collect::<Vec<_>>();
    numbers.len() == 3
        && numbers
            .iter()
            .all(|part| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit()))
}

fn recover_interrupted_output(
    output: &Path,
    parent: &Path,
    name: &str,
) -> Result<(), Box<dyn Error>> {
    let mut backups = Vec::new();
    for entry in fs::read_dir(parent)? {
        let entry = entry?;
        if !interrupted_backup(name, &entry.file_name()) {
            continue;
        }
        let path = entry.path();
        let metadata = fs::symlink_metadata(&path)?;
        if !metadata.is_dir() || metadata.file_type().is_symlink() {
            return Err(format!(
                "interrupted Rust output backup is not a directory: {}",
                path.display()
            )
            .into());
        }
        backups.push(path);
    }
    if backups.len() > 1 {
        return Err(format!(
            "multiple interrupted Rust output backups for {}: {}",
            output.display(),
            backups
                .iter()
                .map(|path| path.display().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        )
        .into());
    }
    match fs::symlink_metadata(output) {
        Ok(metadata) if !metadata.is_dir() || metadata.file_type().is_symlink() => {
            return Err(format!(
                "Rust output must be a directory, not a file or symlink: {}",
                output.display()
            )
            .into());
        }
        Ok(_) => {
            if let Some(backup) = backups.first() {
                eprintln!(
                    "compactc: warning: previous Rust output remains at {} after an interrupted publication",
                    backup.display()
                );
            }
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            if let Some(backup) = backups.first() {
                fs::rename(backup, output)?;
            }
        }
        Err(error) => return Err(error.into()),
    }
    Ok(())
}

impl StagedOutput {
    fn new(output: &Path) -> Result<Self, Box<dyn Error>> {
        let Some(Component::Normal(name)) = output.components().next_back() else {
            return Err(
                "Rust output must name a directory, not current or parent directory".into(),
            );
        };
        let parent = output
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        fs::create_dir_all(parent)?;
        let name = name.to_string_lossy();
        let lock_path = parent.join(format!(".{name}.compactc.lock"));
        match fs::symlink_metadata(&lock_path) {
            Ok(metadata) if !metadata.is_file() || metadata.file_type().is_symlink() => {
                return Err(format!(
                    "Rust output lock must be a regular file, not a directory or symlink: {}",
                    lock_path.display()
                )
                .into());
            }
            Err(error) if error.kind() != io::ErrorKind::NotFound => return Err(error.into()),
            _ => {}
        }
        let lock = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&lock_path)?;
        FileExt::lock_exclusive(&lock)?;
        recover_interrupted_output(output, parent, &name)?;
        let nonce = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
        for attempt in 0..100 {
            let path = parent.join(format!(
                ".{}.compactc-stage-{}-{nonce}-{attempt}",
                name,
                process::id()
            ));
            match fs::create_dir(&path) {
                Ok(()) => {
                    return Ok(Self {
                        path,
                        published: false,
                        _lock: lock,
                    });
                }
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(error) => return Err(error.into()),
            }
        }
        Err("could not create a unique Rust output staging directory".into())
    }

    fn path(&self) -> &Path {
        &self.path
    }

    fn publish(mut self, output: &Path) -> Result<(), Box<dyn Error>> {
        match fs::symlink_metadata(output) {
            Ok(metadata) => {
                if !metadata.is_dir() || metadata.file_type().is_symlink() {
                    return Err(format!(
                        "Rust output changed to a file or symlink before publication: {}",
                        output.display()
                    )
                    .into());
                }
                let backup = self.path.with_file_name(format!(
                    "{}-previous",
                    self.path
                        .file_name()
                        .expect("staged output has a filename")
                        .to_string_lossy()
                ));
                match fs::symlink_metadata(&backup) {
                    Ok(_) => {
                        return Err(format!(
                            "Rust output backup path already exists: {}",
                            backup.display()
                        )
                        .into());
                    }
                    Err(error) if error.kind() != io::ErrorKind::NotFound => {
                        return Err(error.into());
                    }
                    Err(_) => {}
                }
                fs::rename(output, &backup)?;
                if let Err(error) = fs::rename(&self.path, output) {
                    if let Err(restore) = fs::rename(&backup, output) {
                        return Err(format!(
                            "could not publish Rust output: {error}; previous output remains at {} (restore failed: {restore})",
                            backup.display()
                        )
                        .into());
                    }
                    return Err(error.into());
                }
                self.published = true;
                if let Err(error) = fs::remove_dir_all(&backup) {
                    eprintln!(
                        "compactc: warning: generated output is complete, but could not remove previous output at {}: {error}",
                        backup.display()
                    );
                }
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                fs::rename(&self.path, output)?;
                self.published = true;
            }
            Err(error) => return Err(error.into()),
        }
        Ok(())
    }
}

impl Drop for StagedOutput {
    fn drop(&mut self) {
        if !self.published {
            let _ = fs::remove_dir_all(&self.path);
        }
    }
}

fn run() -> Result<i32, Box<dyn Error>> {
    let (targets, mut args) = select_targets(env::args_os().skip(1).collect())?;
    let compiler = scheme_compiler()?;
    let first_query = args.iter().find(|arg| {
        *arg == "--help"
            || [
                "--version",
                "--language-version",
                "--ledger-version",
                "--runtime-version",
            ]
            .iter()
            .any(|query| *arg == query)
    });
    if first_query.is_some_and(|arg| arg == "--help") {
        let output = Command::new(compiler).arg("--help").output()?;
        io::Write::write_all(&mut io::stdout(), &output.stdout)?;
        io::Write::write_all(&mut io::stderr(), &output.stderr)?;
        if output.status.success() {
            print!("{TARGET_HELP}");
        }
        return Ok(output.status.code().unwrap_or(1));
    }
    if first_query.is_some() {
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
    let selected_root = match &targets.runtime_root {
        Some(root) => root.clone(),
        None => runtime_source_root()?,
    };
    // Fail before staging or frontend execution. An explicitly selected invalid
    // root is never replaced with installed or checkout sources.
    let selected = compatibility::validate_root(&selected_root)?;
    let shared_runtime = selected.runtime_path();
    let source = PathBuf::from(args[args.len() - 2].clone());
    let output = PathBuf::from(args[args.len() - 1].clone());
    let staging = StagedOutput::new(&output)?;
    let output_index = args.len() - 1;
    args[output_index] = staging.path().as_os_str().to_os_string();
    let insert_at = args.len() - 2;
    args.insert(insert_at, OsString::from("--emit-rust-ir"));
    if !targets.ts {
        args.insert(insert_at + 1, OsString::from("--skip-ts"));
    }
    let status = Command::new(compiler).args(&args).status()?;
    if !status.success() {
        return Ok(status.code().unwrap_or(1));
    }

    let contract_dir = staging.path().join("contract");
    let ir: Contract =
        serde_json::from_slice(&fs::read(contract_dir.join("compact-rust-ir.json"))?)?;
    let contract_info: Value = serde_json::from_slice(&fs::read(
        staging.path().join("compiler/contract-info.json"),
    )?)?;
    let rendered = render_with_proof_capabilities(&ir, &contract_info)?;
    if targets.require_recording {
        let unavailable = rendered
            .capabilities
            .circuits
            .iter()
            .filter(|circuit| {
                circuit.proof_required == Some(true)
                    && (!circuit.recorded || !circuit.observed_call)
            })
            .map(|circuit| {
                let position = circuit.source.as_ref().map_or_else(
                    || "unknown source".to_owned(),
                    |source| {
                        format!(
                            "{} line {} char {}",
                            source.file, source.line, source.column
                        )
                    },
                );
                let (api, gap) = if !circuit.recorded {
                    ("recorded", circuit.recording_unavailable.as_ref())
                } else {
                    ("observed-call", circuit.observed_call_unavailable.as_ref())
                };
                let gap = gap.expect("false capability has a reason");
                format!(
                    "{position}: exported circuit {:?} has no complete {api} API: {} at {} ({})",
                    circuit.name,
                    gap.code.as_str(),
                    gap.path,
                    gap.detail,
                )
            })
            .collect::<Vec<_>>();
        if !unavailable.is_empty() {
            return Err(format!(
                "--rust-require-recording failed:\n{}",
                unavailable.join("\n")
            )
            .into());
        }
    }
    fs::write(contract_dir.join("lib.rs"), rendered.source)?;
    fs::write(
        contract_dir.join("rust-capabilities.json"),
        serde_json::to_vec_pretty(&rendered.capabilities)?,
    )?;
    let mode = if targets.runtime_registry {
        "registry"
    } else if targets.runtime_root.is_some() {
        "shared-source"
    } else {
        "bundled-source"
    };
    let runtime = if targets.runtime_registry {
        RuntimeDependency::Registry(&selected.compatibility.runtime_version)
    } else if targets.runtime_root.is_some() {
        RuntimeDependency::Path(&shared_runtime)
    } else {
        copy_runtime_sources(selected.root(), &contract_dir)?;
        RuntimeDependency::Path(Path::new("runtime-rs"))
    };
    // Bind the output record to the bytes actually copied/selected. Detect
    // ordinary source edits during frontend generation before publication.
    let checked_root = if mode == "bundled-source" {
        contract_dir.as_path()
    } else {
        selected.root()
    };
    if compatibility::validate_root(checked_root)?.source_sha256 != selected.source_sha256 {
        return Err(
            "Rust runtime source changed during generation; output was not published".into(),
        );
    }
    let frontend_manifest: Value = serde_json::from_slice(&fs::read(
        staging.path().join("compiler/contract-manifest.json"),
    )?)?;
    let metadata = serde_json::json!({
        "schema_version": 1,
        "compatibility": selected.compatibility,
        "distribution": mode,
        "registry_publication_verified": false,
        "source_sha256": selected.source_sha256,
        "compiler": {
            "rust_backend_package": env!("CARGO_PKG_VERSION"),
            "compact": frontend_manifest.get("compiler-version"),
            "language": frontend_manifest.get("language-version"),
            "typescript_runtime": frontend_manifest.get("runtime-version"),
        },
        "assurance": "Developer mismatch checks and source fingerprints; not authenticated provenance or registry availability. Linked runtime ABI assertion remains required."
    });
    fs::write(
        staging.path().join("compiler/rust-compatibility.json"),
        serde_json::to_vec_pretty(&metadata)?,
    )?;
    fs::write(
        contract_dir.join("Cargo.toml"),
        crate_manifest(&source, runtime)?,
    )?;
    refresh_manifest(staging.path())?;
    staging.publish(&output)?;
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
        RuntimeDependency, StagedOutput, crate_manifest, package_name, runtime_package_version,
        select_targets,
    };
    use fs2::FileExt;
    use std::ffi::OsString;
    use std::fs;
    use std::path::Path;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_TEMP_ROOT: AtomicU64 = AtomicU64::new(0);

    struct TempRoot(PathBuf);

    impl TempRoot {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "compactc-output-test-{}-{}",
                std::process::id(),
                NEXT_TEMP_ROOT.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for TempRoot {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.0).unwrap();
        }
    }

    #[test]
    fn generated_runtime_sources_do_not_inherit_archive_timestamps() {
        let root = TempRoot::new();
        let source = root.0.join("archive");
        fs::create_dir(&source).unwrap();
        let file = source.join("lib.rs");
        fs::write(&file, "pub const RUST_RUNTIME_ABI: u32 = 47;\n").unwrap();
        fs::File::options()
            .write(true)
            .open(&file)
            .unwrap()
            .set_times(fs::FileTimes::new().set_modified(std::time::UNIX_EPOCH))
            .unwrap();
        let destination = root.0.join("generated");
        super::copy_source_tree(&source, &destination).unwrap();
        let generated = destination.join("lib.rs");
        assert_eq!(fs::read(&file).unwrap(), fs::read(&generated).unwrap());
        assert!(
            fs::metadata(&generated).unwrap().modified().unwrap()
                > fs::metadata(&file).unwrap().modified().unwrap()
        );
    }

    #[test]
    fn runtime_copy_errors_identify_source_and_destination() {
        let root = TempRoot::new();
        let source = root.0.join("missing-source.rs");
        let destination = root.0.join("generated.rs");
        let error = super::write_source_file(&source, &destination)
            .unwrap_err()
            .to_string();
        assert!(error.contains(&source.display().to_string()), "{error}");
        fs::write(&source, "source bytes").unwrap();
        fs::create_dir(&destination).unwrap();
        let error = super::write_source_file(&source, &destination)
            .unwrap_err()
            .to_string();
        assert!(error.contains(&source.display().to_string()), "{error}");
        assert!(
            error.contains(&destination.display().to_string()),
            "{error}"
        );
        let missing_tree = root.0.join("missing-tree");
        let error = super::copy_source_tree(&missing_tree, &root.0.join("copy"))
            .unwrap_err()
            .to_string();
        assert!(
            error.contains(&missing_tree.display().to_string()),
            "{error}"
        );
    }

    #[test]
    fn output_lock_serializes_one_name_without_blocking_another() {
        let root = TempRoot::new();
        let stage = StagedOutput::new(&root.0.join("contract")).unwrap();
        let same = fs::File::open(root.0.join(".contract.compactc.lock")).unwrap();
        assert!(FileExt::try_lock_exclusive(&same).is_err());

        let other = StagedOutput::new(&root.0.join("other")).unwrap();
        assert!(other.path().is_dir());
        drop(other);
        drop(stage);
        FileExt::try_lock_exclusive(&same).unwrap();
    }

    #[test]
    fn interrupted_replacement_restores_the_previous_directory() {
        let root = TempRoot::new();
        let output = root.0.join("contract");
        let backup = root.0.join(".contract.compactc-stage-42-123-0-previous");
        fs::create_dir(&backup).unwrap();
        fs::write(backup.join("previous.txt"), "complete previous output").unwrap();

        let stage = StagedOutput::new(&output).unwrap();
        assert_eq!(
            fs::read_to_string(output.join("previous.txt")).unwrap(),
            "complete previous output"
        );
        assert!(!backup.exists());
        assert!(stage.path().is_dir());
        drop(stage);
    }

    #[test]
    fn interrupted_replacement_refuses_ambiguous_or_unsafe_backups() {
        let root = TempRoot::new();
        let output = root.0.join("contract");
        let first = root.0.join(".contract.compactc-stage-42-123-0-previous");
        let second = root.0.join(".contract.compactc-stage-43-124-0-previous");
        fs::create_dir(&first).unwrap();
        fs::create_dir(&second).unwrap();
        let error = match StagedOutput::new(&output) {
            Ok(_) => panic!("ambiguous backups must be rejected"),
            Err(error) => error,
        };
        assert!(error.to_string().contains("multiple interrupted"));
        assert!(first.is_dir() && second.is_dir() && !output.exists());

        fs::remove_dir(&second).unwrap();
        fs::remove_dir(&first).unwrap();
        fs::write(&first, "do not move").unwrap();
        let error = match StagedOutput::new(&output) {
            Ok(_) => panic!("a file backup must be rejected"),
            Err(error) => error,
        };
        assert!(error.to_string().contains("not a directory"));
        assert_eq!(fs::read_to_string(&first).unwrap(), "do not move");
        assert!(!output.exists());

        #[cfg(unix)]
        {
            use std::os::unix::fs::symlink;

            fs::remove_file(&first).unwrap();
            let outside = root.0.join("outside");
            fs::create_dir(&outside).unwrap();
            fs::write(outside.join("sentinel.txt"), "preserve").unwrap();
            symlink(&outside, &first).unwrap();
            let error = match StagedOutput::new(&output) {
                Ok(_) => panic!("a symlink backup must be rejected"),
                Err(error) => error,
            };
            assert!(error.to_string().contains("not a directory"));
            assert_eq!(
                fs::read_to_string(outside.join("sentinel.txt")).unwrap(),
                "preserve"
            );
            assert!(!output.exists());
        }
    }

    #[test]
    fn completed_stage_replaces_existing_output_under_the_lock() {
        let root = TempRoot::new();
        let output = root.0.join("contract");
        fs::create_dir(&output).unwrap();
        fs::write(output.join("old.txt"), "old").unwrap();
        let stage = StagedOutput::new(&output).unwrap();
        fs::write(stage.path().join("new.txt"), "new").unwrap();
        stage.publish(&output).unwrap();
        assert_eq!(fs::read_to_string(output.join("new.txt")).unwrap(), "new");
        assert!(!output.join("old.txt").exists());
        assert_eq!(fs::read_dir(&root.0).unwrap().count(), 2); // output and lock sidecar
    }

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
    fn requiring_recording_is_explicit_and_removed_before_scheme() {
        let parse =
            |args: &[&str]| select_targets(args.iter().map(|arg| OsString::from(*arg)).collect());
        let (targets, forwarded) = parse(&[
            "--target=rust",
            "--rust-require-recording",
            "in.compact",
            "out",
        ])
        .unwrap();
        assert!(targets.require_recording);
        assert_eq!(forwarded, ["in.compact", "out"].map(OsString::from));
        for (args, expected) in [
            (
                vec!["--rust-require-recording", "in.compact", "out"],
                "requires --target rust",
            ),
            (
                vec![
                    "--target=rust",
                    "--rust-require-recording",
                    "--rust-require-recording",
                    "in.compact",
                    "out",
                ],
                "may be given only once",
            ),
            (
                vec![
                    "--target=rust",
                    "--rust-require-recording=true",
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
        assert_eq!(
            document["lints"]["clippy"]["bool_comparison"].as_str(),
            Some("allow")
        );
    }

    #[test]
    fn crate_name_is_stable_for_source_filename() {
        assert_eq!(
            package_name(Path::new("Passport_2.compact")),
            "compact-contract-passport-2"
        );
    }
}
