---
id: RUST-ADR-0300
alias: ADR-0300
source_sha256: 9bedbaa52ec05c34ea1c95bd001a0fd14406d0376556c6f2df1dc4b66e644522
publication_status: published
publication_date: 2026-10-09
publication_source_revision: f9a496660a4a69f4dd7c7927111767ad5c0c12b2
---
# RUST-ADR-0300 — Qualify original DID digest public transactions on ledger8.1

[ADR register](README.md) · [0.3.0 record register](index-0.3.0.md) · [Reference conventions](references-0.3.0.md)

## Publication disposition

This published historical source record belongs to the milestone 0.3.0 documentation collection. **Original opening status:** accepted-bounded-compatibility-slice. Read dated amendments below; neither this status nor issue closure certifies the whole milestone. No fresh implementation, security, proof or formal acceptance is implied. Original code examples and local evidence locators are historical; machine-local paths are not portable downloads or current setup instructions.



## Original decision and amendments

```yaml
status: accepted-bounded-compatibility-slice
date: 2026-10-07
parent: R030-16
issue: https://github.com/MediaNoxLabs/compact/issues/424
milestone: "0.3.0"
```

## ADR0300 — Qualify original DID digest public transactions on ledger8.1

### Accepted decision

Root accepted the two-row proposal SHA2562848647a3ab4d9586a7cb3fd3b22f76075fc9b0e517868fcb0997499b8a213ba on2026-10-07. Source baseline1b6c5d02. Freeze and extend only the existing optional public interchange harness for original DID digest lifecycle tuples (`digest`, `insert`, `setSchnorrJubjubVerificationMethod`) and (`digest`, `read-valid`, `verifySchnorrJubjubDigestSignature`). Create the milestone issue before implementation. Prototype the frozen copied harness first; root reviews before live port/signing.

### Problem, before and after

Before, ADR0288 strictly proves the original digest lifecycle natively, while ADR0290 exports and independently applies fourteen other DID tuples across ledger8 profiles. The Digest branch deliberately skips optional public export.

```rust
if !matches!(lifecycle, Lifecycle::Digest) {
    capture_if_requested(/* exact actual transaction, ledger, context */)?;
}
```

After, the same optional hook admits two new reviewed tuples and labels this lifecycle `digest`, with no transaction/recording/argument change:

```rust
capture_if_requested(/* actual balanced transaction, preledger, context,
                        scenario="digest", exact case and operation */)?;
```

The read-valid tuple additionally requires identical serialized contract-before/after in both producer and receiver. The full ledger may change through transaction fees/bookkeeping and must match the actual8.0.3 producer result exactly. No substitution of receiver default parameters is permitted.

### Scope, acceptance and boundaries

- Reuse exact hash-validated existing ADR0288 keys/IR and run only the digest selector: two real proof calls. No complete sixteen-call proof repeat.
- Two independent upstream8.1 receiver applications,12JS public carrier roundtrips and24 trailing/truncated refusals. No continuous8.1 chain claim.
- Preserve old14 tuple allowlist and refusal tests; the new gate requires exactly2 new rows and binds prior14 separately. No16-fresh-run claim.
- Preserve optional absent-env behavior, absolute/new directory guard, whitelist/ref_state guards, default strict verification, real contract proof verification, altered-public-input refusal, complete ledger equality and same-time replay refusal without mutation.
- Require and report producer-initial8.0.3, receiver-default8.1 and actual transported parameter identities. Diagnostic comparison only; actual transported ledger/context remain unchanged.
- Existing pinned official0.31.1 lifecycle rows already exactly equal branch insert/read-valid rows, including key sets; recheck before use. No original TS recapture needed. Compiler-reported ledger8.0.2, app wire8.1 and native8.0.3 stay distinct.
- Source/compiler/runtime/pin changes are excluded. Constructor data is deployed/checked; constructor execution is unproved. Relation remains pending.
- Use existing warm targets under leases, no broad8.1 workspace rebuild or cache clean. Public artifacts only; no private witness or seed export.

### Ownership and delivery

Narrow proof harness files: `did_public_interchange.rs` and the digest optional capture callsite in `did_point_lifecycle.rs`. Coordinate with ADR0295 before eventual live edits. Receiver/JS/driver remain isolated evidence. Root owns review, live-port approval and signed commit. No own push/CI/tag/publication.

The detailed accepted technical plan and refusal matrix follow in the linked proposal: [R03016 — Two-row DID digest bridge proposal — 2026-10-07](references-0.3.0.md#note-165). Every important result or exception is recorded in midnight; receipt/archive preserve exact source and material identities.
