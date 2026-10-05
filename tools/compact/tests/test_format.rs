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
fn test_compact_format_no_compiler_installed() {
    let mut baseline = ReadOnlyBaseline::new();
    let temp_dir = tempfile::tempdir().unwrap();
    let temp_path = temp_dir.path();

    run_command(
        &["--directory", &format!("{}", temp_path.display()), "format"],
        Some(baseline.environment()),
        None,
        Some("./output/format/err_no_compiler.txt"),
        &[],
        Some(1),
    );
    baseline.assert_unchanged();
    ReadOnlyBaseline::assert_no_install_at(temp_path);
}

#[test]
fn test_compact_format_invalid_param() {
    let mut baseline = ReadOnlyBaseline::new();
    run_command(
        &["format", "--bob"],
        Some(baseline.environment()),
        None,
        Some("./output/format/err_invalid_param.txt"),
        &[],
        Some(2),
    );
    baseline.assert_unchanged();
}

#[test]
fn test_compact_format_param_help() {
    run_command(
        &["format", "--help"],
        None,
        Some("./output/format/std_format_help.txt"),
        None,
        &[("[USER_DIR]", env::home_dir().unwrap().to_str().unwrap())],
        Some(0),
    );
}

#[test]
fn test_compact_format_param_h() {
    run_command(
        &["format", "-h"],
        None,
        Some("./output/format/std_format_help_short.txt"),
        None,
        &[("[USER_DIR]", env::home_dir().unwrap().to_str().unwrap())],
        Some(0),
    );
}

#[test]
fn test_compact_format_param_version() {
    let mut baseline = ReadOnlyBaseline::new();
    let temp_dir = tempfile::tempdir().unwrap();
    let temp_path = temp_dir.path();

    run_command(
        &[
            "--directory",
            &format!("{}", temp_path.display()),
            "format",
            "--version",
        ],
        Some(baseline.environment()),
        None,
        Some("./output/format/err_no_compiler.txt"),
        &[],
        Some(1),
    );
    baseline.assert_unchanged();
    ReadOnlyBaseline::assert_no_install_at(temp_path);
}

#[test]
fn test_compact_format_param_v() {
    let mut baseline = ReadOnlyBaseline::new();
    let temp_dir = tempfile::tempdir().unwrap();
    let temp_path = temp_dir.path();

    run_command(
        &[
            "--directory",
            &format!("{}", temp_path.display()),
            "format",
            "-V",
        ],
        Some(baseline.environment()),
        None,
        Some("./output/format/err_no_compiler.txt"),
        &[],
        Some(1),
    );
    baseline.assert_unchanged();
    ReadOnlyBaseline::assert_no_install_at(temp_path);
}

#[test]
fn test_compact_format_param_language_version() {
    let mut baseline = ReadOnlyBaseline::new();
    let temp_dir = tempfile::tempdir().unwrap();
    let temp_path = temp_dir.path();

    run_command(
        &[
            "--directory",
            &format!("{}", temp_path.display()),
            "format",
            "--language-version",
        ],
        Some(baseline.environment()),
        None,
        Some("./output/format/err_no_compiler.txt"),
        &[],
        Some(1),
    );
    baseline.assert_unchanged();
    ReadOnlyBaseline::assert_no_install_at(temp_path);
}
