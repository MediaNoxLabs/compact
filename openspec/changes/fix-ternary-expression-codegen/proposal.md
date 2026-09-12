## Why

`compactc --target rust` rejects conditional (ternary) expressions wherever they appear as a sub-expression rather than in tail/return position. Real third-party code hits this: the digital-passport contract fails at 5 sites in 3 syntactic positions, and the TS backend compiles all of them, so the Rust backend is the divergent one. PR #70 attempted this fix as "one clause" plus per-position patches and grew to ~10 compiler review-fix commits, because each new shape (both-literal arms, mixed arms, Uint-into-Field arms, struct arms, streaming route, ledger width) had to be patched at every use site. With `type-directed-expression-coercion` landed, the ternary fix becomes what it should have been: **one `(if …)` clause that renders each arm through the typed entry**, so type/width correctness is inherited rather than re-specified per position.

## What Changes

- Add the `(if c e1 e2)` clause to `expr-rust`, mirroring the TS clause (`typescript-passes.ss:2850`): render a Rust `if` expression, with each arm rendered by `expr-rust-typed` against the expected type supplied by the use position; `ctor-expr-rust` and the streaming renderers inherit it. No per-position coercion logic.
- Add the recursive `(if …)` arm to `expr-supported?` so impure/streaming bodies admit ternaries.
- Preserve laziness exactly: only the selected branch is evaluated; branch-local `seq` underflow guards render inside the taken arm.
- New neutral fixture `examples/ternary_cond_fixture.compact` + crate registered per the full recipe (workspace member, **`tests-e2e-rust` dev-dependency**, `FIXTURES` row), with executing tests and TS reference captures, plus an explicit `route × position × value-shape` coverage matrix (below) where every reachable cell has a probe and every unsupported cell is a documented refusal — never a blank.
- Ledger-write cells additionally get a serialized **state-byte** parity check against the TS reference (decoded-value checks cannot see alignment divergence).
- Flip the ternary oracle probes from `vendor-digital-passport-harness` from REJECTION to ACCEPTION.
- Version 0.31.118 with the full embed-site sweep + CHANGELOG entry.

Not breaking: no frontend/typechecker/TS-backend/language/runtime change.

## Capabilities

### New Capabilities

- `rust-codegen/conditional-expressions`: the Rust backend emits Compact conditional (ternary) expressions with lazy branch evaluation, in all body routes and all sub-expression positions, with type/width correctness inherited from `rust-codegen/type-directed-coercion`.

### Modified Capabilities

(none)

## Impact

- `compiler/rust-passes-emit.ss` (one `expr-rust` clause), `compiler/rust-passes-walker.ss` (`expr-supported?` arm).
- New example + fixture crate + test files; `root Cargo.toml`, `tests-e2e-rust/Cargo.toml`, `Cargo.lock`, `codegen_regression.rs`.
- `compiler/compiler-version.ss` 0.31.117 → 0.31.118, `flake.nix`, `doc/ledger-adt.mdx`, `CHANGELOG.md`.
- Depends on `type-directed-expression-coercion` and `vendor-digital-passport-harness` (whose probes this flips). Enables the whole-contract dogfood compile (together with `fix-mixed-width-operand`).
