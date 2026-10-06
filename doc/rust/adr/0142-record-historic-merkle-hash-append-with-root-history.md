---
id: RUST-ADR-0142
alias: ADR-0142
title: "Record historic Merkle hash append with root history"
date: 2026-10-05
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["recording", "merkle", "history", "hash"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: 809a14e2b7cd01d52888f2c0ef8ce3edf910c089ca94d9054c9209ffe9ce5257
---
# RUST-ADR-0142 — Record historic Merkle hash append with root history

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Accepted historic Merkle hash append using the canonical historic program, including root-map updates and no second leaf hash. State, history, VM/gas and proof/application evidence are present; plain and indexed operations remain separate.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#244 closure](https://github.com/MediaNoxLabs/compact/issues/244#issuecomment-6017644920). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`01a4c702`](https://github.com/MediaNoxLabs/compact/commit/01a4c702ca1347e7d7d51a5e3a261d4ebc630d25) · [`bd1bc6b1`](https://github.com/MediaNoxLabs/compact/commit/bd1bc6b1135e3383c5e03b869a9c2eb2329f42d7) · [`f72e22ab`](https://github.com/MediaNoxLabs/compact/commit/f72e22abf7f779a7e48063671e28af3c931a1079). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
title: ADR-0142 — Record historic Merkle hash append with root history
status: accepted
date: 2026-10-05
milestone: rust-backend-v2
```

## Historical decision and amendments

### Problem

The original `examples/rust_backend/hmt_insert_oracle.compact` exports `append_hash(hash: Bytes<32>)` on a `HistoricMerkleTree<3, Uint<8>>`. Its schema-12 action is `StateAction::HistoricMerkleInsertHash(t, hash parameter)`. Native Rust uses ledger-8's historic `insertHash` query, but the generated crate has no recorded or observed-call API. A plain Merkle hash append program would omit the historic root-map update and produce a different state, VM trace and proof.

### Before and after

```compact
export circuit append_hash(hash: Bytes<32>): [] {
  t.insertHash(disclose(hash));
}
```

Before, only `ledger_contract::append_hash(context, hash)` is available. After:

```rust
let recorded = ledger_contract::recorded::append_hash(context, hash)?;
let (initial, ordered_verify_ops) = recorded.public.into_parts();
let call = ledger_contract::recorded::Contract
    .append_hash_call(&observed, private_state, hash)?;
```

The emitter checks an exact `HistoricMerkleTree` declaration, one-segment physical path, matching slot index and `Bytes<32>` source. It calls the historic-only slot `record_insert_hash`, which uses a recording frame and the existing ledger-8 `merkle_insert_hashed_program` with `MerkleHistory::Historic`. The supplied bytes are already a leaf hash; no second leaf hash is computed. The VM program must include the history root-map insertion. No new IR or runtime ABI is required.

### Scope and guards

This slice enables `HistoricMerkleInsertHash` only. Plain tree, wrong slot/path, wrong hash width and unsupported conditional hash expressions must not be admitted. Indexed historic hash placement remains a separate action. The public runtime slot method is available only on historic slots so callers cannot silently select plain history semantics.

### Acceptance

Fresh original ledger-8 TypeScript capture at the existing post-`resetHistory` state. Compare generated native and recorded Rust result, serialized state, root history, four gas dimensions, ordered VM, private outputs and independent Verify replay. Add renderer positive/negative guards and generated fixture tests. Generate pinned ZKIR 2.1.0 keys and prove/verify/ledger-8 validate/apply the typed observed call, comparing applied state and history with native execution. Update the broad compiler capability assertions and proof invocation, check fixture freshness and run a focused local gate. Record the exact source-inventory delta and signed local commit here. No remote CI or push.

### Delivery

Recorded before implementation. Tracking issue and local evidence follow.

Tracking issue: https://github.com/MediaNoxLabs/compact/issues/244 (rust-backend-v2). Created before implementation.

### Local verification (2026-10-05)

- Exact original-source IR is one `HistoricMerkleInsertHash` action with a `Bytes<32>` parameter. Historic source inventory changed from 3/8 to 4/8 proof-required calls available; `place_hash`, `forget_history`, `reset_tree` and `known` remain explicit gaps.
- A freshly compiled ledger-8 TypeScript capture adds only `nativeQueries.appendHash` and `historyAfterAppendHash`. The call has 17 ordered VM operations, zero private outputs, gas read 1275000000 / compute 2647946280 / bytes written 1568 / deleted 1206.
- Generated native and recorded Rust match TypeScript result, serialized state, root history, four gas dimensions and effects. The recorded program equals all 17 TypeScript operations; independent Verify replay reaches the same state, effects, history and gas. All five historic Merkle fixture tests pass.
- Renderer admits only the matching historic slot and typed `Bytes<32>` source, rejecting wrong width, wrong tree kind and an unsupported conditional hash expression. All 147 generated fixtures are fresh; formatting and diff checks pass.
- Pinned `midnight-zkir 2.1.0` generated keys. The generated `append_hash_call` matches the manual recording; proof verification, ledger-8 validation and apply pass. Applied state, first-free index and root history equal native execution.
- The broad compiler check now expects `append_hash` recorded/observed with proof artifacts and invokes `--historic-merkle-hash` proof smoke. The exact local-head focused receipt follows signed commit. Nix packaging is delegated to parent integration; no remote CI or push.

Signed local commit: `e919bb99ca741828e30c2d9a6f7847d8340b5dd1` (`feat(rust-backend): record historic Merkle hash append`, GPG good, DCO present). Focused local-head gate passed using the frozen branch compiler and ledger-8 Scheme: 1 original-source fixture, 4/8 recorded/observed, receipt `${LOCAL_EVIDENCE}/compact-focused-e919bb99/receipt.json`. Parent integration will run the packaged exact-head gate; this focused receipt supplements the separate pinned proof run.


### Parent integration

Integrated signed commit bd1bc6b1; 114 renderer tests and five historic tests pass. Combined original-source focused gate at 01a4c702 passes (four fixtures): ${LOCAL_EVIDENCE}/compact-focused-01a4c702/receipt.json. Repository inventory at that head is 283/317 proof-required available, 34 known gaps, 25 unassessed; ${LOCAL_EVIDENCE}/compact-01a4c702-inventory.json. The broad gate explicitly invokes --historic-merkle-hash. Combined Nix packaging and full local gate are in progress at f72e22ab; no claim of a newer full checkpoint until they finish.
