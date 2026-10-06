---
id: RUST-ADR-0150
alias: ADR-0150
title: "Record historic Merkle resetToDefault with blank-root history"
date: 2026-10-05
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["recording", "merkle", "history", "reset"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: 325edefbc5f675c2fa12214e3ded7e71675019859a3b90749bcdf5edd21256cd
---
# RUST-ADR-0150 — Record historic Merkle resetToDefault with blank-root history

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Accepted by signed delivery and integration despite proposed frontmatter: historic resetToDefault restores the blank depth-specific tree and seeds only its blank root in history. Populated-state proof/application confirms old-root removal; resetHistory remains a distinct operation.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#252 closure](https://github.com/MediaNoxLabs/compact/issues/252#issuecomment-6017657754). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`4360d651`](https://github.com/MediaNoxLabs/compact/commit/4360d65180e3626a27e3128bc2610200f359f575). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
title: ADR-0150 — Record historic Merkle resetToDefault with blank-root history
status: proposed
date: 2026-10-05
milestone: rust-backend-v2
```

## Historical decision and amendments

### Problem

`examples/rust_backend/hmt_insert_oracle.compact` exports `reset_tree()` using `t.resetToDefault()` on a `HistoricMerkleTree<3, Uint<8>>`. Native Rust executes a ledger-8 VM program that builds a blank tree and seeds its history with the blank root. The generated Rust crate currently has no recorded or observed call for this proof-required circuit. The operation must discard all old roots and restore the same serialized state as initialization, including history.

### Before and after

```compact
export circuit reset_tree(): [] {
  t.resetToDefault();
}
```

Before, generated `ledger_contract::reset_tree(context)` is native only. After:

```rust
let recorded = ledger_contract::recorded::reset_tree(context)?;
let call = ledger_contract::recorded::Contract
    .reset_tree_call(&observed, private_state)?;
```

The emitter recognizes only `HistoricMerkleResetToDefault` against the exact historic declaration and index, with a one-segment physical path. It delegates to a typed historic Merkle slot. The slot records the same canonical VM program used by native execution. That program constructs the blank depth-specific tree, calculates its root, inserts that root into history, and replaces the old tree. The runtime owns depth and state semantics; the emitter performs shape checking.

### Guards and scope

Reject plain Merkle fields, mismatched field index or physical path, and any noncanonical action sequence. Preserve the declared depth in the slot rather than reading or inferring it from an untyped expression. Do not conflate this operation with `resetHistory`, which keeps the current populated tree. The proof call takes no public input and emits no private transcript outputs.

### Acceptance

Refresh the original ledger-8 TypeScript capture and compare native/recorded result, serialized state, blank root, history, four gas dimensions, effects, ordered VM instructions, and Verify replay. Add positive and negative renderer guards and generated fixture tests. Update broad capability, consumer and proof gates. Generate pinned ZKIR 2.1.0 artifacts; prove/verify and ledger-8 validate/apply from a populated historic tree, asserting old roots are gone and the blank root is seeded. Run an exact-head focused local gate. Parent integration owns package gate. Signed conventional GPG+DCO local commit; no push or remote CI.

### Delivery

Decision recorded before implementation. Issue and evidence follow.

Issue: https://github.com/MediaNoxLabs/compact/issues/252

### Delivery (2026-10-05)
Local signed commit: 090caee10b8ca400afa3b74d09901303a680688a (GPG verified; DCO Signed-off-by). Issue #252. Fresh TypeScript capture added resetTree: nine ordered VM instructions, one blank root in history, and all four gas values. Generated native/recorded state, effects, result, gas, private outputs and Verify replay agree; eight HMT tests, 117 renderer tests and 147 generated fixture checks passed. Pinned ZKIR 2.1.0 proof verified and ledger-8 validated/applied reset_tree against a populated tree, restoring the constructor blank state and removing the old root. Exact-head focused gate passed with 7/8 historic circuits recorded: ${LOCAL_EVIDENCE}/compact-focused-090caee1/receipt.json. Proof artifacts: ${LOCAL_EVIDENCE}/compact-historic-reset-tree-proof. Historic `known` remains. No push or remote CI.


### Main integration evidence — 2026-10-05

Main4360d651 passed the affected consumer/proof/ledger phase at ${LOCAL_EVIDENCE}/compact-consumer-4360d651/receipt.json, including this integrated slice. This closes the phase interrupted by the earlier stack overflow/harness TypeError. It does not claim a whole-workspace rerun at4360. Both Merkle fixtures additionally pass16/16 recorded at ${LOCAL_EVIDENCE}/compact-focused-4360d651/receipt.json; main renderer122 passes.
