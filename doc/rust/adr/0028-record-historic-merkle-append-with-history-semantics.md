---
id: RUST-ADR-0028
alias: ADR-0028
title: "Record historic Merkle append with history semantics"
date: 2026-10-03
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["backend", "recording", "ledger-adts", "merkle"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: dd4680a05d2e48fbec11fd353bf3a6f51024013e86e48920041f2e24ef0bc1c3
---
# RUST-ADR-0028 — Record historic Merkle append with history semantics

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Accept recorded historic Merkle append through the historic branch of the canonical builder, including root-history effects. Do not substitute plain-tree semantics. The proof and source-size evidence applies to the selected append slices; other historic mutations, paths and leaf forms require their own admission evidence.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#128 closure](https://github.com/MediaNoxLabs/compact/issues/128#issuecomment-6017446201). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Referenced repository commits:** [`dc527e08`](https://github.com/MediaNoxLabs/compact/commit/dc527e088b24ac811a0ad38422c0c7927288fe49). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
adr: 28
status: accepted-partial
date: 2026-10-03
milestone: rust-backend-v2
issue: "128"
```

## Historical decision and amendments

### Problem

ADR-0027 makes a plain `MerkleTree.insert` replayable and provable, but the corresponding `HistoricMerkleTree.insert` remains native-only. Its ledger-8 VM program also updates root history, so routing it through the plain builder would silently omit public state. `examples/rust_backend/hmt_insert_oracle.compact` supplies the smallest typed historic append and an independent TypeScript capture.

### Before

```rust
// Native call exists, but no recorded::append is generated.
let step = crate::ledger_slots::t.insert(context, __compact_param_0)?;
```

### Decision and proposed after

```rust
// Generated recorded body for the full historic action.
let frame = crate::ledger_slots::t.record_insert(frame, __compact_param_0)?;
// Consumer obtains the historic verifying program and proof-ready trace.
let call = generated::ledger_contract::recorded::append(context, bounded::<255>(7))?;
```

`MerkleSlot<T, DEPTH, true>::record_insert` delegates to a historic frame operation that uses `leaf_hash_for` and the **Historic** branch of the same `merkle_insert_hashed_program` used by native execution. Its complete VM program includes root-history insertion. The emitter admits typed `StateAction::HistoricMerkleInsert` only for the matching historic declaration and fully supported circuit; indexed/hash/default/reset/read actions remain unavailable until separately proved.

### Alternatives and rationale

Routing historic append through the plain record method would produce an incomplete state change. Duplicating VM instructions in the emitter would risk byte-level drift. A single const-flag slot with specialized recording methods keeps declaration kind in Rust's type system and retains the ledger-8 runtime as the owner of state semantics.

### Emitter and runtime ownership

`recorded.rs` validates typed IR kind, index and root path, then emits Rust AST via `syn`. `slots.rs` exposes the historic-only forwarding method. `recording.rs` retains ordered Verify operations, gas and query effects. `ledger/merkle.rs` owns leaf hashing and the canonical Historic VM builder. No macro or new midnight-zk primitive. Private IR schema remains 8. A new public runtime/generated API bumps ABI 13→14 and regenerates fixtures; ABI-13 crates must rebuild from source.

### Verification and risks

Compare recorded Unit result, exact serialized state, root history, four-dimensional gas and complete VM operands to pinned independent ledger-8 TypeScript append capture. Replay, prove, verify, validate and apply at least one historic append offline. Check a packaged one-dependency consumer and negative wrong-leaf/unsupported-method examples, fixture freshness, full workspace and clean-source archives. Record exact signed/DCO commit, generated source impact and unresolved limits. No delivery is claimed yet; remote CI, release, wallet/node submission and other Merkle operations stay open.

### Tracking and delivery

- Issue: pending focused issue in MediaNoxLabs/compact.
- Milestone: [rust-backend-v2](https://github.com/MediaNoxLabs/compact/milestone/2).
- Parent issues: [#105](https://github.com/MediaNoxLabs/compact/issues/105), [#107](https://github.com/MediaNoxLabs/compact/issues/107), [#108](https://github.com/MediaNoxLabs/compact/issues/108), [#126](https://github.com/MediaNoxLabs/compact/issues/126), [#127](https://github.com/MediaNoxLabs/compact/issues/127).
- Delivery state: proposed, unpushed.

### Amendments

Append reviewed delivery evidence without replacing this proposal.

Issue assigned: [#128](https://github.com/MediaNoxLabs/compact/issues/128) in rust-backend-v2.



### Delivery amendment — 2026-10-03

Local conventional GPG-signed/DCO commit `dc527e088b24ac811a0ad38422c0c7927288fe49` delivers this slice on `codex/rust-backend-ast`, still unpushed. Focused [#128](https://github.com/MediaNoxLabs/compact/issues/128) is assigned to rust-backend-v2.

**Before:** `hmt_insert_oracle` generated only native `append`, so no public verifying program could be replayed or proved. **After:** ABI-14 generated Rust includes:

```rust
pub fn append<Private>(
    context: runtime::context::CircuitContext<Private>,
    __compact_param_0: runtime::BoundedUint<255>,
) -> Result<runtime::recording::RecordedCircuitResult<Private, ()>, runtime::CompactError> {
    let frame = runtime::recording::RecordingFrame::new(context);
    let frame = crate::ledger_slots::t.record_insert(frame, __compact_param_0)?;
    Ok(frame.finish(()))
}
```

The emitter now accepts typed root `HistoricMerkleInsert` only when the declaration is historic and index/path/value match; a circuit with any unsupported action is still omitted from the recorded module. The slot specializes `MerkleSlot<T, DEPTH, true>::record_insert`; `RecordingFrame::insert_historic_merkle` executes and retains `historic_merkle_insert_program`. That program hashes the typed leaf using `leaf_hash_for` and takes the Historic branch of the same `merkle_insert_hashed_program` used by native execution. It includes the root-history update, rather than reusing the plain ten-operation program. No new VM primitive, midnight-zk type, or proc macro; private IR schema remains 8. Runtime/generated public ABI changes 13→14, so consumers rebuild generated source.

**Parity and proof evidence:** The new historic fixture test compares recorded `append(7)` against pinned independent ledger-8 TypeScript: Unit result, exact serialized state, exact two-root history, all four gas dimensions and all 17 VM operations with operands. It independently replays the verifying program to the same state/history. The shared-runtime one-dependency consumer executes and replays historic recorded append. Negative examples reject a `bool` leaf (E0308), indexed `recorded::place`, and `recorded::forget_history`; no incomplete historic trace is published. The full `compactc --target rust --consumer --proof` gate generated the historic append artifacts, replayed/partitioned, proved/verified, validated and applied the transaction. The applied tree has first-free index 1, contains leaf 7 and retains two roots. Plain append and 52 earlier cases also pass, for 54 offline application calls.

**Local gates:** 54 renderer, 4 CLI and 2 historic fixture tests; 132/132 fresh compiler fixtures; 2 rejection probes; 37 pinned oracle sources; `cargo check --workspace --all-targets`; `cargo fmt --all -- --check`; staged diff checks. Clean-source `target/rust-runtime-abi14-clean.json` was written and verified for commit `dc527e08`, tree `36718be7`, `dirty=false`, with 8 macro and 229 runtime archive entries. The unrelated `doc/ledger-adt.mdx` edit was preserved byte for byte and excluded.

**Generated source impact:** `hmt-insert-oracle` adds 32 net lines for a new recorded module/handle; `merkle-path-witness` adds 17 net lines for `append_h` inside an existing recorded module; 130 other fixtures change only ABI assertions. This adds proof capability, not a source-size optimization. Indexed/hash/default insert, history reset, fullness/root reads, other leaf shapes, nested paths, remote CI, registry publication and wallet/node submission remain open. #128 stays open for those gates.
