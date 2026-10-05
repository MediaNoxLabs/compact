# ADR217 private live handoff schema v1

All manifests use `format: "compact-shielded-live/v1"`. Files are local, private (0600 inside a 0700 directory), exclusive writes, with no secret data logged. A binary reference is `{path, sha256, bytes, encoding}`; absolute paths, exact lowercase SHA256, byte count, and fully consumed upstream binary decoding are required. Requests and receipts are private too.

## Checkpoint producer → Rust builder

`kind: "checkpoint"`, `metadata` contains `networkId`, `ledgerVersion: "8.0.3"` (codec), `nodeLedgerConstraint: "=8.0.3"` (actual node dependency), `addressHex`, `transactionHash`, `blockHash`, `blockHeight`, `parentBlockHash`, `nodeZswapRootHex`, `firstFree`, `finalZswapEventId`. Integers use decimal strings; hashes are raw canonical hexadecimal without 0x. Root is the upstream untagged MerkleTreeDigest serialization. `transactionHash` identifies the confirmed action whose contract state is being observed; the adapter verifies final block state, canonicality and event identity.

`wallet` contains `blockHash`, `blockHeight`, `networkId`, `ledgerVersion`, `appliedEventId`, `walletStateSha256`.

`files` contains `nodeContract` (`ContractState` tagged), `contractTree` (`ZswapChainState` tagged, contract projection only), `walletState` (`ZswapLocalState` tagged). Rust consumes the projection's coin tree only and verifies both rehashed roots, frontier, versions, event/network and block consistency before constructing the trusted runtime checkpoint. JS verifies canonical finality and exact acquired wallet bytes before handing off. Neither claims consensus from metadata alone.

## Action request → standalone Rust builder

CLI: `shielded-builder <deploy|bootstrap|accept|release> <request.json> <new-output-directory>`.

Common request: `format`, `kind: "action-request"`, `networkId`, `ttlEpochSeconds` (decimal string), `artifactsRoot` (generated artifact directory with all three keys and bzkir). Calls additionally carry `checkpoint` as a hashed JSON file reference. TTL must be 60 seconds to one hour in the future. Network must match the checkpoint. No private seed is passed to Rust. The exact candidate `compatibilityProfile: "node-0.22.3/indexer-4.0.1/ledger-8.0.2/codec-8.0.3"` additionally requires `nodeSoftwareVersion: "0.22.3-6f0ef437"`, `indexerVersion: "4.0.1"`, `nodeLedgerConstraint: "=8.0.2"` and `compatibilityStatus: "candidate-live-validation-pending"`. Parent review established unchanged relevant node validation/dependency semantics; controlled live tests are still required. Unknown pairings are refused. Metadata is retained in `nodeProfile`; no relabeling or automatic acceptance occurs. Reference ledger8.0.3 fixtures use `offline-ledger-8.0.3-fixture` / `offline-not-live` only behind `cfg(test)`. This branch is absent from the CLI. The CLI accepts only the complete reviewed candidate pins, including the actual node constraint; an otherwise unknown raw `=8.0.3` is refused.

`deploy`: no further fields. The operation map includes bootstrap, accept and release verifier keys before deriving the address.

`bootstrap`: `domainHex`, `nonceHex`, `amount` decimal Uint64, `walletCoinPublicKeyHex`, `walletEncryptionPublicKeyHex` (upstream untagged key serialization). The builder derives color and commitment from the actual observed deployment address and supplied domain, amount and wallet keys; it creates the encrypted output.

`accept`: `input` is a binary reference encoded `Input<ProofPreimage>` from the wallet public initSwap transition; `coin` contains `nonceHex`, `colorHex`, `value`, `mtIndex` decimal fields. The builder retains the complete input and selects exactly that wallet input, creates one equal-value contract output and calls generated accept. The JS owner retains the reservation transaction for public rollback on failure.

`release`: `coin` contains the confirmed contract coin as above, plus actual `walletCoinPublicKeyHex` and `walletEncryptionPublicKeyHex`. Only a full-value release is admitted by this acceptance builder. The builder constructs actual contract membership input, uses generated native preview for the sent nonce, then proves the matching recorded call against the retained offer. No handwritten substitute for send semantics.

## Builder → wallet finalizer

`kind: "action-result"`, `action`, `networkId`, `addressHex`, `contractAddress` (tagged binary ContractAddress for the node SCALE bridge), `transaction` (tagged sealed `Transaction<Signature,Proof,Binding>`), `guaranteedOffer` (tagged `Offer<Proof>` or null), `offerFingerprint` (`{guaranteed: sha256-or-null, fallible: []}`), `coin` (result coin with allocated index and `commitmentHex`, if any), `checkpointSha256` (or null), `policy` (`trusted-observation` or `deployment`), `artifacts` key/ZKIR hashes, and embedded `buildProvenance` (compiler/source/generated/runtime-source-tree/repository-lock hashes). The builder rejects an artifact crate whose Cargo/lib bytes differ from its unedited compiled dependency.

The transaction has contract and Zswap proofs and is sealed but still needs wallet Dust balance/finalization. The wallet finalizer must preserve exact proven shielded offer bytes and placement. Node default strict admission, indexing, wallet recovery and replay refusal remain live gates. Builder output alone is not live acceptance.
