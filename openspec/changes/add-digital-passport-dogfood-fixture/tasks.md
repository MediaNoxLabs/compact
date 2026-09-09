# Tasks: add-digital-passport-dogfood-fixture

Prerequisite: `fix-ternary-expression-codegen` merged (rust target must compile the contract before the crate can be generated). All work inside `nix develop`; byte-parity per AGENT.md §2.1/§3.3 is local-only and mandatory before every push.

## 1. Vendor the upstream tree

- [ ] 1.1 Clone upstream at rev `cdeb860b` and copy `packages/midnight-verifiable-credential-digital-passport/src/` → `examples/dogfood/digital-passport-credential/src/` **verbatim** (upstream headers intact). Verify: `git diff --no-index` against the upstream clone shows zero diffs.
- [ ] 1.2 Stage the core without pnpm: `curl -L $(npm view @midnight-ntwrk/credential-compact@0.1.0-rc3 dist.tarball) | tar xz`, copy `dist/credentials.compact` + `dist/credentials/` → `examples/dogfood/digital-passport-credential/core-compact-staging/` (15 files; mirrors `scripts/stage-core-compact.mjs`'s exports-map resolution). Verify: file count and byte-compare against the tarball.
- [ ] 1.3 Write `examples/dogfood/digital-passport-credential/PROVENANCE.md`: upstream URL, rev `cdeb860b`, license (Apache-2.0), npm core package+version, staging procedure (incl. the no-pnpm variant), refresh policy (explicit manual re-sync; update rev in same commit), note on possible future neutralization. Verify: a reader can re-sync from PROVENANCE alone.
- [ ] 1.4 Run `git status --ignored` over the vendored tree and confirm all files are tracked (`.gitignore` bare `dist`/`gen`/`out`/`artifacts` patterns can silently truncate). Rename any colliding segment and record it in PROVENANCE. Verify: `git ls-files examples/dogfood | wc -l` matches the source count.

## 2. Compile smoke + header exclusion

- [ ] 2.1 Add `excluded_directories` entry `dogfood` to `header_config.json`. Verify: `python add_headers.py --validate` passes with the vendored tree present.
- [ ] 2.2 Local compile smoke both targets: `result/bin/compactc --target ts --skip-zk examples/dogfood/digital-passport-credential/src/digital-passport-credential.compact /tmp/out-ts/` and the rust equivalent. Verify: both exit 0 (rust requires change 1 landed).

## 3. Fixture crate + registration

- [ ] 3.1 Generate: `result/bin/compactc --target rust --skip-zk examples/dogfood/digital-passport-credential/src/digital-passport-credential.compact tests-e2e-rust/contracts/digital-passport-credential/`. Verify: `lib.rs` (~3,509± lines) emitted, rustfmt-clean, zero `unimplemented!`/`todo!`.
- [ ] 3.2 Register: root `Cargo.toml` workspace member, `tests-e2e-rust/Cargo.toml` dev-dep, FIXTURES row `("dogfood/digital-passport-credential/src/digital-passport-credential.compact", "digital-passport-credential")`, updated `Cargo.lock` committed. Verify: `cargo build -p tests-e2e-rust --tests --locked` green.
- [ ] 3.3 Byte-parity: `cargo test -p tests-e2e-rust rust_codegen_byte_parity` green (dogfood row regenerates byte-identically). Verify: full FIXTURES table green.

## 4. CI wiring

- [ ] 4.1 `rust-runtime-test.yml`: add clippy step `cargo clippy -p compact-contract-digital-passport-credential --all-targets --all-features -- -D warnings` to the pre-check allowlist (dev-deps are `--cap-lints allow`; nothing automatic covers the crate). Verify: step present; existing steps untouched.
- [ ] 4.2 `build-compiler.yml` smoke step: add `nix develop .#compiler --command compactc --target ts --skip-zk examples/dogfood/… out/` and the rust-target twin. Verify: workflow YAML lints; codegen-only (no cargo on that lane).

## 5. Phase 2 — parity captures + executing test

- [ ] 5.1 Author `tests-e2e-rust/fixtures/capture-digital-passport-credential.mjs` (Apache header) per the existing capture pattern: civil-date helpers (`assertCivilDateMatchesEpochDays`, `assertValidDigitalPassportAgePredicate` — include the ternary sites and assert-fail paths) + one issuance/presentation/verification round-trip, using upstream `testing/` utils (jubjub, credential fixtures) as construction reference. Verify: script runs, emits committed JSON.
- [ ] 5.2 Write `tests-e2e-rust/tests/digital_passport_credential.rs` (Apache header) asserting Rust outcomes byte-equal the TS reference at each step, modeled on the closest large-fixture parity test. Verify: `cargo test -p tests-e2e-rust digital_passport` green.
- [ ] 5.3 If a capture is disproportionately hard, shrink the protocol subset (never the helpers) and record the decision in the change notes. Verify: helper captures + at least one round-trip present.

## 6. ADR, AGENT.md, version, changelog

- [ ] 6.1 New ADR (next number after existing): dogfood enclave rationale, bounds (`examples/dogfood/` only), explicit supersession of `08decb1`'s stance for this enclave, pinning policy. Verify: cross-references ADR-0001 and commit `08decb1`.
- [ ] 6.2 AGENT.md §1: document the `examples/dogfood/` third-party enclave category (one paragraph; exclusion + refresh pointer to PROVENANCE). Verify: a new contributor understands the category exists and why.
- [ ] 6.3 Version bump 0.31.117 → 0.31.118 with full embed-site sweep (`compiler-version.ss`, `flake.nix`, `doc/ledger-adt.mdx` regen, grep old triple) + CHANGELOG entry (dogfood fixture vendoring + registration + CI). Verify: zero stale embeds; `changelog-check` satisfied.
- [ ] 6.4 Full local gates before push: fmt, clippy (incl. new crate), `cargo test -p midnight-compact-runtime -p tests-e2e-rust`, `add_headers.py --validate`. Verify: all green under `nix develop`.
- [ ] 6.5 Commit signed+DCO on `feature/add-digital-passport-dogfood-fixture` (cut after change 1 merges; merges into `digital-passport-patch`). Verify: `git log --show-signature` clean; vendored bytes unchanged by the commit (`git diff --stat` shows only moves/additions).
