---
id: RUST-ADR-0218
alias: ADR-0218
title: "Direct pure ternary oracle behavior"
date: 2026-10-06
publication_date: 2026-10-07
decision_status: "accepted-test-only"
topics: ["pure", "ternary", "oracle"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: 8857ea0500fb2775a126e44e56e81693d5fdaa066fa15ff445e6d1c909a00660
---
# RUST-ADR-0218 — Direct pure ternary oracle behavior

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-test-only. Direct corrected-TypeScript and Rust tests exercise all 25 ternary compiler-pure exports over 83 selected value/failure cases. This is direct sampled behavior evidence, not exhaustive branches or transaction proof.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#322 closure](https://github.com/MediaNoxLabs/compact/issues/322#issuecomment-6017775750). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

## Historical decision and amendments

Status: approved bounded test-only scope, before implementation.
Date: 2026-10-06.

### Problem

The pinned ternary_cond_oracle has25 compiler-pure exports:23 explicitly pure declarations plus the actionless walkerReturnTail and walkerCallCtor. Existing Rust tests directly invoke four against local expected constants/errors; idf is executed transitively by checked stateful wrappers and constructor expressions. The other20 have no direct execution assertions. Analogous expressions in a constructor or sibling circuit are not execution of these exported pure APIs.

### Decision

Add independent original generated-TypeScript capture and typed direct Rust assertions for all25 exports, preserving unchanged Compact source, generated code, runtime, ABI49 and schema20. Reuse the pinned corrected compact-runtime0.16.101 and record source/generated-JS/runtime hashes. Keep this tranche to one source: the six call_arg direct-boundary gaps and three AssetRegistry pure assertion exports remain explicitly separate audit work because their values/declaration types and error cases require their own reviewed capture.

Exercise both conditional arms, all four nested choices, distinct scalar/aggregate inputs, widened integer boundaries, real curve/hash results, and exact lazy subtraction behavior. In particular, constUnannotatedSeqLifted(false,0) must succeed while true,0 fails; pick(0/5) must avoid subtraction while6/9 trap and10/255 succeed. assertArg must test selected-arm threshold boundaries, including failures. Preserve exact TS exception evidence and assert the corresponding specific Rust error variant; do not pretend language wrapper error strings are universally identical.

Before:
```rust
// Four pure exports checked locally; most exported entry points never invoked.
assert_eq!(returnTailNested(false, true)?.value(), 3);
```
After, expected test structure:
```rust
let actual = dispatch_typed_original_export(case)?;
assert_eq!(normalize(actual), independent_ts_case["result"]);
// Explicitly match UnsignedUnderflow / source AssertionFailed for negative cases.
```

A machine-readable reviewed subset maps source/export and unique case ID to captured input/outcome, explicit typed Rust assertion function and artifact hashes. Mark pure state/query/private/proof/ledger dimensions not applicable. This does not complete the broader37-source audit or claim all valid inputs.

### Alternatives and validation

Keep source availability and direct behavior separate; do not promote constructor metadata or regex matches into coverage. Tests and capture only: if an emitter/runtime discrepancy is found, retain evidence and return for review before code expansion. Validate exact generated pure-export set, repeat capture deterministically, run the affected Rust package and strict Clippy, check unchanged generated fixture/source identities, signed-head result receipt and GPG+DCO commit. No broad proof suite, network service, push or remote CI.


### Delivered behavior and controls

The direct capture now contains all 25 compiler-pure exports and 83 cases: 75 successful values and 8 failures. Both selected arms, four nested choices, distinct aggregates, widened integers, curve points and hash bytes are compared directly. Five source assertion cases retain `AssertionFailed("ternary assert")`; three subtraction cases retain typed `UnsignedUnderflow` and the exact distinct TypeScript assertion message. Lazy unselected paths succeed.

The new reviewed matrix pins each export and case plus source/capture/script/Rust hashes. The affected package passes 12 Rust tests, strict Clippy, repeated byte-identical TypeScript capture, one unchanged generated fixture, Rust/JavaScript formatting and the 37-source provenance checker. No Compact source, compiler, generated crate, runtime, ABI49 or schema20 change; no proof or ledger claim. Six call_arg and three AssetRegistry direct-boundary gaps remain separate, and the full 37-source behavior review is incomplete. Signed-head receipt follows.

### ADR218 signed delivery

Commit `31136879156d63f25e83fa8e5f224bfd05bbe98b` is GPG verified with DCO; checkout clean. All 25 compiler-pure ternary exports now have direct typed Rust calls against 83 independent original TypeScript cases (75 successes, 8 exact expected failures). The reviewed matrix binds case IDs and source/capture/script/test hashes.

Validation: 12 Rust tests and provenance/matrix checker rerun at signed head; strict affected-package Clippy, one unchanged generated fixture, byte-identical repeated capture and formatting passed.

Receipt: `${LOCAL_EVIDENCE}/compact-adr218-delivery-receipt.json`, SHA256 `c45abd08ec98ae31abc30e8b08c51c5a25a830737cee116000856f83cadf3251`. No source/compiler/generated/runtime changes, ABI49/schema20 unchanged; no proof or ledger claim. Broader 37-source audit remains partial, with six call_arg and three AssetRegistry direct boundaries explicitly deferred. Local-only; no push or remote CI.
