---
id: RUST-ADR-0154
alias: ADR-0154
title: "Preserve unused Field ledger reads in unit returns"
date: 2026-10-05
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["recording", "lexical-scope", "unused-values", "cell"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: 51d6f3ef4bdf5a09982fef9ce4060f76d5d949c1a9902d13716fa0ae33fdf71d
---
# RUST-ADR-0154 — Preserve unused Field ledger reads in unit returns

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Accepted ordered unused Field bindings ending in Unit only when they contain an admitted Cell read, preserving observable reads despite unused values. Three original read calls are proved; pure-only, non-Field or unsupported operands remain outside this profile.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#258 closure](https://github.com/MediaNoxLabs/compact/issues/258#issuecomment-6017667293). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`f54f1292`](https://github.com/MediaNoxLabs/compact/commit/f54f12927f3ee1517b185958acb2243c9468cfb7). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
status: accepted
issue: https://github.com/MediaNoxLabs/compact/issues/258
```

## Historical decision and amendments

### Problem
The original PM-19252 example_seven, example_eight_a and example_eight_b have accepted TypeScript/native Rust implementations but no recorded proof API. Their final expression is a lexical Field binding whose body returns unit. Discarding the unused binding would incorrectly erase an observable ledger read.

### Before / after
Compact source remains unchanged:
```compact
export circuit test(a: Field): [] {
  const b = vara + disclose(a);
  return [];
}
```
Before: native execution works; recorded/observed calls report StateReturn::Expression unavailable.
After, conceptually:
```rust
let (frame, read) = ledger_slots::vara.record_read(frame)?;
let _bound_field = read + a;
frame.finish(())
```
The generated wrapper retains the normal typed API and delegates VM ownership to the ledger-8 runtime.

### Design and boundaries
Reuse typed Field expression lowering and lexical locals for ordered Field bindings ending in unit. Preserve reads and arithmetic even when values are unused. Check exact slot/index and declared types; retain fail-closed behavior for unsupported expressions. No runtime primitive, schema or ABI change is intended. Before broadening the implementation, inspect the original IR and validate its source behavior.

### Acceptance
Three original strict compilations; fresh independent TypeScript captures; native/recorded state, four gas dimensions, ordered public queries, effects and replay; pinned proof verification and ledger application; renderer guards, freshness and focused Clippy. Record exact signed delivery SHA and receipts here. No delivery claimed yet.



### Delivered — 2026-10-05

Verified GPG/DCO commit `8c2862ef` (integrated main as `f54f1292`) records ordered Field bindings ending in unit when the expression contains a Cell read. It reuses `field_expression`, lexical local binding and typed `CellSlot::record_read`; runtime/schema/ABI are unchanged. Non-Field bindings, no-read expressions and unsupported operands remain outside this slice. Renderer guards cover wrong slot/index/type, nonunit return, pure-only bindings and an unsupported hash binding.

The unchanged original example_seven, example_eight_a and example_eight_b now pass strict recording compilation: five total APIs available, including three newly supported read calls. Three generated crates are included in fixture freshness and local focused gating. Fresh TS captures exercise the addition read plus zero/populated public/private Cell reads. Each call has three public VM operations, zero private outputs, unchanged state and exact gas (read170000000,compute1249914853,written0,deleted0). Native,recorded and one-query replay agree, including observed Popeq values.

All three pinned ZKIR2.1.0 proofs verify and ledger8 validates/applies without state changes; artifacts `${LOCAL_EVIDENCE}/compact-adr154/{seven,eight-a,eight-b}-proof`, log `${LOCAL_EVIDENCE}/compact-adr154/proof.log`. The generic proof selector is `--unused-field-reads`; the broad source/proof gate now requires it. 123 renderer tests and two executing tests (five TS cases) pass; 151/151 fixtures fresh; all-target/all-feature targeted Clippy passes. Exact-head focused `${LOCAL_EVIDENCE}/compact-focused-8c2862ef/receipt.json` passes5/5 APIs. Main combined integration checks are separate. No push or remote CI.
