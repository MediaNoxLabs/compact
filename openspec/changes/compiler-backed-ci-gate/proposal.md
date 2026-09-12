## Why

The two gates that catch most Rust-codegen regressions — `codegen_regression`'s byte-parity test and the `rejection_corpus` / `rust_backend_*` tests — are excluded by name from CI (`rust-runtime-test.yml:178`, `-- --skip rust_codegen_byte_parity --skip rust_backend_`) because no CI lane has ever had both a built `compactc` and a Rust toolchain. Enforcement has therefore been a local-only pre-push checklist (AGENT.md §2.1 step 3). That is precisely how PR #70's bug family reached six review rounds: each newly broken shape was invisible to CI until a human read the diff. A `git` audit shows no combined lane ever existed — the byte-parity test originally auto-skipped when `compactc` was absent (`b2a4829`), was hardened to hard-fail (`f90c344`), and the skip was then moved to the call site (`3e0c70a`). This change gives CI a real compiler so the existing hard-failing gates actually run.

## What Changes

- Add a compiler-backed CI job that sets up Nix **and** a Rust toolchain, runs `nix build .#compactc`, then `cargo test -p tests-e2e-rust --locked` **without** the `--skip rust_codegen_byte_parity` / `--skip rust_backend_` exclusions. (Self-contained job first; sharing `build-compiler.yml`'s built `compactc` artifact is a later optimisation, tracked in [#23](https://github.com/MediaNoxLabs/compact/issues/23).)
- Add mechanical gate invariants so the gate holes themselves cannot reappear:
  - every `codegen_regression` `FIXTURES` row MUST also be a `tests-e2e-rust` dev-dependency (workspace membership alone does not pull a crate into `cargo build -p tests-e2e-rust --tests`; the current hole is `compact-contract-multi-pl-call-fixture`), enforced by a test rather than prose;
  - a step that `cargo check`s every regenerated emitted crate, independent of byte-parity, so compile failures are caught even for a fixture with no dev-dep;
  - `rustfmt` absence becomes a hard failure (or an explicit normalising step) so byte-parity cannot silently report false drift (`compiler/passes.ss` currently soft-fails the post-pass).
- Correct AGENT.md §2.1/§3.3/§5.1, which currently under-specify the required registration (it says "workspace member" where a dev-dependency is what makes CI compile the crate).

## Capabilities

### New Capabilities

(none — CI/test infrastructure with no compiler-observable behaviour change; `.openspec.yaml` sets `skip_specs: true`)

### Modified Capabilities

(none)

## Impact

- `.github/workflows/rust-runtime-test.yml` (new compiler-backed job), possibly a small shared workflow/action.
- `tests-e2e-rust/Cargo.toml` (dev-dependency completeness), `tests-e2e-rust/tests/codegen_regression.rs` (invariant test), `compiler/passes.ss` (rustfmt hard-fail).
- `AGENT.md` §2.1/§3.3/§5.1.
- Prerequisite for `vendor-digital-passport-harness` (its probes must actually run in CI) and `fix-ternary-expression-codegen` (its position matrix must actually be enforced). Version/changelog: CI-only, so request the maintainer-applied `skip-changelog` label rather than bumping the toolchain.
