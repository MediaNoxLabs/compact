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
4. **CI additions minimal**: one clippy `-p` step and one dual-target smoke pair. No cargo on the smoke lane (no Rust toolchain there).
5. **Fixture-only release.** Request `skip-changelog`; no `compiler-version.ss` bump.

## Risks / Trade-offs

- [Generated crate size (~3.5k–6k lines) slows CI] → one more workspace member among many; dev-dep compile only.
- [Decoded-value tests hide state-byte divergence] → the parity test compares serialized bytes where the capture records state; this is why the change-2 captures include state.
- [Someone "cleans up" the enclave as accidental branding] → the ADR and AGENT.md §1 record the decision and its bounds explicitly.

## Migration Plan

Cut `feature/add-digital-passport-dogfood-fixture` after the four prerequisite changes merge. Rollback = revert commit; the vendored tree and captures remain in the repo.
