# Proposal: fix-ternary-expression-codegen

## Why

`compactc --target rust` rejects conditional (ternary) expressions wherever they appear as a sub-expression rather than in tail/return position. Real third-party code hits this today: the `midnight-verifiable-credential-digital-passport` contract fails with `pure-circuit-body-emission: no walker shape matched pure circuit body` (5 sites, 3 syntactic positions), and the same construct fails in impure circuits (`circuit-body-emission`) and constructors (a location-less `expr-variant` error). The TS backend already compiles all of these, so the rust backend is the divergent one; `doc/rust-codegen-user-guide.md:324` already (incorrectly) claims ternary is supported.

## What Changes

- Add the missing `(if c e1 e2)` clause to `expr-rust` (`compiler/rust-passes-emit.ss`), the shared rust expression renderer — fixing ternaries in const-RHS, assert-argument, interior-operand, and constructor positions in one clause, mirroring the TS clause at `typescript-passes.ss:2850`.
- Add the corresponding recursive arm to `expr-supported?` (`compiler/rust-passes-walker.ss`) so impure/streaming bodies admit ternaries (pure bodies need no walker change).
- Emission must be **lazy** per the language spec (`compiler/compact-reference-proto.mdx:2114`: "only one of e₁ and e₂ is evaluated"): a Rust `if` expression, never eager both-branches evaluation; branch-local underflow guards (`seq` blocks) must render inside the taken branch only.
- New neutral fixture `examples/ternary_cond_fixture.compact` + generated crate `tests-e2e-rust/contracts/ternary-cond-fixture/`, registered per the full AGENT.md §5.1 recipe (workspace member, dev-dep, `codegen_regression.rs` FIXTURES row), with an executing test that pins lazy evaluation (the `c = 9` / `c > 5 ? c - 10 : c` case must not trip the underflow assert) and covers pure + impure + constructor positions, nested/struct/enum branch values, and return-position regression.
- Version bump to 0.31.117 with full embed-site sweep (`compiler/compiler-version.ss`, `flake.nix`, `doc/ledger-adt.mdx` via `./compiler/go`, grep for old triple) + CHANGELOG entry under `### Fixed`.
- Docs sweep: re-run the live-count recipe in `docs/rust-backend-limitations.md`; `doc/rust-codegen-user-guide.md`'s ternary row becomes true (no text change expected).

Not breaking: no frontend/typechecker/TS-backend change, no language-version or runtime change (verified against precedent commit `8018e02`, the structurally identical G1 fix).

## Capabilities

### New Capabilities

- `rust-codegen/conditional-expressions`: the rust backend emits Compact conditional (ternary) expressions with lazy branch evaluation, in all body routes (pure circuit, impure circuit, constructor) and all sub-expression positions.

### Modified Capabilities

(none — `openspec/specs/` is empty; this change bootstraps the capability tree)

## Impact

- `compiler/rust-passes-emit.ss` (new `expr-rust` clause), `compiler/rust-passes-walker.ss` (new `expr-supported?` arm).
- New example + fixture crate + test files; `root Cargo.toml`, `tests-e2e-rust/Cargo.toml`, `Cargo.lock`, `codegen_regression.rs` registrations.
- `compiler/compiler-version.ss` 0.31.116 → 0.31.117, `flake.nix`, `doc/ledger-adt.mdx`, `CHANGELOG.md`.
- Downstream enabler for `add-digital-passport-dogfood-fixture` (its crate cannot be generated until this lands).
