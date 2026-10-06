---
id: RUST-ADR-0033
alias: ADR-0033
title: "Carry proven calls across the wallet serialization boundary"
date: 2026-10-03
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["wallet", "transactions", "observations"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: d0d772719227c3ce5230e8e07eb8f5a2f1485f7510388851c0df05899a0240b3
---
# RUST-ADR-0033 — Carry proven calls across the wallet serialization boundary

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Accept the upstream tagged serialization boundary for sealed Rust transactions and matching pinned JavaScript decoding. Keep wallet fee balancing/finalization/submission outside generated crates. Byte roundtrip and the documented validation sequence are distinct from actual funded live admission and preserve the historical provider version.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#132 closure](https://github.com/MediaNoxLabs/compact/issues/132#issuecomment-6017452996). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Referenced repository commits:** [`b6b06236`](https://github.com/MediaNoxLabs/compact/commit/b6b062369201bbec93978caa0cd59a70a570cae9) · [`ed05c0d8`](https://github.com/MediaNoxLabs/compact/commit/ed05c0d88d21ca6c9cb01663b4d7c30fa41b7a3b) · [`f6cd4a16`](https://github.com/MediaNoxLabs/compact/commit/f6cd4a16df6b0c15de704e8469f731b8706aa057). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
adr: 33
status: accepted-partial
date: 2026-10-03
milestone: rust-backend-v2
issue: https://github.com/MediaNoxLabs/compact/issues/132
```

## Historical decision and amendments

- Status: Proposed (2026-10-03)
- Milestone: rust-backend-v2
- Parent: MediaNoxLabs/compact #105
- Focused issue: pending

### Problem

The generated Rust crate and prepare_call produce a ledger-8 ContractCallPrototype. The packaged proof gate makes an in-process transaction, proves, verifies and applies it with fee balancing disabled. That does not establish that a wallet can receive its bytes, add fees, finalize and submit it. The wallet SDK ledger-v8 boundary consumes FinalizedTransaction; its transaction trait deserializes with Transaction.deserialize('signature', 'proof', 'binding', bytes). The facade then provides balanceFinalizedTransaction, finalizeRecipe and submitTransaction. The node client accepts serialized finalized transaction bytes. Match Rust and JS ledger version 8.0.2 for a measured handoff.

### Before

    let call = prepare_call(recorded, spec)?;
    let unproven = Transaction::from_intents(network_id, intents_with(call));
    let proven = unproven.prove(provider, cost_model).await?;
    ledger.apply(&proven.well_formed(&ledger, relaxed, now)?, &context);

Only a Rust process sees proven. The local test network ID, timestamp zero and disabled balancing make this an offline correctness fixture.

### Decision and after

Keep the transaction lifecycle in ledger and wallet primitives. Do not emit wallet protocol code into each generated crate. Seal the proven ledger transaction and serialize it with midnight_serialize::tagged_serialize; test the resulting bytes with the pinned @midnight-ntwrk/ledger-v8@8.0.2 decoder and byte-for-byte re-serialization. Record network ID and contract address separately as provenance. Document handing the decoded transaction to the wallet facade for balancing and submission, with actual network parameters and a current TTL supplied by the application.

    let call = prepare_call(recorded, spec)?;
    let proven = transaction_with(call).prove(provider, cost_model).await?;
    let sealed = proven.seal(binding_rng);
    let mut bytes = Vec::new();
    midnight_serialize::tagged_serialize(&sealed, &mut bytes)?;

    const tx = ledger.Transaction.deserialize('signature', 'proof', 'binding', bytes);
    const recipe = await facade.balanceFinalizedTransaction(tx, keys, { ttl, tokenKindsToBalance: 'all' });
    const finalTx = await facade.finalizeRecipe(recipe);
    await facade.submitTransaction(finalTx);

The TypeScript example is an intended handoff, not a claim of successful wallet balance or node submission.

### Ownership and compatibility

- Rust emitter: no change. Generated crate continues to expose typed recording methods.
- Rust runtime: prepare_call remains the single ledger adapter. Avoid new wrapper types unless the direct ledger API proves awkward.
- Proof/consumer integration: seal, serialize, decode and document a pinned, versioned handoff. Keep offline simulation values separate from a network-ready transaction.
- Ledger/zk: use existing midnight-ledger and midnight-serialize primitives; do not fork encoding.
- Schema/ABI: no change expected. The optional ledger-transaction feature remains opt-in.

### Alternatives

- Hand-written JSON transaction schema duplicates canonical ledger serialization and can drift.
- Generated methods that submit directly couple contract code to wallet keys, fees, network and transport.
- Treating in-process well_formed/apply as submission misses byte compatibility, fees, TTL and node admission.

### Acceptance

1. A proven, sealed counter transaction serializes and round-trips through ledger-8 Rust tags.
2. The exact bytes deserialize and reserialize with @midnight-ntwrk/ledger-v8@8.0.2; a mismatched ledger package/shape fails clearly.
3. Reproducible commands document the handoff and distinguish offline fixture values from actual wallet/node submission.
4. Parent #105 stays open until a funded, balanced, current-TTL transaction is submitted and observed through the supported wallet/node path with failure behavior, clean CI and release evidence.

### Evidence and limits

Proposal based on local midnight-ledger=8.0.2 crate Transaction::prove/seal, wallet main packages/facade/src/transaction.ts and packages/wallet-sdk/README.md, and node-client README. No delivery or live submission claimed yet.


### Issue assignment — 2026-10-03

Focused issue [#132](https://github.com/MediaNoxLabs/compact/issues/132) is assigned to [rust-backend-v2](https://github.com/MediaNoxLabs/compact/milestone/2). The proposal above remains unchanged; delivery evidence will be appended below.


### Delivery amendment — 2026-10-03

Accepted as a **partial transaction handoff** under [#132](https://github.com/MediaNoxLabs/compact/issues/132). Local conventional GPG-verified/DCO commit b6b062369201bbec93978caa0cd59a70a570cae9 gives the packaged counter proof an optional canonical sealed ledger-8 byte export. The proof smoke now constructs signature-enabled intents so the JavaScript decoder accepts the wallet's signature/proof/binding transaction type. The sealed transaction passes offline well_formed validation, Rust tagged serialization/deserialization/re-serialization, and @midnight-ntwrk/ledger-v8@8.0.2 decoding with exact 3,372-byte reserialization and one call in segment 1. The JS version and tarball integrity are lockfile pinned; CI runs the export and cross-language check.

Local tests: cargo check -p compact-rust-proof-smoke --locked, cargo fmt, pinned npm ci, node byte check, and a 56-call packaged --proof gate. A fresh --consumer rebuild was stopped early because its dedicated Cargo target would exhaust local free disk; the prior consumer gate passed at commit ed05c0d8 before this transaction-only change. Remote CI has not run. The exported fixture remains local-test, TTL zero and unbalanced; the wallet must provide real network state, keys/funds and current TTL before the parent #105 submission exit gate can pass. The branch is local/unpushed. This amendment records evidence without rewriting the proposal.


### Validation sequence amendment — 2026-10-03

Local signed/DCO docs commit f6cd4a16 adds the wallet facade's recommended two validation stages to the backend guide. Before balancing a finalized but unbalanced transaction, validate with enforceBalancing=false, verifySignatures=true, enforceLimits=false. After finalizing the balancing recipe, validate with all three flags true and the recipe block data, then submit. The wallet source says network ID, TTL and structure are enforced regardless of these flags. This sharpens the application handoff example; it is not a live validation or submission result. The decision to keep wallet responsibilities outside generated crates is unchanged.
