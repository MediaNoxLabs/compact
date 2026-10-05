# Standalone shielded acceptance builder

This is a consumer of the **unedited** three-export generated `shielded.compact` crate. It changes no compiler or runtime API. It produces proven and sealed transactions for wallet Dust finalization; only node submission and confirmed wallet recovery establish live acceptance.

Build with an existing warm Cargo target:

```sh
python3 tools/compact-rust-backend/wallet-live/build-shielded-builder.py \
  --generated-contract /absolute/artifacts/contract \
  --compiler /absolute/frozen/compactc \
  --scratch /absolute/new-consumer-directory \
  --cargo-target-dir /absolute/existing-cargo-target
```

Generate the contract using `--rust-runtime-root` pointing to the matching repository. Compile all three `bootstrap`, `accept`, `release` ZKIR files with pinned ZKIR 2.1.0. Preserve their prover/verifier/bzkir files under the same artifact root. The builder installs all three verifier operations at deployment, then derives bootstrap token color from the actual resulting address. No shielded coin is inserted offline for a live claim.

The binary is `TARGET/debug/compact-wallet-shielded-builder`:

```text
compact-wallet-shielded-builder deploy PRIVATE_REQUEST.json NEW_OUTPUT_DIR
compact-wallet-shielded-builder bootstrap PRIVATE_REQUEST.json NEW_OUTPUT_DIR
compact-wallet-shielded-builder accept PRIVATE_REQUEST.json NEW_OUTPUT_DIR
compact-wallet-shielded-builder release PRIVATE_REQUEST.json NEW_OUTPUT_DIR
```

See [SCHEMA.md](SCHEMA.md) for exact manifests. Request/reference files and their immediate directories must be private. Binary decoding consumes every byte. Outputs are exclusive 0600 writes inside a new 0700 directory. No wallet seed enters Rust. Acquisition SHA256 is checked before decoding the wallet snapshot. Candidate node/codec compatibility is an explicit retained identity, not a consensus assertion.

Acceptance intentionally supports one full-value wallet input for receive and one full-value qualified contract input for release. The former must match the checkpoint wallet's exact coin/index under the input nullifier; the latter must be a visible leaf with exact commitment and contract owner before the upstream membership constructor runs. This prevents a collapsed-leaf index panic from becoming a CLI crash. Generated native release preview determines the sent nonce; the retained offer and generated recorded result must agree. It does not introduce a separate send evaluator.

The output `result.json` carries exact sealed transaction and proven offer references, offer fingerprint, coin commitment/index, checkpoint digest, raw node profile and build provenance. Wallet finalization must preserve the shielded offer and placement exactly. Runtime observation checks establish internal consistency only; adapter finality/event checks and node history/nullifier/freshness validation remain required.

## Offline checks

The isolated consumer includes default handoff/decoding/profile tests. The explicit proof test uses actual upstream fee funding, an initially empty shielded tree, real deployment, generated bootstrap, receive and release, default strict ledger admission and encrypted wallet recovery. It is test-only fixture evidence, not a live-chain report.

```sh
CARGO_INCREMENTAL=0 CARGO_TARGET_DIR=/absolute/existing-target \
  cargo +1.99.0 test --offline --manifest-path /absolute/consumer/Cargo.toml

COMPACT_SHIELDED_ARTIFACTS=/absolute/artifacts \
MIDNIGHT_LEDGER_TEST_STATIC_DIR=/absolute/midnight-ledger/ledger/static \
CARGO_INCREMENTAL=0 CARGO_TARGET_DIR=/absolute/existing-target \
  cargo +1.99.0 test --offline --manifest-path /absolute/consumer/Cargo.toml \
  offline_generated_actions_strict -- --ignored --nocapture
```

The strict fixture checks all three deployed operations, wallet ownership `0 → 1 → 0 → 1`, exact offer proof identity across Dust balancing, selected coin mismatch, hidden/collapsed leaf index, out-of-range index, wrong owner, node profile, independent wallet/tree root mismatches, acquired digest mismatch and spent-input replay refusal. Its injected fixture clock is used only by the test's internal function; CLI TTL always uses system time and requires 60 seconds to one hour remaining.
