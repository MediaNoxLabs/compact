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

#![cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
    sync::atomic::{AtomicUsize, Ordering},
};
const LIMIT: usize = 4 * 1024 * 1024;
const IR: &str = r#"{"schema_version":20,"ledger_fields":[],"circuits":[],"stateful_circuits":[]}"#;
struct Scratch(PathBuf);
impl Scratch {
    fn new() -> Self {
        static N: AtomicUsize = AtomicUsize::new(0);
        let p = std::env::temp_dir().join(format!(
            "compact-ingress-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&p).unwrap();
        Self(p)
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}
fn padded(text: &str, n: usize) -> Vec<u8> {
    let mut b = text.as_bytes().to_vec();
    b.resize(n, b' ');
    b
}
fn stdin(bytes: &[u8]) -> Output {
    let mut c = Command::new(env!("CARGO_BIN_EXE_compact-rust-backend"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    c.stdin.take().unwrap().write_all(bytes).unwrap();
    c.wait_with_output().unwrap()
}
fn assert_limit(out: Output) {
    assert!(!out.status.success());
    let s = String::from_utf8(out.stderr).unwrap();
    assert!(
        s.contains("compiler resource input_bytes exceeds 4194304 (observed at least 4194305)"),
        "{s}"
    );
}
#[test]
fn stdin_exact_byte_limit_and_one_over() {
    assert!(stdin(&padded(IR, LIMIT)).status.success());
    assert_limit(stdin(&padded(IR, LIMIT + 1)));
}
#[test]
fn serde_depth_guard_is_retained() {
    let mut value = r#"{"kind":"field_literal","value":"1"}"#.to_owned();
    for _ in 0..140 {
        value = format!(r#"{{"kind":"transient_hash","value":{value}}}"#);
    }
    let text = format!(
        r#"{{"schema_version":20,"ledger_fields":[],"stateful_circuits":[],"circuits":[{{"name":"entry","parameters":[],"result":{{"kind":"field"}},"body":{value}}}]}}"#
    );
    let o = stdin(text.as_bytes());
    assert!(!o.status.success());
    assert!(
        String::from_utf8(o.stderr)
            .unwrap()
            .contains("recursion limit exceeded")
    );
}
fn frontend(t: &Path) -> PathBuf {
    let p = t.join("frontend");
    fs::write(
        &p,
        r#"#!/bin/sh
set -eu
for arg in "$@"; do output="$arg"; done
mkdir -p "$output/contract" "$output/compiler"
cp "$RESOURCE_TEST_IR" "$output/contract/compact-rust-ir.json"
cp "$RESOURCE_TEST_INFO" "$output/compiler/contract-info.json"
printf '{}' > "$output/compiler/contract-manifest.json"
"#,
    )
    .unwrap();
    fs::set_permissions(&p, fs::Permissions::from_mode(0o700)).unwrap();
    p
}
#[test]
fn both_file_ingress_owners_and_contract_info_are_bounded() {
    let t = Scratch::new();
    let f = frontend(&t.0);
    let ir = t.0.join("input.json");
    let info = t.0.join("info.json");
    let source = t.0.join("input.compact");
    fs::write(&source, "// fixture transport only\n").unwrap();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    for (i, large_ir, large_info) in [(0, false, false), (1, true, false), (2, false, true)] {
        fs::write(
            &ir,
            if large_ir {
                padded(IR, LIMIT + 1)
            } else {
                padded(IR, LIMIT)
            },
        )
        .unwrap();
        fs::write(
            &info,
            if large_info {
                padded(r#"{"circuits":[]}"#, LIMIT + 1)
            } else {
                padded(r#"{"circuits":[]}"#, LIMIT)
            },
        )
        .unwrap();
        let out = t.0.join(format!("compactc-{i}"));
        let result = Command::new(env!("CARGO_BIN_EXE_compactc"))
            .args(["--target", "rust", "--rust-runtime-root"])
            .arg(&root)
            .arg(&source)
            .arg(&out)
            .env("COMPACTC_SCHEME", &f)
            .env("RESOURCE_TEST_IR", &ir)
            .env("RESOURCE_TEST_INFO", &info)
            .output()
            .unwrap();
        if i == 0 {
            assert!(
                result.status.success(),
                "{}",
                String::from_utf8_lossy(&result.stderr)
            );
            assert!(out.join("contract/lib.rs").is_file());
        } else {
            assert_limit(result);
            assert!(!out.exists(), "failed staged output published");
        }
        if !large_info {
            let out = t.0.join(format!("rustc-{i}"));
            let result = Command::new(env!("CARGO_BIN_EXE_compact-rustc"))
                .arg(&source)
                .arg(&out)
                .env("COMPACTC", &f)
                .env("RESOURCE_TEST_IR", &ir)
                .env("RESOURCE_TEST_INFO", &info)
                .output()
                .unwrap();
            if large_ir {
                assert_limit(result);
                assert!(!out.join("contract/lib.rs").exists());
            } else {
                assert!(
                    result.status.success(),
                    "{}",
                    String::from_utf8_lossy(&result.stderr)
                );
                assert!(out.join("contract/lib.rs").is_file());
            }
        }
    }
}

#[test]
fn stdin_retains_invalid_utf8_diagnostic() {
    let out = stdin(&[0xff]);
    assert!(!out.status.success());
    assert_eq!(
        String::from_utf8(out.stderr).unwrap(),
        "compact-rust-backend: stream did not contain valid UTF-8\n"
    );
}
