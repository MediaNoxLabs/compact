---
id: RUST-ADR-0144
alias: ADR-0144
title: "Record historic Merkle indexed hash placement"
date: 2026-10-05
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["recording", "merkle", "history", "indexed-allocation"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: 57f5e947368db1bce133eaa15c2d05fb44a5da519e2e9be8865368688cb84d8a
---
# RUST-ADR-0144 — Record historic Merkle indexed hash placement

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Accepted historic indexed hash placement with the declared tree, typed digest/index and canonical root-history updates. Two insertion/replacement behavior captures and a focused proof are retained; the later resumed consumer phase is not represented as a whole-workspace rerun.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#246 closure](https://github.com/MediaNoxLabs/compact/issues/246#issuecomment-6017647962). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`4360d651`](https://github.com/MediaNoxLabs/compact/commit/4360d65180e3626a27e3128bc2610200f359f575). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
title: ADR-0144 — Record historic Merkle indexed hash placement
status: accepted
date: 2026-10-05
milestone: rust-backend-v2
```

## Historical decision and amendments

### Problem

The original `examples/rust_backend/hmt_insert_oracle.compact` exports `place_hash(hash: Bytes<32>, index: Uint<64>)` on `HistoricMerkleTree<3, Uint<8>>`. Its schema-12 action is `HistoricMerkleInsertHashIndex`. Native Rust uses ledger-8's historic indexed hash query, while the generated crate lacks recorded and observed-call APIs. The plain indexed hash program from ADR-0140 omits root-history updates; using it here would break state and proof semantics.

### Before and after

```compact
export circuit place_hash(hash: Bytes<32>, index: Uint<64>): [] {
  t.insertHashIndex(disclose(hash), disclose(index));
}
```

Before, consumers can call only `ledger_contract::place_hash(context, hash, index)`. After:

```rust
let recorded = ledger_contract::recorded::place_hash(context, hash, index)?;
let (initial, ordered_verify_ops) = recorded.public.into_parts();
let call = ledger_contract::recorded::Contract
    .place_hash_call(&observed, private_state, hash, index)?;
```

The emitter checks the exact historic tree declaration and slot index, one-segment path, `Bytes<32>` hash and `Uint<64>` position. It uses a historic-only typed slot, which calls `RecordingFrame::insert_historic_merkle_hash_index`. That frame executes ledger-8's `merkle_insert_index_hashed_program` with `MerkleHistory::Historic` and retains its ordered Verify operations, including root-map updates. The hash is already a leaf hash; no second leaf hash is computed. No new IR node or ABI is needed.

### Guards and scope

Only `HistoricMerkleInsertHashIndex` is enabled. Reject plain tree kind, wrong slot/path, wrong hash width, wrong position width and unsupported conditional arguments. The public slot method exists only for historic trees. `BoundedUint<{u64::MAX as u128}>` and ledger-8 enforce position bounds. The other historic operations remain separate.

### Acceptance

Capture original ledger-8 TypeScript `place_hash` at index 7 and replacement at index 1. Compare generated native/recorded result, serialized state, root history, all four gas dimensions, ordered VM, private outputs and independent Verify replay. Add renderer positive and negative guards, fresh generated fixture and capability inventory. Generate pinned ZKIR 2.1.0 keys; prove/verify and ledger-8 validate/apply the typed observed call, comparing applied state/history to native. Update broad compiler capability assertions and proof invocation. Run focused local-head gate; parent integration runs exact packaged gate. Signed GPG+DCO local commit only, no remote CI or push.

### Delivery

Decision recorded before code. Tracking issue and evidence follow.

Tracking issue: https://github.com/MediaNoxLabs/compact/issues/246 (rust-backend-v2). Created before implementation.

### Local verification (2026-10-05)

- Original source IR has one `HistoricMerkleInsertHashIndex` action with exact `Bytes<32>` and `Uint<64>` parameters. The historic source inventory improves from 4/8 to 5/8 proof-required calls available; `forget_history`, `reset_tree` and `known` remain explicit gaps.
- Fresh original ledger-8 TypeScript capture adds only `nativeQueries.placeHashAt7`, `nativeQueries.placeHashAt1` and the two corresponding history snapshots. Insert at index 7 uses 24 ordered VM operations, zero private outputs and gas read 1360000000 / compute 2735649718 / bytes written 1894 / deleted 1568. Replacement at index 1 uses 24 operations, zero private outputs and gas read 1360000000 / compute 2761242025 / bytes written 148 / deleted 1894.
- Generated native and recorded Rust match the two TypeScript states and root histories, four gas fields, effects and ordered VM programs. Independent Verify replay reaches the same states, histories, gas and effects. All six historic Merkle fixture tests pass.
- Renderer accepts the exact historic slot and typed arguments; it rejects wrong hash/position widths, plain tree kind and unsupported conditional position. All 147 generated fixtures are fresh; formatting, diff and broad-gate Python syntax checks pass.
- Pinned `midnight-zkir 2.1.0` generated keys in `${LOCAL_EVIDENCE}/compact-historic-indexed-hash-proof`. The generated `place_hash_call` matches the manual recording; the proof verified and ledger-8 validated/applied the transaction. Applied state, first-free index and root history equal native execution.
- Broad compiler capability assertions now expect `place_hash` recorded/observed and check its proof artifacts. The broad gate invokes a dedicated `--historic-merkle-indexed-hash` proof selector. A focused local-head receipt follows the signed commit; parent integration owns the packaged exact-head gate. No remote CI or push.

Signed local commit: `e2eab2c64bc5163a7abb8104feafb422f81b0258` (`feat(rust-backend): record historic indexed Merkle hash`, GPG good, DCO present). Focused local-head gate passed with frozen branch compiler and ledger-8 Scheme: 1 original-source fixture, 5/8 recorded/observed, receipt `${LOCAL_EVIDENCE}/compact-focused-e2eab2c6/receipt.json`. Parent integration will run packaged exact-head validation; this focused receipt supplements the separate pinned proof run.


### Main integration evidence — 2026-10-05

Main4360d651 passed the affected consumer/proof/ledger phase at ${LOCAL_EVIDENCE}/compact-consumer-4360d651/receipt.json, including this integrated slice. This closes the phase interrupted by the earlier stack overflow/harness TypeError. It does not claim a whole-workspace rerun at4360. Both Merkle fixtures additionally pass16/16 recorded at ${LOCAL_EVIDENCE}/compact-focused-4360d651/receipt.json; main renderer122 passes.
