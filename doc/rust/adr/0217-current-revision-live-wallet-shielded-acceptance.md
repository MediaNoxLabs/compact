---
id: RUST-ADR-0217
alias: ADR-0217
title: "Current-revision live wallet shielded acceptance"
date: 2026-10-06
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["live-wallet", "checkpoint", "observed-call"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: ca6db7c2b248f37b66728c4b5fbbbd5be2f64dc2d3e2a66043f4cb9d5268e04c
---
# RUST-ADR-0217 — Current-revision live wallet shielded acceptance

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. A trusted finalized checkpoint binder and standalone builder support the bounded bootstrap, wallet receive, contract accept, release and recovery lifecycle on the tested pinned node/indexer pair. The 8.0.2 node/8.0.3 decoder compatibility is an explicitly tested profile, not a general version promise; receipt hashes are evidence, not authentication, and this is not continuous original Coracle/DAO application.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#321 closure](https://github.com/MediaNoxLabs/compact/issues/321#issuecomment-6017774110). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`cc333811`](https://github.com/MediaNoxLabs/compact/commit/cc333811fa8dc3566c59eeccd00b5a7cb71193b5) · [`d1411fe5`](https://github.com/MediaNoxLabs/compact/commit/d1411fe598c3e98527d5dce61adf756bf0448492) · [`d5dd3f2e`](https://github.com/MediaNoxLabs/compact/commit/d5dd3f2eeeaad337ba40fcf2639d540a9738f161) · [`e75f13ba`](https://github.com/MediaNoxLabs/compact/commit/e75f13ba2fdc954a29114177c31994a63e3cbec2). These include historical prerequisites and probes, not only final delivery commits.

## Historical decision and amendments

Status: approved independent harness preparation; live services and acceptance wait for the final frozen source head. Owner: election_admin_recording. ABI49/schema20 baseline. ADR216 is separately owned.

### Problem

The previous successful live flow used ABI28 (`e75f13ba`): generated Counter deployment, first call and a second generated call from confirmed indexed state. Current strict in-process shielded tests use explicitly seeded coins and do not establish wallet funding, live ownership, facade balancing or current-head node admission. Those claims must remain separate.

### Decision and smallest scenario

Retain the existing Counter driver unchanged. Prepare a separate acceptance contract using already-supported generated APIs: a Kernel mint bootstrap (only when a real shielded wallet coin is unavailable), `receiveShielded` and full-value qualified `sendShielded` to the same wallet. Do not introduce partial change, transients, application witnesses, new runtime APIs or compiler policies for this acceptance test.

1. Deploy the fresh acceptance contract, confirm exact address/action/hash and canonical indexed block beneath the trusted node's finalized head.
2. If needed, actually mint a small custom token on chain to the live wallet with encrypted output; wait until the wallet indexes the real coin/value/owner. Never substitute offline insertion.
3. Select one confirmed wallet coin at its full value. Generate its actual upstream Input preimage from synchronized wallet state. Bind that unchanged Input plus one contract Output using explicit `WalletFundingInputs` and canonical allocation; record/prove the generated receive call.
4. After finalized receipt, capture the exact same contract action's serialized ContractState and contract-specific Zswap state, entry point, transaction/block identity. Locate the actual received coin index and owner; build its historical contract Input and one user Output, record/prove full qualified send, submit and confirm receipt in the wallet.
5. Retain nullifier consumption/replay refusal, wrong input/extra funding/changed owner or coin rejection, exact offer identity through finalization, and the observed contract state at every stage. Empty-ledger acceptance has no artificial Counter stale-read guarantee; replay and funding/state binding are checked at their actual boundaries.

### Before / after public flow

```javascript
// Existing Counter flow remains unchanged:
node check.mjs deploy.bin call.bin
// New separate shielded acceptance will orchestrate real wallet preimages,
// Rust generated observed calls, and verified indexed ownership:
node shielded-check.mjs acceptance-config.json
```

```rust
// The driver calls current generated APIs, not a replacement VM evaluator.
let bound = OfferBackedObservedState::with_options(observed, &view, offer, options)?;
let call = contract.recording().accept_call(bound.observed(), (), coin)?;
let prepared = bound.prepare(call, verifier_key, binding)?;
```

Exact function names/types and the final driver CLI remain to be validated by compiled artifacts before delivery. The bootstrap mint is an explicit separate on-chain phase; it is not a fake wallet input or relaxed ledger validation.

### Ownership, snapshot and proof policy

Indexer 4.0.1 exposes `ContractAction.state` and `ContractAction.zswapState` on the same action. The latter is explicitly contract-specific/filtered, not a full authenticated ledger snapshot. Determine and test exactly which allocation/history/root checks this view supports; never claim global nullifier freshness or full ledger authority from a reconstructed partial view. The node's actual default-strict acceptance owns final ledger validity. The connected node/indexer are trusted observations, not consensus proofs.

The builder must preserve actual upstream proof objects and exact selected funding, normalized offer and generated intent correspondence. Wallet fee balancing may add Dust but must not silently change the retained shielded offer. A changed call/address/entry point, additional shielded inputs/change, retargeted proof segment or altered output must be rejected before submission. Record any facade API limitation instead of weakening the policy.

### Pinned stack and harness gaps

Use node 0.22.3, indexer 4.0.1, proof server 8.0.3, ledger-v8 8.0.3, wallet facade 3.0.0, network `undeployed`, the documented disposable genesis wallet and separate NIGHT/DUST. Existing package-lock remains authoritative. Capture actual image digests when services eventually run.

Current driver lacks same-action Zswap/entry-point capture, wallet Input/preimage handoff, a shielded Rust offer-bound builder, retained structured stage receipts and a guaranteed shielded genesis coin. New files stay under `tools/compact-rust-backend/wallet-live` (or explicitly approved acceptance harness scope). No emitter/runtime changes or services during preparation. A standalone generated-consumer builder must use the frozen compiler's unedited generated crate and matching runtime.

### Tests and delivery boundary

Prepare offline malformed-observation/offer-change/receipt tests with pinned ledger objects, strict source/capability compilation, generated Rust consumer build and Rust↔JS byte handoff. Once root freezes the final source corpus, rerun Counter first, then the shielded flow against a fresh isolated stack. Preserve failed attempts and exact source/compiler/lock/toolchain/artifact/transaction hashes. Only actual node/indexer/wallet evidence earns a live pass. No remote CI or push is included.



### 2026-10-06 preparation and architecture finding

Issue: https://github.com/MediaNoxLabs/compact/issues/321. The three-export acceptance source compiles with ABI49 strict recording. Eight offline guards pass, including real pinned ledger-v8 input acceptance against complete state and precise unknown-root refusal against its filtered projection. No live services have started.

The originally planned full-ledger binder cannot consume public filtered observations. See [ADR217 — Confirmed observation binding design](references.md#private-note-04) for the proposed explicit observation policy, executable checkpoint checks, preserved full-ledger route, differential tests and node-owned history/freshness boundary. Runtime implementation awaits root design review; the original no-runtime-change assumption is superseded only as a proposal, not by hidden implementation.



### Signed independent preparation

Local GPG+DCO commit `4759a9815f76bcf8dd26d45c2ea9481fa0230392` adds five wallet-live-only files. Eight offline guards and the unedited generated-crate offline Cargo check pass; bootstrap, accept and release all require and expose recording. Counter unchanged. Receipt `${LOCAL_EVIDENCE}/compact-adr217-preparation-receipt.json`, SHA256 `cb4c67599a398147818947c9264a79e3fd407c62fa92cbaf75b74639df4a9b5e`. Runtime observation-binder proposal remains under review. No proofs, services, push or live acceptance claim.



#### Accepted bounded runtime amendment

Root reviewed and approved the explicit observational binder design on 2026-10-06. Implementation will preserve the existing full-ledger route and expose a distinct trusted-observation API. Checkpoint evidence remains private/immutable; runtime checks internal consistency, while the adapter verifies canonical finalized block and event/network identity. The prepared call retains an explicit admission-policy marker.

The first policy admits only normalized upstream guaranteed persistent offers. Transients/fallible placement are rejected. Allocation follows normalized upstream iteration with checked bounds, never a handwritten alternative sort or fabricated LedgerState. Existing exact funding/owner/recorded-intent reconciliation is shared. Global history, spent nullifiers and concurrent freshness remain node-owned and explicitly documented.

Tests will compare real complete-state and observation routes for identical allocations, recorded effects/gas and prepared bytes, plus malformed checkpoint/root/frontier/event/block/offer/funding negatives. ABI49 remains only if the additive API preserves generated compatibility. Services still wait for the final source freeze. This accepted scope supersedes the initial acceptance-only assumption; no runtime code preceded the decision.


### Checkpoint transport continuation (2026-10-06, before JS edits)

A separate JS-only adapter under tools/compact-rust-backend/wallet-live will acquire one canonical finalized block checkpoint and verify exact node ContractState, node filtered Zswap tree/root, indexer first-free frontier/final Zswap event and one immutable serialized wallet snapshot. It must validate the pinned node state_call SCALE argument and Result<Vec<u8>,LedgerApiError> response against upstream byte layouts, and reject unsupported transport rather than guess bare32 address bytes or fabricate history. It will reject same-block later-tree changes, wallet ahead/behind event IDs, root/frontier mismatch and changed wallet snapshot SHA256. Unit tests use pinned byte fixtures and mocked endpoint responses; no services start and no live acceptance is claimed. Runtime/builder/handoff files belong to separate owners and remain untouched. This continuation implements the approved transport task in ${LOCAL_EVIDENCE}/compact-adr217-live-adapter-handoff.md under existing issue #321.

### ADR217 bounded runtime and strict proof delivery

Signed GPG + DCO commit: `19af530af5385e467b6f24b89c6cbdc89048fe86` (wallet-only preparation: `4759a9815f76bcf8dd26d45c2ea9481fa0230392`). Runtime ABI 49 / schema 20 are unchanged.

Added a distinct trusted-observation offer binder, immutable checkpoint evidence, exact acquired-wallet-byte digest retention and shared exact offer reconciliation. The complete-ledger route still calls upstream `try_apply`. The observation constructor checks internal consistency; adapter canonicality/finality/event/network checks and node history/nullifier/freshness validation remain explicit requirements. Transient/fallible observations and input-only burns are refused.

Local validation: 37 runtime library tests, 7 recording identity tests, 10 JavaScript guards, 34 Python tests; runtime/proof and isolated generated bootstrap consumer pass strict Clippy. Original receive and qualified send-to-self each verified a 4,480-byte call proof and default-strict ledger application with replay refusal. The acceptance-only mint → output → claim bootstrap verified a 2,912-byte proof (k 11 / 1,911 rows), default-strict application and encrypted wallet recovery of value 42; changed amount/color/commitment fail concrete builder validation. Existing full proof gate retains legacy runs and adds observational receive/send selectors using the same keys.

No services were started and no live acceptance is claimed. This issue remains open for the RPC/SCALE checkpoint adapter, standalone generated action builder and isolated live wallet run. Existing Counter driver remains unchanged. Remaining work is documented in the midnight vault note **ADR217 — Remaining live adapter handoff**.

Receipt: `${LOCAL_EVIDENCE}/compact-adr217-delivery/receipt.json`; retained proof logs: `${LOCAL_EVIDENCE}/compact-adr217-strict-proof-final.log`, `${LOCAL_EVIDENCE}/compact-adr217-bootstrap-proof-passed.log`. The qualified offline proof sends to self; live release-to-wallet remains separate.

### Lifecycle orchestration continuation (2026-10-06, before implementation)

Owner: funded_coin_finish. A separate `shielded-check.mjs` will reuse the pinned Counter wallet configuration, key derivation, NIGHT-to-Dust registration and canonical indexed-action checks. Counter `check.mjs` remains unchanged. This JS-only lane uses no Cargo target and starts no services; root owns the isolated 49944/48088/46300 stack and freeze.

The runner will coordinate one versioned private manifest with the standalone generated Rust builder and checkpoint adapter owners. Its real-chain sequence is deployment and confirmation, bootstrap mint and confirmed encrypted wallet recovery, selection of the exact full-value wallet coin and upstream Input, offer-bound accept and confirmed contract ownership, full release to that wallet, confirmed recovered output/value, then node replay refusal of the retained submitted transaction. It must never insert an offline coin, rewrite wallet synchronization metadata, infer filtered-tree frontier, or report an offline test as live acceptance.

Each stage retains exact source/compiler/runtime/lock/key identities, transaction hash and entry point, canonical block and checkpoint root/frontier/event, serialized offer fingerprints before/after Dust balancing, and private file hashes/lengths. Preimages and wallet snapshots stay in an explicit 0700 working directory with exclusive 0600 files; no secret material enters public receipts or error logs. The existing offer-fingerprint guard must pass after facade finalization. Exact pending-spend/reservation handling follows the pinned wallet public APIs, with no private state mutation.

Offline tests will exercise configuration, private-file and manifest identity guards, lifecycle ordering, changed-offer refusal, failed/mismatched confirmations, wallet-recovery selection and replay failure classification using injected dependencies. API assumptions will be checked against installed package sources. Actual live submission waits for root service readiness; unresolved transport, builder or SDK boundaries remain explicit blockers rather than weakened checks. Live pass requires all actual chain phases and final node rejection evidence.


## ADR217 private live handoff schema v1

All manifests use `format: "compact-shielded-live/v1"`. Files are local, private (0600 inside a 0700 directory), exclusive writes, with no secret data logged. A binary reference is `{path, sha256, bytes, encoding}`; absolute paths, exact lowercase SHA256, byte count, and fully consumed upstream binary decoding are required. Requests and receipts are private too.

### Checkpoint producer → Rust builder

`kind: "checkpoint"`, `metadata` contains `networkId`, `ledgerVersion: "8.0.3"`, `addressHex`, `transactionHash`, `blockHash`, `blockHeight`, `parentBlockHash`, `nodeZswapRootHex`, `firstFree`, `finalZswapEventId`. Integers use decimal strings; hashes are raw canonical hexadecimal without 0x. Root is the upstream untagged MerkleTreeDigest serialization. `transactionHash` identifies the confirmed action whose contract state is being observed; the adapter verifies final block state, canonicality and event identity.

`wallet` contains `blockHash`, `blockHeight`, `networkId`, `ledgerVersion`, `appliedEventId`, `walletStateSha256`.

`files` contains `nodeContract` (`ContractState` tagged), `contractTree` (`ZswapChainState` tagged, contract projection only), `walletState` (`ZswapLocalState` tagged). Rust consumes the projection's coin tree only and verifies both rehashed roots, frontier, versions, event/network and block consistency before constructing the trusted runtime checkpoint. JS verifies canonical finality and exact acquired wallet bytes before handing off. Neither claims consensus from metadata alone.

### Action request → standalone Rust builder

CLI: `shielded-builder <deploy|bootstrap|accept|release> <request.json> <new-output-directory>`.

Common request: `format`, `kind: "action-request"`, `networkId`, `ttlEpochSeconds` (decimal string), `artifactsRoot` (generated artifact directory with all three keys and bzkir). Calls additionally carry `checkpoint` as a hashed JSON file reference. TTL must be 60 seconds to one hour in the future. Network must match the checkpoint. No private seed is passed to Rust.

`deploy`: no further fields. The operation map includes bootstrap, accept and release verifier keys before deriving the address.

`bootstrap`: `domainHex`, `nonceHex`, `amount` decimal Uint64, `walletCoinPublicKeyHex`, `walletEncryptionPublicKeyHex` (upstream untagged key serialization). The builder derives color and commitment from the actual observed deployment address and supplied domain, amount and wallet keys; it creates the encrypted output.

`accept`: `input` is a binary reference encoded `Input<ProofPreimage>` from the wallet public initSwap transition; `coin` contains `nonceHex`, `colorHex`, `value`, `mtIndex` decimal fields. The builder retains the complete input and selects exactly that wallet input, creates one equal-value contract output and calls generated accept. The JS owner retains the reservation transaction for public rollback on failure.

`release`: `coin` contains the confirmed contract coin as above, plus actual `walletCoinPublicKeyHex` and `walletEncryptionPublicKeyHex`. Only a full-value release is admitted by this acceptance builder. The builder constructs actual contract membership input, uses generated native preview for the sent nonce, then proves the matching recorded call against the retained offer. No handwritten substitute for send semantics.

### Builder → wallet finalizer

`kind: "action-result"`, `action`, `networkId`, `addressHex`, `contractAddress` (tagged binary ContractAddress for the node SCALE bridge), `transaction` (tagged sealed `Transaction<Signature,Proof,Binding>`), `guaranteedOffer` (tagged `Offer<Proof>` or null), `offerFingerprint` (`{guaranteed: sha256-or-null, fallible: []}`), `coin` (result coin with allocated index, if any), `checkpointSha256` (or null), `policy` (`trusted-observation` or `deployment`), `artifacts` hashes.

The transaction has contract and Zswap proofs and is sealed but still needs wallet Dust balance/finalization. The wallet finalizer must preserve exact proven shielded offer bytes and placement. Node default strict admission, indexing, wallet recovery and replay refusal remain live gates. Builder output alone is not live acceptance.


### Lifecycle preparation delivery

Signed GPG+DCO source `04abb003e0f7712b12954644a120bdbb93b8e4ce` adds the separate shielded lifecycle runner and its private handoff guards. Seven new lifecycle tests plus ten existing provenance/handoff/real-projection tests pass. Node syntax and Prettier checks pass. The exact GraphQL action query was accepted by the isolated indexer in a read-only lookup.

No live shielded submission has occurred. The runner rejects node ledger constraints other than `=8.0.3` before wallet startup, then waits for proof-server HTTP readiness. It uses public atomic `initSwap`, checks the exact selected Input/nullifier and no change, enables Dust-only balancing, checks complete offer fingerprints, and retains private evidence for every confirmed phase. Counter is unchanged.

Receipt: `${LOCAL_EVIDENCE}/compact-adr217-lifecycle-receipt.json`. Checkpoint and builder integration and real chain acceptance remain required; this is not a live acceptance receipt.


### Decision correction — pinned candidate ledger pairing (2026-10-06)

The isolated node 0.22.3 reports raw ledger dependency `=8.0.2`; indexer 4.0.1 also resolves ledger 8.0.2, while the generated runtime and checkpoint builder decode with ledger 8.0.3. The earlier unconditional `=8.0.3` node preflight cannot exercise this established stack. We approve only candidate profile `node-0.22.3/indexer-4.0.1/ledger-8.0.2/codec-8.0.3`, requiring exact node software `0.22.3-6f0ef437`, indexer image/version 4.0.1, raw node constraint `=8.0.2`, and codec 8.0.3. Preserve all four distinct fields and `candidate-live-validation-pending` in the private checkpoint manifest/receipt; reject unknown pairings. `assertNodeProfile` is the shared preflight API before wallet start and checkpoint acquisition repeats it at the action block. This is permission to run controlled local acceptance, not a compatibility claim. Strict bootstrap, receive, release, offer preservation, wallet recovery, node replay rejection and negative checkpoint cases must pass before promotion.

Source audit: pinned zswap 8.0.0 and 8.0.3 source trees are byte-identical (SHA256 `6e47c0e6590a466d7151c55cc0c38567e737e7bb4bdbdcf6954cbeb940705824`); ledger-v8 semantics (`3433192262f9424e44b1ea6a3e5dfe775ef41d43585841a57c7ab441ca683508`), verify (`c028596ab2b9b8ae350973ba9b9685222ce327e6f96ba18f3fa747dc319e959e`) and serialization are unchanged from 8.0.2 to 8.0.3. Changed construct.rs concerns client partition/retarget/gas, while structure.rs changes visibility/constant. Node and indexer lockfiles retain 8.0.2; codec lock retains 8.0.3. Matching official node/indexer tags for 8.0.3 were not found in this release lane. The adapter does not relabel node evidence as 8.0.3.

The pinned runtime API requires a SCALE Vec of an upstream tagged ContractAddress. Real deployed address `d88251c3fc3b7f378b75a0973843ba43ed835616270cca64a6b4434837b1166b` has tagged SHA256 `05e2cff9509fecbf712701a9226e75b98063eefdd33cb26fbe9f3d1fe9362859`. The adapter tests this golden request and the live network/version/root response frames. The JS ledger package lacks wallet/tree root getters, so the Rust builder must decode exact binary refs and rehash both roots; the adapter validates finality, action, indexer frontier/event, node root and immutable wallet bytes. The checkpoint schema adds explicit compatibility metadata; generated ABI and IR schema do not change.

### Standalone builder schema refinements

The agreed schema below records acquired snapshot checks, exact candidate node/codec identities, visible-leaf owner/index guards, generated preview/recording equality and embedded build provenance. This supersedes the earlier temporary refusal of every8.0.2 node: parent source audit `${LOCAL_EVIDENCE}/compact-adr217-version-source-audit.json` authorized only the named candidate pairing for controlled live validation; it is not marked accepted. Runtime and emitter remain unchanged.

## ADR217 private live handoff schema v1

All manifests use `format: "compact-shielded-live/v1"`. Files are local, private (0600 inside a 0700 directory), exclusive writes, with no secret data logged. A binary reference is `{path, sha256, bytes, encoding}`; absolute paths, exact lowercase SHA256, byte count, and fully consumed upstream binary decoding are required. Requests and receipts are private too.

### Checkpoint producer → Rust builder

`kind: "checkpoint"`, `metadata` contains `networkId`, `ledgerVersion: "8.0.3"` (codec), `nodeLedgerConstraint: "=8.0.3"` (actual node dependency), `addressHex`, `transactionHash`, `blockHash`, `blockHeight`, `parentBlockHash`, `nodeZswapRootHex`, `firstFree`, `finalZswapEventId`. Integers use decimal strings; hashes are raw canonical hexadecimal without 0x. Root is the upstream untagged MerkleTreeDigest serialization. `transactionHash` identifies the confirmed action whose contract state is being observed; the adapter verifies final block state, canonicality and event identity.

`wallet` contains `blockHash`, `blockHeight`, `networkId`, `ledgerVersion`, `appliedEventId`, `walletStateSha256`.

`files` contains `nodeContract` (`ContractState` tagged), `contractTree` (`ZswapChainState` tagged, contract projection only), `walletState` (`ZswapLocalState` tagged). Rust consumes the projection's coin tree only and verifies both rehashed roots, frontier, versions, event/network and block consistency before constructing the trusted runtime checkpoint. JS verifies canonical finality and exact acquired wallet bytes before handing off. Neither claims consensus from metadata alone.

### Action request → standalone Rust builder

CLI: `shielded-builder <deploy|bootstrap|accept|release> <request.json> <new-output-directory>`.

Common request: `format`, `kind: "action-request"`, `networkId`, `ttlEpochSeconds` (decimal string), `artifactsRoot` (generated artifact directory with all three keys and bzkir). Calls additionally carry `checkpoint` as a hashed JSON file reference. TTL must be 60 seconds to one hour in the future. Network must match the checkpoint. No private seed is passed to Rust. The exact candidate `compatibilityProfile: "node-0.22.3/indexer-4.0.1/ledger-8.0.2/codec-8.0.3"` additionally requires `nodeSoftwareVersion: "0.22.3-6f0ef437"`, `indexerVersion: "4.0.1"`, `nodeLedgerConstraint: "=8.0.2"` and `compatibilityStatus: "candidate-live-validation-pending"`. Parent review established unchanged relevant node validation/dependency semantics; controlled live tests are still required. Unknown pairings are refused. Metadata is retained in `nodeProfile`; no relabeling or automatic acceptance occurs. Reference ledger8.0.3 offline fixtures retain their actual declaration.

`deploy`: no further fields. The operation map includes bootstrap, accept and release verifier keys before deriving the address.

`bootstrap`: `domainHex`, `nonceHex`, `amount` decimal Uint64, `walletCoinPublicKeyHex`, `walletEncryptionPublicKeyHex` (upstream untagged key serialization). The builder derives color and commitment from the actual observed deployment address and supplied domain, amount and wallet keys; it creates the encrypted output.

`accept`: `input` is a binary reference encoded `Input<ProofPreimage>` from the wallet public initSwap transition; `coin` contains `nonceHex`, `colorHex`, `value`, `mtIndex` decimal fields. The builder retains the complete input and selects exactly that wallet input, creates one equal-value contract output and calls generated accept. The JS owner retains the reservation transaction for public rollback on failure.

`release`: `coin` contains the confirmed contract coin as above, plus actual `walletCoinPublicKeyHex` and `walletEncryptionPublicKeyHex`. Only a full-value release is admitted by this acceptance builder. The builder constructs actual contract membership input, uses generated native preview for the sent nonce, then proves the matching recorded call against the retained offer. No handwritten substitute for send semantics.

### Builder → wallet finalizer

`kind: "action-result"`, `action`, `networkId`, `addressHex`, `contractAddress` (tagged binary ContractAddress for the node SCALE bridge), `transaction` (tagged sealed `Transaction<Signature,Proof,Binding>`), `guaranteedOffer` (tagged `Offer<Proof>` or null), `offerFingerprint` (`{guaranteed: sha256-or-null, fallible: []}`), `coin` (result coin with allocated index and `commitmentHex`, if any), `checkpointSha256` (or null), `policy` (`trusted-observation` or `deployment`), `artifacts` key/ZKIR hashes, and embedded `buildProvenance` (compiler/source/generated/runtime-source-tree/repository-lock hashes). The builder rejects an artifact crate whose Cargo/lib bytes differ from its unedited compiled dependency.

The transaction has contract and Zswap proofs and is sealed but still needs wallet Dust balance/finalization. The wallet finalizer must preserve exact proven shielded offer bytes and placement. Node default strict admission, indexing, wallet recovery and replay refusal remain live gates. Builder output alone is not live acceptance.


### Standalone builder strict delivery evidence

The standalone consumer now builds without changing generated code or runtime/emitter APIs. All three operations are deployed before deriving the address. Offline deploy → bootstrap mint → wallet receive → contract accept → full release passed default-strict ledger admission/application with exact retained offer proof identity after separate Dust balancing. Wallet ownership changes 0 → 1 → 0 → 1; selected-coin mismatch, collapsed/out-of-range/wrong-owner input indices, independent decoded tree/wallet RootMismatch, digest/profile mismatches and spent-input replay are rejected. Three test groups passed in 45.82 seconds; final standalone strict Clippy and actual CLI deployment artifact creation passed. No live submission is claimed.

Preserved failure history: `${LOCAL_EVIDENCE}/compact-adr217-builder-strict.log` exposed a real bad-index boundary: the upstream membership constructor panics when indexing an unrelated collapsed leaf. The builder now enumerates visible leaves and checks exact index, commitment and contract owner before construction. `${LOCAL_EVIDENCE}/compact-adr217-builder-strict-final.log` was a different, test-only wrong-root fixture preparation failure: synthetic mutation at a collapsed index panicked. That negative fixture now constructs a separate blank wrong-root tree with valid serialization/hash, then checks precise RootMismatch. No runtime insertion change was required. The production builder never appends to a filtered tree; allocation follows the immutable normalized offer and observed frontier, while node admission uses full state. Hidden tree portions remain untouched.

Final logs: `${LOCAL_EVIDENCE}/compact-adr217-builder-strict-passed.log`, `${LOCAL_EVIDENCE}/compact-adr217-builder-final-build.log`, `${LOCAL_EVIDENCE}/compact-adr217-builder-final-clippy.log`, `${LOCAL_EVIDENCE}/compact-adr217-builder-cli.log`. Source-matching artifacts `${LOCAL_EVIDENCE}/compact-adr217-bootstrap` include keys for bootstrap (k11/1911), accept (k13/6536), release (k15/19716). The isolated consumer is `${LOCAL_EVIDENCE}/compact-adr217-live-builder-final`. The known node8.0.2/codec8.0.3 candidate profile is retained without relabeling; only later live evidence can establish the bounded acceptance claim.


#### Transport correction from live Counter probe

The preceding tagged-RPC inference was wrong. Pinned node ledger Bridge `get_contract_state` and `get_zswap_chain_state` call `api.deserialize::<ContractAddress>` on the decoded SCALE Vec payload; this is the untagged 32-byte address serialization. `Api::deserialize` does not check trailing bytes, so a tagged payload can silently decode its first 32 tag bytes as another address. The read-only Counter block-74 probe demonstrated the failure: tagged request returned Ok(empty) contract state despite the indexer action containing 1482 bytes. The adapter must validate builder-supplied tagged address bytes against the selected address, retain them as a tagged manifest ref, and send SCALE Vec of the exact bare32 address to these two node APIs. A corrected live probe must show nonempty contract state and equality to the indexed final state. This supersedes the preceding statement that node runtime API requires tagged bytes; the positive pinned golden request is bare32 and a negative tagged request demonstrates wrong-address behavior. No submission or wallet mutation occurred.

### ADR217 standalone action builder delivered

Signed GPG + DCO commits `23d38d44a3abfd5d48f2b322166b6321ab3aa455` and `4baa6482f41f0abe170b629a1445bb74184c4ba8` adds only the wallet-live standalone consumer/build script/schema/tests. No runtime/emitter or ABI/schema changes.

The unedited generated crate deploys all three verifier operations. Its private versioned handoffs retain exact upstream binary identity, checkpoint digest and node/codec profile. Bootstrap derives actual deployed token color; receive selects the full wallet Input and confirmed coin; full-value release checks visible qualified commitment/owner/index before the upstream membership constructor and compares generated preview with recorded output.

Three local test groups pass, including actual offline deploy → encrypted bootstrap → wallet-funded accept → full wallet release under default strictness and separate Dust funding. Exact offer proof identity survives fee balancing; wallet coin counts are 0 → 1 → 0 → 1. Root/digest/profile/selected-coin and collapsed/out-of-range/wrong-owner negatives plus spent-input replay reject. Final standalone build, strict Clippy and actual CLI deployment artifact creation pass. No live submission or compatibility acceptance is claimed; the named node0.22.3/indexer4.0.1/ledger8.0.2/codec8.0.3 profile remains a candidate for controlled validation.

Receipt `${LOCAL_EVIDENCE}/compact-adr217-builder-delivery/receipt.json`; proof log `${LOCAL_EVIDENCE}/compact-adr217-builder-strict-passed.log`; isolated consumer `${LOCAL_EVIDENCE}/compact-adr217-live-builder-final`; matching keys `${LOCAL_EVIDENCE}/compact-adr217-bootstrap`. The ADR retains two distinct failure histories (real collapsed-read input guard, then test-only collapsed mutation) and their precise fixes. Live adapter assembly and node validation remain open in this issue.

Final policy correction removes the raw ledger8.0.3 CLI shortcut: only the complete reviewed candidate pins are admitted. Offline reference fixtures are behind cfg(test), absent from production; an actual CLI invocation rejects that profile before proving and writes no artifacts. Three groups and the full strict sequence passed again (45.92 seconds), with strict Clippy. Final receipt SHA256: `c5e7465e8459828dbc129db0351ee063eef7b97aaa55447fe91eaa78cd15bf57`.


### Coordinated lifecycle preparation

Signed GPG+DCO follow-up `a6740f9761f3cc24a9e8e04edcf64d4a126b5fd5` consumes the signed checkpoint adapter and its exact reviewed candidate profile: node `0.22.3-6f0ef437`, indexer `4.0.1`, actual node ledger `=8.0.2`, pinned codec `8.0.3`. All identities and candidate status are retained separately. Unknown combinations are rejected before wallet startup. This does not claim live compatibility.

All 24 combined checkpoint/lifecycle/provenance/handoff/projection tests pass. Four actual sealed strict-offline builder results also decode and pass the JS action/address/entry-point/offer guards. The shared manifest field review found no mismatch.

The generic replay matcher has been removed. The first controlled live run will retain the actual node refusal privately and leave replay incomplete until its exact rejection is reviewed. No timeout, transport failure or generic invalid-transaction text can produce a passed receipt. No shielded live submissions have occurred yet. Fresh bundled-runtime builder verification and the controlled chain run remain required.

Receipt: `${LOCAL_EVIDENCE}/compact-adr217-lifecycle-coordinated-receipt.json`.


### ADR217 portable generated consumer correction

Signed GPG + DCO commit `08b276dada98105e70ac3ceb83a79a2a12e79c7e` fixes the builder's hardcoded runtime package path. The earlier passing acceptance compiler invocation used `--rust-runtime-root`, so its unedited generated manifest selected the repository runtime. A fresh default portable output instead selects bundled `contract/runtime-rs`; adding a second repository dependency caused Cargo's package collision. No generated file was edited in either case.

The builder now resolves the actual runtime path from the generated Cargo manifest, uses that same package identity directly, verifies both runtime and macro package source inventories against the current reference, and records the selected path. Four focused Python cases cover bundled and absolute paths, changed/missing/extra sources, macro drift and wrong package/nonlocal dependency refusal. The untouched fresh root portable crate built successfully with actual `${LOCAL_EVIDENCE}/compact-d1411fe5-compactc` provenance; strict Clippy and exact scoped repository header checks pass. All four new Python/Rust files now have the complete license template.

Final consumer: `${LOCAL_EVIDENCE}/compact-adr217-root-portable-builder-release`. Final binary: `${COMPACT_SOURCE}/target/compactc-consumer/debug/compact-wallet-shielded-builder`. Receipt: `${LOCAL_EVIDENCE}/compact-adr217-portable-delivery/receipt.json`. Earlier strict offline proof evidence remains unchanged; no live submission is claimed here.

Signing history: the initial local corrective commit `a706e676` failed signature verification and was not delivered for integration. It was amended/re-signed as `08b276da`, whose signature is verified good. A header tool `--help` invocation unexpectedly added headers broadly; all 116 unrelated tracked changes in this isolated checkout were immediately restored to HEAD. Only the intended builder files remain in this correction; validation subsequently imported HeaderManager with explicit paths.


### Live lifecycle and precise replay classification — 2026-10-06

Frozen root cc333811 completed deployment block276, bootstrap280, accept286 and release293 on the pinned candidate stack. Each checkpoint agreed across actual node contract/tree/root, indexer frontier/event and exact decoded wallet snapshot. The actual wallet recovered the full42 after release; offer bytes stayed identical through Dust balancing. The first runner intentionally ended incomplete at replay classification. Raw private node error is retained in shielded-run-cc333811/replay-rejection.txt.

Observed replay response: nested SDK/Effect failure contains RpcError with code1013 and message prefix `1013: Transaction Already Imported`. Decision: unwrap the pinned Effect failure through public Runtime/Cause APIs and standard Error.cause, with bounded traversal; accept only that exact RpcError code/name/message combination. Preserve the private raw response and record this as identical-finalized-transaction duplicate rejection. Arbitrary transport failures, different codes, generic invalid-transaction messages and matching prose on ordinary Error remain incomplete. This result does not establish fresh-transaction spent-nullifier admission behavior; strict in-process spent-input tests remain separately identified.

Before: every replay failure saved an unclassified response and failed the gate. After: only the observed structured node duplicate response closes this bounded replay gate. Emitter/runtime unchanged; runner and pinned direct Effect dependency only. Repeat the actual complete lifecycle at the final integrated revision to verify classification, with independent negative classifier tests.


### Final clean release acceptance — 2026-10-06

Frozen revision `d5dd3f2eeeaad337ba40fcf2639d540a9738f161` passed clean `nix build .#compactc-binary`, all 19 relocated portable checks, and a fresh external Nix consumer whose local Git input is locked to the exact revision. This is **aarch64-darwin** acceptance only.

The archive works from a path with spaces and a relative installer symlink with runtime/Scheme overrides unset. Default TypeScript, strict Rust, offline generated Cargo checks, bundled ZKIR key generation, original Coracle 4 recording APIs and microDAO 7 recording APIs pass. Full bundled source inventory comparison covers 25 runtime and 4 macro files/manifests; the temporary verifier was strengthened from its previous selected-file check.

Receipt: `${LOCAL_EVIDENCE}/compact-d5dd3f2e-release-acceptance-receipt.json`; SHA256 `6c620e0aab97c7668661d0988c66199cb3ac19909e8c61634a172013dc07099e`. Portable archive: `${LOCAL_EVIDENCE}/compact-d5dd3f2e-portable-final/compactc.zip`; SHA256 `a23f6e147bb3f7206d8126ef86e5daa4a6a3437f8072efba18baabd1d427d159`. Full command logs and external flake lock hashes are retained in the receipt.

The clean merkle checkout remains detached at the frozen revision without source edits. The assigned `adr157` Cargo target is released. The initial bare `nix` invocation failed before building because it was absent from shell PATH; the successful build uses `/nix/var/nix/profiles/default/bin/nix`. No product defect or source correction was required.
