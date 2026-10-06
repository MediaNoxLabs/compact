---
id: RUST-ADR-0213
alias: ADR-0213
title: "Record original Coracle start with two-player funding"
date: 2026-10-06
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["recording", "Coracle", "two-player-funding"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: b8b195bff16b7783daf4a50482c5ddc76d3ac6de38019b3d1a47ece2449cb2f1
---
# RUST-ADR-0213 — Record original Coracle start with two-player funding

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. The bounded StartFunding domain records original Coracle start with direct distinct wager/deposit receives before merge and stores. Seeded red and blue strict proofs pass with rollback/replay negatives; no continuous red-to-blue chain or live wallet submission is claimed.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#317 closure](https://github.com/MediaNoxLabs/compact/issues/317#issuecomment-6017767455). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`d1411fe5`](https://github.com/MediaNoxLabs/compact/commit/d1411fe598c3e98527d5dce61adf756bf0448492). These include historical prerequisites and probes, not only final delivery commits.

## Historical decision and amendments

Status: approved bounded design, independent preparation pending, 2026-10-06. Milestone: `rust-backend-v2`. Research: [Original Coracle start — two-player funding research — 2026-10-06](references.md#private-note-14). Based on unchanged `test-center/test-contracts/coracle.compact`, corrected ADR200 TypeScript runtime and pinned ledger-v8.

### Problem

The original `start` is the remaining proof-required Coracle export without a recorded Rust call. Native source execution succeeds, but its red and blue branches combine two received coins, private witnesses, ordered Cell writes and branch-local enum results. Red starts a game from no game. Blue joins an existing red game, merges the received wager with the historical pot, and stores a second player's deposit. Treating both branches as one offer/transaction placement or flattening their effects changes the original source.

An independent corrected-TypeScript probe at `${LOCAL_EVIDENCE}/compact-start-probe.json` records 27 source-level cases: eight successes and nineteen expected failures. Red success has 14 queries, five private outputs and a wholly guaranteed 58-operation public transcript. Blue success has 23 queries, eight private outputs and a wholly fallible 97-operation public transcript. This is measured against exact prestate and captured output commitments, not inferred from helper names.

### Decision and before/after API

Before, the generated Rust crate can call `start` natively but has no recorded or observed call to prepare/prove/apply. After this slice, the unchanged source should expose the existing generated `recorded::start` and `start_call` style APIs. The consumer supplies typed witnesses, an exact observed state and a sealed offer-bound funding envelope; no generated output edits or source-specific method are added.

```rust
// Before: native execution only.
let result = contract::start(context, &witnesses, pos, wager, deposit)?;

// After: illustrative existing recorded/observed surface, with an exact
// branch-specific funding envelope and caller-selected ledger inputs.
let recorded = contract::recorded::start(bound.observed().circuit_context(private),
                                         &witnesses, pos, wager, deposit)?;
let prepared = bound.prepare(RecordedCall::new(bound.observed(), recorded,
                                               "start", (pos, wager, deposit)), verifier, key)?;
```

The concrete generated signature and funding carrier must be verified from emitted output. Both branches receive wager and deposit before secret → fresh nonce → `local_set_board` witnesses. Red stores wager as pot and deposit as red deposit, then writes key, board commitment, phase and private board before returning `Player.red`. Blue reads historical pot, merges it with the received wager, stores merged pot and persistent blue deposit, then writes blue key, board commitment, phase and private board before returning `Player.blue`. Preserve exact witness, VM query, effect, private transcript and result ordering. No execution `ownPublicKey` is required. Zero wager is source-accepted; monetary proof/apply must be tested independently.

### Emitter and runtime boundary

Reuse the existing typed Plan/ReturnPlan for nested Let/Sequence, ordered effects and branch-local enum result. Reuse ADR207 typed merge primitives, ADR211 exact offer policy where applicable, and source declarations for phase, key, board commitment, qualified pot and deposit Cells. Recursively audit unused bindings, both branches and pure/stateful callees, including transient-commit value and opening. Reject ambiguous or cyclic helpers, hidden effects, malformed witness declarations, and type or lexical-scope mismatches. Admit by a bounded structural/effect domain, never a source path, export name, exact query count or copied evaluator. The shared emitter is leased to ADR211 then ADR212; independent capture/native/key work may proceed now, emitter changes wait for their handoffs.

No new IR/schema or generated-facing runtime method is currently justified. Red's strict offer needs two real selected wallet inputs and two persistent contract outputs in the default guaranteed segment. Blue needs two selected wallet inputs, one historical contract pot input, one explicitly selected transient wager, and two persistent outputs in its selected nonzero fallible segment. The upstream offer must remain complete, with exact intent projection, canonical output indices, owner/tree/nullifier checks and full-value input equality. ADR211's runtime policy must actually support this union; if it does not, design a separate explicit policy rather than silently dropping wallet inputs/transients or changing segment. Separate NIGHT-backed Dust funds fees.

### Acceptance and limits

Capture independent corrected-TypeScript source vectors, pin source/runtime hashes, and assert direct native behavior for all 27 rows: full state/result, ledger effects, witnesses/private output, gas and query order. Failures must retain their observed order, including phase, wager, deposit, board, secret, nonce, set-board and overflow cases. Failure rows are not evidence that no intermediate private output occurred.

Then compare native/recorded/replay precisely for successful and rejected rows, source inventory and negative IR admission. Generate original `start` proving keys and prove/verify/apply red and blue under default strict ledger validation with actual upstream input/output/Zswap proofs. Assert offer output order/index/commitment/owner, phase and all written Cells, exact pot/deposit values and nonce evolution, selected nullifiers spent, replay rejection, untouched opponent fields and exact branch placement. A changed eligible public state for blue must show measured fallible refusal and atomic contract/Zswap rollback, while Dust/replay effects are reported separately.

Prefer a real red→blue ledger chain if the first result can fund the second without synthetic intermediate edits. If preparation must seed a prior red state for blue, label both proof paths as independent seeded evidence and keep funded lifecycle as open acceptance. No network submission, remote CI or push is claimed by this ADR.

Issue: to link after creation.

MediaNoxLabs milestone issue: https://github.com/MediaNoxLabs/compact/issues/317 (created before implementation).

### Independent preparation checkpoint, 2026-10-06

Signed GPG/DCO commit `7809cb995988c16c42f1b80575c2373513d3246b` pins the corrected TypeScript capture script, 27-row fixture and direct native Rust test. Fixture SHA-256 `f8b5fab10ae123c25db1cc3d50cbaa40aa00c33bb8a0955785f3ecf732921bde`; the fixture contains source/generated/runtime/ledger hashes. Eight successful rows match complete serialized state, result, witness order, effect claim multisets, private outputs, wallet/contract coin intents, actual commitment/index map and per-query gas sums. Nineteen rejected rows retain assertion/witness ordering. Two diagnostic boundaries are explicit: TS dynamically accepts a negative nonce witness then rejects its Field descriptor, while Rust's typed witness rejects it immediately; the Uint overflow has equivalent rejection with a different diagnostic string. Set-like claimed receive/nullifier effects are sorted solely for equality, retaining every element and duplicate; raw TS arrays and public transcript/query order are unchanged. The prior red/blue states are separately seeded, so no recorded/proof/apply or chained funding claim is made.

The original `start.zkir` compiles at k=16, 63,422 rows. Local keys: `${LOCAL_EVIDENCE}/compact-adr213-keys/start.prover` SHA-256 `6701fb8ac637d7ff2b16b555984c91dffce236247d376cbada7b2b6e3ed4838f`; verifier SHA-256 `822f72bf93438a6a3e86daabc3645bd696786c5ffec4b274971bc63805ae3574`. Strict original red/blue call proofs and ledger application remain pending ADR211/ADR212 shared-emitter handoffs and exact mixed funding policy.

Source-admission snapshot and eight independent refusal mutants are detailed in [ADR213 original Coracle start source admission matrix — 2026-10-06](references.md#private-note-02). The snapshot is schema20 from the unchanged source; this is preparation, not an admission/proof claim.
The independent strict-offer construction and red→blue chain attempt are specified in [ADR213 original Coracle start strict offer proof plan — 2026-10-06](references.md#private-note-03). No recorded call or ledger proof is claimed by that plan.
### Local implementation and validation, 2026-10-06

Signed GPG and DCO implementation commit: `24af3ce4145a16d6d27518c67b51c3adce5eb2c4` (`feat(rust-backend): record original Coracle start funding`). The bounded `StartFunding` profile uses the shared typed Plan; no IR schema, runtime ABI or offer-policy change. The original full source IR and emitted Rust fixture SHA-256 are `61557dd2ef164f6f23a5bf04180fbcabb06e531e2e087675ee1767379622ff13` and `74dbe34fa8ef4d5e2ab070c5df7584573923920132f7f2563378671eb812c536`. Final emitter output is byte-identical to the proof fixture.

Admission enforces distinct direct wager and deposit receives before writes or merge, direct protected-formal forwarding by the receive helper, no root funding-name shadow, and one selected red wager store or blue historical-plus-received-wager merge followed by pot store. The selected path then stores deposit, key, board, phase and witness in order. Unused bindings, both branch bodies, nested arguments and pure/stateful helper declarations are audited. Well-typed mutants that change received coin, move receipt/store/merge, change merge operand or historical slot, hide queries/effects, or change branch result are refused.

Twenty-seven corrected TypeScript rows match native and recorded/replay behavior. Signed-head gate receipt: `${LOCAL_EVIDENCE}/compact-adr213-signed-head-receipt.json` (50 backend unit tests, original Coracle source test, 34 Python tests, strict Clippy and format check all pass). Original `start` call proofs verify at 4480 bytes for red and blue, and both default-strict offers apply with two actual wallet inputs, canonical persistent indices and separate NIGHT-backed Dust. Binary IR SHA-256 `949d76a969ee28e05f092877984d93edd0068b454f08210b8955052bb92f40ac`; prover and verifier key hashes remain those in preparation. Strict proof log: `${LOCAL_EVIDENCE}/compact-adr213-proof/run-final.log` SHA-256 `9dba4c3116e014294a974671543e250264e685f8de68f869619c7ddc4cb149e6`. Blue changed-phase evaluation produces selected-segment ReadMismatch and rolls back contract and shielded effects; replay spends are rejected.

Red and blue proofs use independent seeded states. The blue proof does not establish a red-to-blue ledger lifecycle. Original source corpus inventory now records all four Coracle proof-required exports as recorded; root integration and combined package gate remain the next checkpoint.

### Tracked source admission complete — d1411fe5 (2026-10-06)

Signed/DCO rootd1411fe598c3e98527d5dce61adf756bf0448492 integrates ADR213 original Coracle start (source24af3ce4, IRprep514c5d92 fromdc088e78). Shared typed Plan conflicts retain distinct funding/mint/voting/reset policies. Combined gate passes62 backend unit,13 CLI,153 renderer and34 Python tests; six-source focused parity13commands now20/20 required APIs; exact original Coracle and microDAO TS/Rust cohorts; strict original Coracle compilation; strict five-package Clippy.

Whole-source inventory now **386/386 proof-required APIs recorded and available, zero missing**, across217sources/755exports/196compiledroots/369nonproof/1,024declarations. Zero unassessed/missing compiler proof rows/unmatched exports/baseline drift. This closes tracked proof-required source admission; it is not all-program/all-path/proof coverage. The inventory retains56 nonproof recorded-unavailable rows; no row was silently relabeled to close the required denominator.

ADR213 has27 corrected-TS/native/recorded/replay cases. Original red and blue start call proofs both verify at4480bytes and offers apply with default strict ledger validation; blue changed-state fallible rollback and spent-nullifier replay are rejected. Actual two-wallet receive, historical merge and canonical qualified stores are checked. Prior states are independently seeded, so no continuous red→blue game or live-wallet claim. Final guarded emitter reproduces the proved generated fixture byte-for-byte. Root focused gate does not rerun those cryptographic proofs; source proof evidence is linked explicitly.

- Combined root:${LOCAL_EVIDENCE}/compact-d1411fe5-integration-receipt.json
- Focused:${LOCAL_EVIDENCE}/compact-focused-d1411fe5/receipt.json
- Inventory:${LOCAL_EVIDENCE}/compact-d1411fe5-inventory.json
- Exact cohorts:${LOCAL_EVIDENCE}/compact-d1411fe5-coracle-cohort.json and ${LOCAL_EVIDENCE}/compact-d1411fe5-dao-cohort.json
- Source signed proof/test evidence:${LOCAL_EVIDENCE}/compact-adr213-signed-head-receipt.json

Schema20/ABI49. Last full/portable remains3404c30c. Current remaining work: ADR217 explicit trusted-observation runtime/live adapter; ADR220 direct helper/registry behaviors and durable37-source matrix; ADR221 sampled branches/collections; bounded missing recorded trace coverage; then one frozen-head full/Nix/Counter+shielded live acceptance. No push/remote CI; user doc unchanged. All issues remain open until final acceptance.
