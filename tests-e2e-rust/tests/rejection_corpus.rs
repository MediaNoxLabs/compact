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
// The negative corpus: constructs the Rust backend must REFUSE.
//
// Every other test in this crate pins what the backend emits. This one
// pins what it must *not* emit, which is a distinct property and was
// entirely untested — the gap that let MediaNoxLabs/compact#45 exist.
//
// The backend's central safety claim is:
//
//     a construct the emitter cannot lower must FAIL the compile,
//     never emit something plausible.
//
// Nothing enforced that claim. Byte-parity cannot: it compares committed
// output against regenerated output, so a construct that emits bad Rust
// agrees with itself perfectly and the fixture is green forever. The
// three cases below were all found by hand, and all three had shipped:
//
//   * a constructor whose body no walker shape matched emitted the
//     *default scaffold* — every ledger write in the constructor silently
//     discarded, from a compile that exited 0. A miscompiler.
//   * a ledger field with no decoder emitted `decode_u64` behind a TODO
//     comment, so a `Vector<3, Bytes<32>>` accessor returned the wrong
//     type. Reachable with one ledger field and nothing else.
//   * `Field as Uint<N>` emitted `(x) as u64` where `x: Fr` — a struct.
//     E0605, a non-primitive cast, from a compile that exited 0.
//
// The first fails at run time with wrong state; the other two fail at
// `cargo build`, in generated code, with no pointer back to the Compact
// source. All three exited 0.
//
// Adding a lowering? Delete the entry. Adding a rejection? Add one. An
// entry that stops rejecting is a regression whether or not the emitted
// code happens to compile.
//
// Like the byte-parity gate, these tests never skip themselves: a missing
// compiler is a hard failure, and callers that cannot supply one exclude
// them by name (`-- --skip rust_backend_`, which is why both are named with
// that prefix). See codegen_regression.rs for the same arrangement.

use std::path::{Path, PathBuf};
use std::process::Command;

/// (case name, Compact source, expected `rust-feature-error` kind)
const REJECTIONS: &[(&str, &str, &str)] = &[
    (
        // The miscompiler. `names.insert` + a `for` writing a ledger cell
        // is past what the constructor walker matches, so before #45 this
        // emitted a constructor containing only the bare scaffold seed.
        "constructor body no shape matches",
        "import CompactStandardLibrary;\n\
         export ledger total: Uint<64>;\n\
         export ledger names: Map<Uint<8>, Uint<64>>;\n\
         constructor() {\n\
           names.insert(1, 100);\n\
           names.insert(2, 200);\n\
           for (const i of 0..3) { total = (total + 7) as Uint<64>; }\n\
         }\n",
        "ctor-body-emission",
    ),
    (
        // The smallest contract that reached the bad path: one ledger
        // field, no circuits, no constructor.
        "ledger field with no decoder",
        "export ledger keys: Vector<3, Bytes<32>>;\n",
        "ledger-read-decoder-missing",
    ),
    (
        // Narrowing an `Fr` needs a range-checking runtime helper that
        // does not exist. The emitter reached for Rust's `as` instead.
        //
        // The kind here is the enclosing body's, not the specific
        // `cast-from-field`: the pure-circuit emitter probes shapes under
        // a catch-all `(guard (c [#t #f]) ...)`, which swallows the
        // precise diagnostic and reports the generic one. The refusal is
        // correct; only the message is coarse. Tracked separately.
        // `Uint<128>` arithmetic. This one is load-bearing rather than
        // incidental, and the reasoning is worth keeping next to the test.
        //
        // Rust lowers `a + b` as `wrapping_add` on operands widened to the
        // next width up, then range-checks the narrowing back down. Below
        // u128 that composes correctly — `u64 + u64` cannot lose anything at
        // u128, so the check sees the true sum and matches TypeScript, whose
        // bigints are exact.
        //
        // At u128 there is no next width. A `wrapping_add` there would wrap
        // *before* the check, the check would then see an in-range value, and
        // Rust would silently return a wrapped result where TypeScript throws
        // — reintroducing #51 in a form the narrowing fix cannot catch,
        // because by then the information is gone.
        //
        // The backend refuses instead. `Uint<128>` values themselves are
        // fine; only arithmetic on them is out. If someone lowers this, the
        // wrap has to be detected rather than range-checked afterwards.
        "Uint<128> arithmetic",
        "export ledger n: Uint<128>;\n\
         export pure circuit addBig(a: Uint<128>, b: Uint<128>): Uint<128> { return (a + b) as Uint<128>; }\n",
        "pure-circuit-body-emission",
    ),
    (
        // Enum casts. TypeScript emits a range check that throws for both
        // directions — `cast from Field or Uint value to enum X failed` and
        // `cast from enum X to Uint<0..N> failed`. Rust has no lowering for
        // either, so both refuse.
        //
        // These are here to keep it that way. `x as Uint<N>` was the third
        // value-check TypeScript emits, and it *did* have a Rust lowering
        // that skipped the check — silently returning 44 for an input
        // TypeScript throws on (MediaNoxLabs/compact#51). Anyone adding an
        // enum-cast lowering has to delete an entry here to do it, which is
        // the moment to remember the bound.
        "cast from enum to Uint",
        "enum Color { Red, Green, Blue }\n\
         export ledger c: Uint<8>;\n\
         export pure circuit toNum(x: Color): Uint<0..2> { return x as Uint<0..2>; }\n",
        "pure-circuit-body-emission",
    ),
    (
        "cast from Uint to enum",
        "enum Color { Red, Green, Blue }\n\
         export ledger c: Uint<8>;\n\
         export pure circuit fromNum(n: Uint<0..2>): Color { return n as Color; }\n",
        "pure-circuit-body-emission",
    ),
    (
        "Field narrowed to Uint",
        "export ledger n: Uint<64>;\n\
         export circuit narrow(f: Field): Uint<64> { return f as Uint<64>; }\n",
        "pure-circuit-body-emission",
    ),
];

/// Contracts that must still compile — the other half of the property.
///
/// A rejection guard is only worth having if it is narrow. The #45 fix
/// keyed on "is there a constructor statement?", which is true even when
/// the author wrote no constructor (the front end synthesises one), so
/// the first attempt rejected every constructor-less contract in the
/// world. These three pin the boundary from the accepting side.
const ACCEPTIONS: &[(&str, &str)] = &[
    ("no constructor at all", "export ledger n: Uint<64>;\n"),
    (
        "explicitly empty constructor",
        "export ledger n: Uint<64>;\nconstructor() { }\n",
    ),
    (
        // The boundary above is arithmetic-only; the type itself is
        // supported, and a guard that took out all of Uint<128> would be too
        // wide.
        "Uint<128> without arithmetic",
        "export ledger n: Uint<128>;\n\
         export pure circuit passBig(a: Uint<128>): Uint<128> { return a; }\n",
    ),
    (
        "constructor with plain writes",
        "export ledger admin: Uint<64>;\n\
         export ledger count: Uint<64>;\n\
         constructor() { admin = 42; count = 7; }\n",
    ),
];

fn find_repo_root(start: &Path) -> Option<PathBuf> {
    let mut cur = start.to_path_buf();
    for _ in 0..6 {
        if cur.join("examples").is_dir() && cur.join("Cargo.toml").is_file() {
            return Some(cur);
        }
        if !cur.pop() {
            break;
        }
    }
    None
}

fn compiler() -> PathBuf {
    let root = find_repo_root(Path::new(env!("CARGO_MANIFEST_DIR")))
        .expect("rejection corpus cannot run: no ancestor holds both examples/ and Cargo.toml");
    let (compactc, how) = match std::env::var_os("COMPACTC") {
        Some(p) => (PathBuf::from(p), "COMPACTC"),
        None => (root.join("result/bin/compactc"), "default path"),
    };
    assert!(
        compactc.exists(),
        "rejection corpus cannot run: no compactc at {} (from {}). \
         Run `nix build .#compactc`, or point COMPACTC at a real binary.",
        compactc.display(),
        how
    );
    compactc
}

/// Compile `source` to a fresh directory. Returns (exit code, stderr+stdout,
/// whether a contract crate was emitted).
fn compile(compactc: &Path, case: &str, source: &str) -> (Option<i32>, String, bool) {
    let dir = std::env::temp_dir().join(format!(
        "compact-rejection-{}-{}",
        std::process::id(),
        case.replace(' ', "-")
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create temp dir");

    let src = dir.join("probe.compact");
    std::fs::write(&src, source).expect("write probe source");
    let out = dir.join("out");

    let result = Command::new(compactc)
        // `--rust`, not `--target rust`: the repeatable `--target <language>`
        // flag lives on the ledger-8 line and has not been carried over here
        // yet. Kept in step with codegen_regression.rs, which drives the same
        // compiler.
        .args(["--rust", "--skip-zk"])
        .arg(&src)
        .arg(&out)
        .output()
        .expect("run compactc");

    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&result.stderr),
        String::from_utf8_lossy(&result.stdout)
    );
    let emitted = out.join("contract/lib.rs").exists();
    (result.status.code(), text, emitted)
}

#[test]
fn rust_backend_rejects_what_it_cannot_lower() {
    let compactc = compiler();

    for (case, source, kind) in REJECTIONS {
        let (code, text, emitted) = compile(&compactc, case, source);

        assert_ne!(
            code,
            Some(0),
            "{case}: compactc exited 0. This construct is not lowered, so a \
             successful exit means it emitted a guess — the exact failure #45 \
             was about.\n--- output ---\n{text}"
        );
        assert!(
            !emitted,
            "{case}: compactc failed but still wrote contract/lib.rs. A \
             refused compile must leave no output behind for a build to pick up."
        );
        assert!(
            text.contains(kind),
            "{case}: expected the diagnostic to name `{kind}`, so the reason is \
             greppable and attributable.\n--- output ---\n{text}"
        );
    }
}

#[test]
fn rust_backend_still_accepts_neighbouring_shapes() {
    let compactc = compiler();

    for (case, source) in ACCEPTIONS {
        let (code, text, emitted) = compile(&compactc, case, source);

        assert_eq!(
            code,
            Some(0),
            "{case}: compactc refused a contract it must accept. A rejection \
             guard that is too wide is its own bug.\n--- output ---\n{text}"
        );
        assert!(emitted, "{case}: compactc exited 0 but emitted no lib.rs");
    }
}
