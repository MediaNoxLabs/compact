---
id: RUST-ADR-0187
alias: ADR-0187
title: "Typed composite return recording"
date: 2026-10-05
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["recording", "composite", "proof"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: 0913c2c6dcf8f76a1a2262b2e51a8e311dbdb04017a099240305d73fef1fb064
---
# RUST-ADR-0187 — Typed composite return recording

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Typed composite return recording covers three nonempty stateful-struct calls under the documented unbalanced smoke policy. The empty snapshot refuses and the fourth planned Zswap composite remained a gap until ADR191.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#291 closure](https://github.com/MediaNoxLabs/compact/issues/291#issuecomment-6017722764). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`3fa3b0ba`](https://github.com/MediaNoxLabs/compact/commit/3fa3b0ba87400f9ebe0f31264637dac4e8ff16b6) · [`896cc8b9`](https://github.com/MediaNoxLabs/compact/commit/896cc8b92942a54644fa324604bf02ea26725c6b). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
status: accepted
date: 2026-10-05
```

## Historical decision and amendments

### Problem and history
ADR0179 preserved typed stateful struct member evaluation natively, but snapshot/reverse/nested remain unrecorded. Existing typed_plan already supports exact typed StructLiteral members, struct defaults, canonical Kernel.self queries, typed witnesses and lexical scope. ADR0182 supplies typed frame/result joins; ADR0186 fixes circuit-local witness eligibility. Avoid another contract-shaped renderer.

### Before / after
```compact
return Snapshot {
 first: disclose(next_value(1)) as Uint<128>,
 address: kernel.self(),
 second: disclose(next_value(2)) as Uint<128>
};
return Bundle { head: snapshot(true), tail: disclose(next_value(3)) as Uint<128> };
```
Before: native struct result with recording gap. After, shared typed steps evaluate members exactly once in normalized AST member order, use existing checked unsigned_cast_syntax and canonical Kernel.self recording, and assemble the typed struct. Composite If branches return matching frame/result types.
```rust
let (frame, first) = frame.try_witness_metered(/* tag 1 */)?;
let first = runtime::cast_unsigned::<U64_MAX, U128_MAX>(first)?;
let (frame, address) = frame.kernel_self()?;
let (frame, second) = frame.try_witness_metered(/* tag 2 */)?;
let result = Snapshot { first, address: typed_address(address), second: widen(second)? };
```

### Decision: emitter and runtime
A separate bounded composite profile admits typed structs/defaults, Boolean selection, checked unsigned casts, declared Uint64 witnesses with Uint8 arguments and Kernel.self. Source reversed field spelling still normalizes to declared field order; do not claim textual named-field order. Planned/createZswapOutput remains a recording gap.

Nested stateful expression-only helpers lower into the same recording frame. Audit an acyclic call graph, evaluate all caller arguments exactly once in caller order, create fresh isolated callee parameter/local scope, check argument/result types, and reject unsupported callee effects/actions. Do not use call_local for a helper containing Kernel.self: it cannot supply the helper public VM program. No new runtime API or ABI/schema change; target current schema20/ABI47 after ADR0184 integration.

### Empty branch boundary
snapshot(false) has zero VM operations and no witness. Preserve that exact transcript. ADR0184 independently demonstrated pinned JS ledger8.0.3 partition [undefined,undefined], a retained call in transaction construction, and wellFormed rejection: Calls cannot have empty guaranteed and fallible transcripts. Rust must reject preparation with PrepareCallError::EmptyTranscript. No dummy ops or proof claim for this branch.

### Evidence and acceptance
Unchanged stateful_struct_oracle source and fresh independent TS capture. Compare snapshot(true/false), reverse and nested native/recorded results, aligned outputs, private sequence, full VM programs, state/effects, query gas and replay. Prove/verify/ledger-apply three nonempty API calls; assert empty-branch preparation refusal. Negative tests cover mismatched branch/member/argument/result types, unsupported callee effects, recursion, lexical leakage and planned Zswap refusal. Keep existing profiles unchanged. Signed conventional DCO/GPG delivery, focused tests/Clippy/freshness/receipt, no push/remote CI.

### Validation evidence
Issue #291: https://github.com/MediaNoxLabs/compact/issues/291. Based on combined ABI47 refresh 3fa3b0ba; no new runtime/schema/ABI change. Fresh TS capture and both fixture tests pass all original native cases plus snapshot(true/false), reverse and nested recording/replay parity. snapshot(false) has no operations/private outputs and exact EmptyTranscript preparation refusal. Nonempty snapshot/reverse/nested each proved, verified and ledger-applied; artifacts ${LOCAL_EVIDENCE}/compact-adr187-proof and log ${LOCAL_EVIDENCE}/compact-adr187-proof.log retained. Backend 11 library +12 CLI +150 renderer tests pass. Helper tests cover acyclic transitive admission, fresh callee scope, argument arity/result/branch types, unsupported callee writes, and two effectful caller arguments emitted exactly once before callee witnesses in caller order. Planned output-intent remains native-only. Source guard and one-fixture freshness pass.


### Signed delivery receipt
Commit f228155b is conventional, GPG verified and DCO signed with full body/ADR/issue/test references. Strict targeted Clippy and formatting passed. Exact-head local receipt ${LOCAL_EVIDENCE}/compact-focused-f228155b-stateful-struct/receipt.json: 1 fixture, 3/4 recorded proof-required exports; planned remains the explicit gap. Frozen compiler ${LOCAL_EVIDENCE}/compact-adr187-compactc pairs with combined Scheme ${HISTORICAL_NIX_STORE}/1d5x83y7dgg6abzv0fzvna893na3nfjj-compactc-scheme-local-slice/bin/compactc-scheme. Proof artifacts retained separately ${LOCAL_EVIDENCE}/compact-adr187-proof. No push or remote CI.


### Main integration — 896cc8b9

GPG/DCO verified. Runtime ABI 47 / schema 20 unchanged. Frozen three-source gate passes 6/7 recorded APIs; the remaining planned composite uses Zswap intents and stays deferred. All three nonempty composite calls prove, verify and ledger-apply under the shared unbalanced smoke policy. Exact empty-snapshot refusal remains. Receipts: ${LOCAL_EVIDENCE}/compact-focused-896cc8b9-retry/receipt.json; ${LOCAL_EVIDENCE}/compact-integrated-adr187-proof.log. Exact full-source inventory: 347/360 available, 13 explicit gaps, 0 unassessed, no missing/unmatched rows or baseline drift (${LOCAL_EVIDENCE}/compact-896cc8b9-inventory.json).
