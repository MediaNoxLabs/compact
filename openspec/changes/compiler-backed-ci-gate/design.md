## Context

`codegen_regression` regenerates every `FIXTURES` row with a real `compactc` and byte-compares the emitted `lib.rs` against the committed crate; `rejection_corpus` compiles probe contracts and asserts the expected refusal diagnostics. Both hard-fail without a compiler and neither has a skip path (`codegen_regression.rs:135-142`), which is correct — but it means CI must supply a compiler or omit the tests, and today it omits them.

Evidence for the current state:
- `rust-runtime-test.yml:178` runs `cargo test -p tests-e2e-rust --locked -- --skip rust_codegen_byte_parity --skip rust_backend_`. Header comment lines 12-21 state the reason.
- `build-compiler.yml` builds `compactc` via Nix but its `.#compiler` devshell has no cargo (`flake.nix:606`, `:254`); `rust-runtime-test.yml` has a Rust toolchain but no Nix.
- `tests-e2e-rust/Cargo.toml:82-90` documents the dev-dependency requirement; `compact-contract-multi-pl-call-fixture` is a workspace member and a `FIXTURES` row but has no dev-dep entry.

## Goals / Non-Goals

**Goals:**
- The byte-parity gate and the rejection corpus run in CI on every PR, with no vacuous-skip path.
- The registration invariants that caused a fixture to be invisible to CI become machine-enforced.
- Compiling the emitted crate is a first-class gate, not a side effect of a dev-dep.

**Non-Goals:**
- No `workflow_run` artifact sharing yet (deferred optimisation; note it in the workflow comment).
- No change to what the compiler emits; this change is infra-only.
- No attempt to make CI the first line of defence (AGENT.md §2.1 still requires the local run before pushing).

## Decisions

1. **Self-contained job in `rust-runtime-test.yml` rather than artifact sharing.** The runner already installs a Rust toolchain; adding the Nix cache action + `nix build .#compactc` is contained and reviewable. Artifact sharing saves ~11 min/PR but adds cross-workflow coupling and failure modes. Rejected for now; recorded as a follow-up in #23.
2. **Invariants as executable tests, not documentation.** A `cargo test` (or a small test in `codegen_regression.rs`) that parses the `FIXTURES` table and asserts each row's contract crate appears in `tests-e2e-rust/Cargo.toml` `[dev-dependencies]`. Prose in AGENT.md was already present and did not prevent the hole.
3. **`cargo check` every regenerated crate.** After `codegen_regression` regenerates into a scratch dir, type-check each emitted crate. This catches "compiles-clean-but-emits-E0308" shapes even when no dev-dep exists, and is independent of the byte comparison.
4. **Make `rustfmt` absence loud.** `compiler/passes.ss`'s post-pass currently soft-fails when `rustfmt` is absent, so regeneration produces unformatted bytes that differ from committed formatted bytes → false drift. Either fail the emitter or have the gate run `rustfmt --check` and classify drift as format-vs-semantic.

## Risks / Trade-offs

- [Added CI wall-clock (~11 min/PR for the compiler build)] → acceptable for a gate that prevents multi-round review loops; the artifact-sharing follow-up removes it later.
- [Nix-in-CI flakiness] → reuse the existing `setup-nix-cache` action used by `build-compiler.yml`.
- [`cargo check` on every regen multiplies build cost] → scope it to the fixtures with no dev-dep plus a rotation, or cache the workspace; measure before merging.

## Migration Plan

Add the job and invariants; delete the two `--skip` flags. Rollback = restore the flags (the gate reverts to local-only, its current state). No data migration.
