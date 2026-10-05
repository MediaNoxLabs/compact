# Shielded live acceptance (ADR217)

The local ABI49 lifecycle passed at `628d1c03`: deployment, confirmed bootstrap,
exact wallet-funded receive, confirmed contract ownership, full release to the
wallet, recovered value and rejection of the identical finalized transaction.
The separate Counter `check.mjs` remains the Counter control. Issue:
[#321](https://github.com/MediaNoxLabs/compact/issues/321).

See [the lifecycle runner and prerequisites](SHIELDED-LIFECYCLE.md) for the
current checkpoint, builder and orchestration contract. The later `d5dd3f2e`
header-maintenance revision is distinct from the live receipt's source revision;
its final local/package gates must retain their own exact-head evidence.

## Accepted local evidence and limits

The private run receipt is
`target/rust-live-e671db3b-devnet/shielded-run-628d1c03/receipt.json` in the
acceptance checkout. It retains four confirmed phases, recovered value, exact
offer fingerprints, source/key/tool provenance and the pinned node profile.
Private transaction inputs and wallet snapshots remain private.

The replay result is structured node `RpcError` code `1013`, message
`Transaction Already Imported`, scoped to the identical finalized transaction.
It does not establish live rejection of a newly constructed transaction that
reuses spent inputs. Offline spent-input and binding negatives are separate
evidence. The stack retains node `0.22.3-6f0ef437` / indexer `4.0.1` / node ledger
constraint `=8.0.2` and codec `8.0.3` as distinct identities; this local success
does not establish arbitrary cross-version compatibility. Checkpoints are
trusted local endpoint observations, not authenticated consensus proofs.
Remote CI, publication and other platform acceptance are not claimed.

`shielded.compact` uses three current generated recording APIs: an explicit
bootstrap mint, full-value wallet receive, and qualified full-value release. The
bootstrap is an acceptance-only mint policy; any minted coin must actually be
submitted and observed on chain before it funds the next phase. No offline coin
insertion qualifies as live evidence.

## Offline checks

Use this directory's pinned dependencies (`npm ci` in an isolated checkout), then:

```sh
node --test provenance.test.mjs shielded-handoff.test.mjs shielded-projection.test.mjs
```

When reusing an existing dependency installation without modifying it, set
`COMPACT_LEDGER_V8_MODULE` to its absolute `@midnight-ntwrk/ledger-v8` JavaScript
entry point. The lockfile pins ledger-v8 8.0.3. The projection test uses actual
upstream output/input objects; the same input succeeds against complete state
and fails against filtered state with an unknown-root error.

Compile with a current ABI49 compiler and its matching runtime:

```sh
compactc --target rust --rust-require-recording --skip-zk shielded.compact /tmp/shielded-acceptance
cargo check --manifest-path /tmp/shielded-acceptance/contract/Cargo.toml
```

This checks generated-crate availability; it neither proves the circuits nor
submits a transaction.

## Retained observation boundary

The helpers require exact action/address/entry point/transaction identity and a
canonical block beneath the connected node's finalized head. They preserve the
same action's contract and Zswap bytes with hashes and explicit provenance.
These are trusted endpoint observations, not consensus proofs.

A contract-filtered Zswap tree can provide a real membership witness but omits
global frontier/history/nullifier information. It cannot be passed off as a full
ledger state. `requireOfferReconciliation` preserves this failure instead of
filling missing state. The original full-ledger runtime binder remains unchanged.
The runtime now provides an explicit observation-based binder (see runtime-rs/README.md).
The live runner acquires and validates the exact wallet/block checkpoint through
`checkpoint.mjs`. The helper tests alone do not establish an acquired checkpoint;
the live receipt records the actual acquisition and confirmed actions.

Private pre-proof handoffs can contain wallet secret material. Use a private
working directory and exclusive 0600 files; receipts contain only lengths and
hashes. `decodeWalletSnapshot` verifies the acquired SHA256 before decoding;
that exact acquisition digest is retained in runtime checkpoint evidence.
After Rust proving, wallet Dust balancing must preserve serialized
shielded offers and placement exactly. The helper detects substituted proofs,
extra inputs/change, and segment moves.

The accepted live run includes same-block wallet root/frontier/event validation,
actual bootstrap/receive/release proofs, confirmed ownership and the exact node
replay refusal described above. A new run must satisfy those same lifecycle
requirements; offline checks are not substitutes. Services start only after the
source checkpoint is frozen and the stack owner releases submission.

## Bootstrap strict proof consumer

Bootstrap now explicitly records mint, output creation and its spend claim. The
acceptance builder derives token color from the domain and deployed address,
coin value from the amount, and commitment from the coin and actual wallet key.
The proof consumer checks changed amount/color/commitment refusals, proves the
unedited generated source, applies at default strictness and recovers the
accepted encrypted output in the wallet. It seeds no shielded input; the normal
upstream NIGHT/Dust fixture funds fees. This remains an offline acceptance test.

Compile with `--rust-runtime-root` pointing at this checkout so the generated
consumer uses the additive observation API. Generate the single bootstrap key
with the pinned ZKIR, then run (paths below are placeholders):

```sh
zkir compile /tmp/shielded-acceptance/zkir/bootstrap.zkir \
  /tmp/shielded-acceptance/keys/bootstrap.prover \
  /tmp/shielded-acceptance/keys/bootstrap.verifier

MIDNIGHT_LEDGER_TEST_STATIC_DIR=/path/to/midnight-ledger/ledger/static \
python3 proof-bootstrap.py \
  --generated-contract /tmp/shielded-acceptance/contract \
  --proof-root /tmp/shielded-acceptance \
  --scratch /tmp/new-isolated-bootstrap-consumer \
  --cargo-target-dir /path/to/existing/warm/target
```

Create the `keys` directory first. The scratch directory must be new. The
adapter copies the existing proof utilities with the repository lockfile,
relocates only their support-file references, and adds the generated crate as a
normal dependency. It never edits the generated contract or starts services.
Generated input hashes are saved with the consumer.

The normal full proof gate additionally runs `--observational-receive` and
`--observational-send` using its already-generated receive/send keys, after the
existing complete-ledger runs. The latter proves a full send-to-self. The separate
live lifecycle above supplies release-to-wallet evidence; that result does not
change the narrower scope of these offline proof consumers.
