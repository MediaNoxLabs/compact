## Context

`vendor-digital-passport-harness` landed the compact-only vendored tree, the TS reference captures, and the expected-refusal oracle. The fix changes now let the rust target compile the contract. This change converts the harness into a first-class fixture: it is registered like any other crate and gated in CI.

Precedent: `09263a9` (did-05) vendored `.compact` source under `examples/` with relative imports and full registration; the hand-imported `digital-passport` crate (`e1e963f`) had no registration and was removed. `tests-e2e-rust/Cargo.toml:82-90` documents why a `FIXTURES` row must also be a dev-dependency: workspace membership alone does not make `cargo build -p tests-e2e-rust --tests` compile the crate, and `codegen_regression` is byte-compare only.

## Goals / Non-Goals

**Goals:**
- The dogfood is gated exactly like a first-class fixture: fmt/clippy/compile/integration in CI, byte-parity per AGENT.md.
- The parity test proves behaviour, not just compilation.
- The de-branding decision stays intact outside the enclave; the supersession is explicit and bounded.

**Non-Goals:**
- No porting of upstream's vitest suite, `smoke-consumer`, release tooling, or npm packaging.
- No vendoring of TypeScript or any file beyond the compact-only subset (`vendor-digital-passport-harness` owns the vendored set).
- No scheduled upstream-drift detection; refresh stays an explicit human act.

## Decisions

1. **Crate name `compact-contract-digital-passport-credential`, dir `digital-passport-credential`** — distinct from the removed `digital-passport` crate.
2. **Register as dev-dependency, not just workspace member.** This is the gate that `compiler-backed-ci-gate` makes mechanical; without it the crate would be fmt-only.
3. **Reuse the change-2 captures verbatim; do not re-author.** The Rust parity test consumes the committed JSON; the captured behaviour is the oracle, so changing it in this change would invalidate the oracle.
4. **CI additions minimal**: one crate clippy step and one dual-target smoke pair. The crate's emitted `#![allow(clippy::all, …)]` makes a naive clippy run vacuous, so the step first strips that line from *its own checkout* (guarded) and runs at **default lint levels** — `-- -D warnings` is infeasible (~170 deliberate warn-level findings) and cannot override a source-level group-allow. No cargo on the smoke lane (no Rust toolchain there).
5. **The dogfood gate's own finding is fixed here, not deferred.** The re-armed clippy gate flagged the zero-field `FromFieldRepr` guard (`_repr.len() < Self::FIELD_SIZE`, always false when `FIELD_SIZE = 0`) as clippy's deny-by-default `absurd_extreme_comparisons`; the emitter now emits the guard only for structs with ≥1 field (`rust-passes-decls.ss`). That is a codegen change, so the release carries a `compiler-version.ss` bump (0.31.118 → 0.31.119, with `flake.nix`/`doc/ledger-adt.mdx`) + CHANGELOG; `skip-changelog` no longer applies.
6. **Only the dogfood fixture changes.** It is the sole fixture with zero-field structs, so its `lib.rs` loses exactly three guard lines and every other fixture regenerates byte-identically.

## Risks / Trade-offs

- [Generated crate size (~3.5k–6k lines) slows CI] → one more workspace member among many; dev-dep compile only.
- [Decoded-value tests hide byte-level divergence] → this contract is stateless (no ledger, no constructor), so there is no serialized ledger state to compare; the byte-level check therefore targets the round-trip's 32-byte body roots, which the capture records as hex `result` values, compared byte-for-byte.
- [Someone "cleans up" the enclave as accidental branding] → the ADR and AGENT.md §1 record the decision and its bounds explicitly.

## Migration Plan

Cut `feature/add-digital-passport-dogfood-fixture` after the four prerequisite changes merge. Rollback = revert commit; the vendored tree and captures remain in the repo.
