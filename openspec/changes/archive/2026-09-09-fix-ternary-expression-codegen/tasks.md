# Tasks: fix-ternary-expression-codegen

All compiler/test work happens inside `nix develop` (AGENT.md §1 rule 2). Byte-parity is local-only — never skip AGENT.md §2.1 step 3 (`nix build .#compactc`) before running tests.

## 1. Compiler fix

- [x] 1.1 Add the `(if ,src ,c ,e1 ,e2)` clause to `expr-rust` in `compiler/rust-passes-emit.ss`, mirroring the TS clause at `typescript-passes.ss:2850`: render `if <cond> { <e1> } else { <e2> }`, recursing through the existing condition routing; branch `seq` guard blocks render via the existing `seq` clause. Verify: `nix build .#compactc` succeeds and a minimal `const x = c <= 2 ? c - 1 : c;` pure circuit compiles under `result/bin/compactc --target rust --skip-zk`.
- [x] 1.2 Add the recursive `(if ...)` arm to `expr-supported?` in `compiler/rust-passes-walker.ss` (before the `[else #f]`). Verify: a ternary in a **ledger-writing circuit** and in a **constructor** both compile (previously `circuit-body-emission` / `expr-variant`).
- [x] 1.3 Confirm laziness in the emitted bytes by inspection: the underflow guard (`compact_assert!(...)`) for a branch subtraction appears **inside** the conditional's branch arm, never hoisted. Verify: read the generated Rust for the 1.1 minimal case.

## 2. Neutral fixture (full §5.1 recipe)

- [x] 2.1 Author `examples/ternary_cond_fixture.compact` (with Apache header) covering: const-RHS ternary with guarded `-` inside the conditional's then-branch (`pick(c) { return c > 15 ? c - 10 : c; }` — untaken for `c = 9`, taken for `c = 20`), ternary in an assert argument, ternary as arithmetic operand, boolean `&&`/`||` branches, nested ternary, struct- and enum-valued branches, return-position ternary (regression), the same const ternary in an impure circuit and in a constructor. Verify: file parses and `compactc --target ts --skip-zk` compiles it.
- [x] 2.2 Generate the crate: `result/bin/compactc --target rust --skip-zk examples/ternary_cond_fixture.compact tests-e2e-rust/contracts/ternary-cond-fixture/`. Verify: `lib.rs` emitted, rustfmt-clean, zero `unimplemented!`.
- [x] 2.3 Register: root `Cargo.toml` workspace member, `tests-e2e-rust/Cargo.toml` dev-dep, `codegen_regression.rs` FIXTURES row `("ternary_cond_fixture.compact", "ternary-cond-fixture")`. Verify: `cargo build -p tests-e2e-rust --tests --locked` passes.
- [x] 2.4 Write `tests-e2e-rust/tests/ternary_cond_fixture.rs` executing-test modeled on `guarded_assert_arith_fixture.rs`: laziness oracle (invoke `pick(9)` → `Ok(9)`, no underflow assert; `pick(20)` → `Ok(10)`) plus each circuit's pass and `Err(AssertionFailed)` sides; constructor/impure variants invoked too. Verify: `cargo test -p tests-e2e-rust ternary_cond` green.
- [x] 2.5 TS reference capture per recipe (`fixtures/capture-ternary-cond-fixture.mjs` → JSON) and byte-parity assertions in the executing test, mirroring the closest existing capture fixture. Verify: capture script runs and parity assertions pass.

## 3. Regression sweep & gates

- [x] 3.1 Regen **every** existing fixture (`codegen_regression`) and inspect `git diff tests-e2e-rust/contracts/` — any unrelated diff is an unintended consequence: STOP per AGENT.md §5.2. Verify: byte-parity green across the whole FIXTURES table, zero unexpected diffs.
- [x] 3.2 Local gates: `nix develop --command cargo fmt --all --check`, clippy for touched crates, full `cargo test -p midnight-compact-runtime -p tests-e2e-rust` (AGENT.md §2.1). Verify: all green locally before any push.

## 4. Version, docs, changelog

- [x] 4.1 Bump `compiler/compiler-version.ss` to 0.31.117, `flake.nix` `packages.compactc.version`, regenerate `doc/ledger-adt.mdx` via `./compiler/go`, and grep-sweep the old triple (`compiler/ flake.nix doc/ledger-adt.mdx`). Verify: zero stale `0.31.116` embeds.
- [x] 4.2 CHANGELOG.md: new `## [Toolchain 0.31.117, language 0.23.103, runtime 0.16.100]` heading under `### Fixed`, describing the ternary codegen gap (5 sites / 3 positions / 3 body routes) and the new fixture. Verify: `changelog-check` preconditions (CHANGELOG.md + compiler-version.ss both in diff).
- [x] 4.3 Docs sweep: re-run the live-count recipe in `docs/rust-backend-limitations.md` and update counts; confirm `doc/rust-codegen-user-guide.md` ternary row is now accurate. Verify: counts match a fresh grep.
- [x] 4.4 Commit signed+DCO (`git commit -S -s`), compiler change + regenerated fixtures in the same logical commit set, on branch `feature/fix-ternary-expression-codegen` (cut from `codegen-rust`, merges into `digital-passport-patch`). Verify: `git log --show-signature` clean.

## 5. Dogfood cross-check (evidence, not implementation)

- [x] 5.1 Post-merge evidence: once `add-digital-passport-dogfood-fixture`'s vendoring task has landed its `examples/dogfood/` sources (if needed earlier, re-vendor them with the no-pnpm `curl`-based procedure recorded in that change's design — never a `/tmp` probe copy), compile those sources with the fixed compiler: `--target rust --skip-zk` and `--target ts --skip-zk` both exit 0. Verify: 3,509±lines `lib.rs`, zero `unimplemented!` — confirming this change is sufficient for change 2. Nothing in tasks 1–4 depends on this task.
