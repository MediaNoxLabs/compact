---
id: RUST-ADR-0017
alias: ADR-0017
title: "Meter Map witness queries through a typed projection"
date: 2026-10-02
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["runtime", "witnesses", "gas"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: df7256a0179b5393e1f15ef3a1c0586636e0686d801089e1f7cdbf1163d0c615
---
# RUST-ADR-0017 — Meter Map witness queries through a typed projection

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Accept metered scalar Map witness projections delegated to existing canonical VM query functions, while retaining structural Map inspection for explicit local use. The initial ABI 8 delivery predates fallible witness propagation and other view families; its captured query costs and failure behavior do not establish arbitrary nested Map support.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#118 closure](https://github.com/MediaNoxLabs/compact/issues/118#issuecomment-6017428454). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`cf7ddf46`](https://github.com/MediaNoxLabs/compact/commit/cf7ddf4672b27b49da42f941853d726e4cac5f2b). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
adr: 17
status: accepted-partial
date: 2026-10-02
milestone: rust-backend-v2
issue: 118
```

## Historical decision and amendments

### Problem and evidence

Generated Rust `LedgerView::table()` returns a structural `MapView`. Its `member` and `is_empty` return plain `bool`; `size` and `lookup` decode state without running a ledger VM query. A TypeScript witness over the unchanged `witness_ledger_map.compact` source executes three queries before insertion (`member`, `isEmpty`, `size`) and four after insertion (those three plus `lookup`). The saved TypeScript capture records 510,000,000/765,000,000 readTime before/after, 3,756,612,609/5,095,502,024 computeTime, and zero bytes written/deleted. Rust currently reports no witness query cost for these reads. This extends [ADR-0015 — Meter witness ledger reads through typed projections](0015-meter-witness-ledger-reads-through-typed-projections.md) without changing its Cell/Counter/Set decision.

### Before and after Rust

```rust
// Before: generated witness view returns a structural projection.
let table = context.ledger.table().unwrap();
let present: bool = table.member(true); // direct state read, no VM gas
let empty: bool = table.is_empty();
let value = table.lookup(true)?; // direct state decode, no VM gas
```

```rust
// Decision: each successful read executes the canonical ledger-8 query.
let table = context.ledger.table().unwrap();
let present: bool = table.member(true)?;
let empty: bool = table.is_empty()?;
let value = table.lookup(true)?; // actual query cost is observed
```

### Decision and ownership

The runtime will provide `MeteredMapView<'a, K, V, D>` backed by the existing `WitnessReadMeter` and the declared physical ledger path. Its constructor validates that the path resolves to a Map in the meter's current state. `member`, `is_empty`, `size`, and `lookup` delegate to the existing `member_map`, `is_empty_map`, `size_map`, and `lookup_map` VM query functions; the meter accumulates returned `RunningCost` only after successful queries. The existing structural `MapView` remains available for explicit low-level inspection. The Rust emitter selects the metered projection from typed Map declarations and emits the physical path. It adds no opcode, direct cost constant, or new Compact IR variant.

The generated/runtime ABI advances 7→8 because witness-facing Map methods that previously returned `bool` now return `Result<bool, CompactError>`. The private compiler IR remains schema 8. User witness implementations must handle rejection explicitly until a separate fallible `Witnesses` trait design propagates `CompactError` through generated circuits.

### Alternatives and risks

Direct structural reads undercharge conditional and repeated witness queries. A fixed emitter surcharge cannot reflect how many reads the user witness performs or the true ledger cost. A thread-local meter hides ownership and is unsafe for nested/concurrent use. The typed projection maintains explicit effects and reuses ledger-8's VM programs. Failed `lookup` and invalid path behavior require focused checks: no successful value or accumulated cost may be reported on rejection. The API change requires regenerated fixture libraries and an ABI guard. List/Merkle witness views and cumulative gas-limit semantics remain separate work.

### Verification and delivery gate

- Recompile the unchanged TypeScript Map oracle and reproduce saved state, result, private FAB, and four-dimensional query capture byte for byte.
- Compare generated Rust before/after result, private state/FAB, exact tagged serialized state, and each prefix and total of TypeScript's ordered query costs.
- Check repeated reads, nested physical paths, absent lookup and invalid path behavior; test both view API errors and meter cost.
- Regenerate all affected libraries and pass renderer, runtime, workspace all-target compile, fixture/oracle/rejection, external consumer, packaged proof/application and clean-source package compatibility gates.
- Record local conventional signed/DCO commit and test evidence here, in the issue and in [Milestone 2 — ADR delivery map](references.md#private-note-08). Keep the issue open for remote CI and publication.

### Decision history

- 2026-10-02: Proposed after a fresh TypeScript Map witness query capture. Implementation and delivery gates pending. Tracking: [#118](https://github.com/MediaNoxLabs/compact/issues/118), parent [#116](https://github.com/MediaNoxLabs/compact/issues/116) / [#104](https://github.com/MediaNoxLabs/compact/issues/104), milestone [rust-backend-v2](https://github.com/MediaNoxLabs/compact/milestone/2).


### Delivery amendment — 2026-10-02: ABI-8 Map witness VM views

Local conventional GPG-signed/DCO commit `cf7ddf4672b27b49da42f941853d726e4cac5f2b` implements the decision. Before, generated `LedgerView::table()` returned structural `MapView`; `member`/`is_empty` returned `bool`, while `size`/`lookup` read state without query gas. After, the view returns `MeteredMapView`, and all four operations return `Result` from canonical ledger-8 VM queries. The runtime owns the metered wrapper, path validation, query execution and actual `RunningCost` accumulation; the emitter selects that wrapper from the typed Map declaration and its physical path. Structural `MapView` remains an explicit low-level API. Generated/runtime ABI advances 7→8, private IR schema remains 8, and all 131 generated fixture libraries were regenerated.

The fresh TypeScript compile of unchanged `witness_ledger_map.compact` reproduced the saved oracle capture byte for byte. Generated Rust matches the before/after result, private state and FAB atoms/alignment, exact tagged serialized state, and every ordered four-dimensional query-cost prefix. Before insertion the three queries total 510,000,000 readTime and 3,756,612,609 computeTime; after insertion a 255,000,000 readTime `lookup` brings the four-query totals to 765,000,000 and 5,095,502,024, with zero bytes written/deleted. An absent lookup errors without adding cost. Focused tests verify repeat charging, nested physical paths, invalid-path rejection and no failed-query accrual.

Six focused Map tests, 53 renderer/four CLI tests, the runtime suite, all-target workspace compile, 131/131 fixture freshness, 37 pinned oracle sources, two rejection probes, formatting and scoped diff checks pass. The Nix packaged external consumer and all 45 offline ledger-8 replay/proof/verification/validation/application cases pass. Clean-source manifest `target/rust-runtime-abi8-clean.json` was written and verified from commit `cf7ddf46` with `dirty: false`; the macro/runtime archives have 8/227 entries and compile after unpacking. The unrelated `doc/ledger-adt.mdx` edit was preserved byte for byte and excluded from the commit.

This is accepted-partial. The generated witness trait still returns `(Private, T)`, so a witness implementation handles `Result` itself rather than using `?` through the generated circuit. List/Merkle witness views, cumulative gas-limit semantics, remote CI, registry publication and wallet/node submission remain open. The branch is local and unpushed. Track Map delivery under [#118](https://github.com/MediaNoxLabs/compact/issues/118), parent [#116](https://github.com/MediaNoxLabs/compact/issues/116) / [#104](https://github.com/MediaNoxLabs/compact/issues/104), in [rust-backend-v2](https://github.com/MediaNoxLabs/compact/milestone/2).
