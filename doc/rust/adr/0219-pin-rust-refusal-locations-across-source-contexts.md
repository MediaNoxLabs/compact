---
id: RUST-ADR-0219
alias: ADR-0219
title: "Pin Rust refusal locations across source contexts"
date: 2026-10-06
publication_date: 2026-10-07
decision_status: "accepted-test-only"
topics: ["compiler", "diagnostic", "source-span"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: 18e51ecb1583e6da78eb2867d0d569cf9e47e8b1127200f6d38a09c5d6a6f82a
---
# RUST-ADR-0219 — Pin Rust refusal locations across source contexts

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-test-only. Tests pin Rust refusal locations across source contexts using the actual compiler diagnostics. It is a diagnostic stability boundary, not new runtime or contract behavior.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#323 closure](https://github.com/MediaNoxLabs/compact/issues/323#issuecomment-6017777529). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`1704935b`](https://github.com/MediaNoxLabs/compact/commit/1704935bcd7dc5e97e144705ea5b90a3aada51d1) · [`78efe528`](https://github.com/MediaNoxLabs/compact/commit/78efe5285092d59dedcd2293b2f412ddd175ed78) · [`db853a2b`](https://github.com/MediaNoxLabs/compact/commit/db853a2b822050699f79822a994819bc889a849c). These include historical prerequisites and probes, not only final delivery commits.

## Historical decision and amendments

Status: accepted bounded test-only work, 2026-10-06. Milestone: rust-backend-v2.

### Problem

The public rejection gate pins four source refusals; two only require any line/column. Existing publication and strict capability checks cover late failure and prior output preservation, but constructor, nested expression, imported definition, witness result, exported struct field and ledger-map value locations are not explicitly covered. Exploratory immutable-compiler probes confirm the current backend reports exact locations for these contexts. This work retains those guarantees without changing admitted language behavior.

### Before / after

Before: `constructor(f: Field) { n = disclose(f as Uint<64>); }` fails, but no focused test checks that the diagnostic points to the cast. An imported helper failure could regress from its defining file to the import call without detection.

After: the gate checks exact diagnostic text and filename/line/column for each context, checks the TypeScript control (casts compile successfully; deliberately unknown opaque types are rejected there too), and checks both fresh rejection and preservation of a previously complete Rust output. Imported helper attribution uses the defining file. Generated output must never be partially published.

### Changes

Extend check_rejections.py with data-driven source files and expected locations; reuse existing snapshot/publication helpers. No emitter/runtime/generated code, schema or ABI change. Include the current two loose positions as exact positions if verified. Keep existing strict proof-bound capability/path and collision diagnostics tests. This bounded matrix does not enumerate every invalid program or claim all unsupported operations have exact expression spans.

### Validation

Run the focused rejection gate against the immutable compiler whose source trees match the root; run Python syntax/unit checks as relevant. Retain per-case diagnostics/receipt. Final full acceptance will include this gate. No remote CI or push.

Issue: https://github.com/MediaNoxLabs/compact/issues/323

The probes confirmed unknown opaque types are a shared target refusal, not a valid-TS/Rust-only unsupported feature. The type-context cases will be labeled shared rejection; nested/constructor/imported casts are actual Rust-only refusals of TS-accepted programs. The first exploratory target spelling `typescript` was rejected; the supported flag is `--target ts`.


### Delivered

### ADR218 / ADR219 verified — 78efe528 (2026-10-06)

Signed/DCO root1704935b integrates source31136879 (ADR218/#322):25 pure ternary exports,83 independently captured TypeScript cases (75 values/8 specific assertion or underflow failures). Focused one-source fixture/tests, matrix checks and strict package Clippy pass. Receipt:${LOCAL_EVIDENCE}/compact-1704935b-integration-receipt.json; source:${LOCAL_EVIDENCE}/compact-adr218-delivery-receipt.json. No emitter/runtime/generated source changes or new proof claim.

Signed/DCO root78efe528 delivers ADR219/#323: exact defining-file line/column refusals for nested expressions, constructors, imported helper definitions, witness result types, exported struct fields and ledger-map values. Casts are TS-accepted; unknown opaque tags deliberately exercise a shared target refusal. All six contexts verify fresh refusal and preservation of existing complete output. The four prior refusals plus existing strict capability, late publication, recovery and output serialization checks also pass. Receipt:${LOCAL_EVIDENCE}/compact-adr219-delivery-receipt.json. Gate ran immediately before commit; tested checker bytes equal signed commit. Immutable compiler/runtime/backend source equivalence to db853a2b verified.

These are focused test-only deliveries, schema20/ABI49 unchanged. Source availability remains385/386 pending signed ADR213. Broad37-source behavior review, ADR217 explicit trusted observation binding, final full/Nix and current-head live acceptance remain open. Last completed full/portable checkpoint3404c30c; no remote CI/push; user documentation preserved.
