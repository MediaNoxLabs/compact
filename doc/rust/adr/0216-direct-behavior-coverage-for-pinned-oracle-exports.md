---
id: RUST-ADR-0216
alias: ADR-0216
title: "Direct behavior coverage for pinned oracle exports"
date: 2026-10-06
publication_date: 2026-10-07
decision_status: "accepted-test-only"
topics: ["behavior-matrix", "pure", "oracle"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: 680fefe37fc51ac609f791f850bff1132a884cd401c8a7f6f7a34fe72d154e45
---
# RUST-ADR-0216 — Direct behavior coverage for pinned oracle exports

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-test-only. Independent corrected-TypeScript captures add direct pure-export behavior and reviewed matrix identities across the pinned corpus. Source and runtime remain unchanged; direct case coverage is sampled and not proof, ledger or every-path parity.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#320 closure](https://github.com/MediaNoxLabs/compact/issues/320#issuecomment-6017772595). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`1ab0fffd`](https://github.com/MediaNoxLabs/compact/commit/1ab0fffd2ff94355d16c3c4376d3d3e3fe4e37bf) · [`db853a2b`](https://github.com/MediaNoxLabs/compact/commit/db853a2b822050699f79822a994819bc889a849c). These include historical prerequisites and probes, not only final delivery commits.

## Historical decision and amendments

Status: implemented, signed and locally verified; ready for parent integration.
Date:2026-10-06.

### Problem

Source availability and constructor metadata are not exported behavior evidence. A read-only review of the pinned37-oracle cohort found two stateful exports listed only in operations metadata: assert_parity_oracle.ping and ternary_cond_oracle.walkerVectorElement. Neither is invoked by its Rust behavior test or proof harness. The literal-coercion source has43 pure exports: nine are directly invoked by current Rust tests and idf is reached transitively;33 have no execution evidence in the inspected test/capture path. The constructor writes its own expressions and does not execute those33 helpers. Two existing direct calls compare Rust-to-Rust while asserting the captured TS constant separately, so their returned values are not independently anchored.

The larger37-source review remains separate and partial. In particular, the six mixed-width assertProduct exports are genuinely exercised through a function-pointer table and must not be mislabeled missing by regex searches. Transitive helper coverage will be labeled separately from a direct exported invocation.

### Decision and scope

Only tests, independent TypeScript capture scripts/data, reviewed behavior metadata and documentation change. No Compact source, shared emitter, runtime, schema20 or ABI49 changes. Preserve the existing source identity. Use corrected pinned compact-runtime0.16.101 and retain generated JS/source/runtime hashes.

1. Capture and assert ping from its default false ledger flag, including Unit result, changed state, full public program/query gas, empty private transcript, native/recorded parity and upstream replay.
2. Capture walkerVectorElement for both Boolean branches and assert the actual vector values/state, Unit result, complete public program/query gas, privacy, native/recorded parity and replay.
3. Execute all43 pure literal-coercion exports directly through an explicit typed Rust dispatch against independent TS cases. Add width edges0/MAX for Uint8/32/128, distinct Field inputs, huge/Field-only equality true and false, aggregate Field-aligned hash paths, and actual curve coordinates. Reuse established subgroup/boundary evidence while independently checking every export result. No inference from constructor execution.
4. Replace the disconnected Field-only equality test with individual comparisons to the TS captured value.

Before (insufficient independent value link):
```rust
assert_eq!(callArgFieldOnlyLiteral()?, retFieldOnlyLiteral()?);
assert_eq!(oracle["fieldOnly"], EXPECTED_DECIMAL);
```
After (expected test shape):
```rust
assert_eq!(field_hex(callArgFieldOnlyLiteral()?), row("callArgFieldOnlyLiteral")["result"]);
assert_eq!(field_hex(retFieldOnlyLiteral()?), row("retFieldOnlyLiteral")["result"]);
```

The checked matrix identifies each source/export, exact case IDs, capture provenance, Rust assertion function and tested dimensions. Unknown and unreviewed dimensions remain explicit; no blanket all-path/proof/ledger claim follows from this delivery. No new cryptographic proof is required for unchanged generated code; public replay is distinct from proof/ledger acceptance.

### Alternatives and limits

Regex invocation matches are useful candidates but miss function-pointer dispatch and cannot establish asserted outcomes. Constructor state hashes prove the actual constructor expressions, not every pure export. Counting sibling-circuit proofs as direct proof coverage is rejected. A sweeping all37 coverage badge is deferred until every row has case-level review.

### Validation

Capture from unchanged generated TypeScript with pinned runtime; exact43-export set check and per-case typed Rust result assertions; both omitted stateful API tests with upstream replay; focused tests for the three affected packages, strict Clippy and source freshness/hash controls. Record actual failures and stop for review if an emitter/runtime defect is exposed rather than widening this test-only task. Signed conventional GPG+DCO local commit, no push/remote CI.

### Signed delivery

Implemented as signed GPG+DCO commit `9771cf919c2fdc98ddf57e20743e40fd800cfdd4`, based on ADR212 `4bb4faf3`. No Compact source, generated Rust, emitter or runtime changes. Schema20 and ABI49 stay unchanged.

The independent generated-TypeScript capture contains91 direct pure cases spanning all43 literal-coercion exports, plus ping and both walkerVectorElement Boolean branches. Its repeat capture is byte-identical. The direct Rust dispatcher asserts the exact export set and unique case IDs, then checks every typed result against TS. Native curve input coordinates are also compared to the captured point. The two stateful cases compare Unit result, exact serialized state and typed flag/vector, the complete ordered VM program, summed query gas, captured last-query gas, empty private outputs, and upstream replay state/effects.

The previous disconnected Field-only equality test was removed; each exported result is now independently anchored by the new direct test. Constructor metadata is not counted as execution.

`oracle_direct_behavior_review.json` is a reviewed45-export subset with94 case IDs, links and per-dimension scope. It freezes the exact source, Rust assertion, capture and script hashes. `check_oracle_acceptance.py` checks those identities and case links, requiring re-review on changes; it does not infer semantic coverage from source text. Actual assertions are exercised by the Rust tests. The broader203-row/37-source manual review remains explicitly incomplete in `${LOCAL_EVIDENCE}/compact-37-behavior-review-progress.json` and `${LOCAL_EVIDENCE}/compact-37-behavior-gap-report.md`. Function-pointer mixed-width comparisons are correctly classified as exercised.

Validation passed:17 Rust tests across the three affected packages, strict Clippy for all three, three fresh generated fixtures,37 pinned source-link checks plus reviewed-matrix identity checks, Rust and JS formatting. The17 Rust tests and identity checks were rerun at the signed head. No cryptographic proof run or new ledger-acceptance claim is included.

Receipt `${LOCAL_EVIDENCE}/compact-adr216-delivery-receipt.json`, SHA256 `5d4a628332309fa45698158b02986d7ccf1a64722d83a57209223393f167c2b7`. The first freshness invocation omitted the required `.compact` suffix and refused all requested basenames; its failure log is retained, and the corrected invocation checked all three successfully.


#### Root integration verified

Signed/DCO1ab0fffd passed focused three-source parity, oracle identity checks and strict three-package Clippy. Receipt:${LOCAL_EVIDENCE}/compact-1ab0fffd-integration-receipt.json. The compiler/runtime/backend trees are identical to db853a2b. This test-only delivery adds94direct cases for45reviewed exports; broader behavioral review remains partial. No new proof/ledger claim.
