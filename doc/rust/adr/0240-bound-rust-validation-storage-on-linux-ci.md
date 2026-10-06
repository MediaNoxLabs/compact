---
id: RUST-ADR-0240
alias: ADR-0240
title: "Bound Rust validation storage on Linux CI"
date: 2026-10-06
publication_date: 2026-10-07
decision_status: "accepted-ci"
topics: ["CI", "Linux", "storage"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: 07781c8b99b632b4e71b32b26a1a822c06158ec02ed6d6cb83538eeb692d1d1a
---
# RUST-ADR-0240 — Bound Rust validation storage on Linux CI

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-ci. The Linux validation job shares one Cargo target and disables dev/test debug symbols while preserving assertions and all existing gates. It remedies observed runner disk exhaustion; no identical Nix derivation/store output is claimed across heads.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#344 closure](https://github.com/MediaNoxLabs/compact/issues/344#issuecomment-6017814824). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c) · [`eb0e4019`](https://github.com/MediaNoxLabs/compact/commit/eb0e4019994629338e655179ddc6fc440e2c48d0). These include historical prerequisites and probes, not only final delivery commits.

## Historical decision and amendments

<!-- compact-adr-final-acceptance:03c03a39a2d39c43c91143b9526ca1c2ffae706c:ADR-0240 -->
### Final remote acceptance — 2026-10-06

At published commit `03c03a39a2d39c43c91143b9526ca1c2ffae706c`, all 10 required remote workflows passed on the same revision; the root-reviewed acceptance record and closed rust-backend-v2 milestone cover this bounded decision. [Issue #344 closure record](https://github.com/MediaNoxLabs/compact/issues/344#issuecomment-6017814824) and the [milestone closeout](https://github.com/MediaNoxLabs/compact/issues/103#issuecomment-6017383781) document the verified scope.

[Final evidence bundle](references.md#private-note-06) · [Architecture successor](references.md#private-note-07)

Earlier status statements, observations, tests and limits below remain historical evidence. Final acceptance supersedes their pending wording and does not expand implementation scope.

### Historical decision and delivery record


Date: 2026-10-06
Status: accepted; implementation and validation pending
Milestone: rust-backend-v2

### Problem

Candidate eb0e4019994629338e655179ddc6fc440e2c48d0 passed nine required workflows. The Linux compiler job passed explicit built-in proof preparation and funded strict proof cases, then exhausted runner disk while compiling the runtime integration test `recording_map`: rustc reported `No space left on device`. Later packaging and E2E steps were skipped. This is failed remote acceptance.

The workflow uses default `target/` for preparation and workspace tests. Python consumer gates default to `target/compactc-consumer` inside their own processes, duplicating heavy dependencies. Both trees retain Cargo's default development/test debug symbols. Incremental compilation is already disabled.

### Decision and before/after

Before:
```yaml
# Prepare/workspace: target/; generated consumers: target/compactc-consumer
# Development and test profiles retain default debug symbols.
```

After, scoped to the Linux compiler job:
```yaml
env:
  CARGO_TARGET_DIR: ${{ github.workspace }}/target
  CARGO_PROFILE_DEV_DEBUG: 0
  CARGO_PROFILE_TEST_DEBUG: 0
```

Use one absolute Cargo target directory across preparation, generated consumers, workspace tests, Clippy and package rehearsal. Existing Python `setdefault` handling honors it. Keep `target/package` consistent with the package helper and artifact upload. Disable development/test debug symbols for this job; retain the default optimization level, debug assertions, overflow checks, all tests and strict proof assertions. Keep local developer profiles and Apple validation unchanged. Add bounded storage telemetry around the native gate so future resource failures have useful context.

### Emitter/runtime impact

No emitter, generated API, runtime semantics, IR 20, ABI 49 or dependency lock changes. This changes only CI artifact storage. Nix derivations keep their declared build environments. The changed source closure can produce new derivation and store identities even when implementation bytes are unchanged. Shared Cargo caches use Cargo's existing package/features/profile fingerprints.

### Alternatives and costs

Sharing the directory alone still accumulates debug information across many integration-test binaries. Debug-symbol reduction alone retains redundant dependency trees. Removing tests or ignoring errors weakens acceptance and is rejected. Deleting arbitrary preinstalled tools or caches is broader and less predictable. Increasing timeout does not solve disk exhaustion. Debugger symbol quality is reduced for this Linux CI job; local debug builds retain their original behavior.

### Validation and acceptance

Independently review target/package assumptions and environment propagation. Check YAML scope and confirm all existing workflow test/build commands remain present. Run the formerly failing runtime recording_map test using the same profile overrides and inspect Cargo's resolved target directory. Confirm debug assertions remain enabled in a focused executable probe. Record representative binary/storage measurements without extrapolating an exact runner-space guarantee. Run all ten workflows on the final signed revision; only actual success authorizes closure.

Failure: https://github.com/MediaNoxLabs/compact/actions/runs/37448331896 . The raw proof/wallet-bearing step log remains private. Retain only the concise resource failure receipt in the public evidence bundle.


### Publication and focused acceptance

Delivered in conventional GPG-signed/DCO commit [03c03a39](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c), [issue #344](https://github.com/MediaNoxLabs/compact/issues/344). The initiative now has 266 audited commits and 243 milestone issues.

The formerly failing runtime `recording_map` target passed 2/2 with all features and the CI profile overrides. Cargo artifact metadata and a separate executable probe confirmed zero debug information, optimization level 0, enabled debug assertions and enabled overflow checks. The absolute target directory was verified. A retained historical binary is not a controlled size baseline, so no exact runner-space saving is claimed. Independent workflow review confirmed every original command, all other jobs and the timeout are unchanged. Package/archive paths and Nix environment propagation were reviewed.

All ten workflows were dispatched at the exact new revision. [Compiler validation](https://github.com/MediaNoxLabs/compact/actions/runs/37460145204) must confirm actual runner storage and the complete native/package/E2E gates. Closure remains pending. The preceding eb0e4019 round remains a failed 9/10 result. Its cold proof-material validation stays valid at its original revision; the new change touches only the workflow and backend README.
