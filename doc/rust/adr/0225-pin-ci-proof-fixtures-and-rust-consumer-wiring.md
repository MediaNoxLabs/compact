---
id: RUST-ADR-0225
alias: ADR-0225
title: "Pin CI proof fixtures and Rust consumer wiring"
date: 2026-10-06
publication_date: 2026-10-07
decision_status: "accepted-ci"
topics: ["CI", "proof-fixture", "consumer"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: fbf58a1f21385bd5034623701cbcd692408bd909bb6ff2f5374a4ffbee096d5b
---
# RUST-ADR-0225 — Pin CI proof fixtures and Rust consumer wiring

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-ci. CI resolves proof fixtures from the Cargo-locked ledger 8.0.3 source and wires the same Rust consumer/toolchain gates with explicit provenance. This fixes workflow preparation; genuine installer archive and remote results were separate later decisions.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#329 closure](https://github.com/MediaNoxLabs/compact/issues/329#issuecomment-6017787860). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`3ed70f4b`](https://github.com/MediaNoxLabs/compact/commit/3ed70f4be1ee4ed39ea72ef7bf1016b3225ea9c8) · [`d0e499f3`](https://github.com/MediaNoxLabs/compact/commit/d0e499f33d65dd568670ed34ed64fff32a7ce54b) · [`d5dd3f2e`](https://github.com/MediaNoxLabs/compact/commit/d5dd3f2eeeaad337ba40fcf2639d540a9738f161). These include historical prerequisites and probes, not only final delivery commits.

## Historical decision and amendments

### Local CI preflight integrated — d0e499f3 (2026-10-06)

ADR225 / [#329](https://github.com/MediaNoxLabs/compact/issues/329) is delivered locally at **d0e499f3**. It pins locked ledger 8.0.3 fixture provenance and Rust 1.99.0, fixes workspace cache/package wiring, and preserves installer tests and backend Clippy coverage. All 43 Python checks pass on the integrated root; the delivery also passed actionlint, real fixture resolution, Nix toolchain precedence, YAML coverage invariants and tracked-source headers. See [ADR-0225 — Pin CI proof fixtures and Rust consumer wiring](0225-pin-ci-proof-fixtures-and-rust-consumer-wiring.md) and `${LOCAL_EVIDENCE}/compact-adr225-receipt.json` for exact evidence.

There are **no runtime, schema 20 or ABI 49 changes**, and **no push or remote CI execution**. The frozen d5dd3f2e product/full-gate and live acceptance evidence remains the reference; this CI-only follow-up does not replace those tested heads. Root's completed audit verifies all 250 commits have conventional subjects, valid GPG signatures and DCO (`${LOCAL_EVIDENCE}/compact-d0e499f3-signature-audit.json`); `${LOCAL_EVIDENCE}/compact-d0e499f3-local-closeout.json` verifies the scope delta and unchanged user document. **Installer integration review remains ongoing, not resolved**; existing mutable-release tests stay enabled. Earlier entries below preserve their dated evidence and status.


- Status: **Implemented and locally verified; remote execution remains pending.**
- Date: 2026-10-06
- Milestone: rust-backend-v2
- Issue: https://github.com/MediaNoxLabs/compact/issues/329
- Source baseline inspected: d5dd3f2eeeaad337ba40fcf2639d540a9738f161

### Problem

Successful local proof acceptance used an explicit upstream fixture environment and Rust 1.99.0, but `build-compiler.yml` invokes `check_compactc_target.py --consumer --proof` without supplying `MIDNIGHT_LEDGER_TEST_STATIC_DIR` and installs floating stable. Some target-check branches explicitly invoke `cargo +1.99.0`, which can be absent on a fresh stable-only runner. `compact-test.yml` hashes nonexistent `tools/compact/Cargo.lock`, caches unused `tools/compact/target`, and invokes root-workspace commands without the installer package selector.

These are concrete wiring gaps; this ADR does not add tests or change language/runtime semantics. All existing compiler, TS, proof and installer suites remain. Mutable installer-release integration remains enabled and unchanged in this slice; its later stabilization is a separate explicit decision. No remote run or push is authorized by this proposal.

### Decision

1. Resolve the proof static directory from the **Cargo-locked registry midnight-ledger 8.0.3 package**, using `cargo metadata --locked` and its manifest path. Validate exact package identity, published VCS provenance, and retained fixture digests; emit a small public provenance JSON and the environment path. Do not use an arbitrary developer checkout or clone a moving branch.
2. Install Rust 1.99.0 with rustfmt/clippy and set `RUSTUP_TOOLCHAIN=1.99.0`, `CARGO_INCREMENTAL=0` for consumer jobs. Preserve pinned action SHAs. The compiler's Nix build toolchain remains controlled by flake.lock.
3. Key installer Cargo caches from root `Cargo.lock`, use the actual explicit job target directory, and scope its Cargo build/Clippy/nextest commands with `-p compact`. Keep the entire compact package test selection and serial integration execution; do not disable, skip, mock, or split mutable-release cases here. Backend workspace suites remain in build-compiler.

### Before / after commands

Before (proof step has no static fixture environment):

```sh
nix develop .#compiler --command env COMPACTC=compactc python3 tools/compact-rust-backend/check_compactc_target.py --consumer --proof
```

After (proposed helper interface; exact spelling may be finalized during implementation):

```sh
cargo +1.99.0 fetch --locked
cargo +1.99.0 metadata --locked --offline --format-version 1 > "$RUNNER_TEMP/compact-cargo-metadata.json"
python3 tools/compact-rust-backend/resolve_ledger_test_static.py --metadata "$RUNNER_TEMP/compact-cargo-metadata.json" --lock Cargo.lock --provenance "$RUNNER_TEMP/ledger-static-provenance.json" --github-env "$GITHUB_ENV"
# A later step receives MIDNIGHT_LEDGER_TEST_STATIC_DIR through GITHUB_ENV.
nix develop .#compiler --command env COMPACTC=compactc python3 tools/compact-rust-backend/check_compactc_target.py --consumer --proof
```

The existing handoff environment variables in the workflow are retained. The resolver does not fetch by itself or accept a substitute ledger state. Metadata resolution and checksum-verified Cargo acquisition supply its inputs.

Before installer wiring:

```text
cache key: hashFiles('tools/compact/Cargo.lock')
cache path: tools/compact/target
cargo build
cargo clippy --all-targets --all-features -- -D warnings
cargo nextest run --no-fail-fast --test-threads=1
```

After installer wiring:

```text
cache key: root Cargo.lock + runner platform/architecture + pinned Rust
cache path: explicit job CARGO_TARGET_DIR matching the Cargo commands
cargo build -p compact --locked
cargo clippy -p compact --all-targets --all-features --locked -- -D warnings
cargo nextest run -p compact --locked --no-fail-fast --test-threads=1
```

There is no change to the tests selected within the compact package. Adding the package selector prevents unrelated generated/backend workspace work from being repeated in the installer job.

### Fixture and proof-material provenance

- Registry package: midnight-ledger 8.0.3, Cargo checksum `78ab921a746fc8cceb4d9f4f68800e29cdd038dbd3486f6675428b79ebe04ded`.
- Published VCS: `615be91b079ed8df4026c1fd75352ea6d49de1a4`, path `ledger`; flake.lock ZKIR2 pins the same revision.
- Fixture bzkir SHA256: `904181287e75b0fb596ba5fcc116c882ee5d28e3115304c93ebd913722ce5841`.
- Verifier SHA256: `3f1569ebcab0655c5c145b28947c74edc4e3f5c6b276e4404b661cf0905b49d3`.
- ZKIR text SHA256: `9274ac7708818a9a6d9e2f9caab54f9c9d639ef83166d1d200a378eaa2213115`.
- Expected large prover SHA256: `996602da7ca386284e656c78ea03e55bffdba29475e6a67965c50de05e13efc2`; it is supplied by the upstream hash-checking data provider, not included as bytes in the registry static directory.

The developer ledger checkout at 6df23297 has different static files and must not become the CI pin. Existing successful proofs use compiled upstream provider hashes for Dust/Zswap/parameters; the test static environment is an external resolver base, not proof that arbitrary checkout fixtures are compatible. This proposal removes that misleading provenance. The data provider's public key/parameter acquisition remains necessary on cold CI and must retain hash checks. A proof-cache optimization is optional and outside the required minimal change.

### Ownership and affected implementation

Proposed owned files: `.github/workflows/build-compiler.yml`, `.github/workflows/compact-test.yml`, new small resolver helper and its focused Python tests under `tools/compact-rust-backend/`. Coordinate workflow ownership with root before editing. No emitter, runtime, generated crate, Cargo dependency, schema20 or ABI49 changes. Do not modify `build-runtime-test.yml` or installer test implementations in this slice.

### Local-only preflight and acceptance

After the implementation hold is lifted:

- Unit tests with synthetic metadata/package fixtures: exact single locked package accepted; missing/duplicate/wrong version/source, VCS mismatch, missing fixture and digest mismatch rejected; no environment output committed on validation failure; paths with spaces supported.
- Real offline metadata against the already cached locked package verifies the resolver and public provenance. No new full proof run merely for environment wiring; compare the resolved digests to pinned provider/fixture provenance and use the existing proof gate only if a concrete mismatch requires it.
- Parse/review workflow YAML and exact command/environment changes; preserve all pre-existing suites, action SHAs, permissions, concurrency, handoff variables, and integration test filters.
- Rust version/component preflight and package-scope/cache path assertions. No remote dispatch, push, installer execution, global rustup/default changes, or credential changes.
- Signed conventional GPG + DCO implementation commit only after local verification; issue/ADR receipt records exact source and helper outputs.

### Deferred decisions and limits

The mutable installer integration suite is not made reproducible by package-scoping or correct caches. Its real-release versus fixture/mock policy remains open and unchanged. Remote platform results, branch availability and remote CI stability are not claimed by local preparation. This proposal is ready for review while the final local gate finishes; repository edits wait for root's explicit confirmation.

### References

- `${LOCAL_EVIDENCE}/compact-v2-ci-stabilization-plan.md`
- `${LOCAL_EVIDENCE}/compact-v2-ci-fixture-provenance.json`
- `${LOCAL_EVIDENCE}/compact-final-ci-coverage-map.md`
- ADR217 live/public evidence, ADR223 headers, ADR224 deterministic E2E expectations


### Accepted implementation and delivery — 2026-10-06

Root lifted the implementation hold after the frozen d5dd local gate passed all 374 commands and 176 generated fixtures. The original proposal above is retained as decision history. Implementation starts from 3ed70f4b and is signed as **7bb99667b7784d3378866ba2a8aa895fe3d2867a** (good GPG signature, DCO, clean checkout).

#### Final changes

- The resolver verifies the exact registry package/version/source/checksum, published VCS revision/path, actual manifest identity, three static file digests and their declarations, and the upstream prover digest declaration. All validation precedes environment/provenance output. CR/LF paths fail closed; spaces work.
- The compiler workflow runs the full existing Python harness (43 tests including nine resolver tests), fetches the locked package, emits offline metadata, resolves the verified fixture path into GITHUB_ENV and uploads public provenance.
- Rust 1.99.0 is explicit in both workflows. The Nix preflight checks the actual cargo and rustc selected inside the compiler shell, avoiding an assumed PATH ordering.
- Installer jobs use root Cargo.lock, platform/architecture/toolchain cache keys, and the real workspace target directory. Root Cargo.toml/Cargo.lock changes now trigger those jobs. Package-scoped nextest retains every prior integration test and the original serial flags.
- Moving installer Clippy to `-p compact` is balanced by `cargo clippy --workspace --exclude compact --all-targets --all-features --locked -- -D warnings` in the compiler workflow. Existing jobs, actions/pinned SHAs, named steps, permissions and concurrency are preserved.
- Quoted the existing timestamped coverage destination after actionlint identified its unquoted command substitution. No coverage behavior changes.

#### Local evidence

Receipt: `${LOCAL_EVIDENCE}/compact-adr225-receipt.json`.
SHA256: `ab30c9e305f0cb6a3847ca8797a6f7178239031af703fb5274ccb7a219fcd7fb`.

Passed: 43 Python tests; actionlint 1.7.12; actual locked registry metadata/resolver; exact Nix cargo/rustc 1.99.0 preflight; parsed YAML coverage/permissions/action-pin invariants; header validation on tracked sources plus both new files; diff whitespace and signed-commit verification. Independent review has no remaining actionable findings.

Whole working-tree header validation also encounters ignored generated JS/TS files retained from the earlier compiler/go run. Its log is preserved; the identical read-only header CLI passes on a temporary tracked-source snapshot plus the new files. No generated evidence was deleted. Initial macOS canonical-path test expectation and actionlint diagnostics remain recorded and were corrected before signing.

No remote workflow was dispatched and no push occurred. Existing mutable installer-release integration remains enabled; remote platform execution and future reproducibility policy are still pending. No emitter/runtime/ABI49/schema20 changes or dependency updates occur here.
