# Tasks: compiler-backed-ci-gate

CI-only change; request the maintainer-applied `skip-changelog` label (no `compiler-version.ss` bump). All work inside `nix develop` where a compiler is needed.

## 1. Compiler-backed CI lane

- [ ] 1.1 Add a job to `.github/workflows/rust-runtime-test.yml` that installs the Nix cache + a Rust toolchain, runs `nix build .#compactc`, then `cargo test -p tests-e2e-rust --locked` with **no** `--skip` flags. Verify: the job's log shows `codegen_regression` and `rejection_corpus` executing (non-zero test counts), not filtered out.
- [ ] 1.2 Remove the `-- --skip rust_codegen_byte_parity --skip rust_backend_` exclusions from the existing test invocation and update the workflow header comment (lines 8-21) to describe the new lane. Verify: `grep -n "skip rust" .github/workflows/rust-runtime-test.yml` is empty.
- [ ] 1.3 Confirm the lane fails when it should: on a scratch branch, break one fixture's committed `lib.rs` and confirm the job goes red; revert. Verify: recorded red/green evidence in the PR.

## 2. Gate invariants

- [ ] 2.1 Add an invariant test (in `codegen_regression.rs` or a new test) asserting every `FIXTURES` row's contract crate is listed under `tests-e2e-rust/Cargo.toml` `[dev-dependencies]`. Verify: it fails today for `compact-contract-multi-pl-call-fixture`, then passes once 2.2 lands.
- [ ] 2.2 Add `compact-contract-multi-pl-call-fixture` (and any other missing row) as a `tests-e2e-rust` dev-dependency and reference it from a test so `cargo build -p tests-e2e-rust --tests` compiles it. Verify: `cargo build -p tests-e2e-rust --tests --locked` type-checks it.
- [ ] 2.3 Add a step that `cargo check`s every regenerated emitted crate from a scratch regen (independent of byte-parity). Verify: the step runs in the new lane and reports each crate.
- [ ] 2.4 Make `rustfmt` absence loud: either hard-fail the emitter post-pass in `compiler/passes.ss`, or add `rustfmt --check` to the regen path so format drift is separable from semantic drift. Verify: running the gate without `rustfmt` on PATH fails with a clear message instead of producing false byte drift.

## 3. Docs

- [ ] 3.1 Correct AGENT.md §5.1 step 3 and §3.3 to state the dev-dependency requirement (workspace membership alone does not compile the crate in CI) and the new lane. Verify: AGENT.md's recipe matches the enforced invariant.
- [ ] 3.2 Record in the workflow comment that artifact sharing from `build-compiler.yml` is the planned follow-up (#23). Verify: comment present.
