## Why

With the upstream contract vendored and its gaps fixed (`vendor-digital-passport-harness`, `type-directed-expression-coercion`, `fix-ternary-expression-codegen`), the digital-passport contract must become a **first-class, standing regression fixture** rather than a one-off probe: its generated crate registered and byte-parity-gated, its compile/fmt/lint gated in CI, and its Rust↔TS behaviour pinned. This is the payoff of dogfooding — the toolchain continuously proves it builds a real, named, third-party production contract.

## What Changes

- Generate and register the fixture crate `tests-e2e-rust/contracts/digital-passport-credential/` (`compact-contract-digital-passport-credential`) per the full recipe: root `Cargo.toml` workspace member, `tests-e2e-rust` **dev-dependency**, and a `codegen_regression` `FIXTURES` row with the nested source path `("dogfood/digital-passport-credential/src/digital-passport-credential.compact", "digital-passport-credential")`; commit the updated `Cargo.lock` (`--locked` gates).
- Confirm the whole-entry compile gate is a hard requirement (the flip from `vendor-digital-passport-harness`'s expected-failure gate landed in `fix-ternary-expression-codegen`, which this change does not re-own): both `compactc --target ts --skip-zk` and `compactc --target rust --skip-zk` compile the vendored entry with zero `unimplemented!`/`todo!`.
- Execute the parity test that runs against the TS reference captures authored by `vendor-digital-passport-harness`: `tests-e2e-rust/tests/digital_passport_credential.rs` asserts Rust outcomes (including assert-fail paths) equal the TS reference at every captured step, comparing the round-trip's 32-byte body roots byte-for-byte.
- CI wiring: a crate-specific clippy step in `rust-runtime-test.yml` (the crate is a dev-dependency built with `--cap-lints allow`, so nothing else lints it) and a dual-target codegen smoke in `build-compiler.yml`'s smoke step (codegen-only; that lane has no Rust toolchain).
- Fold in the one codegen fix the new clippy gate surfaces: `--target rust` emits an always-false `if _repr.len() < Self::FIELD_SIZE { return None; }` guard in the zero-field `FromFieldRepr` scaffold (clippy's deny-by-default `absurd_extreme_comparisons`), so the guard is emitted only for structs with at least one field and the dogfood fixture is regenerated.
- New ADR recording the bounded third-party enclave (superseding `08decb1`'s de-branding for `examples/dogfood/` only) and AGENT.md §1 documenting the category, the header exclusion, and the PROVENANCE refresh pointer.

## Capabilities

### New Capabilities

- `dogfooding/digital-passport-fixture`: the repo compiles the pinned upstream digital-passport contract with both codegen targets, regenerates its Rust crate byte-identically, gates it in CI, and pins Rust↔TS behaviour parity.

### Modified Capabilities

(none)

## Impact

- New fixture crate + registrations; `Cargo.lock`; `tests-e2e-rust/tests/digital_passport_credential.rs`.
- `.github/workflows/rust-runtime-test.yml`, `.github/workflows/build-compiler.yml`.
- New ADR; AGENT.md §1.
- One small `rust-passes` codegen fix + a `compiler-version.ss` bump (0.31.118 → 0.31.119, with `flake.nix`/`doc/ledger-adt.mdx`) and a CHANGELOG entry — the dogfood clippy gate surfaced the zero-field `FromFieldRepr` guard. The vendored fixtures regenerate byte-identically; `skip-changelog` no longer applies.
- Depends on `compiler-backed-ci-gate`, `vendor-digital-passport-harness`, `type-directed-expression-coercion`, and `fix-ternary-expression-codegen`.
