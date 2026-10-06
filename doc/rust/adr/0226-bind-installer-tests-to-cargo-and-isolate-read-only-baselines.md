---
id: RUST-ADR-0226
alias: ADR-0226
title: "Bind installer tests to Cargo and isolate read-only baselines"
date: 2026-10-06
publication_date: 2026-10-07
decision_status: "accepted-test-only"
topics: ["installer", "Cargo", "read-only"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: ee468ab77408a35a51f19a163ab9285bef380e447fbb958c5ccc62ce0ffaf0b2
---
# RUST-ADR-0226 — Bind installer tests to Cargo and isolate read-only baselines

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-test-only. Installer test helpers select Cargo's actual test-built compact binary for defaults and retain explicit downloaded-binary overrides. Four check/list baselines use private cache/home and prove no install; this does not validate mutable remote releases.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#330 closure](https://github.com/MediaNoxLabs/compact/issues/330#issuecomment-6017789436). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`2895c986`](https://github.com/MediaNoxLabs/compact/commit/2895c98603b04d3a9fcb9a2a94ad0fba23a30ece) · [`b1ffd086`](https://github.com/MediaNoxLabs/compact/commit/b1ffd08621b913ed6a3d334c39c3d5820a9e2775) · [`d0e499f3`](https://github.com/MediaNoxLabs/compact/commit/d0e499f33d65dd568670ed34ed64fff32a7ce54b). These include historical prerequisites and probes, not only final delivery commits.

## Historical decision and amendments

- Status: accepted for bounded local implementation; evidence pending
- Date: 2026-10-06
- Base: d0e499f3
- Milestone: rust-backend-v2

### Problem and prior evidence

The installer integration helpers default to ../../target/debug/compact even when Cargo builds tests in another CARGO_TARGET_DIR. This can exercise stale executable bytes. At 2895c986, the retained workspace log ${LOCAL_EVIDENCE}/compact-local-full-abi36-2895c986/logs/277-workspace-tests.log:501-531 records test_compact_check_no_param failing its snapshot: host compiler0.31.1/latest0.35.0 versus expected no compiler/latest0.31.0. The command itself exited0. Five sibling tests passed. That historical run did not hash the hardcoded executed binary, so it cannot establish an exact tested-binary identity. Installer source/tests are unchanged from remote baseline1495bf64 through d0e499f3.

### Decision and ownership

Change only tools/compact integration-test support and four read-only no-install check/list cases. Both shared default executable selections use env!("CARGO_BIN_EXE_compact"); explicit supplied downloaded binary paths remain first choice. A typed test fixture creates private HOME, platform cache, XDG cache/config and COMPACT_DIRECTORY, seeds exact synthetic parsed MidnightArtifacts metadata matching the existing latest0.31.0/version/platform assertions, and supplies a fresh per-fixture cache timestamp. No public latest query, release-version bump, blanket expected-output rewrite, production endpoint override or dependency change.

Child proxy configuration points to a local refusal endpoint to fail unexpected HTTP use; retain a request counter if practical. The read-only fixture must leave its seeded cache unchanged and create no installed compiler. Existing explicit caller environments, downloaded binaries and multi-command update/install/format/clean/self scenarios remain unchanged.

### Before / after

Before:
```rust
let binary = binary_path.unwrap_or("../../target/debug/compact");
run_command(&["check"], None, expected_stdout, ...);
```

After:
```rust
let binary = binary_path.unwrap_or(env!("CARGO_BIN_EXE_compact"));
let baseline = ReadOnlyBaseline::new();
run_command(&["check"], Some(baseline.environment()), expected_stdout, ...);
baseline.assert_unchanged();
```

The fixture uses fixed synthetic release metadata. It exercises existing parsed-cache/version selection and output behavior; it is not a live GitHub response or proof that release assets exist. The existing no-installed-state output expectations remain exact.

### Alternatives and limits

Rejected: bumping latest to a current public release, changing product network endpoints, skipping failing tests, or treating Rust backend proofs as installer coverage. A global HOME/cache override for every scenario would interfere with explicit downloaded-tool and multi-step tests; injection is limited to the selected read-only cases.

A pure binary-identity change is independently useful but would not close the known mutable check baseline. This slice does not establish full installer integration acceptance. Published installer download/self-update, network release availability, archive identities, real installed compiler behavior and other platform execution remain separate residuals. No ABI/schema/emitter/runtime or public product behavior changes.

### Validation plan

Use existing owned target/adr157 with Rust1.99.0 and CARGO_INCREMENTAL=0. Run test_help plus test_check/test_list from that nondefault target, verifying the invoked executable is Cargo's exact selected binary. Exercise exact no-installed/latest/version-list outputs from private fixture state, refuse unexpected network and assert unchanged cache/no installed outputs. Run package all-target/all-feature strict Clippy, formatting and scoped diff checks. No broad proof gate, update/download/clean/self installer suite, push or remote CI.

### Delivery

Pending signed GPG+DCO commit, focused receipts and independent review. Preserve the old failure log and explain the binary-identity caveat in final evidence.

Issue created before implementation: https://github.com/MediaNoxLabs/compact/issues/330 (rust-backend-v2).
### ADR226 signed local delivery

Signed GPG/DCO commit `f99c8df2bba9e7d9626a2c73d172f9e57d35647d` from d0e499f3. Both shared default helpers now use Cargo's exact CARGO_BIN_EXE_compact; explicit downloaded executable overrides remain unchanged. Four read-only no-install check/list cases use test-owned HOME/cache/artifact directories, fixed synthetic nine-version metadata and per-child cache freshness. Exact old output fixtures are unchanged. The local refusal proxy observed zero connections; seeded caches stayed byte-identical and artifact state contains only optional empty real bin/versions directories.

Exact-head validation on aarch64-darwin / Rust1.99 / existing nondefault target/adr157: all24 help/check/list tests passed (6 check,10 help,8 list); strict package all-target/all-feature Clippy, scoped rustfmt/diff/source checks passed. Every helper invocation logged the target/adr157/debug/compact path; executable SHA256 `33e3da1a4699c65fe9a8cc08d816123252db807bead92bdfca5d1bc2d57062e1`. Host HOME/.compact top-level/two-level metadata was unchanged across the signed run. Independent review's installed-state assertion gap was corrected before signing and re-reviewed with no remaining finding.

Receipt: ${LOCAL_EVIDENCE}/compact-adr226-delivery-receipt.json. Logs: ${LOCAL_EVIDENCE}/compact-adr226-focused-signed.log and ${LOCAL_EVIDENCE}/compact-adr226-clippy-signed.log. Old failure ${LOCAL_EVIDENCE}/compact-local-full-abi36-2895c986/logs/277-workspace-tests.log remains preserved with the stale-binary identity caveat.

No production/dependency/expected-output/scenario edits, network release operations, broad proof suite, push or remote CI. This is not full installer acceptance: other no-install host assumptions and actual release/download/update/self-update integration remain explicit residuals. Synthetic cache values are not a live release catalog.

ADR226 integration checkpoint: root signed GPG+DCO commit b1ffd08621b913ed6a3d334c39c3d5820a9e2775 integrates the reviewed Cargo-binary and private check/list baselines. ADR227 builds from that head; live release/download coverage remains separate.
