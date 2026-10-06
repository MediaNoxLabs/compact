---
id: RUST-ADR-0208
alias: ADR-0208
title: "Direct pure literal boundary and cryptographic coercion evidence"
date: 2026-10-06
publication_date: 2026-10-07
decision_status: "accepted-test-only"
topics: ["pure", "coercion", "crypto"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: aa584506aecc4c3a3da5bdf4f439899b54cb3b13bb879545adb6dc9ad328b4b6
---
# RUST-ADR-0208 — Direct pure literal boundary and cryptographic coercion evidence

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-test-only. Six direct corrected-TypeScript/Rust cases cover four original pure literal/hash/subgroup exports at meaningful boundaries. No source, emitter or runtime change, and pure results carry no transaction-proof claim.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#312 closure](https://github.com/MediaNoxLabs/compact/issues/312#issuecomment-6017759027). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`86cd2dd9`](https://github.com/MediaNoxLabs/compact/commit/86cd2dd9fc93014b99e714117c83eb6035e0aa52) · [`a7e14034`](https://github.com/MediaNoxLabs/compact/commit/a7e14034f6153212a356b47a1be73c72182e4fa5). These include historical prerequisites and probes, not only final delivery commits.

## Historical decision and amendments

Status: delivered locally on rust-backend-v2, 2026-10-06; awaiting root integration.

### Problem

`examples/rust_backend/literal_coercion_oracle.compact` exports 43 compiler-pure circuits, but its linked TypeScript capture and Rust test invoke only five selected pure exports. Source compilation proves shape acceptance, not value parity. The exact `2^248` Field-only call argument, Uint-to-Field FAB alignment in flat and nested persistent hashes, and subgroup identity through a large scalar lack direct source-export assertions. These are concrete semantic boundaries, not a request to duplicate all 43 tests.

### Existing transitive evidence and remaining gap

- `wide_field_literal` asserts a different Field constant above `u128`; it does not hit the `max-unsigned + 1` call-argument boundary.
- `field_cast_uint128` covers `2^80 + 7` through `Uint<128> as Field`; it does not cover a `Uint<8>` var-ref used as a `Field` atom inside a native aggregate argument.
- `persistent_hash` covers an already-typed `Vector<2, Field>`; `literal_coercion_oracle` already checks same-typed nested/default hashes. None checks a coerced Uint var-ref in flat or nested hash argument.
- `jubjub_arithmetic` checks small point multiplication, generator, reduction, and invalid scalar rejection. It does not assert the source's `(r + 1)/8` nested multiplication identity.
- The constructor's state-byte parity covers literals in ledger writes, not these pure return values.

### Decision and before/after example

Retain the original source and existing compiler/runtime. Extend its oracle capture and test with six direct cases from four exports:

```compact
callArgMaxUnsignedPlusOne()  // 2^248 as Field passed to idf
hashUintVarRefElem(x)       // x = 0 and 255
hashNestedUintVarRefElem(x) // x = 0 and 255
subgroupCheck()             // ecMul(ecMul(generator, (r+1)/8), 8)
```

Before: these exports merely compile into the generated Rust library. After: an independent corrected TypeScript capture invokes each source export and records exact Field/Bytes32/Jubjub values; Rust directly invokes each generated pure function and compares those values. For hashes, compute reference values with the corrected TypeScript runtime's explicit `CompactTypeVector(2, CompactTypeField)` and nested `CompactTypeVector(1, CompactTypeVector(2, CompactTypeField))`, plus `midnight_compact_runtime::natives::persistent_hash` over explicitly Field-typed `FixedVector` values. Assert hash output is distinct from a Byte-aligned value where meaningful, to catch a shared mistaken coercion. For the subgroup case, independently compare generated output to `ecMulGenerator(1)` in each runtime as well as cross-language coordinates. For the boundary, parse 2^248 from a decimal string into Rust `Field::from_le_bytes`, then compare exact value.

### Ownership and compatibility

Emitter: no change. Runtime: no change. IR schema/ABI: no change. The capture uses isolated corrected TypeScript runtime `${LOCAL_EVIDENCE}/compact-adr200-runtime` (ADR197 u128 descriptor, ADR198 scalar sampler, ADR200 reducer); preserve its revision/build receipt and exact generated contract provenance. Do not rebuild or mutate shared `runtime/dist`. Compiler-pure exports have `proof:false`: no recorded API, ZK key generation, proof, or ledger apply is applicable to this slice.

### Acceptance

1. Capture generated TypeScript pure results and independent runtime primitive references for six cases, retaining source, compiler, runtime package and revision provenance. No hardcoded expected digest copied from generated Rust.
2. Direct generated Rust assertions compare exact bytes, Field value and point coordinates to captured output and ledger-8 runtime primitive reference. Include both zero and max Uint8 hash inputs.
3. Focused literal-coercion package test and capture replay pass with owned warm Cargo target; strict Clippy and fixture freshness for this package pass. Existing five assertions and constructor byte parity remain.
4. Any parity mismatch is diagnosed as a separate implementation defect; this test-only ADR does not silently alter coercion semantics.

Issue: [MediaNoxLabs/compact #312](https://github.com/MediaNoxLabs/compact/issues/312), milestone `rust-backend-v2`.

### Local delivery

Conventional GPG-signed/DCO commit `90a6390d0f7536e567b030271d86ee1796dfb367` on `codex/adr208-pure-boundaries`. Exact-head receipt: `${LOCAL_EVIDENCE}/compact-adr208-delivery/receipt.json`. The unchanged source was compiled with the pinned Scheme20 compiler into `${LOCAL_EVIDENCE}/compact-adr208-ts`; its generated contract was linked only to isolated corrected `${LOCAL_EVIDENCE}/compact-adr200-runtime` (build receipt `${LOCAL_EVIDENCE}/compact-adr200-delivery/receipt.json`). Capture pins SHA256 for Compact source, generated JS, runtime index, built-ins and type descriptors. Six direct cases match generated TypeScript, typed runtime reference, generated Rust, and ledger-8 runtime primitive references. Byte-aligned hash counterexamples differ at zero and 255. Focused Rust test 2/2, strict Clippy, one Rust fixture fresh/zero stale, rustfmt, capture replay, and GPG verification passed. No runtime/emitter, schema, ABI, or proof change; pure operations remain `proof:false`. No push or remote CI.

### Pure boundary behavior integrated — 86cd2dd9 (2026-10-06)

ADR208 / #312 is integrated as GPG-signed/DCO `86cd2dd9` (original `90a6390d`). Six direct compiler-pure cases cover four original literal-coercion exports: exact 2^248 call-argument boundary; flat and nested Uint8-to-Field persistent hashes at 0 and 255; large subgroup scalar composition equal to the generator. Independent corrected-TypeScript captures include source/generated/runtime hashes; Rust checks the captured outputs and direct ledger-8 primitive results, including distinct Byte-aligned hash controls.

Root focused package tests pass (2/2) and strict package Clippy passes. Signed source delivery also passed exact capture replay, targeted fixture freshness and rustfmt. Receipt `${LOCAL_EVIDENCE}/compact-86cd2dd9-integration-receipt.json`; source receipt `${LOCAL_EVIDENCE}/compact-adr208-delivery/receipt.json`. This test-only slice changes no emitter, runtime, generated files, schema or ABI. No proof is applicable to these pure exports and no availability count increases. The previous same-head full/package checkpoint remains `a7e14034`; no redundant full rerun was performed. User documentation remains untouched; no push or remote CI.


Issue remains open under final milestone acceptance policy.
