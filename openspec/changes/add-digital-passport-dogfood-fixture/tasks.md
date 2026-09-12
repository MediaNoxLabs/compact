# Tasks: add-digital-passport-dogfood-fixture

Prerequisites merged: `compiler-backed-ci-gate`, `vendor-digital-passport-harness`, `type-directed-expression-coercion`, `fix-ternary-expression-codegen`. Fixture/docs-only; request the maintainer-applied `skip-changelog` label. All work inside `nix develop`.

## 1. Generate and register the fixture crate

- [ ] 1.1 Generate: `result/bin/compactc --target rust --skip-zk examples/dogfood/digital-passport-credential/src/digital-passport-credential.compact tests-e2e-rust/contracts/digital-passport-credential/`. Verify: `lib.rs` emitted, rustfmt-clean, zero `unimplemented!`/`todo!`.
- [ ] 1.2 Register: root `Cargo.toml` workspace member, `tests-e2e-rust/Cargo.toml` **dev-dependency**, `codegen_regression.rs` FIXTURES row `("dogfood/digital-passport-credential/src/digital-passport-credential.compact", "digital-passport-credential")`, and commit the updated `Cargo.lock`. Verify: `cargo build -p tests-e2e-rust --tests --locked` compiles it; the gate-invariant test from `compiler-backed-ci-gate` passes.
- [ ] 1.3 Byte-parity: `cargo test -p tests-e2e-rust rust_codegen_byte_parity` green (the dogfood row regenerates byte-identically). Verify: full FIXTURES table green.

## 2. Compile requirement and behaviour parity

- [ ] 2.1 Flip the whole-entry expected-failure gate from `vendor-digital-passport-harness` to a hard pass: both `--target ts --skip-zk` and `--target rust --skip-zk` compile the entry. Verify: the gate is green and no longer marked expected-fail.
- [ ] 2.2 Write `tests-e2e-rust/tests/digital_passport_credential.rs` (Apache header) asserting Rust outcomes byte-equal the change-2 TS reference at each step (civil-date helpers incl. assert-fail paths, plus the protocol round-trip), with serialized state-byte comparison where state is captured. Verify: `cargo test -p tests-e2e-rust digital_passport` green; an induced wrong-width/literal bug makes it red.

## 3. CI wiring

- [ ] 3.1 `rust-runtime-test.yml`: add `cargo clippy -p compact-contract-digital-passport-credential --all-targets --all-features -- -D warnings`, with the strip-clippy-allow pre-check guarded so it fails loudly if the strip matches nothing. Verify: the step lints the crate (not vacuously green).
- [ ] 3.2 `build-compiler.yml` smoke step: add `nix develop .#compiler --command compactc --target ts --skip-zk …` and the `--target rust` twin for the dogfood entry. Verify: workflow YAML lints; codegen-only (no cargo on that lane).

## 4. ADR, AGENT.md, gates

- [ ] 4.1 New ADR (next number): dogfood enclave rationale, bounds (`examples/dogfood/` only), explicit supersession of `08decb1`'s stance for this enclave, and the pinning/refresh policy; cross-reference ADR-0001 and `08decb1`. Verify: discoverable and bounded.
- [ ] 4.2 AGENT.md §1: document the `examples/dogfood/` category (one paragraph; header exclusion + PROVENANCE refresh pointer). Verify: a new contributor understands the category and why it does not reverse corpus neutrality.
- [ ] 4.3 Full local gates before push: fmt, clippy (incl. the new crate), `cargo test -p midnight-compact-runtime -p tests-e2e-rust`, `add_headers.py --validate`. Verify: all green under `nix develop`, and the byte-parity + rejection gates run in the new CI lane.
- [ ] 4.4 Commit signed+DCO on `feature/add-digital-passport-dogfood-fixture`. Verify: `git log --show-signature` clean; vendored bytes unchanged by the commit.
