## Context

`codegen_regression` regenerates every `FIXTURES` row with a real `compactc` and byte-compares the emitted `lib.rs` against the committed crate; `rejection_corpus` compiles probe contracts and asserts the expected refusal diagnostics. Both hard-fail without a compiler and neither has a skip path (`codegen_regression.rs:135-142`), which is correct — but it means CI must supply a compiler or omit the tests, and today it omits them.

Evidence for the current state:
- `rust-runtime-test.yml:178` runs `cargo test -p tests-e2e-rust --locked -- --skip rust_codegen_byte_parity --skip rust_backend_`. Header comment lines 12-21 state the reason.
- `build-compiler.yml` builds `compactc` via Nix but its `.#compiler` devshell has no cargo (`flake.nix:606`, `:254`); `rust-runtime-test.yml` has a Rust toolchain but no Nix.
- `tests-e2e-rust/Cargo.toml:69-72` documents the dev-dependency requirement; `compact-contract-multi-pl-call-fixture` is a workspace member and a `FIXTURES` row but has no dev-dep entry.

## Goals / Non-Goals

**Goals:**
- The byte-parity gate and the rejection corpus run in CI on every PR, with no vacuous-skip path.
- The registration invariants that caused a fixture to be invisible to CI become machine-enforced.
- Compiling the emitted crate is a first-class gate, not a side effect of a dev-dep.

**Non-Goals:**
- No `workflow_run` artifact sharing yet (deferred optimisation; note it in the workflow comment).
- No change to what the compiler emits; this change is infra-only.
- No attempt to make CI the first line of defence (AGENT.md §2.1 still requires the local run before pushing).
- No macOS byte-parity lane in this change: the `test` matrix leg on `macos-latest` keeps its `--skip` flags. (Nix + `nix build` on macOS is precedented via `release-build.yml`, but per-PR compiler builds on both matrix legs roughly double the added wall-clock, and `internal-release.yml` already disables the Attic cache for Intel macOS — deferred.)

## Decisions

1. **A self-contained `ubuntu-latest` job in `rust-runtime-test.yml`, rather than artifact sharing or a matrix lane.** Adding the Nix cache action + `nix build .#compactc` to a single Linux job is contained and reviewable; the existing `test` matrix legs keep their `--skip` flags so their cost is unchanged. Artifact sharing saves ~11 min/PR but adds cross-workflow coupling and failure modes. Rejected for now; recorded as a follow-up in #23. Extending the lane to the macOS matrix leg is deferred (see Non-Goals).
2. **Invariants as executable tests, not documentation.** A `cargo test` (or a small test in `codegen_regression.rs`) that parses the `FIXTURES` table and asserts each row's contract crate appears in `tests-e2e-rust/Cargo.toml` `[dev-dependencies]`. Prose in AGENT.md was already present and did not prevent the hole.
3. **`cargo check` each freshly regenerated fixture `lib.rs` against the committed fixture manifest.** The emitted `contract/Cargo.toml` cannot be used directly: it declares the registry dep `compact-runtime = "<runtime-version-string>"` (the TypeScript runtime version from `runtime/package.json`) while the emitted `lib.rs` uses `midnight_compact_runtime`, and `compact-runtime` is not in `Cargo.lock`. Instead, after `codegen_regression` regenerates a fixture, copy the regenerated `lib.rs` into a scratch crate whose `Cargo.toml` is the committed fixture manifest (which path-deps `midnight-compact-runtime` at `../../../runtime-rs`) and `cargo check --manifest-path` it. This is independent of the byte comparison and catches "compiles-clean-but-emits-E0308" shapes, and it is deliberately independent of the dev-dep invariant (Decision 2): a future `FIXTURES` row could be added before its dev-dep entry.
4. **Make `rustfmt` absence loud.** `compiler/passes.ss`'s post-pass currently soft-fails when `rustfmt` is absent, so regeneration produces unformatted bytes that differ from committed formatted bytes → false drift. Either fail the emitter or have the gate run `rustfmt --check` and classify drift as format-vs-semantic.

## Risks / Trade-offs

- [Added CI wall-clock (~11 min/PR for the compiler build on `ubuntu-latest`, assuming the Attic cache token resolves; `build-compiler.yml` notes 90+ min without the cache, and the token is absent on fork PRs)] → acceptable for a gate that prevents multi-round review loops; the artifact-sharing follow-up removes it later. Linux-only scope keeps the macOS matrix legs' cost unchanged.
- [Nix-in-CI flakiness] → reuse the existing `setup-nix-cache` action used by `build-compiler.yml`.
- [`cargo check` on every regen multiplies build cost] → cache the workspace, or scope the check to fixtures whose `lib.rs` differs in the diff (a rotation reintroduces blind spots); measure before merging.

## Migration Plan

Add the `ubuntu-latest` job and invariants; the new job runs `cargo test -p tests-e2e-rust` without the two `--skip` flags. The existing `test` matrix legs keep their flags. Rollback = delete the new job (the gate reverts to local-only, its current state). No data migration.
