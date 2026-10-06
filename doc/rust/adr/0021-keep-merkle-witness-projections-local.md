---
id: RUST-ADR-0021
alias: ADR-0021
title: "Keep Merkle witness projections local"
date: 2026-10-02
publication_date: 2026-10-07
decision_status: "superseded"
topics: ["runtime", "witnesses", "gas", "merkle"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: 31f6ade763529cab0fb09e4677c1f8fcb4b457cb5bdaac71604ebafd250dbdf5
superseded_by: RUST-ADR-0022
---
# RUST-ADR-0021 — Keep Merkle witness projections local

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** superseded. Superseded during the design probe by ADR0022. Local Merkle projections remain valid, but the premise that charged isFull/checkRoot are circuit-only was false. Do not adopt this record as the complete witness API; retain the rejected premise and correction as history.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#122 closure](https://github.com/MediaNoxLabs/compact/issues/122#issuecomment-6017435295). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Superseded by:** [RUST-ADR-0022](0022-meter-merkle-witness-vm-reads-while-keeping-local-projections.md). The original premise and correction are retained below.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

### Original source metadata

```yaml
adr: 21
status: superseded
date: 2026-10-02
milestone: rust-backend-v2
issue:
```

## Historical decision and amendments

### Problem and evidence

Parent #116 lists Merkle witness-view metering as an open gap. The generated Rust `LedgerView::t()` currently returns an unmetered `MerkleTreeView` or `HistoricMerkleTreeView`. Before introducing a meter, inspect the ledger-8 Compact ADT definition. In `compiler/midnight-ledger.ss`, plain and historic `root`, `first_free`, `path_for_leaf`, and `find_path_for_leaf` are `js-only`; historic `history` is also `js-only`. They read the current state locally. `isFull` and `checkRoot` are `read` operations with explicit VM programs, and the Rust circuit emitter already routes them through `CircuitContext` and its canonical ledger queries. The generated witness view does not expose those read operations.

### Before and after Rust

```rust
// Current generated witness; local tree projection, no VM read.
let tree = context.ledger.t()?;
let path = tree.path_for_leaf(0, leaf)?;
let root = tree.root();
```

```rust
// Retained API. A test proves the witnessed circuit has zero query cost.
let tree = context.ledger.t()?;
let path = tree.path_for_leaf(0, leaf)?;
assert_eq!(Some(path.root()), tree.root());
// CircuitResult.gas_cost == RunningCost::ZERO when no VM operation follows.
```

A circuit call remains separately metered:

```rust
let result = contract.known(context, digest)?; // Compact `t.checkRoot(...)`
assert!(result.gas_cost.compute_time > RunningCost::ZERO.compute_time);
```

### Decision and ownership

Keep Merkle witness structural views local and unmetered for `js-only` operations. The runtime owns local upstream Merkle tree and state-value projections. The emitter returns typed views but does not synthesize VM reads for these operations. Canonical ledger-8 VM query functions remain the owner for `isFull` and `checkRoot` inside Compact circuits. No Rust ABI or private IR schema change is required. Document this distinction to remove a false M2 metering gap.

### Alternatives and risks

Wrapping `root` or `path_for_leaf` in a ledger VM query would invent gas absent from the TypeScript witness and may be impossible to match because there is no corresponding `path_for_leaf` VM opcode. Treating `isFull` and `checkRoot` as local witness methods would bypass their public VM programs. The risk is conflating a Rust-only local helper with a Compact circuit read; tests and docs should name the boundary clearly.

### Verification and delivery gate

- Capture a fresh TypeScript `merkle_path_witness` call with query count/cost after its insertion step and compare Rust result/FAB and zero call gas.
- Cover both plain and historic structural projections, including root/path/first-free and historic history, or explicitly leave historic verification open.
- Verify plain and historic circuit `isFull`/`checkRoot` still charge canonical query gas.
- Pass focused tests, 131 fixture freshness, oracle acceptance and relevant packaged replay/proof gate if emitted code changes.
- Append signed/DCO commit, evidence and remaining limits to this ADR, focused issue, and [Milestone 2 — ADR delivery map](references.md#private-note-08). Remote CI remains a separate closure gate.


### Superseded during design probe — 2026-10-02

Inspecting freshly generated TypeScript `ledger.t` showed that the same witness-visible object exposes `isFull` and `checkRoot` as charged VM queries alongside the `js-only` methods. The premise that VM reads are circuit-only was false. Keep the local-method finding, but do not adopt this ADR as the complete witness API. [ADR-0022 — Meter Merkle witness VM reads while keeping local projections](0022-meter-merkle-witness-vm-reads-while-keeping-local-projections.md) supersedes it under issue #122.
