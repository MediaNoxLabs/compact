---
id: RUST-ADR-0148
alias: ADR-0148
title: "Record historic Merkle resetHistory with current-root retention"
date: 2026-10-05
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["recording", "merkle", "history", "reset"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: 81c350258d03bdc1ec0f1f4aa9ef695e53e5fac79fbd08170e34fc577ad1b7a4
---
# RUST-ADR-0148 — Record historic Merkle resetHistory with current-root retention

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Accepted by the later signed delivery and integration despite proposed frontmatter: historic resetHistory drops old roots while retaining the populated tree and exactly its current root. Original trace/gas and proof/application confirm this distinction from resetToDefault.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#250 closure](https://github.com/MediaNoxLabs/compact/issues/250#issuecomment-6017654411). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`4360d651`](https://github.com/MediaNoxLabs/compact/commit/4360d65180e3626a27e3128bc2610200f359f575). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
title: ADR-0148 — Record historic Merkle resetHistory with current-root retention
status: proposed
date: 2026-10-05
milestone: rust-backend-v2
```

## Historical decision and amendments

### Problem

The original `examples/rust_backend/hmt_insert_oracle.compact` exports `forget_history()`, implemented by `t.resetHistory()` on a `HistoricMerkleTree<3, Uint<8>>`. Schema-12 IR has a single `HistoricMerkleResetHistory` action. Native Rust uses ledger-8's reset-history VM program, but the generated crate does not expose a recorded or observed proof call. The operation must keep the current root while dropping old roots; a plain Merkle reset-to-default changes the tree and is a separate operation.

### Before and after

```compact
export circuit forget_history(): [] {
  t.resetHistory();
}
```

Before, `ledger_contract::forget_history(context)` is native only. After:

```rust
let recorded = ledger_contract::recorded::forget_history(context)?;
let (initial, ordered_verify_ops) = recorded.public.into_parts();
let call = ledger_contract::recorded::Contract
    .forget_history_call(&observed, private_state)?;
```

The emitter admits only the exact `HistoricMerkleResetHistory` action on a declared historic slot at a matching index and one-segment path. The historic slot calls a recording frame operation that executes the same ledger-8 `historic_reset_history_program` as native execution and retains the ordered Verify instructions. No new IR node or runtime ABI is needed.

### Guards and scope

Reject a plain Merkle slot, wrong field index or physical path, and unsupported surrounding actions. Keep this operation distinct from `HistoricMerkleResetToDefault` and plain `MerkleResetToDefault`. The generated typed observed call has no public input and no private transcript output. The runtime VM program is the canonical source of state and gas semantics.

### Acceptance

Fresh original ledger-8 TypeScript capture of `forget_history` after several root changes. Compare generated native/recorded result, serialized state, retained current root, deleted old roots, all four gas dimensions, ordered VM, private outputs and independent Verify replay. Add positive and negative renderer guards, generated fixture test, broad capability and consumer expectation updates. Generate pinned ZKIR 2.1.0 keys; prove/verify and ledger-8 validate/apply a typed observed call from a state with old roots, comparing applied state/history to native. Run focused local-head gate; parent integration owns packaged exact-head gate. Signed GPG+DCO local commit only, no remote CI or push.

### Delivery

Decision recorded before implementation. Tracking issue and evidence follow.


Issue: https://github.com/MediaNoxLabs/compact/issues/250

### Delivery (2026-10-05)
Local signed commit: c4c97710aecd43d6fa592c0f3388aa79ddbe5631 (GPG valid; DCO Signed-off-by). Issue #250. Fresh TypeScript capture equals the committed ledger-8 oracle fixture, including the 8-operation resetHistory VM program and all four gas values. Seven HMT integration tests, 116 renderer tests, and 147 generated fixture checks passed. Pinned ZKIR 2.1.0 proof verified; ledger-8 validated and applied forget_history against a tree with old roots, retaining exactly its current root. Focused exact-head parity gate passed with 6/8 historic circuits recorded: ${LOCAL_EVIDENCE}/compact-focused-c4c97710/receipt.json. Proof artifacts: ${LOCAL_EVIDENCE}/compact-historic-reset-history-proof. Remaining historic circuits: reset_tree and known. No push or remote CI.


### Main integration evidence — 2026-10-05

Main4360d651 passed the affected consumer/proof/ledger phase at ${LOCAL_EVIDENCE}/compact-consumer-4360d651/receipt.json, including this integrated slice. This closes the phase interrupted by the earlier stack overflow/harness TypeError. It does not claim a whole-workspace rerun at4360. Both Merkle fixtures additionally pass16/16 recorded at ${LOCAL_EVIDENCE}/compact-focused-4360d651/receipt.json; main renderer122 passes.
