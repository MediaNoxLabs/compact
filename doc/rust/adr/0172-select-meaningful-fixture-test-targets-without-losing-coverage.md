---
id: RUST-ADR-0172
alias: ADR-0172
title: "Select meaningful fixture test targets without losing coverage"
date: 2026-10-05
publication_date: 2026-10-07
decision_status: "accepted-test-only"
topics: ["test-gate", "Cargo", "performance"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: b0c19f9088d11d49d20c3fa1c93ba49e27bc01fa5fbd6d4bff1fd88340cc47bd
---
# RUST-ADR-0172 — Select meaningful fixture test targets without losing coverage

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-test-only. The generated-target gate skips proven empty library harnesses while retaining integration targets and fail-safe fallback for unknown test structure. The local probe verifies selection, not an independently measured full-gate speedup.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#276 closure](https://github.com/MediaNoxLabs/compact/issues/276#issuecomment-6017697580). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`3508739c`](https://github.com/MediaNoxLabs/compact/commit/3508739cb20b8bc72d6d046bd84797ea0e794285) · [`aa0b48a6`](https://github.com/MediaNoxLabs/compact/commit/aa0b48a6fed2a185d7e5993467cf298c29cd2aa8) · [`d076db0b`](https://github.com/MediaNoxLabs/compact/commit/d076db0bc59470b237f7ba41c22627396e690ab3). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
title: ADR-0172 — Select meaningful fixture test targets without losing coverage
status: accepted
date: 2026-10-05
milestone: rust-backend-v2
```

## Historical decision and amendments

At aa0b48a6 the broad Cargo gate launches154 empty generated-library unit binaries and157 behavioral integration targets. Serial startup on macOS dominates the run. A test=false Cargo setting is overridden by --all-targets, so changing manifests alone is ineffective.

## Before / after
Before: cargo test --workspace --exclude compact --all-targets --all-features --locked.
After: derive the exact workspace target plan from Cargo metadata; core packages and suspicious fixture packages retain --all-targets, while freshly verified generated libraries without test constructs use --test * to run every integration target. The compact CLI unit gate and all-target Clippy remain.

## Boundaries
Only freshly compared generated libraries qualify. Inline test attributes, test configuration, custom macros/includes or additional target kinds cause all-target fallback. The receipt records selected packages, targets and omitted empty harnesses. New workspace packages automatically receive the full selection. No emitter, runtime, ABI or schema change.

## Evidence / decision
Proposed before code. Metadata at aa0b48a6:154 generated libraries,157 integration targets, no other fixture target kinds. ${LOCAL_EVIDENCE}/compact-empty-harness-probe confirms --all-targets runs an empty library even with test=false, while --tests respects it. Implemented target partition must be checked against Cargo execution and a synthetic inline-test fallback before delivery. No measured speedup claimed yet.



### Local delivery
Signed GPG+DCO8a3d0879 on codex/parity-test-targets. Issue: https://github.com/MediaNoxLabs/compact/issues/276. New workspace_test_plan receipt uses Cargo metadata and verified-source hashes. Current exact partition:152 generated libraries omit empty unit harnesses;157 integration targets preserved;4 core packages plus2 fixtures without integration targets retain --all-targets. Unknown attributes/derives/macros, external modules, inline tests and extra targets fall back. All-target Clippy and consumer/proof remain. Full gate now executes27 Python harness tests, including corrected stale bboard capability assertion.

### Validation
27 Python tests passed. Real Cargo probe ${LOCAL_EVIDENCE}/compact-adr172-cargo-probe/receipt.json: baseline and optimized selections execute exactly core_unit and fixture_behavior; empty library launch count2→1. Adding newly_added_unit to generated fixture restores all-target fallback and executes all3tests. Current repository plan ${LOCAL_EVIDENCE}/compact-adr172-plan.json records152/157. No full elapsed-time improvement claimed before next measured run; current aa0b48a6 broad gate stays unchanged.


Integrated as signed GPG/DCO 3508739c. Main d076db0b reran all 27 Python harness tests successfully (${LOCAL_EVIDENCE}/compact-d076db0b-python.log). Decision accepted; measured full-gate time remains pending the next combined runtime checkpoint.
