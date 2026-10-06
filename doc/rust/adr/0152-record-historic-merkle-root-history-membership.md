---
id: RUST-ADR-0152
alias: ADR-0152
title: "Record historic Merkle root-history membership"
date: 2026-10-05
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["recording", "merkle", "history", "readonly"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: dc658a64a8301ce28db06aa872cf2fa6b88fde22cc14136c21ddbf901db38f94
---
# RUST-ADR-0152 — Record historic Merkle root-history membership

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Accepted by signed delivery and integration despite proposed frontmatter: historic root membership uses the history map, not plain current-root equality. Six behavior cases and historic-true/forgotten-false proofs are retained; Gather null Popeq and concrete observed Verify output are explicitly distinguished.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#256 closure](https://github.com/MediaNoxLabs/compact/issues/256#issuecomment-6017663937). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`4360d651`](https://github.com/MediaNoxLabs/compact/commit/4360d65180e3626a27e3128bc2610200f359f575). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
title: ADR-0152 — Record historic Merkle root-history membership
status: proposed
date: 2026-10-05
milestone: rust-backend-v2
```

## Historical decision and amendments

### Problem

`examples/rust_backend/hmt_insert_oracle.compact` exports `known(root)` using `HistoricMerkleTree.checkRoot`. Native Rust follows ledger-8 history-map membership, but the generated crate lacks a recorded and observed proof call. A historic root need not be the current root: old roots remain known after inserts, `resetHistory` forgets old roots, and `resetToDefault` forgets the populated root while retaining the blank root. The plain Merkle `known` implementation (ADR-0143) compares only the current root and cannot be used as a substitute.

### Before and after

```compact
export circuit known(root: MerkleTreeDigest): Boolean {
  return t.checkRoot(disclose(root));
}
```

Before, `ledger_contract::known(context, root)` is native only. After:

```rust
let recorded = ledger_contract::recorded::known(context, root.clone())?;
let call = ledger_contract::recorded::Contract
    .known_call(&observed, private_state, root)?;
```

The emitter admits a direct `HistoricMerkleCheckRoot` return only with no state actions, a declared historic slot at the exact index and one-segment physical path, Boolean result, and an exact `MerkleTreeDigest { field: Field }` parameter. The typed historic slot records a `Gather` history-membership read and converts its last Boolean event to an ordered `Verify` program, using the same ledger-8 primitive as native. There is no state write.

### Guards and scope

Reject plain Merkle fields, wrong index/path, wrong digest structure or source, computed/effectful roots, and surrounding writes. The result is public Boolean; no private transcript output. The Verify program contains the observed Boolean so ledger proof validation checks the actual history map. This is separate from the plain current-root comparison.

### Acceptance

Refresh original ledger-8 TypeScript capture for current, historic, missing, forgotten, and reset-root cases. Compare native/recorded result, unchanged serialized state, all four gas dimensions, effects, ordered VM, private outputs, and independent Verify replay for true and false. Add positive/negative renderer guards and generated fixture tests. Generate pinned ZKIR 2.1.0 keys; prove/verify and ledger-8 validate/apply both outcomes, asserting unchanged state. Broad capability/consumer/proof gate must report 8/8 historic circuits recorded. Run exact-head focused local gate. Parent integration owns packaged gate. Conventional signed GPG+DCO local commit; no push or remote CI.

### Delivery

Decision recorded before implementation. Issue and evidence follow.

Issue: https://github.com/MediaNoxLabs/compact/issues/256

### Delivery (2026-10-05)
Verified GPG+DCO local commit: 7e3f60b4d513ce79f157d4b2a819a341c8bbad0b. Issue #256. Fresh original ledger-8 TypeScript capture covers six cases: initial/current true, prior historic true, forgotten initial false, retained current true, old root after resetToDefault false, blank root after resetToDefault true. Each uses the six-operation history-map query. TypeScript Gather leaves the final Popeq result null; the generated Verify trace fills that slot with the observed Boolean. The preceding operations, results, state, all four gas values, effects, private outputs and replay agree. Nine HMT tests, 117 renderer tests, and 147 generated fixture checks pass. Pinned ZKIR 2.1.0 proofs for historic true and forgotten false verified; ledger-8 validated/applied both without state change. Exact-head focused gate passed with all 8/8 historic circuits recorded: ${LOCAL_EVIDENCE}/compact-focused-7e3f60b4/receipt.json. Proof artifacts: ${LOCAL_EVIDENCE}/compact-historic-known-proof. No push or remote CI.


### Main integration evidence — 2026-10-05

Main4360d651 passed the affected consumer/proof/ledger phase at ${LOCAL_EVIDENCE}/compact-consumer-4360d651/receipt.json, including this integrated slice. This closes the phase interrupted by the earlier stack overflow/harness TypeError. It does not claim a whole-workspace rerun at4360. Both Merkle fixtures additionally pass16/16 recorded at ${LOCAL_EVIDENCE}/compact-focused-4360d651/receipt.json; main renderer122 passes.
