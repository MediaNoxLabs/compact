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

//! Every `nanopass-case` in the Rust emitter that dispatches on a curve or
//! field qualifier must carry an `else` arm.
//!
//! # Why this test exists
//!
//! A `nanopass-case` with no `else` is *total today and partial tomorrow*.
//! It compiles, it passes every test, and it keeps doing so right up until
//! upstream adds a variant to the grammar it matches on — at which point the
//! missing clause becomes a nanopass "no matching clause" **internal
//! compiler error** rather than the named `rust-feature-error` diagnostic
//! that the backend's contract promises.
//!
//! That is not hypothetical. Upstream grew `Curve-Type` from two variants to
//! four (curve25519 in #781, secp256r1 in #778). Three matches in
//! `rust-passes-types.ss` handled only `curve-jubjub` and `curve-secp256k1`,
//! and nothing failed, because nothing could reach them — see below. The
//! rebase that brought the new grammar in could not have told anyone.
//!
//! # What this test does NOT claim
//!
//! It does not claim those sites were reachable. They were not, and still
//! are not. Three independent barriers sit upstream of the emitter:
//!
//! 1. `Curve25519*`, `Secp256r1*` and `Secp256k1*` are declared only in
//!    `compiler/zkir-v3-natives.ss`, never in `midnight-natives.ss`, so
//!    through `--target rust` they are `unbound identifier` at the front
//!    end. Only the jubjub pair is in the default native set.
//! 2. `analysis-passes/infer-types.ss` asserts that a zkir-v3-only curve
//!    type requires `(feature-zkir-v3)`.
//! 3. `passes.ss` refuses `--target rust` together with `--feature-zkir-v3`
//!    outright, before any emitter pass runs.
//!
//! So no Compact contract can exercise those `else` arms, and deliberately
//! none is added to the rejection corpus: a probe that cannot reach the
//! guard would pass for an unrelated reason and report coverage that does
//! not exist.
//!
//! This test pins the *invariant* instead of the behaviour. Barrier 3 is
//! explicitly temporary — the diagnostic says `--target rust does not
//! support --feature-zkir-v3 **yet**` — and on the day it is lifted, a
//! partial match turns into an internal error on real user contracts. This
//! test fails at the moment someone writes the partial match, which is when
//! it is cheap to fix.
//!
//! `Curve-Type` and `Field-Type` are singled out because they are small
//! closed sets that upstream demonstrably grows. A blanket rule over every
//! `nanopass-case` would be wrong: most match a single variant of a
//! nonterminal they have already destructured, and an `else` there would
//! bury real pass bugs behind a fallback.

use std::path::{Path, PathBuf};

/// Nonterminals whose variant set upstream has grown, or may grow, and whose
/// matches in the emitter must therefore be total.
const GUARDED_NONTERMINALS: &[&str] = &["Curve-Type", "Field-Type"];

fn find_repo_root(start: &Path) -> Option<PathBuf> {
    let mut dir = Some(start);
    while let Some(d) = dir {
        if d.join("compiler").is_dir() && d.join("Cargo.toml").is_file() {
            return Some(d.to_path_buf());
        }
        dir = d.parent();
    }
    None
}

/// Byte offset just past the form opening at `open` (which must index the
/// `(` of the form), honouring Scheme string literals so a paren inside a
/// diagnostic message does not unbalance the scan.
fn form_end(src: &str, open: usize) -> usize {
    let bytes = src.as_bytes();
    let mut depth = 0usize;
    let mut i = open;
    let mut in_string = false;
    while i < bytes.len() {
        match bytes[i] {
            b'\\' if in_string => i += 1,
            b'"' => in_string = !in_string,
            b'(' if !in_string => depth += 1,
            b')' if !in_string => {
                depth -= 1;
                if depth == 0 {
                    return i + 1;
                }
            }
            _ => {}
        }
        i += 1;
    }
    bytes.len()
}

/// Remove every nested `nanopass-case` form from `body` so that arms
/// belonging to an inner dispatch are not credited to the outer one.
fn strip_nested(body: &str) -> String {
    let mut out = String::new();
    let mut cursor = 0usize;
    let mut search = 1usize; // skip the outermost form's own opening paren
    while let Some(rel) = body[search..].find("(nanopass-case") {
        let start = search + rel;
        let end = form_end(body, start);
        out.push_str(&body[cursor..start]);
        cursor = end;
        search = end;
    }
    out.push_str(&body[cursor..]);
    out
}

#[test]
fn curve_and_field_dispatches_are_total() {
    let root = find_repo_root(Path::new(env!("CARGO_MANIFEST_DIR")))
        .expect("exhaustiveness check cannot run: no ancestor holds both compiler/ and Cargo.toml");
    let compiler = root.join("compiler");

    let mut sources: Vec<PathBuf> = std::fs::read_dir(&compiler)
        .expect("read compiler/")
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with("rust-passes") && n.ends_with(".ss"))
        })
        .collect();
    sources.sort();

    assert!(
        !sources.is_empty(),
        "no compiler/rust-passes*.ss files found under {} — the check would \
         vacuously pass, which is worse than not running it",
        compiler.display()
    );

    let mut scanned = 0usize;
    let mut offenders: Vec<String> = Vec::new();

    for path in &sources {
        let src = std::fs::read_to_string(path).expect("read emitter source");
        let name = path.file_name().unwrap().to_string_lossy().into_owned();

        let mut search = 0usize;
        while let Some(rel) = src[search..].find("(nanopass-case") {
            let start = search + rel;
            let end = form_end(&src, start);
            // Advance by one byte, not to `end`: the sites that matter most
            // are *nested* dispatches (a Curve-Type match inside a Field-Type
            // match), and skipping to the end of the outer form would step
            // straight over them. An earlier draft of this test did exactly
            // that and reported one offender instead of four.
            search = start + 1;

            let body = &src[start..end];
            // The header reads `(nanopass-case (Lang Nonterminal) subject`.
            let Some(header_end) = body.find(')') else {
                continue;
            };
            let header = &body[..header_end];
            let Some(nonterminal) = header.split_whitespace().last() else {
                continue;
            };
            if !GUARDED_NONTERMINALS.contains(&nonterminal) {
                continue;
            }

            scanned += 1;
            let own = strip_nested(body);
            if !own.contains("[else") && !own.contains("[ else") {
                let line = src[..start].matches('\n').count() + 1;
                offenders.push(format!("  {name}:{line}  nanopass-case on {nonterminal}"));
            }
        }
    }

    assert!(
        scanned > 0,
        "scanned {} emitter source files but found no nanopass-case on any of \
         {GUARDED_NONTERMINALS:?}. Either the emitter stopped dispatching on \
         these nonterminals or the header parse broke — both make this test \
         vacuous, so it fails rather than reporting a false pass.",
        sources.len()
    );

    assert!(
        offenders.is_empty(),
        "these matches on a grammar that upstream grows have no `else` arm, so \
         a variant added upstream becomes a nanopass internal error instead of \
         a named rust-feature-error:\n{}\n\n\
         Add an `else` raising rust-feature-error that names the offending \
         variant. See this file's header for why these nonterminals in \
         particular, and why the arms get no rejection-corpus entry.",
        offenders.join("\n")
    );
}
