---
id: RUST-ADR-0140
alias: ADR-0140
title: "Record plain Merkle indexed hash placement"
date: 2026-10-05
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["recording", "merkle", "hash", "indexed-allocation"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: d3f9a6b8341c2ab2d174f2b7d6ebc2fd11ccde2b04c78417ebacab5b5448308f
---
# RUST-ADR-0140 — Record plain Merkle indexed hash placement

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Accepted plain Merkle indexed insertion of an already hashed Bytes32 value with an exact Uint64 position. Replacement parity and proof/application reuse the indexed hash program without double hashing; historic trees, unsupported expressions and invalid paths remain refused.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#242 closure](https://github.com/MediaNoxLabs/compact/issues/242#issuecomment-6017641619). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

### Original source metadata

```yaml
title: ADR-0140 — Record plain Merkle indexed hash placement
status: accepted
date: 2026-10-05
milestone: rust-backend-v2
```

## Historical decision and amendments

### Problem

The original `examples/rust_backend/merkle_tree_oracle.compact` exports `place_hash(hash: Bytes<32>, index: Uint<64>)`. Schema-12 IR contains a single `StateAction::MerkleInsertHashIndex(t, hash parameter, index parameter)`. The native generated call already uses ledger-8's indexed hash insertion, but the Rust capability report marks recording and its typed observed-call API unavailable at `actions[0]`. ADR-0137 added plain Merkle hash append; indexed placement remains a separate VM program and proof gap.

### Before and after

```compact
export circuit place_hash(hash: Bytes<32>, index: Uint<64>): [] {
  t.insertHashIndex(disclose(hash), disclose(index));
}
```

Before, generated consumers can call only `ledger_contract::place_hash(context, hash, index)`. After, they can call:

```rust
let recorded = ledger_contract::recorded::place_hash(context, hash, index)?;
let (initial, ordered_verify_ops) = recorded.public.into_parts();
let call = ledger_contract::recorded::Contract
    .place_hash_call(&observed, private_state, hash, index)?;
```

The emitter uses `ledger_slots::t.record_insert_hash_index(frame, hash, index)` after checking the declared plain tree, exact `Bytes<32>` and `Uint<64>` sources, slot index and physical path. The slot asks `RecordingFrame` to execute the same ledger-8 indexed hash Verify program as the native path. The supplied bytes are already the leaf hash; no second leaf hashing is allowed. No IR or runtime ABI change is expected.

### Guards and scope

Enable only plain `MerkleInsertHashIndex`, leaving the historic variant separate. Wrong field kind or slot index, nested physical path, non-Bytes32 hash, non-Uint64 position and unsupported conditional argument expressions must fail recording or compiler typing. Bounds remain enforced by `BoundedUint<{u64::MAX as u128}>` and the ledger query. Do not route through typed-leaf `insertIndex`, which would hash the digest again.

### Acceptance

Capture the original ledger-8 TypeScript `place_hash` call freshly at the existing full tree, replacing index 1. Generated native and recorded Rust must match TypeScript result, serialized state, four gas dimensions, ordered VM, private effects and independent Verify replay. Add positive and negative renderer tests, refresh the generated crate, check all fixtures and a focused local gate. Generate pinned ZKIR 2.1.0 keys and prove/verify/ledger-8 validate/apply the typed observed call, comparing applied state to native execution. Record the exact source-inventory delta and signed local commit here. No remote CI or push.

### Delivery

Recorded before implementation. Tracking issue and local evidence follow.

Tracking issue: https://github.com/MediaNoxLabs/compact/issues/242 (rust-backend-v2). Created before implementation.

### Local verification (2026-10-05)

- Original source IR retains one `MerkleInsertHashIndex` action with `Bytes<32>` and `Uint<64>` parameters. On the integrated ADR-0137 base, `place_hash` was unavailable; this slice changes the one-source inventory from 5/8 to 6/8 proof-required calls available. `reset_tree` and the remaining source gap stay explicit.
- Fresh original ledger-8 TypeScript capture adds only `nativeQueries.placeHashAt1`. At a full tree, the replacement has 18 ordered VM operations, zero private outputs, gas read 935000000 / compute 2277002266 / bytes written 1122 / deleted 1122.
- Generated native and recorded Rust agree on result, serialized state, all four gas dimensions and ledger effects. The recorded Verify program equals the TypeScript operation list; independent replay reaches the same state, effects and gas. All six Merkle fixture tests pass.
- Renderer accepts the exact plain tree, `Bytes<32>` hash and `Uint<64>` position. It rejects unsupported conditional position, mismatched widths, historic field and invalid physical path/slot. All 147 generated fixtures are fresh; formatting and diff checks pass.
- Pinned `midnight-zkir 2.1.0` generated keys for the original source. `place_hash_call` matched the manual recorded prototype, and a local proof verified, ledger-8 validated and applied it. The applied state equals native execution, with first-free index 2 in the proof scenario.
- Focused exact-head local gate follows the signed commit. No remote CI or push.

Signed local commit: `77e617a2bf32b18f547deef5f030b81b45b7583f` (`feat(rust-backend): record indexed Merkle hash placement`, GPG good, DCO present). Exact-head focused gate is in progress; no push or remote CI.
Exact-head focused gate passed for `77e617a2bf32b18f547deef5f030b81b45b7583f` with packaged compiler `${HISTORICAL_NIX_STORE}/1v8zashgm74hb9zmyy7mr38x8p6csxns-compactc/bin/compactc`: 1 original-source fixture, 6/8 recorded/observed, receipt `${LOCAL_EVIDENCE}/compact-focused-77e617a2/receipt.json`. Remaining source gaps are `reset_tree` and `known`. This focused receipt supplements the separate pinned proof run; it is not a full workspace gate.
