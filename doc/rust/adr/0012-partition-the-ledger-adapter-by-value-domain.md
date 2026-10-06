---
id: RUST-ADR-0012
alias: ADR-0012
title: "Partition the ledger adapter by value domain"
date: 2026-10-02
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["architecture", "generated-api"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: 149a5406213e371361ecc9622314ebd540b509793711cb9331605639ba0a17d2
---
# RUST-ADR-0012 — Partition the ledger adapter by value domain

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Accept partitioning canonical ledger VM builders into Cell, Counter, collection and Merkle modules behind the existing public facade. The delivered refactor preserves generated paths and semantics; it does not imply smaller modules forever, new primitives, or package publication. Historical local and package checks retain their own scope.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#113 closure](https://github.com/MediaNoxLabs/compact/issues/113#issuecomment-6017419610). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Referenced repository commits:** [`716bb0f6`](https://github.com/MediaNoxLabs/compact/commit/716bb0f649c8d7633f43302541676ddbbb9a237d). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
adr: 0012
status: accepted-partial
date: 2026-10-02
milestone: rust-backend-v2
issue: https://github.com/MediaNoxLabs/compact/issues/113
```

## Historical decision and amendments

### Problem

`runtime-rs/src/ledger.rs` is 2,172 lines and owns Cell, Counter, Set, Map, List, and two Merkle tree families alongside state views and constructor helpers. Generated code already calls typed slots and the recording frame, but reviewers still have to find protocol-critical VM instruction sequences inside this one file. The compiler's `suppress-null`/`suppress-zero` rules and List `Concat` alignment bug were only caught at proof time; a clear boundary around each program family will make those sequences easier to review without changing contract behavior.

### Before

```rust
// runtime-rs/src/ledger.rs contains all these unrelated programs:
pub(crate) fn cell_write_program<T: CellValue, D: DB>(...) -> Vec<Op<ResultModeVerify, D>>;
pub(crate) fn list_head_program<T: CellValue + Default, R: ResultMode<D>, D: DB>(...) -> Vec<Op<R, D>>;
pub(crate) fn set_reset_program<D: DB>(...) -> Vec<Op<ResultModeVerify, D>>;
pub fn historic_insert_index<T: CellValue, D: DB>(...) -> ...;
```

### Decision and after

Keep `runtime::ledger` as the stable public facade and move complete domain sections, including their canonical VM program constructors and native execution wrappers, into sibling internal modules. Start with `collections` (List/Map/Set) and `merkle` (current and historic trees), then move Cell and Counter operations to their own files. Re-export the existing public functions and crate-visible programs from `ledger`; generated crates, `CellSlot`/`MapSlot`/`ListSlot`, and `RecordingFrame` keep exactly the same paths and types.

```rust
// Public call sites remain unchanged:
let (frame, head) = ledger_slots::items.record_head::<Maybe, _, _>(frame)?;
let program = ledger::list_head_program::<Field, ResultModeGather, _>(index, ());
// Ownership after extraction: runtime-rs/src/ledger/collections.rs.
```

### Alternatives and rationale

Splitting only the file with `include!` would leave all private helpers in one implicit namespace and make imports harder to audit. Duplicating VM programs in `recording.rs` or generated contract code would make native and recorded paths diverge. The chosen Rust modules keep compiler-derived paths and Midnight primitives shared, while giving each family one reviewable owner. Public API and generated ABI remain unchanged.

### Emitter and runtime ownership

This is a runtime layout decision. The emitter, typed IR schema, generated crate source and proc macros do not change. `ledger.rs` retains common `CellValue`, path and state helpers; internal modules own the moved function implementations. `ledger.rs` explicitly re-exports stable public APIs and the crate-visible program builders used by `RecordingFrame`. The emitted VM operations, gas accounting and proof adapter must remain byte-for-byte equivalent.

### Verification and risks

Before/after Rust call sites are the same; compare existing runtime tests, generated fixture tests, `cargo check --workspace`, and the packaged compiler/consumer/proof gate with 44 offline ledger-8 calls. A module extraction can accidentally hide a function, change a re-export, or split a helper from its callers. Compiler errors and the external consumer gate check API visibility; replay/proof/application checks catch changed program semantics. The exact gate results and local commit belong in a dated delivery amendment.

### Tracking and delivery

- Focused issue: [#113](https://github.com/MediaNoxLabs/compact/issues/113), linked to parent [#106](https://github.com/MediaNoxLabs/compact/issues/106).
- Milestone: [rust-backend-v2](https://github.com/MediaNoxLabs/compact/milestone/2).
- Local commit: pending implementation and validation; branch remains local.
- Delivery state: proposed.

### Amendments

Append dated delivery evidence and preserve this proposal rationale.



### Delivery amendment — 2026-10-02: four runtime domains behind one facade

**Before/after source and API.** Local conventional GPG-signed/DCO commit `716bb0f649c8d7633f43302541676ddbbb9a237d` moves the existing implementations from `runtime-rs/src/ledger.rs` into `runtime-rs/src/ledger/{cell,counter,collections,merkle}.rs`. The facade fell from 2,172 to 278 lines and now names each exported public function/type and crate-visible VM program builder explicitly. These are source line counts, not performance measurements. Call sites remain unchanged:

```rust
// After: stable facade in runtime-rs/src/ledger.rs
mod cell;
mod collections;
mod counter;
mod merkle;
pub use cell::{constructor_cell, read_cell, /* ... */ write_cell};
pub(crate) use collections::{list_head_program, map_insert_program, set_reset_program, /* ... */};

// Existing generated/runtime callers still compile:
let frame = crate::ledger_slots::items.record_push_front(frame, item)?;
let program = ledger::set_reset_program::<DefaultDB>(&[index]);
```

The examples abbreviate only the export list. `collections.rs` owns Set/Map/List witness views, constructors, native wrappers and shared VM programs; `merkle.rs` owns current/historic views, constructors and VM programs; `cell.rs` owns typed Cell codec/query/write programs; `counter.rs` owns Counter constructor/read/update program. `ledger.rs` retains `CellValue`, physical `LedgerPath`, shared path/state helpers and the explicit stable facade. No generated emitter, IR, macro, runtime ABI, storage representation, gas model or VM opcode sequence changed.

**Verification.** `cargo test -q -p midnight-compact-runtime` passed the full runtime suite; `cargo check -q --workspace` passed; all 130 generated fixture outputs matched (0 stale/failed). The packaged `compactc --target rust --consumer --proof` gate rebuilt the aarch64 Darwin compiler, compiled separate consumers, and replayed, partitioned, proved, verified, validated and applied all 44 existing offline ledger-8 call cases, including `tiny` present/absent branches. The macro and runtime `.crate` release rehearsal passed from unpacked packages; the runtime archive now includes all domain files (225 entries). `add_headers.py --validate` reported 0 missing across 1,461 files; Rust format and staged diff checks passed. The commit has a good GPG signature and DCO trailer.

**Limits.** This establishes API-preserving local source ownership and offline behavior, not remote CI or a published crate. The `collections` and `merkle` modules remain sizeable and may need finer internal submodules if future changes justify them. Issue [#113](https://github.com/MediaNoxLabs/compact/issues/113) stays open for clean remote CI; parent [#106](https://github.com/MediaNoxLabs/compact/issues/106) retains package publication, provenance and tagged release. The branch remains local. The initial proposal's pending-test text is historical; this amendment is the verified local result.
