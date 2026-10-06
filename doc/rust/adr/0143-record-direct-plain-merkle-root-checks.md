---
id: RUST-ADR-0143
alias: ADR-0143
title: "Record direct plain Merkle root checks"
date: 2026-10-05
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["recording", "merkle", "readonly"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: 513aea03b8bb99dcdc6f0c10f083478569318d7a5b4487790066dfd48b056f8f
---
# RUST-ADR-0143 — Record direct plain Merkle root checks

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Accepted an action-free direct plain-tree root check with an exact typed digest parameter. Matching and nonmatching outcomes are both proved without state change; historic membership and computed/effectful roots remain separate.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#245 closure](https://github.com/MediaNoxLabs/compact/issues/245#issuecomment-6017646458). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`67688226`](https://github.com/MediaNoxLabs/compact/commit/67688226bdc65c98a88af0c7821da1d155aa11ad) · [`d9fbd5d9`](https://github.com/MediaNoxLabs/compact/commit/d9fbd5d9b6e419303b84f417feeaa9ecdfacc19e). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
title: ADR-0143 — Record direct plain Merkle root checks
status: accepted
date: 2026-10-05
milestone: rust-backend-v2
```

## Historical decision and amendments

### Problem

The original `merkle_tree_oracle.compact` exports `known(root: MerkleTreeDigest): Boolean`. Schema 12 represents its result as `StateReturn::MerkleCheckRoot` with an exact typed parameter and no actions. Native Rust executes the query, but the recorded lowerer rejects this return. The runtime already records the same plain-tree check for witnessed `merkleTreePathRoot` calls.

### Before and after

```compact
export circuit known(root: MerkleTreeDigest): Boolean {
  return t.checkRoot(disclose(root));
}
```

Before: only native `known(context, root)` exists. After:

```rust
let frame = RecordingFrame::new(context);
let (frame, known) = ledger_slots::t.record_check_root(frame, root)?;
Ok(frame.finish(known))
```

The crate also exposes the typed observed `known_call` API.

### Decision

Admit only an action-free Boolean return that names the matching declared plain Merkle slot and an exact one-Field MerkleTreeDigest parameter. Reuse the ledger-8 typed slot and existing recording query. No runtime or wire-schema change. Historic root/history semantics remain a separate slice. Reject wrong slot/index, historic slot, wrong digest shape, computed or effectful root, and preceding actions.

### Acceptance

Capture original TypeScript true and false root checks after insertion; compare native and recorded result, serialized state, four gas dimensions, ordered VM including observed result, private outputs and Verify replay. Prove both outcomes through generated observed calls using pinned ZKIR 2.1.0 and ledger-8 validation/application. Add renderer negative guards and fresh fixture; update broad gate expectations. Track exact compiler source/inventory separately from semantic proof evidence.

### Delivery

Proposed before code. Local work only; signed GPG/DCO conventional commit and issue evidence follow.


Tracking issue: https://github.com/MediaNoxLabs/compact/issues/245 (rust-backend-v2), created before implementation.


### Local delivery

Conventional GPG-good/DCO commit `67688226` delivers typed direct `known` and observed-call APIs. Fresh TypeScript matching and nonmatching root queries each use 7 ordered VM operations, no private outputs, gas read 340000000 / compute 1425433517 / written 0 / deleted 0. Native/recorded result, state, effects, gas and replay match. Both generated observed outcomes proved with pinned ZKIR 2.1.0, verified, validated and applied on ledger-8; public state remains unchanged. Negative renderer guards cover computed roots, prior actions, wrong slot and type, and historic calls.

All 147 generated fixtures are fresh. Targeted Clippy passes. Frozen local compiler `${LOCAL_EVIDENCE}/compact-67688226/compactc` and pinned packaged Scheme passed the combined focused gate `${LOCAL_EVIDENCE}/compact-focused-67688226/receipt.json` (two sources, 12/18 proof-required APIs available). Repository inventory `${LOCAL_EVIDENCE}/compact-67688226-inventory.json`: 281/316 available, 35 known gaps, 25 unassessed. This is source/compiler availability plus the explicit behavioral evidence above, not full TS parity. Broad full checkpoint remains `d9fbd5d9`; integration gate will follow agent slices. No push or remote CI.
