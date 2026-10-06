---
id: RUST-ADR-0214
alias: ADR-0214
title: "Original microDAO vote-commit recording"
date: 2026-10-06
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["recording", "microDAO", "vote"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: b310dca8871f3b033b3c909666e4bd4cf4b23220ea29d3ed2109859cf498f36b
---
# RUST-ADR-0214 — Original microDAO vote-commit recording

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. A separate voting-commit domain records original microDAO vote_commit with selected wallet/transient input and exact whole-fallible offer. Seeded strict proof/application and rollback are covered; prior token issuance and all-zero recipient security are not established here.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#318 closure](https://github.com/MediaNoxLabs/compact/issues/318#issuecomment-6017769177). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`3404c30c`](https://github.com/MediaNoxLabs/compact/commit/3404c30c965248d69f07e759e22902fc4faf5de6) · [`53eb9a1e`](https://github.com/MediaNoxLabs/compact/commit/53eb9a1e6e329ca9f224de41396dbe5320921e3d). These include historical prerequisites and probes, not only final delivery commits.

## Historical decision and amendments

Status: approved independent preparation at root `3404c30c`; shared emitter implementation follows a later handoff. Original source bytes are unchanged. Research uses the corrected ADR200 TypeScript runtime and pinned ledger-v8 partitioner, plus original generated microDAO. No call proof, strict application or funded DAO lifecycle is claimed.

### Observed behavior

18 cases: four successes (yes/no ballot, round 0 and u64 maximum) and fourteen refusals. Every success has 12 public queries, seven private outputs and a wholly fallible 56-operation transcript; guaranteed transcript is absent. The independent query replay succeeds after installing captured commitment mappings. Source order:

1. Read public phase; only then consult private local_state when phase is commit. Reject other phase without calling that witness.
2. Check instance-specific voting-token color via kernel.self and tokenType, then exact value 1.
3. Read secret; read round and compute participant nullifier; reject existing participant before receiving funds.
4. Receive the wallet coin as a contract output; consume that exact received coin via singleton qualification; send its full value 1 to the all-zero public-key recipient (the source describes this as burning the voting token). Preserve the output→input→output event sequence and evolved nonce. This research makes no independent cryptographic unspendability claim for that recipient.
5. Invoke local_record_vote(ballot); compute the ballot commitment using round and secret; insert into depth-10 Merkle tree; insert participant nullifier into Set; invoke local_advance_state.

Success preserves public phase/round, appends one tree leaf and one participant, changes private phase from initial to committed, saves ballot and records four declared witness invocations. Earlier helpers add three Unit outputs, bringing the exact private transcript to seven outputs. Runtime-generated key/native witness output is not needed for this circuit.

Negative ordering: phase failure before state witness (one query); invalid local state also after one query; color/value failures after two queries before secret; duplicate participant after four queries; record-vote refusal after nine queries and all three coin events; full-tree failure after ten queries and record witness; advance refusal after twelve queries. Negative private witness effects remain caller-owned, not magically rolled back by a later contract or ledger error. Raw captured error strings and all query prefixes are retained.

### Next bounded implementation

After ADR211, the existing explicit wallet funding plus placement-aware selected transient policy should cover this path with H empty, one selected wallet Input W, one selected contract Transient T, and one user persistent Output O. Whole fallible segment s requires exact [1,s] input and [s] output tags; both transient halves must match. Actual owner information and full retained input/transient equality determine ownership, not the leading 1. Canonical allocation must derive authoritative indices from the retained offer; TS provisional output indices are not proof of allocation.

The emitter should compose existing checked voting/reveal leaves with receive→full immediate send. It needs typed Boolean parameter, ShieldedCoinInfo parameter, LocalState enum witness, Bytes32 secret witness, Boolean-argument Unit witness and terminal Unit witness; phase/round reads, token derivation, lazy assertions, nullifier Set membership, bounded Merkle insert and Set insert. Reuse typed Plan and native ledger primitives. Audit every action, argument, binding, branch, pure helper and callee; preserve witness and write ordering. Do not enable arbitrary actions merely because a helper has a familiar name. No schema/ABI extension is anticipated, but it must be reassessed against actual implementation.

### Minimum strict original-source acceptance

Use unchanged original vote_commit keys and seeded commit-phase state plus one genuine wallet voting token of exact instance color and value 1, with independent padding so real frontier exceeds 1. Build actual wallet Input, received contract Output and matching Transient, plus final zero-public-key recipient Output at the chosen nonzero logical segment. Bind explicit W/T and canonical indices once, run generated recording, prove original call and all actual upstream components, add separate Night-backed Dust, and require default-strict verification/application. Assert exact tree leaf/root/index, participant set membership, private witness order/state, unchanged unrelated fields, both spent nullifiers, retained owner/index/nonce and replay refusal. Run yes/no ballot controls, not only a synthetic wrapper. Prior token minting is an explicit seed prerequisite unless an independently funded buy_in lifecycle is also proved.

Concurrency refusal should change public phase or participant membership relative to the proved state and require precise fallible ReadMismatch. Whole shielded segment and contract mutations must roll back, while guaranteed Dust/replay bookkeeping may persist. Do not claim off-chain witness rollback. Existing ADR211 malformed segment/selection/binding negatives can be reused with an empty historical-input set control.

### Research artifacts and limits

- Probe: `${LOCAL_EVIDENCE}/compact-vote-commit-probe.mjs`
- Raw evidence: `${LOCAL_EVIDENCE}/compact-vote-commit-probe.json`
- Initial probe with invalid ledger-view method: `${LOCAL_EVIDENCE}/compact-vote-commit-probe-initial-view-error.json`; retained as a research setup error, not a contract failure.
- Runtime provenance and generated-source hash are embedded in final evidence.
- Source map insertion and a final ledger-view query were corrected during probe setup. The final capture takes query/witness snapshots before inspecting the final ledger view.

No ADR/issue or implementation has been opened for this slice yet. ADR209 owns the shared emitter, followed by ADR211. This research is intended to remove planning latency when that lane becomes free.

Evidence SHA-256: `bc3e4a3ddf30f3f5cae3ccdec65996e5c80c91cdab3ee1ac68bc8f75a3a9444d`.

### Compiler IR shape

The unchanged schema-20 root has one Sequence action and a Unit return. Lexical Lets retain secret, nullifier and ballot commitment. `receiveShielded` and `sendImmediateShielded` occur as CircuitCall actions, not expression Calls; the audit must traverse both variants. The full immediate-send amount is literal 1 after an explicit exact-value assertion, so the existing two-parameter wrapper detector (requiring `coin.value` as the amount expression) cannot simply be reused as root admission. Reuse the underlying audited send lowering while preserving the original literal and its checked arithmetic/conditional-change branches. Never infer full-send equivalence from a name or omit the preceding value assertion.


### Decision and developer API

Approved on2026-10-06 for original vote_commit only. Prepare independent18-case capture/native tests and selective original keys in codex/adr214-micro-dao-vote-commit. Shared emitter/runtime files remain untouched while ADR211 owns the lane. Reuse the shared typed Plan for eventual recording; maintain literal1 amount and exact lazy/witness/effect ordering. No runtime/schema/ABI change is assumed without concrete reassessment.

```rust
// Existing native execution (generated generic types omitted):
let result = ledger_contract::vote_commit(context, witnesses, ballot, coin)?;
// Expected after recorded admission is implemented:
let recorded = ledger_contract::recorded::vote_commit(context, witnesses, ballot, coin)?;
```

Exact emitted signatures and bound preparation examples will be verified during implementation. Caller must provide complete, same-segment selected wallet and transient objects with canonical allocation. Preparation neither constructs wallet funding nor silently retargets proof preimages.

Alternatives rejected: source-name admission, rewriting literal1 to coin.value, a separate evaluator, treating output provisional indices as authoritative, or broadly admitting actions by helper names. Initial keys/captures do not establish strict transaction acceptance, funded token issuance, voting lifecycle, network settlement or recipient unspendability.


### Independent preparation delivered

Issue318, milestone rust-backend-v2. Signed GPG+DCO preparation commit `6164dcbe82dba4a69f12075ffff2ae47a5bd69aa` on `codex/adr214-micro-dao-vote-commit`, based on3404c30c in the existing merkle-root checkout; clean after delivery. No shared emitter/runtime edits.

All18 independent original TS/native cases pass on the exact signed head:4 success and14 expected refusals. Exact state/effects/private outputs, gas, caller-owned witness order, qualified input and two output values/recipients, tree leaf/cursor and participant membership are checked. Normal debug worker, no stack override. Targeted strict Clippy and formatting pass. Two initial test-harness API spelling errors are preserved in native logs; no source behavior change was required.

Selective original vote_commit key generation passed: k=16,39,704 rows. Keys and ZKIR retained at `${LOCAL_EVIDENCE}/compact-adr214-proof`. Receipt `${LOCAL_EVIDENCE}/compact-adr214-preparation-receipt.json`, SHA256 `0bf982dd621c76ff15852f724f9921775562f299babb8f4ffe01a6d60b2d8bd0`; exact-head test log `${LOCAL_EVIDENCE}/compact-adr214-prep-final.log`; key log `${LOCAL_EVIDENCE}/compact-adr214-keygen.log`.

Recorded lowering, whole-fallible wallet/transient proof/application and actual rollback remain pending the later emitter delivery. These seeded captures/keys do not prove prior token issuance or a funded DAO lifecycle. ADR212 remains isolated and unchanged in its own checkout, ready to resume after ADR211 handoff.



### Implementation decision — isolated ADR214 lane (2026-10-06)

Root authorized parallel isolated implementation from `53eb9a1e`, with integration serialized after the other original-source lanes. `VotingCommit` is a separate admission domain; it does not merge the payout, merge or reset policies. The audit retains the exact ordered root stages and every lexical binding. It requires the preceding `coin.value == 1` assertion and the original literal `1` send amount, same received coin, an audited singleton index-zero bridge, the closed zero-key recipient, private record witness, Merkle insertion, matching participant Set insertion and private advance witness. Every pure helper body and unselected branch remains audited. Root parameter shadowing, missing declarations, hidden effects, helper cycles and out-of-scope locals fail closed.

Before: the native function existed but recording metadata refused the first stateful assertion. After: `Contract::from(witnesses).recording().vote_commit_call(observed, private_state, ballot, coin)` and `ledger_contract::recorded::vote_commit(context, witnesses, ballot, coin)` use the shared same-frame typed Plan. The original immediate-send helper returns `ShieldedSendResult`; only this audited voting domain may discard that typed action result. Evaluation and its private/public effects are retained. No separate evaluator or generated VM-program interpreter is introduced.

Shared lowering changes: extract the existing exact singleton wrapper audit for reuse; resolve the context-token helper and scalar Counter hash helper by declaration/body domain before ordinary shielded helper routing; scope the context-query flag to that inline call; retain scalar Counter provenance for the two round hashes. A three-element scalar hash tuple takes precedence over the two-element Unit-helper hash domain while inside the audited scalar helper. ABI49/schema20 and runtime APIs remain unchanged.

Progress evidence: 18 original TS/native/recorded cases pass, including all fourteen failure witness prefixes; fresh corrected-runtime TS capture is byte-identical to the retained fixture. Native/recorded gas equals the TS sum of query costs; wrapper-last-query and whole-program replay costs remain distinct captured evidence. Backend 50 unit tests pass, including new provenance/order/type/hidden-effect negatives. Two default-strict original-source proof/application cases are running; no proof completion claim yet. Prior wallet voting-token issuance is explicitly seeded, not established by these cases.



### Local implementation delivered — 2026-10-06

Signed GPG+DCO commit `4e0936942bf73024c81fcfd82ed7fd2660a6332c` on `codex/adr214-vote-commit-recording`, based on `53eb9a1e`. Original microDAO source is unchanged.

- Separate voting admission domain reuses the shared typed Plan, scoped context-token and Counter-hash helpers, and exact singleton bridge audit. Literal `1` and the preceding exact-value assertion remain intact; no broad policy union or separate evaluator.
- All 18 independent TS/native/recorded cases pass: four successes, fourteen precise failures, selected witness order, state/private outputs, VM program, query-sum gas and raw replay. Fresh corrected-runtime TS capture byte-matches the retained fixture.
- Both original yes/round 0 and no/u64MAX cases pass 4,480-byte call proof verification, real upstream wallet/transient/output proofs, separate Night-backed Dust, default-strict transaction validation and ledger application. Explicit W + T with empty H; sole whole-fallible segment 1, frontier 2, canonical persistent/transient indices 2/3. Precise ReadMismatch rollback, wrong-placement InvalidProof and nullifier replay refusals pass.
- 50 backend unit + 13 CLI + 153 renderer tests, 34 Python tests, strict affected-package all-targets Clippy, formatting and all 176 fixture freshness checks pass. Exact clean-commit five-source gate passes 11 commands with 15/18 required APIs recorded, including original microDAO 5/7 at this isolated checkpoint.

Runtime ABI49/schema20 unchanged. Prior token issuance and complete DAO lifecycle are explicitly seeded prerequisites; no independent cryptographic unspendability claim for the original all-zero recipient. No push or remote CI. Keep the issue open pending root integration and final milestone gates.

Receipt: `${LOCAL_EVIDENCE}/compact-adr214-delivery/receipt.json`, SHA256 `e1bad66fc0b9c4d8435c832c5d5f40ad357272334bc0c64d1e8750d01683576c`. Proof artifacts and sealed transactions: `${LOCAL_EVIDENCE}/compact-adr214-proof`. Exact-head gate: `${LOCAL_EVIDENCE}/compact-adr214-exact-gate/receipt.json`. Detailed before/after, policy and evidence are recorded in midnight ADR-0214.
