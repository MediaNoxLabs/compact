# Tasks: fix-ternary-expression-codegen

Prerequisite: `type-directed-expression-coercion` merged. All work inside `nix develop`; `nix build .#compactc` before any byte-parity test.

## 1. Compiler fix (one clause, no per-position logic)

- [ ] 1.1 Add the `(if ,src ,c ,e1 ,e2)` clause to `expr-rust` in `compiler/rust-passes-emit.ss`, mirroring the TS clause at `typescript-passes.ss:2850`: render `if <cond> { <arm1> } else { <arm2> }` with each arm via `expr-rust-typed` at the expected type supplied by the use position; branch `seq` guard blocks render via the existing `seq` clause **inside the arm**. Verify: a minimal `const x = c <= 2 ? c - 1 : c;` pure circuit compiles under `result/bin/compactc --target rust --skip-zk`.
- [ ] 1.2 Add the recursive `(if ...)` arm to `expr-supported?` in `compiler/rust-passes-walker.ss` (before `[else #f]`). Verify: a ternary in a ledger-writing circuit and in a constructor both compile.
- [ ] 1.3 Confirm **no** ternary-specific coercion was added: `git diff` touches only the clause and the gate arm (no literal/width logic). Verify: grep the diff for coercion helpers returns nothing new.
- [ ] 1.4 Confirm laziness in the emitted bytes: the underflow guard for a branch subtraction appears **inside** the arm, never hoisted. Verify: read the generated Rust for 1.1.

## 2. Coverage matrix (every reachable cell has a probe)

- [ ] 2.1 Author probes for **routes × positions**: {pure, impure-walker, impure-streaming, constructor} × {const RHS annotated, const RHS unannotated/seq-lifted, return tail (pure stmt-lifted / I3b-4 / A19 chain), assert arg, arith operand, comparison operand, call arg (pure/witness/ctor), struct member, vector element, native arg, nested if, ledger cell write, inline write value}. Verify: a table in the PR/notes shows each cell checked or explicitly refused (with a `docs/rust-backend-limitations.md` link).
- [ ] 2.2 Author probes for **value shapes** in each applicable position: {both-literal, mixed literal/expr, Uint arm into Field, differing Uint widths, Field-typed arms, struct-valued (non-Copy), enum-valued, literal > `i32::MAX`, literal > `u64::MAX` (Uint<128>), branch-local underflow guard}. Verify: each shape has an executing assertion.
- [ ] 2.3 Author `examples/ternary_cond_fixture.compact` (Apache header) collecting the probes; verify `compactc --target ts --skip-zk` compiles it.
- [ ] 2.4 Generate the crate; register it (root workspace member, `tests-e2e-rust` **dev-dependency**, `FIXTURES` row). Verify: `cargo build -p tests-e2e-rust --tests --locked` compiles it.
- [ ] 2.5 Write `tests-e2e-rust/tests/ternary_cond_fixture.rs` with an executing assertion per probe: laziness both ways (`pick(9)` → `Ok(9)` no trap; `pick(6)` → computes), each circuit's pass and `Err(AssertionFailed)` sides, and the constructor/impure variants. Verify: `cargo test -p tests-e2e-rust ternary_cond` green.
- [ ] 2.6 Add TS reference captures (`fixtures/capture-ternary-cond-fixture.mjs` → JSON) and a **serialized state-byte** parity assertion for every ledger-write cell. Verify: parity assertions pass and would fail if the destination width were wrong.

## 3. Flip the oracle probes

- [ ] 3.1 Convert the ternary-site probes added by `vendor-digital-passport-harness` from REJECTION to ACCEPTION (they must now compile). Verify: `rejection_corpus` green with the fixed compiler; the probes assert successful emission.
- [ ] 3.2 Confirm the mixed-width oracle probe was already flipped by `type-directed-expression-coercion` and is not left as a stale REJECTION. Verify: `grep` finds no mixed-width REJECTION entry.

## 4. Regression sweep, traces, version

- [ ] 4.1 Regenerate **every** fixture and classify each diff per AGENT.md §5.2; STOP on unexplained diffs. Verify: `codegen_regression` green.
- [ ] 4.2 Traceability: map every `#### Scenario:` in the delta spec to a named test/command. Verify: no scenario without a test, no test without a scenario.
- [ ] 4.3 Mutation check: temporarily revert the `if` clause (or the `expr-supported?` arm) and confirm a specific matrix test goes red; restore. Verify: recorded evidence.
- [ ] 4.4 Local gates: `cargo fmt --all --check`, touched-crate clippy, full `cargo test -p midnight-compact-runtime -p tests-e2e-rust`. Verify: all green.
- [ ] 4.5 Bump 0.31.117 → 0.31.118 (`compiler-version.ss`, `flake.nix`, regenerate `doc/ledger-adt.mdx`, grep old triple) + CHANGELOG `### Fixed` entry. Verify: zero stale embeds; `changelog-check` satisfied.
