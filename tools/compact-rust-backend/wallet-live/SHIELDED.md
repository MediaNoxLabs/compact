# Shielded live acceptance preparation (ADR217)

Status: offline preparation, **not a live acceptance result**. The existing Counter
`check.mjs` remains the live baseline. Issue: [#321](https://github.com/MediaNoxLabs/compact/issues/321).

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
An explicit observation-based binder and exact wallet/block checkpoint adapter
are under ADR217 design review; they are not implemented by these helpers.

Private pre-proof handoffs can contain wallet secret material. Use a private
working directory and exclusive 0600 files; receipts contain only lengths and
hashes. After Rust proving, wallet Dust balancing must preserve serialized
shielded offers and placement exactly. The helper detects substituted proofs,
extra inputs/change, and segment moves.

Live orchestration, same-block wallet root/frontier/event validation, actual
bootstrap/receive/release proofs, confirmed ownership and node replay refusal
remain required before this lane can claim live acceptance. Services start only
after the final source checkpoint is frozen.
