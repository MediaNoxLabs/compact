# Proposal: add-digital-passport-dogfood-fixture

## Why

The compiler team needs standing proof that this toolchain builds the real, named, third-party contracts it exists to serve — not just synthetic fixtures. The removal of the identity fixtures (`08decb1`) de-branded the corpus but also ended direct dogfooding. `midnight-verifiable-credential-digital-passport` is the natural first dogfood target: it is a production pure-circuit library that already exposed a real rust-backend gap (fixed by `fix-ternary-expression-codegen`), and its full source (plus staged npm core) is now available where the previously-removed `digital-passport` crate had none.

## What Changes

- New third-party enclave `examples/dogfood/` (a new category under `examples/`, documented in AGENT.md §1): first tenant `examples/dogfood/digital-passport-credential/` carrying the upstream package's `src/` tree **verbatim** at pinned rev `cdeb860b`, plus `core-compact-staging/` staged from `@midnight-ntwrk/credential-compact@0.1.0-rc3` `dist/` (15 files, relative includes preserved).
- `PROVENANCE.md` in the vendored dir recording upstream URL, rev, npm package + version, staging procedure (no-pnpm `curl` + `tar` variant included), refresh policy (explicit manual re-sync), and the deliberate relationship to `08decb1`'s de-branding.
- Generated fixture crate `tests-e2e-rust/contracts/digital-passport-credential/` (`compact-contract-digital-passport-credential`), registered per the full recipe: root `Cargo.toml` workspace member, `tests-e2e-rust/Cargo.toml` dev-dep, FIXTURES row `("dogfood/digital-passport-credential/src/digital-passport-credential.compact", "digital-passport-credential")` (nested source path, did-05 precedent).
- Header exclusion via a single `excluded_directories` entry for the enclave (upstream Apache-2.0 headers stay verbatim); post-vendor `git status --ignored` truncation check against `.gitignore`'s bare `dist`/`gen`/`out` patterns.
- CI: new explicit clippy step for the crate in `rust-runtime-test.yml` (clippy is `-p`-allowlisted, not automatic); TS+rust codegen-only compile smoke of the dogfood entry added to `build-compiler.yml`'s smoke step; updated `Cargo.lock` committed (`--locked` gates).
- Phase 2 (same change, later tasks): TS reference captures (`fixtures/capture-digital-passport-credential.mjs` → JSON — representative subset: civil-date helpers incl. all ternary sites and assert-fail paths, one issuance/presentation/verification round-trip) + executing parity test `tests/digital_passport_credential.rs`. Upstream's own vitest suite and smoke-consumer are **not** ported (different purpose, node ecosystem).
- Short new ADR superseding `08decb1`'s de-branding for a clearly-bounded dogfood enclave, plus AGENT.md §1 update for the new category.
- Version bump to 0.31.118 (own patch bump — `changelog-check.yml` requires CHANGELOG + compiler-version in the diff) + full embed-site sweep + CHANGELOG entry.

Depends on `fix-ternary-expression-codegen` (0.31.117) — the crate cannot be generated until the rust backend compiles the contract.

## Capabilities

### New Capabilities

- `dogfooding/digital-passport-fixture`: the repo vendors the upstream digital-passport contract at a pinned revision, regenerates its rust crate byte-identically from source, and gates it in CI (compile/fmt/clippy/integration + local byte-parity per AGENT.md convention).

### Modified Capabilities

(none — capability tree is being bootstrapped; the ADR + AGENT.md updates are documentation, not spec deltas)

## Impact

- New vendored third-party tree under `examples/dogfood/` (license: Apache-2.0, upstream headers intact); `header_config.json` exclusion.
- New fixture crate + registrations; `Cargo.lock`; two workflow files (`rust-runtime-test.yml`, `build-compiler.yml`).
- New ADR; AGENT.md §1; CHANGELOG; version triple 0.31.117 → 0.31.118 across embed sites.
- Reverses (with explicit rationale, bounded to the enclave) the third-party-material stance of `08decb1` / ADR-0001's follow-up note.
