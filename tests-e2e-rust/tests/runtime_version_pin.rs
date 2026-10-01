// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//  	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! The `check_runtime_version!` pin the Rust backend stamps must name the
//! version of the Rust runtime crate the generated code links.
//!
//! # Why this is its own test
//!
//! The backend used to stamp the **TypeScript** runtime's version here,
//! reading `runtime/package.json` while generated crates link the Rust crate
//! declared in `runtime-rs/Cargo.toml`. The two runtimes version
//! independently, and once they diverged (`0.19.105` npm vs `0.19.101` Rust)
//! every generated crate stopped compiling:
//!
//! ```text
//! error[E0080]: evaluation panicked: midnight-compact-runtime version mismatch
//!   --> tests-e2e-rust/contracts/election/lib.rs:32:1
//!    |
//! 32 | midnight_compact_runtime::check_runtime_version!("0.19.105");
//! ```
//!
//! The suite did catch that — but as 41 fixture crates failing to build with
//! `E0080`, which reads like the fixtures are stale. The tempting repair is
//! to regenerate them, and it is the wrong one: it turns the byte-parity gate
//! green while leaving every generated crate uncompilable. (Every one of the
//! 41 diffs was a single version line, which is what makes that repair look
//! safe.)
//!
//! This test states the actual invariant, so the next regression reports the
//! cause instead of 41 symptoms.

use std::path::{Path, PathBuf};
use std::process::Command;

fn find_repo_root(start: &Path) -> Option<PathBuf> {
    let mut dir = Some(start);
    while let Some(d) = dir {
        if d.join("examples").is_dir() && d.join("Cargo.toml").is_file() {
            return Some(d.to_path_buf());
        }
        dir = d.parent();
    }
    None
}

/// The `[package]` version of the `midnight-compact-runtime` crate — the
/// first line in runtime-rs/Cargo.toml whose first non-space token is
/// `version`, matching what `runtime-rs/extract-version.ss` reads at compile
/// time.
fn rust_runtime_version(repo_root: &Path) -> String {
    let manifest = repo_root.join("runtime-rs/Cargo.toml");
    let text = std::fs::read_to_string(&manifest)
        .unwrap_or_else(|e| panic!("read {}: {e}", manifest.display()));
    for line in text.lines() {
        let t = line.trim_start();
        if let Some(rest) = t.strip_prefix("version") {
            if let Some(open) = rest.find('"') {
                let after = &rest[open + 1..];
                if let Some(close) = after.find('"') {
                    return after[..close].to_string();
                }
            }
        }
    }
    panic!("no `version = \"…\"` line in {}", manifest.display());
}

#[test]
fn rust_backend_pins_the_rust_runtime_version() {
    let repo_root = find_repo_root(Path::new(env!("CARGO_MANIFEST_DIR"))).expect("repo root");
    let compactc = match std::env::var_os("COMPACTC") {
        Some(p) => PathBuf::from(p),
        None => repo_root.join("result/bin/compactc"),
    };
    assert!(
        compactc.exists(),
        "no compactc at {} — this gate cannot run, so it fails rather than \
         reporting a pass it did not earn",
        compactc.display()
    );

    let expected = rust_runtime_version(&repo_root);

    let out = std::env::temp_dir().join(format!("compact-version-pin-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&out);
    let status = Command::new(&compactc)
        .args(["--target", "rust", "--skip-zk"])
        .arg(repo_root.join("examples/tiny.compact"))
        .arg(&out)
        .status()
        .expect("run compactc");
    assert!(status.success(), "compactc failed on tiny.compact");

    let lib = std::fs::read_to_string(out.join("contract/lib.rs")).expect("generated lib.rs");
    let pin = format!("midnight_compact_runtime::check_runtime_version!(\"{expected}\");");
    assert!(
        lib.contains(&pin),
        "the emitted crate must pin the Rust runtime version from \
         runtime-rs/Cargo.toml ({expected}).\nExpected line: {pin}\nGot: {}\n\n\
         If this names the npm version from runtime/package.json, the version \
         source has regressed — do NOT regenerate the fixtures to match; the \
         pin is const-evaluated and every generated crate will fail to build.",
        lib.lines()
            .find(|l| l.contains("check_runtime_version"))
            .unwrap_or("<no check_runtime_version line at all>")
    );

    // The generated manifest's dependency requirement must be satisfiable by
    // that same crate, or `[patch.crates-io]` pointing at it cannot resolve.
    let manifest =
        std::fs::read_to_string(out.join("contract/Cargo.toml")).expect("generated Cargo.toml");
    let dep = format!("midnight-compact-runtime = \"{expected}\"");
    assert!(
        manifest.contains(&dep),
        "the generated manifest must require the Rust runtime version \
         ({expected}).\nExpected: {dep}\nGot: {}",
        manifest
            .lines()
            .find(|l| l.contains("midnight-compact-runtime"))
            .unwrap_or("<no midnight-compact-runtime line at all>")
    );

    let _ = std::fs::remove_dir_all(&out);
}
