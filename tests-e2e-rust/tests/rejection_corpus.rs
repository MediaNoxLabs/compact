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
//
// ---------------------------------------------------------------------
// Oracle probes
//
// The vendored digital-passport contract is the oracle for the rust
// backend's real-idiom gaps. `vendor-digital-passport-harness` recorded
// each gap of its day as an executable expectation; the change that closes
// a gap flips its probe from REJECTION to ACCEPTION:
//
//   * the three ternary extracts (const RHS, assert argument, interior
//     arithmetic operand) refused under the pre-fix emitter;
//     `fix-ternary-expression-codegen` added the `(if ...)` clause to
//     `expr-rust`, so they are now ACCEPTIONS.
//   * `rust_backend_dogfood_entry_compiles` compiles the whole vendored
//     entry in place (by repo-relative path — see `compile_repo_relative`)
//     and asserts the end-to-end acceptance: exit 0 and `contract/lib.rs`
//     emitted.
//
// The mixed-width comparison operand (`q * 4` vs a `Uint<32>` value) used to
// be a REJECTION on the constructor route and an ill-typed emission
// (`EMITS_UNCOMPILABLE`) on the pure route. `type-directed-expression-coercion`
// widened the narrow operand on both routes (task 2.3/2.9) and taught the ctor
// walker to lower the lifted temp (task 3.1), so both are now plain ACCEPTIONS.
//
// The whole-entry gate flipped here rather than in
// `add-digital-passport-dogfood-fixture`: `fix-ternary-expression-codegen`'s
// `(if ...)` clause is what makes the ternary at `helpers.compact:257` compile,
// and the pure-route mixed-width comparison (`helpers.compact:268`) had already
// stopped blocking when coercion widened the narrow operand.

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
        "Field narrowed to Uint",
        "export ledger n: Uint<64>;\n\
         export circuit narrow(f: Field): Uint<64> { return f as Uint<64>; }\n",
        "pure-circuit-body-emission",
    ),
    // Task 4.2: a Uint source range wider than `u128::MAX` has no lossless
    // Rust cast into `Fr` (`From<u128> for Fr` is the widest available), so
    // the coercion must refuse rather than emit a truncating `as u64`/`as
    // u128` or a bare (wrong-width) value. `Uint<248>` is Compact's maximum
    // width; its range runs past `u128::MAX`.
    (
        "Uint range wider than u128 coerced to Field",
        "import CompactStandardLibrary;\n\
         export ledger f: Field;\n\
         constructor(x: Uint<248>) { f = disclose(x); }\n",
        "field-uint-coercion",
    ),
];

/// Contracts that must still compile — the other half of the property.
///
/// A rejection guard is only worth having if it is narrow. The #45 fix
/// keyed on "is there a constructor statement?", which is true even when
/// the author wrote no constructor (the front end synthesises one), so
/// the first attempt rejected every constructor-less contract in the
/// world. These pin the boundary from the accepting side.
const ACCEPTIONS: &[(&str, &str)] = &[
    ("no constructor at all", "export ledger n: Uint<64>;\n"),
    (
        "explicitly empty constructor",
        "export ledger n: Uint<64>;\nconstructor() { }\n",
    ),
    (
        "constructor with plain writes",
        "export ledger admin: Uint<64>;\n\
         export ledger count: Uint<64>;\n\
         constructor() { admin = 42; count = 7; }\n",
    ),
    (
        // The uniform-width neighbour of the mixed-width ACCEPTION below:
        // the SAME constructor with a uniform-width comparison compiles, so
        // the mixed-width entry pins the width, not "a comparison in a
        // constructor".
        "uniform-width comparison in a constructor",
        "import CompactStandardLibrary;\n\
         export ledger last: Uint<32>;\n\
         constructor(q: Uint<32>, y: Uint<32>) {\n\
           assert(q <= y, \"bounded\");\n\
           last = disclose(y);\n\
         }\n",
    ),
    (
        // Flipped by `type-directed-expression-coercion` (task 4.4): the
        // typer wraps the narrower operand in a coercion wrapper, and the
        // ctor walker now skips the lifted temp's declaration-only `const`
        // and renders its assignment, so the mixed-width comparison lowers
        // with the narrow `y` widened to `((y) as u64)`.
        "mixed-width comparison operand in a constructor",
        "import CompactStandardLibrary;\n\
         export ledger last: Uint<32>;\n\
         constructor(q: Uint<32>, y: Uint<32>) {\n\
           assert(q * 4 <= y, \"product must not exceed the bound\");\n\
           last = disclose(y);\n\
         }\n",
    ),
    (
        // The pure-route half of the same mixed-width gap. Pre-fix compactc
        // exited 0 here but emitted an un-widened `bound` (E0308 at cargo
        // build); the coercion now widens it too.
        "pure-route mixed-width comparison operand",
        "import CompactStandardLibrary;\n\
         export ledger dummy: Uint<64>;\n\
         export pure circuit mixedWidthPure(q: Uint<32>, bound: Uint<32>): [] {\n\
           assert(q * 4 <= bound, \"product must not exceed the bound\");\n\
         }\n",
    ),
    (
        // Task 4.2's boundary, from the accepting side: `Uint<128>` maxes at
        // exactly `u128::MAX`, so `From<u128> for Fr` is lossless and the
        // coercion emits `Fr::from((x) as u128)` rather than refusing.
        "Uint<128> coerced to Field",
        "import CompactStandardLibrary;\n\
         export ledger f: Field;\n\
         constructor(x: Uint<128>) { f = disclose(x); }\n",
    ),
    // ---- Vendored digital-passport oracle: ternary sites -------------
    //
    // The three ternary extracts `vendor-digital-passport-harness` recorded
    // as REJECTIONS, at the contract's real gap sites. Each is a minimal
    // extract in the upstream source
    // (`examples/dogfood/digital-passport-credential/.../helpers.compact`),
    // in source order:
    //   const yearAdjusted = date.month <= 2 ? date.year - 1 : date.year;
    //   assert(isLeap ? date.day <= 29 : date.day <= 28, "...");
    //   ... - (beforeBirthdayThisYear ? 1 : 0);
    // `fix-ternary-expression-codegen` added the `(if ...)` clause to
    // `expr-rust`, so the pure-circuit emitter renders a lazy Rust `if`
    // expression instead of bailing on `expr-variant`; all three are now
    // accepted (task 3.1).
    (
        "ternary in const RHS",
        "export ledger dummy: Uint<64>;\n\
         export pure circuit ternaryConstRhs(c: Uint<32>): Uint<32> {\n\
           const x = c <= 2 ? c - 1 : c;\n\
           return x;\n\
         }\n",
    ),
    (
        "ternary in assert argument",
        "export ledger dummy: Uint<64>;\n\
         export pure circuit ternaryAssertArg(c: Uint<32>, flag: Boolean): [] {\n\
           assert(flag ? c <= 29 : c <= 28, \"bounded\");\n\
         }\n",
    ),
    (
        "ternary as interior arithmetic operand",
        "export ledger dummy: Uint<64>;\n\
         export pure circuit ternaryArithOperand(a: Uint<32>, b: Uint<32>, flag: Boolean): Uint<32> {\n\
           return a - b - (flag ? 1 : 0);\n\
         }\n",
    ),
    // ---- Ternary condition is a call, on the impure / constructor routes ----
    //
    // `expr-supported?` validates the ternary condition against the REAL
    // witness / circuit id tables it receives as arguments, but the emitter's
    // `cond-rust` reads them from the dynamic `current-witness-id-ht` /
    // `current-circuit-id-ht` parameters — which were bound only inside
    // `emit-pure-circuit`. On the impure and constructor routes those stayed
    // at their empty default, so the predicate accepted the body and then
    // `cond-rust` RAISED (`ctor-if-condition-inline`) instead of letting the
    // streaming fallback try. Binding the tables in `emit-impure-circuit` /
    // `emit-initial-state` too (mirroring the pure emitter) fixes it; these
    // two probes pin the condition-call shape on both routes, and the third
    // pins a pure-circuit call in an ARM (resolved through `call-rust`, which
    // reads the same dynamic table).
    (
        "ternary with a pure-circuit-call condition, impure route",
        "import CompactStandardLibrary;\n\
         export ledger f: Field;\n\
         export pure circuit isBig(x: Uint<8>): Boolean { return x > 5; }\n\
         export circuit impureCondCall(): [] {\n\
           f = disclose(isBig(7) ? 1 : 2);\n\
         }\n",
    ),
    (
        "ternary with a pure-circuit-call condition, constructor route",
        "import CompactStandardLibrary;\n\
         export ledger f: Field;\n\
         export pure circuit isBig(x: Uint<8>): Boolean { return x > 5; }\n\
         constructor() { f = disclose(isBig(7) ? 1 : 2); }\n",
    ),
    (
        "ternary with a pure-circuit call in an arm, impure route",
        "import CompactStandardLibrary;\n\
         export ledger f: Field;\n\
         export pure circuit idf(x: Field): Field { return x; }\n\
         export circuit impureArmCall(c: Boolean): [] {\n\
           f = disclose(c ? idf(1) : idf(2));\n\
         }\n",
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
        .args(["--target", "rust", "--skip-zk"])
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

/// Compile a source file **in place** by repo-relative path, writing output
/// to a fresh temp dir. Returns (exit code, stderr+stdout, whether a contract
/// crate was emitted).
///
/// Unlike [`compile`], the probe is NOT relocated to a temp dir. That matters
/// for the vendored dogfood entry, whose body resolves relative `include
/// ../core-compact-staging/...` directives against the source file's directory:
/// copying it elsewhere would break that resolution and report a bogus
/// missing-include error instead of the real codegen refusal. Mirrors
/// `codegen_regression.rs`'s invocation (`compactc --target rust --skip-zk
/// <src> <outdir>`).
fn compile_repo_relative(source_path: &str) -> (Option<i32>, String, bool) {
    let root = find_repo_root(Path::new(env!("CARGO_MANIFEST_DIR")))
        .expect("rejection corpus cannot run: no ancestor holds both examples/ and Cargo.toml");
    let compactc = compiler();

    let src = root.join(source_path);
    assert!(
        src.exists(),
        "compile_repo_relative: source {} is missing",
        src.display()
    );
    let out = std::env::temp_dir().join(format!("compact-repo-relative-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&out);

    let result = Command::new(&compactc)
        .args(["--target", "rust", "--skip-zk"])
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
    let _ = std::fs::remove_dir_all(&out);
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

/// Whole-entry acceptance gate for the vendored digital-passport contract.
///
/// The inline `ACCEPTIONS` cases isolate each former gap site; this gate
/// pins the end-to-end state: compiling the real contract's entry with the
/// fixed rust target exits 0 and emits a crate. It compiles the entry **in
/// place** (see [`compile_repo_relative`]) so the contract's relative core
/// imports resolve; the temp-dir `compile()` cannot.
///
/// History: `vendor-digital-passport-harness` recorded this as an
/// expected-failure gate (`rust_backend_dogfood_entry_is_refused_pre_fix`),
/// asserting a `pure-circuit-body-emission` refusal at
/// `src/digital-passport-credential/helpers.compact:257` — the ternary in
/// `assertCivilDateMatchesEpochDays` bailing before emission.
/// `fix-ternary-expression-codegen` added the `(if ...)` clause to
/// `expr-rust`, which is what makes that ternary compile, so this change
/// flips the gate to acceptance (task 3.3). The pure-route mixed-width
/// comparison further down at `:268` had already stopped blocking when
/// `type-directed-expression-coercion` widened the narrow operand.
///
/// `add-digital-passport-dogfood-fixture` no longer owns a flip here; it
/// registers the crate this gate now proves compiles.
#[test]
fn rust_backend_dogfood_entry_compiles() {
    let (code, text, emitted) = compile_repo_relative(
        "examples/dogfood/digital-passport-credential/src/digital-passport-credential.compact",
    );

    assert_eq!(
        code,
        Some(0),
        "the fixed rust target refused the vendored dogfood entry. Its remaining \
         block was the ternary at helpers.compact:257, now lowered by the \
         `(if ...)` clause; a refusal means that clause regressed.\n--- output ---\n{text}"
    );
    assert!(
        emitted,
        "the dogfood entry compiled but wrote no contract/lib.rs; a successful \
         compile must leave the crate for a build to pick up."
    );
    assert!(
        !text.contains("pure-circuit-body-emission"),
        "the diagnostic still names `pure-circuit-body-emission`; the entry no \
         longer refuses on a ternary, so this kind should be gone.\n--- output ---\n{text}"
    );
}
