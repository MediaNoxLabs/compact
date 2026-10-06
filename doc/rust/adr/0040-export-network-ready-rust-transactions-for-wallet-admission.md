---
id: RUST-ADR-0040
alias: ADR-0040
title: "Export network-ready Rust transactions for wallet admission"
date: 2026-10-03
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["wallet", "transactions", "observations"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: 5d1ffeb8ef053f5be65fa0bbda064dc07e559e492408fc8af675e0c202b2a0ff
---
# RUST-ADR-0040 — Export network-ready Rust transactions for wallet admission

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Accept opt-in network/TTL configuration and upstream transaction export for real wallet admission, keeping deterministic offline defaults. Later dated Counter runs establish the recorded local live slice. Neither those connected services nor trusted indexed results establish authenticated consensus, all-contract network support or a new generated API.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#139 closure](https://github.com/MediaNoxLabs/compact/issues/139#issuecomment-6017464840). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`0f2870f7`](https://github.com/MediaNoxLabs/compact/commit/0f2870f73830c7612ad60656980d4b79aa91a36c) · [`83bb4910`](https://github.com/MediaNoxLabs/compact/commit/83bb4910bc27b794af65fc3da023013a939fbd71) · [`b16491ab`](https://github.com/MediaNoxLabs/compact/commit/b16491ab3d11006915993a99da2e390c7e7db48b) · [`e75f13ba`](https://github.com/MediaNoxLabs/compact/commit/e75f13ba2fdc954a29114177c31994a63e3cbec2). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
adr: 40
status: accepted
date: 2026-10-03
milestone: rust-backend-v2
issue: https://github.com/MediaNoxLabs/compact/issues/139
```

## Historical decision and amendments

### Problem

The Rust proof smoke produces sealed ledger-8.0.3 deploy and call transactions, proves and applies them locally, and exports exact bytes. Both transactions are offline fixtures: `Transaction::from_intents("local-test", ...)` and `Intent::empty(rng, Timestamp::from_secs(0))`. On the pinned local stack (node 0.22.3, proof server 8.0.3, ledger 8.0.3, wallet facade 3.0.0, network `undeployed`), a funded, synced genesis wallet rejected the Rust deploy at `balanceFinalizedTransaction`: `invalid network ID - expect 'undeployed' found 'local-test'`. This is a measured admission failure, not an inference from the byte decoder. The exact-head Nix proof gate still passed 57 offline calls and wrote both sealed counter transactions.

The README suggests `facade.validateTransaction(...)`, which is absent from the pinned wallet facade 3.0.0 public type surface. The guide must use the actual balancing, finalization, submission and confirmation APIs for this compatibility set.

### Before and after

Current proof smoke (simplified):

```rust
let intent = Intent::empty(rng, Timestamp::from_secs(0)).add_deploy(deploy);
let tx = Transaction::from_intents("local-test", HashMap::new().insert(1, intent));
```

Proposed opt-in live handoff keeps the offline gate default and stamps an explicit target network and current, future TTL:

```rust
let handoff = HandoffConfig::from_env()?; // network ID plus Unix expiry seconds
let intent = Intent::empty(rng, handoff.ttl).add_deploy(deploy);
let tx = Transaction::from_intents(&handoff.network_id, HashMap::new().insert(1, intent));
```

The same network ID and TTL apply to the paired call. The wallet integration must deserialize the sealed bytes, balance with funded application keys and a matching future TTL, finalize, submit deploy, await confirmation, and only then construct/submit the call using the confirmed contract state. The exact final API sequence and failure handling will be amended after running against the pinned local stack.

### Ownership and compatibility

`tools/compact-rust-proof-smoke` owns transaction assembly and opt-in export configuration; no generated contract API, emitter, runtime crate, VM, gas or private IR schema changes are expected. The wallet driver owns funds, key material, fee balancing and node admission. The Rust smoke still validates/applies the same artifacts against ledger-8 semantics before export. Default `local-test` / timestamp-zero behavior remains for deterministic offline proof regression. The live setting is explicit and should fail early for missing/invalid network or expired TTL. No ABI bump is warranted unless implementation changes the runtime package.

### Acceptance and risk

1. Reproduce the network mismatch and verify that the new live export decodes as `undeployed` with a future TTL.
2. Balance and finalize the deploy with wallet facade 3.0.0 using funded DUST, submit to node 0.22.3, and observe confirmation plus deployed address.
3. Build the call against the confirmed deployment state, then balance/finalize/submit/confirm the call. A prebuilt offline call is only a local proof fixture until this state boundary is satisfied.
4. Preserve the exact-head 57-call offline proof gate and byte roundtrip checks. Add focused configuration rejection tests and an executable, version-pinned integration recipe.
5. Record actual error or limitation rather than equating local `well_formed` with network admission. Keep credentials and network-specific details out of generated crates.

### Tracking

The focused GitHub issue will be assigned to `rust-backend-v2`. This ADR is proposed until live behavior is measured and implementation evidence is recorded. Parent issue [#105](https://github.com/MediaNoxLabs/compact/issues/105) still owns production wallet/node closure. No push is planned.


### Local implementation and measured admission — 2026-10-03

The proof smoke now accepts `COMPACT_RUST_HANDOFF_NETWORK_ID` and `COMPACT_RUST_HANDOFF_TTL_SECS` together. It rejects empty/whitespace network IDs, missing pairs, nonnumeric/expired TTL and expiry beyond ledger-8's one-hour global TTL. The opt-in pair sets the network on `Transaction::from_intents` and `LedgerState::new`, the expiry on deploy and call `Intent::empty`, and the current timestamp on both local well-formed checks and `BlockContext` for application. The default remains `local-test` with timestamp zero. This change is confined to `tools/compact-rust-proof-smoke`: no generated crate, AST emitter, runtime API, VM, private IR or ABI change.

Actual Rust after shape (simplified):

```rust
let handoff = HandoffConfig::from_env()?;
let deploy_intent = Intent::empty(rng, handoff.ttl).add_deploy(deploy);
let deploy_tx = Transaction::from_intents(handoff.network_id.as_str(), intents);
let context = TransactionContext {
    ref_state: ledger,
    block_context: BlockContext {
        tblock: handoff.validation_time,
        last_block_time: handoff.validation_time,
        ..BlockContext::default()
    },
    whitelist: None,
};
```

A first live proof attempt with a future TTL and default `BlockContext` failed locally with `IntentTtlTooFarInFuture`; stamping the matching block time fixed that. The rebuilt Nix `compactc` `--proof` gate then passed all 57 offline proof/verification/validation/application calls while exporting paired `undeployed` transactions with a 30-minute future expiry. The original `local-test` export was rejected by wallet facade 3.0.0 as `invalid network ID - expect 'undeployed' found 'local-test'`; the new deploy returned `FINALIZED_TRANSACTION` from `balanceFinalizedTransaction`.

The new `wallet-live` package pins ledger-v8 8.0.3, wallet facade 3.0.0 and its wallet dependencies. Its checked-in driver takes a funded local-devnet seed and endpoints from environment, builds a synced wallet, registers NIGHT for DUST if needed, balances, finalizes and submits the deployment, then waits for indexed `ContractDeploy` at the exact Rust address before submitting the paired call. It waits for indexed `ContractCall` at the same address with a different transaction hash. On a fresh isolated stack (node 0.22.3, indexer 4.0.1, proof server 8.0.3), the driver exited 0: deploy indexed at block 4 and call at block 8. The wallet's first early-start balance could report DUST balance yet reject spendability; the driver now retries only that exact insufficient-DUST error for up to two minutes. A prior fresh-stack run passed deploy at block 8 and call at block 12. The README replaces an unsupported `validateTransaction` example with facade 3.0.0's actual balance/finalize/submit calls and the runnable integration recipe.

The proof smoke uses deterministic counter RNG, so the same exported contract address is a one-shot fixture on a given chain; the driver rejects an address already indexed instead of replaying it. The check establishes local ledger admission and indexed actions, not clean remote CI, registry publication, broad contract parity, production key handling, finality beyond indexer observation, or a general SDK for constructing arbitrary post-deploy calls. These remain open under #105 and #139. The branch remains local and unpushed.


### Signed local delivery — 2026-10-03

Conventional GPG-verified/DCO commit `0f2870f73830c7612ad60656980d4b79aa91a36c` contains the proof-smoke live configuration, wallet-live package and corrected backend guide. `git log -1 --format=%G?` returned `G`; the sign-off trailer is present. The only remaining tracked worktree edit is the unrelated user-owned `doc/ledger-adt.mdx`, left unstaged. No push occurred.

At this exact source head, the rebuilt Nix `compactc` `check_compactc_target.py --consumer --proof` exited 0: separate generated-crate consumers, compile-fail checks and all 57 offline ledger-8.0.3 proof/verification/validation/application calls passed. Four invalid handoff configurations (missing pair, blank network, expired TTL, overlong TTL) rejected before artifact access. `cargo fmt --all -- --check`, Node syntax, pinned `npm ci`, package dependency pins, staged diff check and JS ledger-v8 byte roundtrip also passed. The live paired export was produced and independently submitted on a freshly reset devnet; the checked-in wallet driver exited 0 with deployment indexed in block 4 and call in block 8 at the same address. The isolated stack was stopped after the run. Remote CI, release and general post-deployment state-aware call construction remain open.

### ABI-18 live admission and indexed state assertion — 2026-10-03

Problem: the earlier ABI-16 local admission and the first ABI-18 rerun established indexed `ContractDeploy` and `ContractCall` actions at the Rust-exported address, but the driver did not inspect the indexed contract state. An indexed call alone did not prove the counter transition visible to a network consumer.

Before (wallet driver after `waitForAction`):
```js
const called = await waitForAction("ContractCall", deployed.transaction.hash);
console.log(`call indexed: block ${called.transaction.block.height}, address ${address}`);
```
After:
```js
const called = await waitForAction("ContractCall", deployed.transaction.hash);
assertCounterIncremented(called);
console.log(`call indexed: block ${called.transaction.block.height}, address ${address}`);
console.log("indexed contract state: round = 1");
```

The indexer query now requests `state` alongside action type, address and transaction. The driver uses the pinned ledger-v8 8.0.3 `ContractState.deserialize` API, checks that the primary state is one Counter cell with an 8-byte alignment and value 1, and fails if the shape or value differs. This is a contract-specific live smoke assertion, not a generic state decoder. The AST emitter, generated crate, runtime, private IR, VM, gas and ABI do not change; generated/runtime ABI remains 18. The backend guide describes the new state check.

Evidence: the fresh ABI-18 proof export used network `undeployed` and a future TTL. The rebuilt Nix `check_compactc_target.py --proof` gate passed all 57 offline calls and wrote the paired sealed bytes. On a first isolated node 0.22.3/indexer 4.0.1/proof server 8.0.3 stack, wallet facade 3.0.0 indexed deployment at block 84 and call at block 88. Independent ledger-v8 decoding of the indexer `ContractAction.state` returned one bytes<8> Counter cell containing 1. The driver was then changed and rerun on a freshly reset stack with the same ABI-18 export: deployment indexed at block 5, call at block 8, and the driver exited 0 after printing `indexed contract state: round = 1`. `node --check`, scoped diff validation and the live run passed.

Conventional GPG-verified/DCO local commit `b16491ab3d11006915993a99da2e390c7e7db48b` delivers the driver and guide update. The branch remains unpushed and user-owned `doc/ledger-adt.mdx` remains untouched. This confirms one funded local counter deploy/call and its indexed state effect at ABI 18. It does not establish finality beyond indexer observation, generic calls built from confirmed state, registry publication, clean remote CI or production key handling; [#139](https://github.com/MediaNoxLabs/compact/issues/139) and parent #105 stay open.

### Follow-through — 2026-10-03

The first indexed call now has a bounded later-call follow-through in [ADR-0041 — Build Rust calls from confirmed contract state](0041-build-rust-calls-from-confirmed-contract-state.md) / [#140](https://github.com/MediaNoxLabs/compact/issues/140), local signed/DCO `83bb4910`. A fresh pinned wallet run observed Counter `round = 1` after the first call and `round = 2` after a second Rust-proved call built from those indexed `ContractState` bytes. ADR-0040 still owns initial network-ready handoff; ADR-0041 owns the typed observed-state bridge and second-call evidence. General stateful calls, finality, remote CI and release remain open.

### ABI-28 local admission refresh — 2026-10-04

The existing network-ready handoff was re-exercised at current signed/DCO `e75f13ba` and generated/runtime ABI 28. Exact-head packaged `compactc` plus current proof smoke wrote 1,769-byte deploy and 3,376-byte proven call for `undeployed` with a future TTL; ledger-v8 8.0.3 decoded/re-serialized both. Wallet facade 3.0.0 on a fresh node 0.22.3/indexer 4.0.1/proof server 8.0.3 stack balanced, finalized and submitted them; indexed deployment and call at blocks 50/54 gave Counter `round = 1`. A reset fresh-chain run repeated admission at blocks 6/9 before the ADR-0041 later call. [ABI-28 live wallet admission — 2026-10-04](references.md#private-note-01) has exact commands, address, images, node provenance and limits. No code/API/ABI change; #139 stays open for remote and broader production gates.
