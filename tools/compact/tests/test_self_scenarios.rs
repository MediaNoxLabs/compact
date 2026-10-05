// This file is part of Compact.
// Copyright (C) 2025 Midnight Foundation
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

use crate::common::{COMPACT_VERSION, load_and_replace};
use self_update_fixture::SelfUpdateFixture;
use std::process::Output;

mod common;
#[path = "common/self_update_fixture.rs"]
mod self_update_fixture;

// The archive target differs from the compactc archive target on macOS ARM.
fn historical_archive_target() -> &'static str {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("macos", "aarch64") => "aarch64-apple-darwin",
        ("macos", "x86_64") => "x86_64-apple-darwin",
        ("linux", "x86_64") => "x86_64-unknown-linux-musl-static",
        _ => "unknown",
    }
}

fn assert_historical_output(
    output: &Output,
    stdout_fixture: &str,
    stderr_fixture: Option<&str>,
    replacements: &[(&str, &str)],
) {
    let expected_stdout = load_and_replace(stdout_fixture, replacements);
    let expected_stderr = stderr_fixture
        .map(|path| load_and_replace(path, replacements))
        .unwrap_or_default();
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        expected_stdout.trim()
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stderr).trim(),
        expected_stderr.trim()
    );
    assert_eq!(output.status.code(), Some(0));
}

#[test]
fn test_self_sc1_download_release_and_check() {
    let fixture = SelfUpdateFixture::new();
    let output = fixture.check();
    assert_historical_output(
        &output,
        "./output/self_scenarios/std_update_available.txt",
        None,
        &[("[COMPACT_VERSION]", COMPACT_VERSION)],
    );
}

#[test]
#[cfg(not(all(target_os = "macos", target_arch = "x86_64")))]
fn test_self_sc2_download_release_and_update() {
    let fixture = SelfUpdateFixture::new();
    let output = fixture.update();
    assert_historical_output(
        &output,
        "./output/self_scenarios/std_update_downloaded.txt",
        Some("./output/self_scenarios/err_update_downloading.txt"),
        &[
            ("[COMPACT_VERSION]", COMPACT_VERSION),
            (
                "[USER_DIR]",
                fixture.home().to_str().expect("UTF-8 private home"),
            ),
            ("[SYSTEM_VERSION]", historical_archive_target()),
        ],
    );
    fixture.assert_up_to_date();
}

#[test]
#[cfg(all(target_os = "macos", target_arch = "x86_64"))]
fn test_self_sc2a_download_release_and_update() {
    let fixture = SelfUpdateFixture::new();
    let output = fixture.update();
    assert_historical_output(
        &output,
        "./output/self_scenarios/std_update_downloaded_macos_x86_64.txt",
        Some("./output/self_scenarios/err_update_downloading.txt"),
        &[
            ("[COMPACT_VERSION]", COMPACT_VERSION),
            (
                "[USER_DIR]",
                fixture.home().to_str().expect("UTF-8 private home"),
            ),
            ("[SYSTEM_VERSION]", historical_archive_target()),
        ],
    );
    fixture.assert_up_to_date();
}

#[test]
fn test_self_fixture_rejects_missing_and_changed_asset() {
    self_update_fixture::assert_asset_validation_refusals();
}
