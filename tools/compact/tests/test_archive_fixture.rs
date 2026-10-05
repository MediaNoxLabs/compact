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

#[path = "common/archive_fixture.rs"]
mod archive_fixture;

use archive_fixture::ArchiveFixture;
use std::fs;
use std::process::Output;

fn success(output: Output) -> String {
    assert!(
        output.status.success(),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.stderr.is_empty(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap().trim().into()
}

#[test]
fn genuine_archive_install_default_repeat_compile_and_clean() {
    let mut fixture = ArchiveFixture::new();
    let platform = match (std::env::consts::OS, std::env::consts::ARCH) {
        ("macos", "aarch64") => "aarch64-darwin",
        ("macos", "x86_64") => "x86_64-apple-darwin",
        ("linux", "x86_64") => "x86_64-unknown-linux-musl",
        ("linux", "aarch64") => "aarch64-unknown-linux-musl",
        _ => unreachable!(),
    };
    assert_eq!(
        success(fixture.run(&["update"])),
        format!(
            "compact: {platform} -- 0.31.0 -- installed\ncompact: {platform} -- 0.31.0 -- default."
        )
    );
    assert_eq!(fixture.downloads(), 1);
    assert_eq!(success(fixture.run(&["compile", "--version"])), "0.31.0");
    assert_eq!(
        success(fixture.run(&["update", "0.31.0"])),
        format!(
            "compact: {platform} -- 0.31.0 -- already installed\ncompact: {platform} -- 0.31.0 -- default."
        )
    );
    assert_eq!(fixture.downloads(), 1);
    let output = tempfile::tempdir().unwrap();
    // Genuine compiler execution; proving-key generation remains in the
    // existing original scenarios, not duplicated by this fixture smoke.
    let compiled = fixture.run(&[
        "compile",
        "--skip-zk",
        "./contract/counter.compact",
        output.path().to_str().unwrap(),
    ]);
    assert!(
        compiled.status.success(),
        "{}",
        String::from_utf8_lossy(&compiled.stderr)
    );
    for file in [
        "contract/index.js",
        "contract/index.d.ts",
        "compiler/contract-info.json",
        "zkir/increment.zkir",
    ] {
        assert!(
            output.path().join(file).is_file(),
            "missing genuine compiler artifact {file}"
        );
    }
    assert_eq!(
        success(fixture.run(&["clean"])),
        "compact: removing versions\ncompact: removed 0.31.0"
    );
    assert_eq!(
        fs::read_dir(fixture.directory().join("versions"))
            .unwrap()
            .count(),
        0
    );
    assert_eq!(fixture.downloads(), 1);
    fixture.assert_no_external_requests();
}

#[test]
fn missing_archive_cache_fails_before_installer_execution() {
    let empty = tempfile::tempdir().unwrap();
    let error = ArchiveFixture::from_cache(empty.path())
        .err()
        .expect("missing cache admitted");
    assert!(error.contains("fixture not acquired"), "{error}");
    assert_eq!(fs::read_dir(empty.path()).unwrap().count(), 0);
}

#[test]
fn corrupt_archive_cache_fails_before_installer_execution() {
    let temp = tempfile::tempdir().unwrap();
    let manifest: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/compiler-archives.json")).unwrap();
    // All platforms have a 0.31.0 asset. An invalid cache entry must be refused
    // rather than silently downloaded or installed.
    let platform = match (std::env::consts::OS, std::env::consts::ARCH) {
        ("macos", "aarch64") => "aarch64-darwin",
        ("macos", "x86_64") => "x86_64-apple-darwin",
        ("linux", "x86_64") => "x86_64-unknown-linux-musl",
        ("linux", "aarch64") => "aarch64-unknown-linux-musl",
        _ => unreachable!(),
    };
    let row = manifest["archives"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["platform"] == platform)
        .unwrap();
    let file = temp
        .path()
        .join(format!("{}.zip", row["sha256"].as_str().unwrap()));
    fs::write(&file, b"invalid archive").unwrap();
    let error = ArchiveFixture::from_cache(temp.path())
        .err()
        .expect("corrupt cache admitted");
    assert!(error.contains("fixture size mismatch"), "{error}");
    assert_eq!(fs::read(file).unwrap(), b"invalid archive");
}
