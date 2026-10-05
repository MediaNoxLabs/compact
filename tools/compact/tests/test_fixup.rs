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

use crate::common::{ReadOnlyBaseline, run_command};
use std::env;

mod common;

#[test]
fn test_compact_fixup_no_compiler_installed() {
    let mut baseline = ReadOnlyBaseline::new();
    let temp_dir = tempfile::tempdir().unwrap();
    let temp_path = temp_dir.path();

    run_command(
        &[
            "--directory",
            &format!("{}", temp_path.display()),
            "fixup",
            "test.compact",
        ],
        Some(baseline.environment()),
        None,
        Some("./output/fixup/err_no_compiler_installed.txt"),
        &[],
        Some(1),
    );
    baseline.assert_unchanged();
    ReadOnlyBaseline::assert_no_install_at(temp_path);
}

#[test]
fn test_compact_fixup_invalid_param() {
    let mut baseline = ReadOnlyBaseline::new();
    run_command(
        &["fixup", "--invalid-flag"],
        Some(baseline.environment()),
        None,
        Some("./output/fixup/err_invalid_param.txt"),
        &[],
        Some(2),
    );
    baseline.assert_unchanged();
}

#[test]
fn test_compact_fixup_param_help() {
    run_command(
        &["fixup", "--help"],
        None,
        Some("./output/fixup/std_fixup_help.txt"),
        None,
        &[(
            "[COMPACT_DIR]",
            &format!("{}/.compact", env::var("HOME").unwrap()),
        )],
        None,
    );
}

#[test]
fn test_compact_fixup_param_h() {
    run_command(
        &["fixup", "-h"],
        None,
        Some("./output/fixup/std_fixup_help_short.txt"),
        None,
        &[(
            "[COMPACT_DIR]",
            &format!("{}/.compact", env::var("HOME").unwrap()),
        )],
        None,
    );
}

#[test]
fn test_compact_fixup_param_version() {
    let mut baseline = ReadOnlyBaseline::new();
    let temp_dir = tempfile::tempdir().unwrap();
    let temp_path = temp_dir.path();

    run_command(
        &[
            "--directory",
            &format!("{}", temp_path.display()),
            "fixup",
            "--version",
        ],
        Some(baseline.environment()),
        None,
        Some("./output/fixup/err_no_compiler_installed.txt"),
        &[],
        Some(1),
    );
    baseline.assert_unchanged();
    ReadOnlyBaseline::assert_no_install_at(temp_path);
}

#[test]
fn test_compact_fixup_param_v() {
    let mut baseline = ReadOnlyBaseline::new();
    let temp_dir = tempfile::tempdir().unwrap();
    let temp_path = temp_dir.path();

    run_command(
        &[
            "--directory",
            &format!("{}", temp_path.display()),
            "fixup",
            "-V",
        ],
        Some(baseline.environment()),
        None,
        Some("./output/fixup/err_no_compiler_installed.txt"),
        &[],
        Some(1),
    );
    baseline.assert_unchanged();
    ReadOnlyBaseline::assert_no_install_at(temp_path);
}

#[test]
fn test_compact_fixup_param_language_version() {
    let mut baseline = ReadOnlyBaseline::new();
    let temp_dir = tempfile::tempdir().unwrap();
    let temp_path = temp_dir.path();

    run_command(
        &[
            "--directory",
            &format!("{}", temp_path.display()),
            "fixup",
            "--language-version",
        ],
        Some(baseline.environment()),
        None,
        Some("./output/fixup/err_no_compiler_installed.txt"),
        &[],
        Some(1),
    );
    baseline.assert_unchanged();
    ReadOnlyBaseline::assert_no_install_at(temp_path);
}

#[test]
fn test_compact_fixup_directory_no_compiler() {
    let mut baseline = ReadOnlyBaseline::new();
    let temp_dir = tempfile::tempdir().unwrap();
    let temp_path = temp_dir.path();

    run_command(
        &[
            "--directory",
            &format!("{}", temp_path.display()),
            "fixup",
            ".",
        ],
        Some(baseline.environment()),
        None,
        Some("./output/fixup/err_no_compiler_installed.txt"),
        &[],
        Some(1),
    );
    baseline.assert_unchanged();
    ReadOnlyBaseline::assert_no_install_at(temp_path);
}

#[test]
fn test_compact_fixup_multiple_files() {
    let mut baseline = ReadOnlyBaseline::new();
    let temp_dir = tempfile::tempdir().unwrap();
    let temp_path = temp_dir.path();

    run_command(
        &[
            "--directory",
            &format!("{}", temp_path.display()),
            "fixup",
            "file1.compact",
            "file2.compact",
        ],
        Some(baseline.environment()),
        None,
        Some("./output/fixup/err_no_compiler_installed.txt"),
        &[],
        Some(1),
    );
    baseline.assert_unchanged();
    ReadOnlyBaseline::assert_no_install_at(temp_path);
}

#[test]
fn test_compact_fixup_no_compiler_with_directory() {
    let mut baseline = ReadOnlyBaseline::new();
    let temp_dir = tempfile::tempdir().unwrap();
    let temp_path = temp_dir.path();

    run_command(
        &[
            "--directory",
            &format!("{}", temp_path.display()),
            "fixup",
            "test.compact",
        ],
        Some(baseline.environment()),
        None,
        Some("./output/fixup/err_no_compiler_installed.txt"),
        &[],
        Some(1),
    );
    baseline.assert_unchanged();
    ReadOnlyBaseline::assert_no_install_at(temp_path);
}

#[test]
fn test_compact_fixup_directory_with_custom_directory() {
    let mut baseline = ReadOnlyBaseline::new();
    let temp_dir = tempfile::tempdir().unwrap();
    let temp_path = temp_dir.path();

    run_command(
        &[
            "--directory",
            &format!("{}", temp_path.display()),
            "fixup",
            ".",
        ],
        Some(baseline.environment()),
        None,
        Some("./output/fixup/err_no_compiler_installed.txt"),
        &[],
        Some(1),
    );
    baseline.assert_unchanged();
    ReadOnlyBaseline::assert_no_install_at(temp_path);
}

#[test]
fn test_compact_fixup_with_invalid_flag() {
    let mut baseline = ReadOnlyBaseline::new();
    let temp_dir = tempfile::tempdir().unwrap();
    let temp_path = temp_dir.path();

    run_command(
        &[
            "--directory",
            &format!("{}", temp_path.display()),
            "fixup",
            "--invalid-flag",
        ],
        Some(baseline.environment()),
        None,
        Some("./output/fixup/err_invalid_param.txt"),
        &[],
        Some(2),
    );
    baseline.assert_unchanged();
    ReadOnlyBaseline::assert_no_install_at(temp_path);
}

#[test]
fn test_compact_fixup_multiple_files_with_directory() {
    let mut baseline = ReadOnlyBaseline::new();
    let temp_dir = tempfile::tempdir().unwrap();
    let temp_path = temp_dir.path();

    run_command(
        &[
            "--directory",
            &format!("{}", temp_path.display()),
            "fixup",
            "file1.compact",
            "file2.compact",
        ],
        Some(baseline.environment()),
        None,
        Some("./output/fixup/err_no_compiler_installed.txt"),
        &[],
        Some(1),
    );
    baseline.assert_unchanged();
    ReadOnlyBaseline::assert_no_install_at(temp_path);
}

#[test]
fn test_compact_fixup_update_uint_ranges_flag() {
    let mut baseline = ReadOnlyBaseline::new();
    let temp_dir = tempfile::tempdir().unwrap();
    let temp_path = temp_dir.path();

    run_command(
        &[
            "--directory",
            &format!("{}", temp_path.display()),
            "fixup",
            "--update-Uint-ranges",
            "test.compact",
        ],
        Some(baseline.environment()),
        None,
        Some("./output/fixup/err_no_compiler_installed.txt"),
        &[],
        Some(1),
    );
    baseline.assert_unchanged();
    ReadOnlyBaseline::assert_no_install_at(temp_path);
}

#[test]
fn test_compact_fixup_check_flag() {
    let mut baseline = ReadOnlyBaseline::new();
    let temp_dir = tempfile::tempdir().unwrap();
    let temp_path = temp_dir.path();

    run_command(
        &[
            "--directory",
            &format!("{}", temp_path.display()),
            "fixup",
            "--check",
            "test.compact",
        ],
        Some(baseline.environment()),
        None,
        Some("./output/fixup/err_no_compiler_installed.txt"),
        &[],
        Some(1),
    );
    baseline.assert_unchanged();
    ReadOnlyBaseline::assert_no_install_at(temp_path);
}

#[test]
fn test_compact_fixup_check_short_flag() {
    let mut baseline = ReadOnlyBaseline::new();
    let temp_dir = tempfile::tempdir().unwrap();
    let temp_path = temp_dir.path();

    run_command(
        &[
            "--directory",
            &format!("{}", temp_path.display()),
            "fixup",
            "-c",
            "test.compact",
        ],
        Some(baseline.environment()),
        None,
        Some("./output/fixup/err_no_compiler_installed.txt"),
        &[],
        Some(1),
    );
    baseline.assert_unchanged();
    ReadOnlyBaseline::assert_no_install_at(temp_path);
}

#[test]
fn test_compact_fixup_verbose_flag() {
    let mut baseline = ReadOnlyBaseline::new();
    let temp_dir = tempfile::tempdir().unwrap();
    let temp_path = temp_dir.path();

    run_command(
        &[
            "--directory",
            &format!("{}", temp_path.display()),
            "fixup",
            "--verbose",
            "test.compact",
        ],
        Some(baseline.environment()),
        None,
        Some("./output/fixup/err_no_compiler_installed.txt"),
        &[],
        Some(1),
    );
    baseline.assert_unchanged();
    ReadOnlyBaseline::assert_no_install_at(temp_path);
}

#[test]
fn test_compact_fixup_verbose_short_flag() {
    let mut baseline = ReadOnlyBaseline::new();
    let temp_dir = tempfile::tempdir().unwrap();
    let temp_path = temp_dir.path();

    run_command(
        &[
            "--directory",
            &format!("{}", temp_path.display()),
            "fixup",
            "-v",
            "test.compact",
        ],
        Some(baseline.environment()),
        None,
        Some("./output/fixup/err_no_compiler_installed.txt"),
        &[],
        Some(1),
    );
    baseline.assert_unchanged();
    ReadOnlyBaseline::assert_no_install_at(temp_path);
}
