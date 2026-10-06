---
id: RUST-ADR-0167
alias: ADR-0167
title: "Typed historic commitment spend recording"
date: 2026-10-05
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["recording", "historic-Merkle", "spend"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: fc717452b6092e4308746371aaf715b6b28a8b60761428a4fdfb6b982e2651aa
---
# RUST-ADR-0167 — Typed historic commitment spend recording

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. The historic-spend profile reuses typed planning and upstream tree/commitment operations for an audited Set and Merkle path. Its deterministic secret witness adapter is test evidence, not production encryption.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#271 closure](https://github.com/MediaNoxLabs/compact/issues/271#issuecomment-6017688583). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`dab9ab25`](https://github.com/MediaNoxLabs/compact/commit/dab9ab25c172c5e1ec8f547dd8c5f8d8d33e1551). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
status: accepted
date: 2026-10-05
```

## Historical decision and amendments

### Problem
Original Zerocash mint is recorded after ADR0159, while spend is native-only. Spend consumes typed coin/key structures, rejects a repeated nullifier, inserts that nullifier before membership checking, validates a historic commitment root and exact leaf, creates a recipient commitment, invokes an opaque encryption witness, writes its ciphertext and removes the private input coin. Reordering these operations changes witness context, query/gas or private effects.

### Before / after
```rust
// Before: native-only Unit result, ciphertext in the declared Cell
ledger_contract::spend(context, &witnesses, destination, input_coin)?;
// After: typed recorded and observed-call APIs
contract.recording().spend(context, destination.clone(), input_coin.clone())?;
contract.recording().spend_call(&observed, private, destination, input_coin)?;
```

### Decision
Extend the existing typed_plan module with a bounded historic-spend admission profile. Reuse actual-type lexical bindings, transitive pure-helper auditing and selected-branch frames. Support one-field Bytes32 wrapper Set keys and HistoricMerkleTree leaves, historic root checks, opaque byte witness results and ciphertext Cell writes. Require typed struct inputs and the profile's historic-root/write/nullifier/ciphertext effects; no contract names. Reuse existing ledger8 slots and aligned private outputs. Source remains Unit-returning; recipient bytes are verified in the ciphertext Cell. No runtime/schema change expected.

### Required evidence
Compose with recorded/native mint. Independently capture unchanged-source TS for current and known historical roots, private state/witness arguments and ordering, recipient ciphertext, full state/effects, VM queries, four gas dimensions and replay. Reject spent nullifiers, unknown roots, mismatched leaves, malformed paths and failed encryption without pretending failed-call partial traces are available. Pinned spend proofs verify and ledger apply for current and historical roots. Renderer negatives, fixture freshness, source capability gates, Clippy and signed local delivery.

### Boundaries
Encryption remains the source-defined witness supplied by callers; tests use a deterministic recipient/coin-dependent adapter and do not claim production cryptography. Historic path snapshots are taken from real ledger trees, with local unmetered inspection kept distinct from metered witness VM reads. Nullifier insertion before a later failure is part of consumed execution context; the Err API exposes no resulting partial trace/state/gas. No remote CI or push.



### Delivery — 2026-10-05

Issue https://github.com/MediaNoxLabs/compact/issues/271 in rust-backend-v2. Signed conventional GPG+DCO commit `8c0d3bf5f4e304272b96e2e8775f27f3051bfea5`, parent `dab9ab25`, branch `codex/adr167-zerocash-spend`. No push or remote CI.

#### Changes
The shared typed plan now accepts one-field Bytes32 wrapper Set keys and historic Merkle leaves, recorded historic-root checks, OpaqueBytes witness results and typed ciphertext Cell writes. Actual declared types and slot indexes remain checked. A separate bounded two-struct-input historic-spend profile requires historic-root/read-write, Set insertion and opaque Cell output effects. Existing membership and optional-message profiles stay separate; pure helper auditing remains transitive. No runtime ABI or schema change.

Original spend remains Unit-returning. Its recipient output is the ciphertext Cell. Ordered effects are preserved: secret witness → nullifier membership → nullifier insertion → commitment path witness/root/leaf checks → fresh coin witness → recipient commitment insertion → encryption witness → ciphertext write → private input removal. The typed input and generated observed-call APIs retain the source structures.

#### Independent execution evidence
Fresh `capture-zerocash-spend.mjs` / `zerocash-spend.json` capture the unchanged source. The shared test witness adapter composes with actual recorded mint, checks native mint state/private agreement, and obtains paths from actual ledger trees. One scenario uses the current root; the other caches a real path after the first mint and spends it after another mint advances the tree. Both accepted paths verify against historic state.

Native/recorded Rust match TS serialized final state, effects, private state, all five private output atoms/alignment, complete VM program and four-dimensional gas/replay. Both have five queries and thirty-six VM operations. Witness order is secret, path, new_coin, encrypt, remove_coin. Private callback count is five; the input coin is removed while remaining minted coins are retained. Recipient ciphertext is deterministic and depends on the source-defined encryption-key and fresh-coin arguments, which are checked in the witness. This test adapter is not production encryption.

Actual summed-query gas (also checked against replay):
- Current root: readTime 4675000000; computeTime 12176080286; bytesWritten 3763; bytesDeleted 4737.
- Historical root: readTime 4675000000; computeTime 12217544725; bytesWritten 1317; bytesDeleted 939.

TS's reported aggregate reflects a smaller portion of the call; Rust comparisons intentionally use independently captured query sums and replay, not that aggregate. No witness-read cost is removed: path snapshots/local tree inspection are unmetered in both adapters, distinct from actual VM queries.

Duplicate nullifier rejects after secret; wrong root, wrong leaf and malformed path reject after secret/path; encryption failure rejects after secret/path/new_coin/encrypt and never invokes private removal. Wrong-leaf test supplies a valid path for the second minted commitment, so root membership passes before leaf inequality rejects. Malformed Rust paths fail typed depth conversion; TS fails witness-return validation. Failed Rust calls expose no partial state/trace/gas, so rejection parity claims cover error and witness prefix only. Source nullifier insertion before later failure remains in the consumed execution context.

#### Proof and local gates
Pinned ZKIR 2.1.0 spend k=15, 26332 rows. Both current and historical-root proofs verified and their transactions ledger-applied; applied ciphertext/commitment state matches native. Typed observed-call preparation matches the checked recorded trace.

- Eight backend library tests, 137 renderer tests and four Zerocash integration tests passed.
- Renderer guards check source-name independence, effectful path-helper rejection, wrong historic slot and Set-removal exclusion; fixture schema is normalized to the current constant.
- Targeted all-target/all-feature Clippy passed.
- All 153 generated fixtures fresh.
- Original election and Zerocash source cohort passed seven required proof/recorded calls; spend's stale gap is now a positive expectation.
- Exact signed-head focus passed 2/2 recorded: `${LOCAL_EVIDENCE}/compact-focused-8c0d3bf5-zerocash/receipt.json`.
- Broad proof gate now includes `--zerocash-spend`.

#### Retained artifacts and rerun
`${LOCAL_EVIDENCE}/compact-adr167-proof` is separate from native and focused outputs; spend prover (9993300 bytes) and verifier (2119 bytes) are retained. Rerun with an isolated target: `cargo +1.99.0 run --quiet -p compact-rust-proof-smoke -- --zerocash-spend ${LOCAL_EVIDENCE}/compact-adr167-proof`. Native output is `${LOCAL_EVIDENCE}/compact-adr167-native`, frozen compiler `${LOCAL_EVIDENCE}/compact-adr167-local/compactc`.

#### Remaining scope
Broader source parity, production witness implementations and failed-call observability remain outside this slice. Root integration will combine later ABI/schema work independently and regenerate fixtures as needed.
