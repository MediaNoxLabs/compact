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
// Oracle probes (expected refusals)
//
// The vendored digital-passport contract is the oracle for the rust
// backend's real-idiom gaps. `vendor-digital-passport-harness` records
// each current gap as an executable expectation:
//
//   * `REJECTIONS` gains a minimal extract per real ternary site (const
//     RHS, assert argument, interior arithmetic operand), plus the
//     mixed-width comparison operand (`q * 4` vs a `Uint<32>` value) that
//     the constructor route cannot lower. Each entry NAMES the fix change
//     that flips it.
//   * `EMITS_UNCOMPILABLE` records the *other* half of that same mixed-width
//     gap: on the **pure** route the shape does not refuse — compactc exits
//     0 — but the emitted `lib.rs` compares a widened (u64) value against an
//     un-widened `Uint<32>` (E0308 at `cargo build`). It is not a REJECTION
//     (the compile succeeds) and not an ACCEPTION (the output is ill-typed),
//     so it gets its own structural gate keyed on the un-widened comparison.
//     This is the site the whole-entry gate masks: the ternary refuses first,
//     so the pure mixed-width site is only reachable once the ternary fix has
//     landed.
//   * `rust_backend_dogfood_entry_is_refused_pre_fix` compiles the whole
//     vendored entry in place (by repo-relative path — see
//     `compile_repo_relative`) and asserts the end-to-end refusal with the
//     kind task 2.2 recorded.
//
// The whole-entry gate is expected-fail by design: it flips to
// "exits 0 + contract/lib.rs emitted" in `add-digital-passport-dogfood-fixture`,
// once the ternary fix lands. That flip alone is NOT sufficient: the entry's
// pure-route mixed-width comparison (`helpers.compact:268`) still emits
// ill-typed Rust until `type-directed-expression-coercion` lands, which is why
// that shape is gated independently by `EMITS_UNCOMPILABLE`.

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
    // ---- Vendored digital-passport oracle ----------------------------
    //
    // Minimal extracts at the contract's real gap sites. The upstream
    // source (`examples/dogfood/digital-passport-credential/.../helpers.compact`)
    // has a ternary in each of three syntactic positions and a mixed-width
    // comparison operand; each extract below is the smallest form that
    // still refuses, and each names the fix change that flips it.
    //
    // Ternary extracts — flipped by `fix-ternary-expression-codegen`
    // (task 3.1 converts these from refusal to acceptance). Upstream sites:
    //   const yearAdjusted = date.month <= 2 ? date.year - 1 : date.year;
    //   assert(isLeap ? date.day <= 29 : date.day <= 28, "...");
    //   ... - (beforeBirthdayThisYear ? 1 : 0);
    // `expr-rust` has no `(if ...)` clause, so the pure-circuit emitter
    // bails; its catch-all guard swallows the precise diagnostic and reports
    // the generic body error (task 2.2's recorded failure), which is why the
    // expected kind is `pure-circuit-body-emission`, not ternary-specific.
    (
        "ternary in const RHS [flips in fix-ternary-expression-codegen]",
        "export ledger dummy: Uint<64>;\n\
         export pure circuit ternaryConstRhs(c: Uint<32>): Uint<32> {\n\
           const x = c <= 2 ? c - 1 : c;\n\
           return x;\n\
         }\n",
        "pure-circuit-body-emission",
    ),
    (
        "ternary in assert argument [flips in fix-ternary-expression-codegen]",
        "export ledger dummy: Uint<64>;\n\
         export pure circuit ternaryAssertArg(c: Uint<32>, flag: Boolean): [] {\n\
           assert(flag ? c <= 29 : c <= 28, \"bounded\");\n\
         }\n",
        "pure-circuit-body-emission",
    ),
    (
        "ternary as interior arithmetic operand [flips in fix-ternary-expression-codegen]",
        "export ledger dummy: Uint<64>;\n\
         export pure circuit ternaryArithOperand(a: Uint<32>, b: Uint<32>, flag: Boolean): Uint<32> {\n\
           return a - b - (flag ? 1 : 0);\n\
         }\n",
        "pure-circuit-body-emission",
    ),
    // Mixed-width comparison operand — flipped by
    // `type-directed-expression-coercion` (task 4.4 owns the flip). `q * 4`
    // on `q: Uint<32>` range-types wider than `y`, so the typer wraps the
    // narrower operand in a coercion wrapper; on the constructor route that
    // lift is a construct the ctor walker cannot lower, so the body refuses.
    // The uniform-width neighbour (`assert(q <= y, ...)`) in the SAME
    // constructor shape ACCEPTS — pinned in ACCEPTIONS below — so this entry
    // keys on the mixed width, not merely on a comparison in a constructor.
    // The SAME comparison on the **pure** route does not refuse — it exits 0
    // and emits ill-typed Rust — so it is pinned separately, by shape, in
    // `EMITS_UNCOMPILABLE` below.
    (
        "mixed-width comparison operand [flips in type-directed-expression-coercion]",
        "import CompactStandardLibrary;\n\
         export ledger last: Uint<32>;\n\
         constructor(q: Uint<32>, y: Uint<32>) {\n\
           assert(q * 4 <= y, \"product must not exceed the bound\");\n\
           last = disclose(y);\n\
         }\n",
        "ctor-body-emission",
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
        // The mixed-width comparison REJECTION's neighbour, from the
        // accepting side: the SAME constructor with a uniform-width
        // comparison compiles. Pins that the oracle entry keys on the
        // mixed width, not on "a comparison in a constructor".
        "uniform-width comparison in a constructor",
        "import CompactStandardLibrary;\n\
         export ledger last: Uint<32>;\n\
         constructor(q: Uint<32>, y: Uint<32>) {\n\
           assert(q <= y, \"bounded\");\n\
           last = disclose(y);\n\
         }\n",
    ),
];

/// Shapes the backend currently *accepts* (compactc exits 0) but emits
/// **ill-typed** Rust for — the second failure mode of the central safety
/// claim ("a construct the emitter cannot lower must FAIL the compile, never
/// emit something plausible", see the file header). These are neither
/// [`REJECTIONS`] (nothing refuses) nor [`ACCEPTIONS`] (the emitted crate does
/// not type-check), so they get their own gate.
///
/// Entry: `(case name, Compact source, the un-widened comparison the current
/// emission MUST contain)`. Each NAMES the change that must flip it.
///
/// The gate is deliberately structural: the generated crate depends on the
/// published `compact-runtime` and cannot be built inside this test, so the
/// defect is pinned at the source level. The executing fixture that compiles
/// the shape for real lands with `type-directed-expression-coercion` (task
/// 5.5), which is also the change that owns the flip — at which point this
/// entry is converted, not left to pass silently.
const EMITS_UNCOMPILABLE: &[(&str, &str, &str)] = &[(
    // Mirrors the vendored contract's `helpers.compact:268`
    // (`date.yearAdjustedQuotient4 * 4 <= yearAdjusted`), the same
    // comparison the constructor REJECTION above keys on — but here on
    // the pure route, where the typer's coercion wrapper is not lowered
    // and compactc still exits 0.
    "pure-route mixed-width comparison operand [flips in type-directed-expression-coercion]",
    "import CompactStandardLibrary;\n\
         export ledger dummy: Uint<64>;\n\
         export pure circuit mixedWidthPure(q: Uint<32>, bound: Uint<32>): [] {\n\
           assert(q * 4 <= bound, \"product must not exceed the bound\");\n\
         }\n",
    // `q * 4` is emitted wrapped in `((q) as u64)` (the `*` widens), but
    // the narrow `bound: Uint<32>` is compared against it with no
    // coercion, so the emitted crate is `E0308` (mismatched types) at
    // `cargo build` even though compactc exited 0. Pinned with the closing
    // paren so ANY widening of `bound` (`(bound) as u64`, `bound as u64`)
    // stops matching and flips the gate — not just the exact fixed form.
    "<= bound)",
)];

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

/// Like [`compile`], but also returns the emitted `contract/lib.rs` text so a
/// gate can inspect the *shape* of the output, not merely its presence.
/// Returns `(exit code, stderr+stdout, emitted lib.rs or None)`.
fn compile_emitting(
    compactc: &Path,
    case: &str,
    source: &str,
) -> (Option<i32>, String, Option<String>) {
    let dir = std::env::temp_dir().join(format!(
        "compact-emission-{}-{}",
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
    let lib = std::fs::read_to_string(out.join("contract/lib.rs")).ok();
    (result.status.code(), text, lib)
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

/// Gate for shapes the backend accepts but emits **ill-typed** Rust for — the
/// silent failure mode [`REJECTIONS`] cannot reach (the compile exits 0) and
/// [`ACCEPTIONS`] must not claim (the output does not build). Each entry pins
/// the un-widened comparison the current emitter produces; when the narrow
/// operand is widened, or the shape starts refusing, this fails and forces the
/// fix change to convert the entry instead of leaving a silently-green hole.
///
/// Unlike the whole-entry gate below, this isolates the **pure**-route
/// mixed-width site (`helpers.compact:268` in the vendored contract), which the
/// whole-entry gate masks behind the earlier ternary refusal.
#[test]
fn rust_backend_pure_route_mixed_width_emits_ill_typed_rust_pre_fix() {
    let compactc = compiler();

    for (case, source, un_widened) in EMITS_UNCOMPILABLE {
        let (code, text, lib) = compile_emitting(&compactc, case, source);

        assert_eq!(
            code,
            Some(0),
            "{case}: expected the shape to compile today (the gap is a bad \
             emission, not a refusal). A refusal means the backend now rejects \
             it — move the case to REJECTIONS and record the new kind.\n--- output ---\n{text}"
        );
        let lib = lib.unwrap_or_else(|| {
            panic!(
                "{case}: compactc exited 0 but emitted no contract/lib.rs.\n--- output ---\n{text}"
            )
        });
        assert!(
            lib.contains(un_widened),
            "{case}: the emitted comparison is no longer the un-widened \
             `{un_widened}`. Either the narrow operand is now widened (the fix \
             landed — flip this entry, e.g. into ACCEPTIONS, or delete it once the \
             type-directed mixed-width fixture compiles the shape), or the emission \
             was refactored and the defect must be re-derived before deleting the \
             gate. Do NOT let it pass silently: an ill-typed emission from exit 0 is \
             the exact failure this corpus exists to catch.\n\
             --- contract/lib.rs ---\n{lib}"
        );
    }
}

/// Whole-entry expected-failure gate for the vendored digital-passport
/// contract (task 4.3).
///
/// The inline `REJECTIONS` cases isolate each gap site; this gate pins the
/// end-to-end state: compiling the real contract's entry with the pre-fix
/// rust target refuses, emits no crate, and reports the kind task 2.2
/// recorded (`pure-circuit-body-emission` at
/// `src/digital-passport-credential/helpers.compact:257`). It compiles the
/// entry **in place** (see [`compile_repo_relative`]) so the contract's
/// relative core imports resolve; the temp-dir `compile()` cannot.
///
/// Attribution: this refusal is the *ternary* at `helpers.compact:257`
/// (`assertCivilDateMatchesEpochDays` bails before emission), NOT the
/// mixed-width comparison further down at `:268`. That later site is masked
/// while the ternary refuses first; it is a pure-route shape that exits 0 and
/// emits ill-typed Rust, gated independently by
/// [`rust_backend_pure_route_mixed_width_emits_ill_typed_rust_pre_fix`].
///
/// Expected-fail by design: it flips to "exits 0 + `contract/lib.rs`
/// emitted" in `add-digital-passport-dogfood-fixture`, which registers the
/// generated crate. `fix-ternary-expression-codegen` owns that flip, but it is
/// NOT sufficient alone — the emitted crate still fails to build until
/// `type-directed-expression-coercion` widens the narrow mixed-width operand
/// (see [`EMITS_UNCOMPILABLE`]). Until both land, this gate must stay green by
/// asserting the refusal.
#[test]
fn rust_backend_dogfood_entry_is_refused_pre_fix() {
    let (code, text, emitted) = compile_repo_relative(
        "examples/dogfood/digital-passport-credential/src/digital-passport-credential.compact",
    );

    assert_ne!(
        code,
        Some(0),
        "the vendored dogfood entry compiled. The pre-fix rust target refuses it \
         (the ternary at helpers.compact:257; the mixed-width site at :268 is gated \
         separately by rust_backend_pure_route_mixed_width_emits_ill_typed_rust_pre_fix). \
         A successful exit means this gate has flipped to acceptance — update it in \
         `add-digital-passport-dogfood-fixture` rather than letting it pass silently.\n--- output ---\n{text}"
    );
    assert!(
        !emitted,
        "the refused dogfood entry still wrote contract/lib.rs; a refused compile \
         must leave no output behind for a build to pick up."
    );
    assert!(
        text.contains("pure-circuit-body-emission"),
        "expected the diagnostic to name `pure-circuit-body-emission` (the kind \
         recorded by task 2.2), so the refusal is greppable and attributable.\n--- output ---\n{text}"
    );
}
