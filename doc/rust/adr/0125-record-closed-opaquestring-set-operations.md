---
id: RUST-ADR-0125
alias: ADR-0125
title: "Record closed OpaqueString Set operations"
date: 2026-10-05
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["recording", "opaque-string", "set"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: 2c5e3beda385fef3ad3b40a8757895bb6d2e7ae06bf18db3332135e7492afbc9
---
# RUST-ADR-0125 — Record closed OpaqueString Set operations

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Accepted only the audited opaque-string parameter Set insertion and membership-return shapes through existing typed slots. Both original APIs have parity and proof/application evidence; arbitrary opaque expressions, removal and Map behavior are separate decisions.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#226 closure](https://github.com/MediaNoxLabs/compact/issues/226#issuecomment-6017614473). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`3c75882d`](https://github.com/MediaNoxLabs/compact/commit/3c75882db9d58475f582787c76c0f633ccc8640b) · [`e41ad486`](https://github.com/MediaNoxLabs/compact/commit/e41ad4869ede55236b5b23e161b1faa69aea3958). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
adr: 125
status: accepted
date: 2026-10-05
milestone: rust-backend-v2
issue: https://github.com/MediaNoxLabs/compact/issues/226
```

## Historical decision and amendments

### Problem

The checked TypeScript-positive opaque-string Set oracle has two proof-required native Rust exports but no recorded/observed API. Schema-12 reports `addName` at `StateAction::SetInsert` and `hasName` at `StateReturn::SetMember`. The source has one `Set<Opaque<"string">>` and each export has one OpaqueString parameter. Existing `SetSlot<OpaqueString>` and ledger-8 aligned encoding already support recorded Set insert/member in original Welcome; a broad OpaqueString recording guard currently refuses these unproved shapes.

### Before and after generated Rust

Before, `addName` and `hasName` are native-only. After, the generated recorded bodies use the existing typed slot:

```rust
let frame = crate::ledger_slots::names.record_insert(frame, name.clone())?;
let (frame, observed) = crate::ledger_slots::names.record_member(frame, name.clone())?;
```

These statements belong to separate circuit bodies. The generated crate also exposes `addName_call` and `hasName_call` for observed-state proof calls. Native method signatures remain source-derived.

### Decision and limits

Admit only a closed schema-12 Unit circuit with exactly one SetInsert of its OpaqueString parameter, and a Boolean circuit with no actions whose return is exactly SetMember of that parameter. Check the Set field, physical index, OpaqueString element type and expression source. Reuse existing `syn`-backed recorded lowering, runtime `SetSlot<OpaqueString>`, ledger-8 VM and recording frame. There is no IR schema or runtime API change. Changed action count, type/index/field or parameter source remains unavailable. Map insert, member and lookup, arbitrary opaque expressions and variable-length decoded values remain out of scope.

### Acceptance

Capture fresh original TypeScript constructor and before/add/after/other sequence. Compare generated native and recorded result/state, four gas dimensions, ordered VM, private effects and Verify replay. Compile pinned ZKIR 2.1.0 for both calls; prove, verify and ledger-8 validate/apply generated observed calls. Add renderer negative guards, generated fixture and checked inventory; retain Map gaps. Run focused local formatting, tests and Clippy. No push or remote CI.

### Tracking

- Milestone issue: https://github.com/MediaNoxLabs/compact/issues/226.
- Delivery: signed local commit `38646d20051ca638da13cdcd25ecfd243ee1d67a`, integrated as signed root `e41ad4869ede55236b5b23e161b1faa69aea3958`.


### Local validation (2026-10-05)

- Fresh original TypeScript before/add/after/other capture matches generated native and recorded Rust result, serialized state, four gas dimensions, all five ordered VM operations per call, zero private transcript outputs and Verify replay. Insert gas is readTime 170000000, computeTime 1335092941, bytesWritten 162, bytesDeleted 34. Membership of the registry key costs readTime 170000000 and computeTime 1252354311 with zero writes/deletes; the shorter other key has computeTime 1252345939.
- Pinned ZKIR 2.1.0 compiles addName at k=6/26 rows and hasName at k=6/48 rows. Both generated observed calls prove, verify, ledger-8 validate and apply; addName inserts registry-key, and hasName returns true on the inserted-key state.
- Exact source scope passes 1/1 TypeScript-positive source and 2 proof-required calls. The local full inventory after this slice has 195 sources, 935 declarations, 316 proof-required exports, 258 available and 58 missing; source-scoped opaque Set changes 0/2 to 2/2. An exact packaged baseline delta will be checked at root integration.
- Renderer negative guard rejects an extra insert and a non-parameter opaque membership expression. The existing Welcome check-in guard still passes after its expected diagnostic path was updated to the more precise nested argument path. The Map oracle remains unavailable at MapInsert actions[0] and Expression return_value.
- Focused renderer, generated fixture, source scope, inventory, rustfmt, Clippy and proof smoke pass locally. No push or remote CI.

### Integrated package receipt

- Exact packaged compiler for combined root `3c75882d` includes ADR-0125: `${HISTORICAL_NIX_STORE}/97ypd4b51ykv4jhgw6v0a7v5jdj0c9l2-compactc/bin/compactc`. Its full inventory receipt `${LOCAL_EVIDENCE}/compact-3c75882d-inventory.json` reports 262/316 proof-required exports available, 54 missing and 25 unassessed; all 147 fresh fixture checks pass. The package includes later independent slices, so its total is a combined checkpoint, while this ADR's source-scoped gain remains the two OpaqueString Set calls.
