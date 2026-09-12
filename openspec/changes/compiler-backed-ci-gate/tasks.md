# Tasks: compiler-backed-ci-gate

CI-only change; request the maintainer-applied `skip-changelog` label (no `compiler-version.ss` bump). All work inside `nix develop` where a compiler is needed.

## 1. Compiler-backed CI lane

- [x] 1.1 Add a **new `ubuntu-latest` job** to `.github/workflows/rust-runtime-test.yml` (e.g. `byte-parity-compiler-backed`) with `permissions: contents: read` and an explicit `timeout-minutes`. It installs Nix via `./.github/actions/setup-nix-cache` (`attic_token: ${{ secrets.ATTIC_CACHE_TOKEN }}`) plus a Rust toolchain, runs `nix build .#compactc`, then `cargo test -p tests-e2e-rust --locked` with **no** `--skip` flags. Verify: the job's log shows `codegen_regression` and `rejection_corpus` executing (non-zero test counts), not filtered out.
- [x] 1.2 Keep the existing Linux+macOS `test` matrix job unchanged in behaviour: it retains `-- --skip rust_codegen_byte_parity --skip rust_backend_` on `cargo test -p tests-e2e-rust`, because neither matrix leg has Nix. Update the workflow header comment (lines 8-21) and the `test` job's step comment (lines ~165-177) to describe the new `ubuntu-latest` lane and remove the claim that the matrix runs the byte-parity suite. Verify: the `--skip` flags still appear on the matrix invocation, and no comment claims the matrix runs the gated tests.
- [x] 1.3 Add `compiler/passes.ss` to the `pull_request` and `push` `paths` filters in `.github/workflows/rust-runtime-test.yml` — it is edited by 2.4 and is not currently a trigger path. Verify: a PR touching only `compiler/passes.ss` starts this workflow.
- [ ] 1.4 Confirm the lane fails when it should: on a scratch branch, break one fixture's committed `lib.rs` and confirm the new job goes red; revert. Verify: the failing run URL is recorded in the PR description.

## 2. Gate invariants

- [x] 2.1 Add an invariant test (in `codegen_regression.rs` or a new test) asserting every `FIXTURES` row's contract crate is listed under `tests-e2e-rust/Cargo.toml` `[dev-dependencies]`. Verify: it fails today for `compact-contract-multi-pl-call-fixture`, then passes once 2.2 lands.
- [x] 2.2 Add `compact-contract-multi-pl-call-fixture` (and any other missing row) as a `tests-e2e-rust` dev-dependency and reference it from a test so `cargo build -p tests-e2e-rust --tests` compiles it. Verify: `cargo build -p tests-e2e-rust --tests --locked` type-checks it.
- [x] 2.3 `cargo check` each freshly regenerated fixture `lib.rs` against the committed fixture manifest, independent of byte-parity. In `codegen_regression.rs` (before the tempdir is removed) or a sibling test, copy the regenerated `contract/lib.rs` into a scratch crate whose `Cargo.toml` is `tests-e2e-rust/contracts/<dir>/Cargo.toml`, then run `cargo check --manifest-path` on it. Do **not** use the emitted `contract/Cargo.toml`: it declares `compact-runtime = "<TypeScript-runtime version>"` while the emitted `lib.rs` uses `midnight_compact_runtime`, and that registry crate is not in `Cargo.lock`. Verify: the check runs in the new lane, reports each fixture, and fails on an emitted `lib.rs` that does not compile. Note: this is independent of the 2.1 dev-dep invariant — a future `FIXTURES` row can be added before its dev-dep entry.
- [x] 2.4 Make `rustfmt` absence loud: either hard-fail the emitter post-pass in `compiler/passes.ss`, or add `rustfmt --check` to the regen path so format drift is separable from semantic drift. Verify: running the gate without `rustfmt` on PATH fails with a clear message instead of producing false byte drift.

## 3. Docs

- [x] 3.1 Correct AGENT.md §5.1 step 3 and §3.3 to state the dev-dependency requirement (workspace membership alone does not compile the crate in CI) and the new lane. Verify: AGENT.md's recipe matches the enforced invariant.
- [x] 3.2 Record in the workflow comment that artifact sharing from `build-compiler.yml` is the planned follow-up (#23). Verify: comment present.
