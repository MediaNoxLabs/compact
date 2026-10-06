---
id: RUST-ADR-0220
alias: ADR-0220
title: "Direct pure call and registry boundaries"
date: not-recorded
publication_date: 2026-10-07
decision_status: "accepted-test-only"
topics: ["pure", "call", "registry"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: d78a2ea2c65603810bab6ab6f4e82f4f8569579e0e018b7dc36df824fddb82ad
---
# RUST-ADR-0220 — Direct pure call and registry boundaries

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-test-only. Direct pure-call and AssetRegistry guard cases complete a bounded reviewed matrix slice; later ADR221/222 rows are incorporated by a separate matrix-only follow-up. Captured pure results do not imply proof/ledger behavior or exhaustive source-path coverage.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#324 closure](https://github.com/MediaNoxLabs/compact/issues/324#issuecomment-6017779509). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

## Historical decision and amendments

Approved before implementation after ADR216/218. Test/capture/matrix/docs only; no emitter, runtime, generated code, ABI49 or schema20 changes.

### Why this is a distinct gap

Six exported pure APIs in call_arg_declared_type are only exercised transitively by already-checked stateful wrappers: idf, sumTup, vecFromPureBody, fieldOnlyFromPureBody, tupleIntoVec and vecIntoTuple. sumVec also has a direct Rust call, but only as a local expected value for impureConst; its standalone result is not independently compared to TypeScript. This is evidence of existing semantics, not seven wholly untested functions.

AssetRegistry's assertRecordFreshEnough, assertRecordClassKnown and assertGrantNotFuture have good success/error evidence through stateful callers, but no direct pure-export call. registrationGap already has independent direct forward/reverse checks and should remain classified as covered.

### Bounded plan

One shared capture script imports original generated pureCircuits from the two unchanged sources using corrected runtime 0.16.101. Explicit typed dispatch in one new test file per package compares captured outcomes. Preserve exact source/generated/runtime/script/test hashes and unique case IDs. No constructor expressions counted as export calls.

For call-argument source, capture all seven pure exports: idf at 0,1,2^200; sumVec and sumTup at distinct/reversed pairs (0,1), (1,0), (7,2^200); direct no-arg calls to the four body/bridge functions. This is 13 small cases, including actual hash outputs and wide Field values.

For AssetRegistry, capture three pure assertions with actual typed record/grant/enum values: current==registered, exact max-age boundary, one beyond, unchecked old record, future even when unchecked, zero-age and u64MAX boundaries; all four AssetClass variants; grant earlier/equal/future at ordinary and u64MAX values. Roughly17 cases, finalized explicitly in the capture. Assert specific AssertionFailed variants and exact messages. Opaque strings and byte/address fields remain real structurally distinct values even where the source only reads one member.

Before: wrapper state is checked, but exported pure facade is not independently called.
After: every listed export is directly invoked and its typed result or exact source assertion failure is tied to an independent TS case. The reviewed matrix continues to distinguish this case-level subset from the broader inventory review.

### Validation and limits

Affected two packages only: Rust tests, strict Clippy, byte-identical repeat capture, unchanged fixture freshness, reviewed matrix identity/links and formatting. Signed GPG+DCO commit with explanatory body; local only. No cryptographic proof or ledger claim for pure calls. AssetRegistry.close's missing independent TS capture and selected false-branch gaps remain separate reviewed work, not quietly declared covered.

### Approved additions

Include independent AssetRegistry.close TypeScript/native/recorded success and repeated-close refusal if supported by the existing capture harness without emitter/runtime changes. Preserve the complete 37-source manual inventory review durably in the repository with reliable repository-relative evidence paths, artifact hashes, explicit per-dimension limits and no all-path/proof claims.


### Implemented cases and review scope

ADR220 / MediaNoxLabs issue324 now contains 13 direct cases for all seven call-argument pure exports and 17 cases for three AssetRegistry pure guards (24 successes and six source assertion failures). The independent original close capture fits the same harness: native and recorded success compare complete state, private output/order, summed query gas and normalized full VM; upstream replay matches state/effects. Repeated close fails with the exact source assertion before another witness call.

The full 37-source manual inventory review is now durable as tools/compact-rust-backend/oracle_behavior_review.json and .md, with 203 rows, repository-relative evidence paths, pinned hashes and explicit dimensions/limits. Complete inventory review does not imply complete behavior, branch, proof or ledger coverage. ADR221 owns separate sampled-case followups and will report reviewed matrix deltas.

First controls passed: all 29 tests in two affected packages, strict Clippy, two unchanged fresh generated fixtures, deterministic repeated TS capture, formatting and provenance/matrix integrity. Signed-head receipt follows; no source/emitter/runtime/generated or ABI49/schema20 change.

### ADR220 signed local delivery

Commit `a57e7ee89496855693383676899fd777fa169b00` is GPG verified with DCO and explanatory body; checkout clean.

- All seven call-argument pure exports: 13 direct independent TS cases.
- Three AssetRegistry pure guards: 17 cases, including six exact source assertion failures.
- AssetRegistry.close: independent native/recorded success state, private values/order, summed gas, normalized full VM and replay; exact repeat-close refusal before another witness call.
- Complete manual inventory review is durable in `tools/compact-rust-backend/oracle_behavior_review.json` and `.md`: 37 sources / 203 rows, pinned evidence hashes and explicit dimensions/limits. This is not complete behavioral, branch, proof or ledger coverage.

All 29 tests and provenance/matrix checks passed again at signed head. Strict two-package Clippy, two unchanged generated fixtures, deterministic capture repeat and formatting passed. Receipt `${LOCAL_EVIDENCE}/compact-adr220-delivery-receipt.json`, SHA256 `c9b5ab63b0036242330ae8bac59b74e8d5bfa73476de2c8dc0bdd5a1395be39f`.

No compiler/runtime/generated/source changes; ABI49/schema20 unchanged. No proof/ledger claim, push or remote CI. ADR221 and ADR222 will report independent sampled-branch/trace deltas for matrix maintenance.

### Reviewed matrix follow-up for ADR221/222

Signed matrix-only commit `1aaba070d56bd7c168069fccd8c4a13bfa1f2252` (GPG+DCO verified) incorporates ADR221 seven sampled cases plus ADR222 nine direct recorded trace cases. ADR221 full-program correction `38702dd9` is included. The combined ternary test is verified as corrected signed221 source plus the byte-identical ADR216 appended block; both affected matrices now pin its integrated hash.

All 11 export deltas /16 case identities match exact captured artifacts, and the signed-head 37-source/evidence checker passes. No new builds were needed for this metadata review; source deliveries retain their own tests/Clippy receipts and root owns combined validation. Complete inventory review remains distinct from all-path/proof/ledger coverage.

Receipt `${LOCAL_EVIDENCE}/compact-adr221-222-matrix-receipt.json`, SHA256 `a3ca0f956b61287240e849c51f8132ffb4643cd08871e9473975bd8eaa680bcd`. Cherry-pick only this matrix commit after the source deliveries. No push or remote CI.
