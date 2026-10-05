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

use crate::common::{ReadOnlyBaseline, get_version, run_command};

mod common;

#[test]
fn test_compact_compile_no_param() {
    let mut baseline = ReadOnlyBaseline::new();
    run_command(
        &["compile"],
        Some(baseline.environment()),
        None,
        Some("./output/compile/err_default.txt"),
        &[],
        Some(1),
    );
    baseline.assert_unchanged();
}

#[test]
fn test_compact_compile_no_compiler_installed_default_directory() {
    let mut baseline = ReadOnlyBaseline::new();
    let compiler = "+0.21.0";

    run_command(
        &["compile", compiler, "--version"],
        Some(baseline.environment()),
        None,
        Some("./output/compile/err_invalid_compiler.txt"),
        &[
            ("[USER_DIR]", baseline.home().to_str().unwrap()),
            ("[COMPILER]", compiler.strip_prefix("+").unwrap()),
            ("[SYSTEM_VERSION]", get_version()),
        ],
        Some(1),
    );
    baseline.assert_unchanged();
}

#[test]
fn test_compact_compile_no_compiler_installed_pass_directory() {
    let mut baseline = ReadOnlyBaseline::new();
    let temp_dir = tempfile::tempdir().unwrap();
    let temp_path = temp_dir.path();
    let compiler = "+0.21.0";

    run_command(
        &[
            "--directory",
            &format!("{}", temp_path.display()),
            "compile",
            compiler,
            "--version",
        ],
        Some(baseline.environment()),
        None,
        Some("./output/compile/err_invalid_compiler_dir.txt"),
        &[
            ("[USER_DIR]", &format!("{}", temp_path.display())),
            ("[COMPILER]", compiler.strip_prefix("+").unwrap()),
            ("[SYSTEM_VERSION]", get_version()),
        ],
        Some(1),
    );
    baseline.assert_unchanged();
    ReadOnlyBaseline::assert_no_install_at(temp_path);
}

#[test]
fn test_compile_contract_no_compiler_installed() {
    let mut baseline = ReadOnlyBaseline::new();
    let temp_output = tempfile::tempdir().unwrap();
    let temp_output_path = temp_output.path();

    run_command(
        &[
            "--directory",
            &format!("{}", temp_output_path.display()),
            "compile",
            "./contract/counter.compact",
            temp_output_path.to_str().unwrap(),
        ],
        Some(baseline.environment()),
        None,
        Some("./output/compile/err_default.txt"),
        &[],
        Some(1),
    );
    baseline.assert_unchanged();
    ReadOnlyBaseline::assert_no_install_at(temp_output_path);
}

#[test]
fn test_compact_compile_invalid_param() {
    let mut baseline = ReadOnlyBaseline::new();
    run_command(
        &["compile", "latest"],
        Some(baseline.environment()),
        None,
        Some("./output/compile/err_default.txt"),
        &[],
        Some(1),
    );
    baseline.assert_unchanged();
}
