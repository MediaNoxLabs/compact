# ADR217 live shielded lifecycle runner

`shielded-check.mjs` is separate from the unchanged Counter runner. It consumes
`shielded-builder` and `checkpoint.mjs` using the private
`compact-shielded-live/v1` handoff. Offline guard tests establish orchestration
checks; they do not establish live wallet or chain acceptance.

## Run prerequisites

The stack owner first freezes the compiler, generated source and keys, runs the
Counter control, and releases the isolated services for sequential submission.
This runner accepts only local node `49944`, indexer `48088`, and proof server
`46300`; it refuses the protected oxid ports. The proof server must serve HTTP
and must not enable verbose witness/preimage logging.

Set these environment variables without putting the seed in shell history:

- `COMPACT_RUST_WALLET_SEED_HEX`: the funded local wallet's 32-byte seed.
- `COMPACT_RUST_HANDOFF_NETWORK_ID`: the isolated node network identity.
- `COMPACT_RUST_NODE_URL`: `http://127.0.0.1:49944`.
- `COMPACT_RUST_INDEXER_URL`: `http://127.0.0.1:48088/api/v3/graphql`.
- `COMPACT_RUST_PROOF_SERVER_URL`: `http://127.0.0.1:46300`.
- `COMPACT_RUST_SHIELDED_BUILDER`: the frozen standalone executable.
- `COMPACT_RUST_SHIELDED_ARTIFACTS`: the matching generated artifacts and all
  three proof/verifier key pairs.
- `COMPACT_RUST_SHIELDED_RUN_DIR`: a new absolute private directory; reusing an
  existing run directory is rejected.

Run `node shielded-check.mjs`. The directory and every handoff are retained on
failure. It is not a resume command: inspect the confirmed chain stage before
starting another run. The script never starts, stops or modifies services.

## Required live sequence

1. Sync the wallet and register available NIGHT for Dust if necessary.
2. Build and submit a deployment containing all three operation keys; wait for
   the exact submitted action in a canonical finalized block.
3. Acquire an immutable checkpoint from the node, indexer and one exact wallet
   SDK snapshot. The adapter and builder must agree on version, network, block,
   event, root and frontier. A version mismatch fails closed.
4. Bootstrap a fresh token of value 42 to the actual wallet encryption/public
   keys. Confirm the action and recover its exact nonce, type, value, commitment
   and allocated index in the wallet's available coins.
5. Reserve that one coin using the pinned wallet's public atomic `initSwap`.
   Require one exact user Input and its expected value delta, with no change,
   transients, other token inputs or intent. Export the complete tagged Input
   to the Rust accept builder; keep the reservation until submission.
6. Submit the wallet-funded receive and confirm the contract-owned output. The
   release builder verifies its qualified membership against the checkpoint
   projection before preparing the full-value contract spend.
7. Submit the release to the same wallet and recover the exact returned coin.
8. Resubmit the identical finalized release through the SDK node submission
   service, bypassing its wallet pending cache. Require an actual node refusal;
   a timeout, connection failure or local cache response is not replay evidence.

Only Dust balancing is enabled. The runner checks exact serialized proven
shielded offer hashes and segment placement before balancing and after final
binding; any extra fee-side shielded offer is rejected. A failed pre-submit
accept build returns the reservation using the public wallet API. Once node
submission begins, an ambiguous response leaves the private evidence for
inspection rather than manually changing the wallet state.

## Evidence and limits

Each phase retains the builder request/result, finalized transaction bytes,
exact offer fingerprint, submitted hash, checkpoint and immutable source/key
hashes. The final private receipt records recovered value and the concrete node
replay refusal. The runner additionally hashes its own source, package lock,
acceptance Compact source and builder executable. Only paths/status are printed;
private Input witnesses and wallet snapshots are never printed.

No offline leaf insertion, invented frontier, snapshot relabeling, manual wallet
state mutation, relaxed ledger strictness or regenerated VM transcript is used.
The trusted node/indexer checkpoint remains an explicit local observation
boundary. Successful offline tests or a builder proof do not substitute for the
final `status: passed` live receipt.

Run the independent guards with:

```sh
node --test shielded-check.test.mjs shielded-handoff.test.mjs provenance.test.mjs shielded-projection.test.mjs
```
