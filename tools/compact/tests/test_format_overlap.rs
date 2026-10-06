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

#![cfg(unix)]

use std::{
    fs,
    os::unix::fs::{PermissionsExt, symlink},
    process::{Command, Output},
};

struct FormatterFixture {
    root: tempfile::TempDir,
}

impl FormatterFixture {
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir_all(root.path().join("install/bin")).unwrap();
        fs::create_dir(root.path().join("inputs")).unwrap();
        fs::create_dir(root.path().join("started")).unwrap();
        let formatter = root.path().join("install/bin/format-compact");
        fs::write(
            &formatter,
            r#"#!/bin/sh
printf '%s\n' "$1" >> "$PROBE_ROOT/invocations"
if [ "$PROBE_MODE" = parallel ]; then
    name=${1##*/}
    mkdir "$PROBE_ROOT/started/$name" || exit 21
    attempts=0
    while [ ! -d "$PROBE_ROOT/started/a.compact" ] || [ ! -d "$PROBE_ROOT/started/b.compact" ]; do
        attempts=$((attempts + 1))
        [ "$attempts" -lt 1000 ] || exit 22
        sleep 0.01
    done
fi
[ "$PROBE_MODE" != fail ] || exit 23
if [ "$#" -eq 1 ]; then
    printf 'formatted\n'
else
    printf 'formatted\n' > "$2"
fi
"#,
        )
        .unwrap();
        fs::set_permissions(formatter, fs::Permissions::from_mode(0o755)).unwrap();
        Self { root }
    }

    fn file(&self, name: &str) {
        fs::write(self.root.path().join("inputs").join(name), "original\n").unwrap();
    }

    fn run(&self, mode: &str, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_compact"))
            .current_dir(self.root.path())
            .env("HOME", self.root.path())
            .env("COMPACT_DIRECTORY", self.root.path().join("install"))
            .env("PROBE_ROOT", self.root.path())
            .env("PROBE_MODE", mode)
            .env("NO_COLOR", "1")
            .args(["--directory", "install", "format", "--verbose"])
            .args(args)
            .output()
            .unwrap()
    }

    fn invocations(&self) -> usize {
        fs::read_to_string(self.root.path().join("invocations"))
            .unwrap()
            .lines()
            .count()
    }
}

#[test]
fn overlapping_paths_format_once_each_and_keep_distinct_files_parallel() {
    let fixture = FormatterFixture::new();
    fixture.file("a.compact");
    fixture.file("b.compact");
    symlink(
        "inputs/a.compact",
        fixture.root.path().join("alias.compact"),
    )
    .unwrap();
    let output = fixture.run(
        "parallel",
        &[
            "alias.compact",
            "inputs",
            "inputs/a.compact",
            "./inputs/a.compact",
            "inputs/a.compact",
        ],
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(fixture.invocations(), 2);
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert_eq!(
        stdout
            .lines()
            .filter(|line| line.ends_with(": formatted"))
            .count(),
        6
    );
    assert!(stdout.contains("./inputs/a.compact: formatted"));
    assert!(stdout.contains("alias.compact: formatted"));
    for name in ["a.compact", "b.compact"] {
        assert_eq!(
            fs::read_to_string(fixture.root.path().join("inputs").join(name)).unwrap(),
            "formatted\n"
        );
    }
}

#[test]
fn duplicate_check_reports_each_diff_without_writing() {
    let fixture = FormatterFixture::new();
    fixture.file("a.compact");
    let output = fixture.run(
        "check",
        &["--check", "inputs/a.compact", "inputs/a.compact"],
    );
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(fixture.invocations(), 1);
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert_eq!(stderr.matches("inputs/a.compact:").count(), 2);
    assert_eq!(
        fs::read_to_string(fixture.root.path().join("inputs/a.compact")).unwrap(),
        "original\n"
    );
}

#[test]
fn duplicate_formatter_failure_reports_each_occurrence() {
    let fixture = FormatterFixture::new();
    fixture.file("a.compact");
    let output = fixture.run("fail", &["inputs/a.compact", "inputs/a.compact"]);
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(fixture.invocations(), 1);
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert_eq!(stderr.matches("inputs/a.compact: failed").count(), 2);
}
