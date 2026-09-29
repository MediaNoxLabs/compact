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

//
// Source-level guard for call-argument rendering (#91, design D8).
//
// Every call argument must render at the callee's declared formal type,
// which the Rust backend does in one place: `call-args-rust`
// (compiler/rust-passes-walker.ss). The per-argument renderers it replaced,
// `arg-rust-clone-if-var` and `pure-call-arg-rust`, take the expected type
// only from the argument's own `safe-cast`, which the typechecker omits when
// the argument already has the formal's type. A call site that calls them
// directly silently loses the formal again, whatever its arguments are, so
// this test fails on any direct call outside `call-args-rust` and the
// allowlisted non-call users below.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// The per-argument renderers only `call-args-rust` may use for calls.
const GUARDED: &[&str] = &["arg-rust-clone-if-var", "pure-call-arg-rust"];

/// The entry point every call site goes through.
const ENTRY_POINT: &str = "call-args-rust";

/// Non-call users, by enclosing top-level definition, with the number of
/// direct calls each may contain. They render a ledger ADT operation's
/// arguments or a cell write's value, which are not calls and keep their own
/// expected type. Counted because `emit-streaming-body` holds both a cell
/// write and call sites: a new direct call there must still fail.
const ALLOWED: &[(&str, &str, usize)] = &[
    // Ledger ADT operation arguments.
    ("rust-passes-walker.ss", "compute-pl-builder-lines", 2),
    ("rust-passes-walker.ss", "pl-call-builder-lines", 1),
    (
        "rust-passes-walker.ss",
        "emit-non-write-public-ledger-terminal",
        1,
    ),
    // Cell-write values.
    ("rust-passes-walker.ss", "cell-write-op-lines", 1),
    ("rust-passes-walker.ss", "emit-body-mutations", 1),
    ("rust-passes-streaming.ss", "emit-streaming-body", 1),
];

fn compiler_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("repo root")
        .join("compiler")
}

/// The name a top-level definition line defines: `(define (name ...` or
/// `(define name`, at the six-space indent of the pass's `definitions` block.
fn defined_name(line: &str) -> Option<&str> {
    let rest = line.strip_prefix("      (define ")?;
    let rest = rest.strip_prefix('(').unwrap_or(rest);
    let end = rest
        .find(|c: char| c.is_whitespace() || c == ')')
        .unwrap_or(rest.len());
    Some(&rest[..end])
}

/// Direct calls to a guarded renderer, as `(file, enclosing definition)` →
/// the line numbers of the calls. Text after `;` is a comment.
fn direct_calls() -> BTreeMap<(String, String), Vec<usize>> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(compiler_dir())
        .expect("read compiler/")
        .map(|e| e.expect("dir entry").path())
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with("rust-passes-") && n.ends_with(".ss"))
        })
        .collect();
    files.sort();
    assert!(
        !files.is_empty(),
        "no compiler/rust-passes-*.ss files found"
    );

    let mut calls: BTreeMap<(String, String), Vec<usize>> = BTreeMap::new();
    for path in files {
        let file = path.file_name().unwrap().to_str().unwrap().to_string();
        let text = std::fs::read_to_string(&path).expect("read source");
        let mut enclosing = String::from("<top level>");
        for (i, line) in text.lines().enumerate() {
            if let Some(name) = defined_name(line) {
                enclosing = name.to_string();
            }
            let code = line.split(';').next().unwrap_or("");
            for name in GUARDED {
                let call = format!("({name}");
                let is_call = code.match_indices(&call).any(|(at, _)| {
                    // `(name` must be the whole symbol, not a prefix of one.
                    code[at + call.len()..]
                        .chars()
                        .next()
                        .is_none_or(|c| c.is_whitespace() || c == ')')
                });
                if is_call && !code.contains(&format!("(define ({name}")) {
                    calls
                        .entry((file.clone(), enclosing.clone()))
                        .or_default()
                        .push(i + 1);
                }
            }
        }
    }
    calls
}

#[test]
fn call_arguments_render_through_call_args_rust() {
    let mut violations = Vec::new();
    for ((file, function), lines) in direct_calls() {
        if function == ENTRY_POINT {
            continue;
        }
        let allowed = ALLOWED
            .iter()
            .find(|(f, name, _)| *f == file && *name == function)
            .map_or(0, |(_, _, n)| *n);
        if lines.len() > allowed {
            violations.push(format!(
                "  - compiler/{file}: `{function}` calls {} directly at line(s) {:?} \
                 (allowed: {allowed})",
                GUARDED.join(" / "),
                lines
            ));
        }
    }
    assert!(
        violations.is_empty(),
        "call arguments must be rendered through `{ENTRY_POINT}` so they take the \
         callee's declared formal type (#91):\n{}\n\n\
         Route the call site through `{ENTRY_POINT}`. If the site is not a call \
         (a ledger ADT operation argument or a cell-write value), add it to \
         ALLOWED in this test.",
        violations.join("\n")
    );
}

/// Every allowlisted function still exists, so a rename fails here instead
/// of silently widening the guard's blind spot.
#[test]
fn allowlist_names_existing_definitions() {
    for (file, function, _) in ALLOWED {
        let text = std::fs::read_to_string(compiler_dir().join(file)).expect("read source");
        assert!(
            text.lines().any(|l| defined_name(l) == Some(*function)),
            "ALLOWED names `{function}` in compiler/{file}, which no longer defines it"
        );
    }
    let walker =
        std::fs::read_to_string(compiler_dir().join("rust-passes-walker.ss")).expect("read source");
    assert!(
        walker.lines().any(|l| defined_name(l) == Some(ENTRY_POINT)),
        "`{ENTRY_POINT}` is no longer defined in compiler/rust-passes-walker.ss"
    );
}
