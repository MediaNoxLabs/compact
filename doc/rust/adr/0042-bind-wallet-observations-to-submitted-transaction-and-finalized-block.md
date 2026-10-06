---
id: RUST-ADR-0042
alias: ADR-0042
title: "Bind wallet observations to submitted transaction and finalized block"
date: 2026-10-03
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["wallet", "transactions", "observations"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: 15310278769c9226439230175f31bd087b165563a4a1b2851b30d531bc2379e6
---
# RUST-ADR-0042 — Bind wallet observations to submitted transaction and finalized block

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Accept binding indexed contract observations to the finalized submitted transaction hash, canonical block identity and node finality checks. Normalize only the documented hash representation differences. This strengthens connected-service provenance but does not verify consensus proofs or defend against a dishonest node/indexer pair.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#141 closure](https://github.com/MediaNoxLabs/compact/issues/141#issuecomment-6017468260). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`83bb4910`](https://github.com/MediaNoxLabs/compact/commit/83bb4910bc27b794af65fc3da023013a939fbd71) · [`9ee0d987`](https://github.com/MediaNoxLabs/compact/commit/9ee0d987f6bae729914991a6727f04eff78432de). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
adr: 42
status: accepted-partial
date: 2026-10-03
milestone: rust-backend-v2
issue: https://github.com/MediaNoxLabs/compact/issues/141
```

## Historical decision and amendments

### Problem

The ABI-18 local wallet smoke waits for `contractAction(address)` to report an action of the requested kind whose transaction hash differs from the previous action. Another writer could act at that address first and be mistaken for the Rust transaction. The second-call builder would then use another transaction's state. The indexer schema exposes an action transaction hash and block hash/height, while the finalized wallet transaction exposes `transactionHash()`. The pinned node 0.22.3 local RPC accepts `chain_getFinalizedHead`, `chain_getHeader` and `chain_getBlockHash`; the pinned indexer 4.0.1 serves a block hash at the same height. This should be checked before treating the indexed bytes as the submitted Rust call's state.

### Before

```js
await submit(call, 'call');
const called = await waitForAction('ContractCall', deployed.transaction.hash);
// waitForAction accepts any newer action at address with a different hash.
```

### Decision and after

```js
const submitted = await submit(call, 'call'); // final transaction hash
const called = await waitForAction('ContractCall', submitted.hash);
await requireFinalizedBlock(called.transaction.block, nodeRpc);
// Require action address/type/exact hash, then canonical block hash at a
// height no greater than the node's finalized head before using state.
```

The wallet's `submitTransaction` return value is an intent identifier, which may differ from the final merged transaction hash. Capture `finalized.transactionHash()` after balancing/finalization and before submission. Normalize only an optional `0x` prefix and case, and reject malformed hashes. Keep waiting when the indexer shows another action; never use that action's bytes. Query node RPC independently for finalized head, header height and canonical block hash at the action height.

### Alternatives and rationale

Address/type plus a changed hash is the current shortcut and does not bind the state to the submitted transaction. Comparing to the wallet's returned intent identifier is unsound when fee balancing merges transactions. Comparing indexer block and latest indexer block uses one source. Node finalized-head and canonical hash checking adds a second service boundary, though it still trusts the connected node and indexer.

### Emitter and runtime ownership

This is a wallet integration/provenance gate. No AST emitter, generated crate, runtime, macro, private IR, or generated ABI change is expected (schema 8, ABI 18). Keep a small testable identity/finality helper in `tools/compact-rust-backend/wallet-live/`; the live driver owns waiting and state handoff. Generic Rust observed-state provenance remains a separate generated API decision after ADR-0041.

### Acceptance and limits

1. Tests reject another action at the right address but wrong transaction hash, malformed hashes, a block above finalized height, and a canonical block-hash mismatch; they accept exact hash and finalized matching block.
2. A fresh pinned funded devnet run records the final merged transaction hash and observes its exact indexed action and node-finalized canonical block for deployment and both calls, with Counter state 1 then 2.
3. Keep the rebuilt compactc consumer/57-call offline proof gate and package checks unchanged because no emitter/runtime output changes.

Node RPC is trusted; this is not a cryptographic GRANDPA proof or an authenticated full ledger state. The indexer's latest-action query may miss our transaction if another later action appears before polling and then time out safely. Querying a historical action by transaction offset, generalized concurrent-writer handling and rollback recovery are separate M2 work. The branch remains local and unpushed.

### Tracking and delivery

- Issue: pending MediaNoxLabs/compact issue in rust-backend-v2.
- Parent: ADR-0041 / issue #140; initial wallet gate ADR-0040 / issue #139.
- Append signed/DCO commit, exact tests, live block/hash evidence and unresolved limits after implementation.

Tracking issue created and assigned to rust-backend-v2: https://github.com/MediaNoxLabs/compact/issues/141.

### Decision and local delivery — 2026-10-03

Accept exact final-transaction identity plus node finalized-chain cross-check for the local wallet gate. Conventional GPG-verified/DCO commit `9ee0d987f6bae729914991a6727f04eff78432de` implements this without changing compiler, emitter, generated crate, runtime, macros, private IR schema 8 or generated/runtime ABI 18. The proposal above remains as decision history.

#### Delivered before and after

Previously `waitForAction(type, previousHash)` accepted any later matching-kind action at the address with a different hash. The wallet's `submitTransaction()` return value is an intent identifier; in the live run it differed from the final transaction hash after fee balancing and merge.
```js
const finalized = await wallet.finalizeRecipe(recipe);
const hash = normalizeHash(finalized.transactionHash());
await wallet.submitTransaction(finalized);
const action = await waitForAction('ContractCall', hash);
// waitForAction requires exact action type, address and final tx hash;
// then requires node finalized height >= action height and canonical hash match.
```

`provenance.mjs` owns strict 32-byte hex normalization, exact action matching, and the node finalized-header/canonical-block check. The driver owns JSON-RPC transport and bounded polling. A different latest action is ignored until the exact submitted hash appears; a canonical hash mismatch fails immediately. If another writer later hides the action from the latest-action query, the gate times out safely.

#### Evidence

- Four Node tests passed: exact versus wrong transaction hash/type/address, malformed hashes, action above finalized height, and canonical mismatch. `node --check` passed for the driver and helper; staged diff check passed.
- A fresh isolated node 0.22.3/indexer 4.0.1/proof server 8.0.3/wallet facade 3.0.0 devnet admitted the Rust deployment at block 76, first call at 79, and Rust second call built from indexed state at 83. The driver required exact final transaction hash and node-finalized canonical block for all three. Indexed Counter state was `round = 1` then `round = 2`.
- The final second-call hash `0ca54620d58a581a080c1d96a8a83c398eb221da66a27f85c3ab10bc4eb0d255` matched the indexer. Its block 83 hash `e58857299bb3a9790c6af597df08c7631fe9d04ae4b306d1954eb65681eed581` matched node `chain_getBlockHash(83)` while the node finalized head was at height 86. The wallet returned a distinct intent identifier `000d240c...`, demonstrating that this cannot be compared directly to the indexer's transaction hash.
- Pinned ledger-v8 8.0.3 independently decoded the new 1,769-byte deploy and 3,376-byte call handoff. Prior exact-HEAD `83bb4910` rebuilt compactc separate-consumer/57-call offline proof and macro/runtime archive gates remain applicable because this commit changes only wallet tooling and documentation; they were not rerun for this wallet-only edit.

#### Limits

This establishes consistency among the finalized wallet transaction, indexer action, and trusted node canonical finalized chain. It does not verify a GRANDPA justification, authenticate the full `ContractState`, handle rollback across providers, or make latest-action polling reliable under concurrent later writers. Remote CI, published runtime/registry consumer, generic Rust observed-state API and clean release remain open. Issue #141 stays in rust-backend-v2; branch local and unpushed. The unrelated user-owned `doc/ledger-adt.mdx` edit was untouched.

### ABI-28 node/indexer provenance refresh — 2026-10-04

The current wallet driver on pinned local node 0.22.3/indexer 4.0.1/proof server 8.0.3 checked the exact merged submitted transaction hash and action type/address for the ABI-28 deployment, first call and generated confirmed-state second call. Each indexed block had to equal the node's canonical hash at a height no greater than finalized head. The last `ContractCall` has merged hash `3c2e2ebbca4a6ce79577011f3c30b80e31d9de016ebd1cf0081e1b9178796869`, block 13 hash `3587bc39a25df2f04a0547ea080bf50efb35d855607ef3d2c88b2b065ee06864`, and a subsequently queried finalized head of 17. Independent `chain_getBlockHash(13)` matched the indexer hash after normalizing the node RPC `0x` prefix. [ABI-28 live wallet admission — 2026-10-04](references.md#private-note-01) records the full run. This proves the driver's bound local observation on the connected services, not a consensus finality proof or defense against a dishonest node/indexer; #141 remains open.
