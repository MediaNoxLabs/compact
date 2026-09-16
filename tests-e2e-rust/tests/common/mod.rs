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

//! Shared helpers for the e2e fixture tests.
//!
//! Lives in a subdirectory so Cargo treats it as a module rather than as
//! another integration-test binary; consumers declare `mod common;`.

/// Extracts the source text of the first function named `fn_name` from
/// generated contract source, from the `fn` keyword through its closing
/// brace.
///
/// Some invariants are about the ORDER of statements in emitted code
/// rather than about any value it produces — e.g. that a constructor
/// flushes its ledger writes *before* an assert that reads them back.
/// Executing the contract cannot see that ordering (a read-before-write
/// yields the same final state when the asserted condition happens to
/// hold anyway), and byte-parity would happily lock a regenerated
/// wrong order as the new baseline. Such tests therefore assert
/// structurally, against the generated text, and need its body isolated
/// so a match elsewhere in the file cannot satisfy them.
///
/// Brace-counting is sufficient here because the input is rustfmt'd
/// generated code with no string literals or comments containing
/// unbalanced braces. It is not a general Rust parser; if a future
/// emitter starts embedding braces in string literals this needs to
/// become one.
///
/// Panics with a specific message if the function is absent or its
/// braces are unbalanced — a silent `None` would let the caller's
/// assertions pass vacuously.
pub fn generated_fn_body<'a>(source: &'a str, fn_name: &str) -> &'a str {
    let needle = format!("fn {fn_name}");
    let start = source
        .find(&needle)
        .unwrap_or_else(|| panic!("generated source has no `{needle}`"));
    let open = start
        + source[start..]
            .find('{')
            .unwrap_or_else(|| panic!("`{needle}` has no opening brace"));

    let mut depth = 0usize;
    for (i, ch) in source[open..].char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return &source[start..=open + i];
                }
            }
            _ => {}
        }
    }
    panic!("unbalanced braces in the body of `{needle}`");
}
