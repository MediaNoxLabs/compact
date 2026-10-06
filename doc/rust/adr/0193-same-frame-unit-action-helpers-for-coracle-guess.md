---
id: RUST-ADR-0193
alias: ADR-0193
title: "Same-frame Unit action helpers for Coracle guess"
date: 2026-10-05
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["recording", "Coracle", "helper"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: 4e74a6fc442b37ed07bf2e7b9c488f1d5bd2dc6b00096381b7546b58aecc32e5
---
# RUST-ADR-0193 — Same-frame Unit action helpers for Coracle guess

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Same-frame Field-to-Unit Cell action helpers record original Coracle guess while preserving witness, assertion, write and turn order. Both seeded colors prove/apply, but funding setup and payout remain separate.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#297 closure](https://github.com/MediaNoxLabs/compact/issues/297#issuecomment-6017732605). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`40279a39`](https://github.com/MediaNoxLabs/compact/commit/40279a39a6fda82443ba9f9d595d6f599dbdb264) · [`89bd7764`](https://github.com/MediaNoxLabs/compact/commit/89bd7764d2ff0fd71eccd8b634c866005f671b2f) · [`ee912835`](https://github.com/MediaNoxLabs/compact/commit/ee912835169d314851b99fc92c3e543ffaa2fd2e). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
status: accepted
date: 2026-10-05
```

## Historical decision and amendments

### Problem and source evidence
The unchanged original `test-center/test-contracts/coracle.compact:205–246` compiles natively, but `guess(position: Field): []` cannot be recorded. Its root obtains a secret witness, reads both public player keys, checks membership, then selects `red_guess` or `blue_guess`. Each helper returns Unit after its own secret and composite board witnesses, five ordered assertions, a `Maybe<Field>` Cell write and an enum Cell write. Existing declaration-directed dispatch only admits bounded action-free helpers. The private board check calls pure `is_player_honest → is_match → commit → transientCommit`, which the generic pure recording audit currently excludes.

### Before / after
```compact
return disclose(player_is_red
  ? red_guess(disclose(position))
  : blue_guess(disclose(position)));
```
Before: native helpers execute correctly; the recorded capability remains unavailable at `StateReturn::Expression`. After: the root and selected actionful Unit helper execute on one typed recording frame:
```rust
let argument = evaluated_position;
let (frame, ()) = if player_is_red {
    // Isolated helper scope, selected witnesses/assertions only.
    let (frame, board) = frame.try_witness_metered(/* local_board */)?;
    // Canonical typed Cell observations and pure commitment check.
    let frame = last_guess.record_write(frame, some_guess)?;
    let frame = state.record_write(frame, State::blue_turn)?;
    (frame, ())
} else {
    // Independently typed Blue branch, no Red witness calls.
    /* ... */
};
```
The snippet omits preceding ordered queries/assertions for readability; generated code retains every source operation. No separate native-helper call bridge or concatenation of independently executed traces.

### Decision and emitter/runtime ownership
Introduce a bounded Field→Unit Cell-action-helper admission profile that uses the shared `Plan`, declaration-directed call dispatch, lexical scopes, action lowering and typed branch joins. Do not conflate the stateful declaration map with profile eligibility. Evaluate arguments once in caller order before entering a callee's active-call guard and fresh scope. Admit only audited Unit-return helpers with the supported Cell/witness/assert action graph; reject cycles, ambiguous declarations, hidden effects, non-Unit returns and malformed types/slots.

Use declared enum, Bytes32 and recursively bounded Field/Boolean composite Cell types needed for `Commitment<Board>` and `Maybe<Field>`. Preserve exact declaration/member types and projection indices. Reuse generated witness codecs for `Committable<Board>`. Admit the audited pure helper closure including canonical transient commitment; actual call signatures, scoped locals and results remain checked. Preserve existing membership, readonly assertion, composite-return and intent profile bounds. No global Counter/witness guard relaxation, new runtime API, ABI or schema change is expected (ABI48/schema20).

Runtime operations already exist: typed Cell `record_read`/`record_write`, metered witness observation, canonical `runtime::transient_commit`, public-key persistent hashes, Field values and derived struct codecs. The new capability belongs in emitter composition and admission, not a fabricated ledger slot or VM program.

### Explicit scope and limits
Only the original `guess` proof-required export is the delivery target: all nine original exports remain present, with one of four proof-required exports recorded. `start`, `concede` and `withdraw` retain their current gaps. No shielded start/payout, coin allocations, Kernel effects, Counter/Merkle operations, general action-helper returns or funded game lifecycle claim.

The source directly reads `last_guess.value` without checking `is_some`. Tests must preserve the actual empty-Maybe behavior rather than adding a guard. Expected success path has three witnesses (root secret, selected helper secret, selected board) and eight queries (root red/blue; selected key/commitment/state/last_guess; two writes). Both root key queries happen even when Red matches. Helper witnesses and writes from the unselected branch must never execute.

### Evidence plan
Prepare independent fresh TS captures and a generated crate from the complete unchanged original source. Exercise Red turn, Blue turn and first Blue move (`blue_started`); board positions at boundaries, empty/present last guess, impersonator, changed second secret, invalid local board/nonce/commitment, wrong turn, dead player, invalid guesses 0/10, repeated turn, zero/prefix gas budgets. Compare selected witness argument/order/private outputs, full VM, public/private state, effects, result, query-cost sum and replay. Retain TS wrapper last-query gas separately; do not invent post-error contexts or aggregate wrapper equality.

Use canonical explicit prior-state fixtures with matching TS/Rust serialized state. Prove, verify and ledger-apply both colors using pinned ZKIR and original source keys; inspect applied turn/guess and reject repeated same-color move. Shared unbalanced smoke policy remains explicit, without claiming historical funded game setup. Add exact source/cohort applicability checks and strict remaining-gap checks, typed IR mutation guards, targeted tests/Clippy/freshness and a signed exact-head receipt. GPG+DCO local commit, no push/remote CI.

### Coordination
ADR0191 currently owns shared typed_plan/stateful files. Prepare only independent capture/fixture/tests/proof setup until its signed handoff; then integrate ADR0191 in the isolated worktree before shared planner edits. Reuse the existing warm target. ADR0192 declaration resolution and Counter provenance must remain intact.

### Independent baseline before shared planner changes

Issue: https://github.com/MediaNoxLabs/compact/issues/297. Prepared on main-integrated ADR0192 base `89bd7764`, with no typed_plan/stateful edits while ADR0191 owns them.

Fresh original-source TS capture and native generated crate pass 24 scenarios. Successful Red/Blue/first-Blue/empty-Maybe calls have eight queries and three private outputs (`secret`, `secret`, `board`). Both empty-Maybe cases succeed by the source's direct `.value` behavior. Both colors have independent impostor, changed second secret, wrong nonce, invalid board, wrong turn, dead player, repeated-turn and invalid guess (0/10) rejections. Zero gas rejects after the root secret and before an accepted query.

The budget equal to the first query succeeds through all eight queries in both TS and native Rust. It is retained as `perQueryBudget`, not misrepresented as a prefix failure: the query limit applies separately to each query. Success gas still has separate reported-last-query, accepted-query-sum and whole-program replay measurements, following ADR0192. No gas semantics change is part of this delivery.

### Native generated Unit-result cleanup

Full original Coracle native fixture passes the execution baseline but exposes strict Clippy `let_unit_value` on its final conditional Unit expression:
```rust
let result = if is_red { /* ordered red effects */ red_call.result }
             else { /* ordered blue effects */ blue_call.result };
Ok(CircuitResult { context, result, /* ... */ })
```
After the typed Unit-result cleanup, existing `discard_expression` executes the expression and the result field is `()`:
```rust
if is_red { /* same ordered red effects */ red_call.result }
else { /* same ordered blue effects */ blue_call.result };
Ok(CircuitResult { context, result: (), /* ... */ })
```
All selected-branch context updates, transcript appends and gas additions remain evaluated. No lint suppression. Root approved this bounded cleanup after ADR0191 handoff; full fixture freshness will enumerate mechanical generated changes before updating only affected outputs.

### Delivery evidence

The shared planner now has a separate `unit_actions` admission module. `Plan::call` still resolves declarations before profile policy; audited pure helpers inline with no action prefix, while bounded stateful Unit helpers pass their ordered actions and Unit tail to the same `inline_call`. Formal arguments are evaluated exactly once in caller scope before the active-call guard and fresh callee scope. Pure commitment/hash expressions are admitted only within the audited pure closure; both transient-commit operands are checked. New admission flags do not enable composite intents, Counter provenance or unrelated profiles.

Twenty-six independent TS/native/recorded scenarios now pass. Added dual-player identity cases seed both public keys from the same secret under their distinct domains: Red turn succeeds through Red, and Blue turn rejects `Not Red's turn`, confirming Red priority and no Blue helper evaluation. All successful paths preserve eight queries and three private outputs. Error witness prefixes, authority/board/turn/alive/position failures and empty-Maybe behavior match the source.

For the first Red success, three gas measurements are preserved:

| Measurement | readTime | computeTime | bytesWritten | bytesDeleted |
| --- | ---: | ---: | ---: | ---: |
| TS last-query wrapper | 85000000 | 1233942932 | 670 | 670 |
| TS query sum = native = recorded | 1190000000 | 9967377067 | 1340 | 1340 |
| Whole-program replay | 1190000000 | 2380277151 | 670 | 670 |

Both colors were proved, verified and ledger-applied using the original `guess` keys (k15, 16951 rows), then same-color repeated moves were rejected against applied state. Evidence and preserved keys: `${LOCAL_EVIDENCE}/compact-adr193-proof`, `${LOCAL_EVIDENCE}/compact-adr193-proof.log`, `${LOCAL_EVIDENCE}/compact-adr193-keygen.log`. Explicit prior states and shared unbalanced smoke policy remain the boundary; no funded start/payout claim.

Validation: 14 backend unit, 13 CLI and 152 renderer tests; four generated consumer packages including original Coracle, original microDAO, stateful composites and composite intents; 34 Python tests; strict backend/fixture/proof-runner Clippy. Negative IR guards include wrong declaration/slot/member/arity/result, leaked caller local, cycle/ambiguous helper, mixed Unit branches, unsupported coin Cell effect, hidden Cell read in commitment opening, hidden witness in value, and an effectful helper in an unselected pure branch.

Full fixture freshness enumerated 169 fixtures and found only the new Coracle output affected after narrowing the Unit cleanup to preserve existing empty-tuple output. That new fixture was updated; four directly related fixtures then passed freshness with no stale output. Original-source cohort and strict-source guards join nine exports, four proof-required APIs and exactly `guess` recorded; predecessor compiler fails the new guard. One direct cohort invocation accidentally used a temporary Cargo cache; it completed and its temporary directory was removed. All subsequent Cargo/cohort commands use the assigned warm target.

### Signed delivery and handoff

- Commit `d171fe189805687634061919f8a1ce69661b455e`, conventional explanatory body, GPG verified, DCO included. Local parent `46c3ea86` is the signed ADR0191 cherry-pick; integrate only the ADR0193 commit after ADR0191.
- Exact-head receipt `${LOCAL_EVIDENCE}/compact-focused-adr193-coracle-guess/receipt.json`: **passed**, one original source fixture, **1/4 proof-required exports recorded**. Clean worktree.
- Frozen compiler `${LOCAL_EVIDENCE}/compact-adr193-compactc`; combined Scheme `${HISTORICAL_NIX_STORE}/1d5x83y7dgg6abzv0fzvna893na3nfjj-compactc-scheme-local-slice/bin/compactc-scheme`.
- Proof replay command: `CARGO_TARGET_DIR=${COMPACT_SOURCE}/target/compactc-consumer CARGO_INCREMENTAL=0 cargo +1.99.0 run -p compact-rust-proof-smoke -- --coracle-guess ${LOCAL_EVIDENCE}/compact-adr193-proof`.
- Issue #297, milestone rust-backend-v2. No remote CI or push.
- Shared compiler lease handed to ADR0195 receiveShielded after signing. ADR0194 microDAO advance remains a separate follow-up research scope.
### Integrated original Coracle guess — 40279a39

ADR193/#297 is integrated as GPG/DCO-verified `40279a39`. Private schema 20 / runtime ABI 48 unchanged. Both colors prove, verify and ledger-apply from the combined main branch, with repeated-turn rejection on applied state. These use explicit prior-state fixtures and the shared unbalanced smoke policy; funded game setup/payout is not claimed.

- **169 fixtures fresh**, zero stale/failed; existing generated libraries were unaffected by the narrowed native Unit-expression cleanup.
- Backend 14 library + 13 CLI + 152 renderer tests, targeted strict backend/Coracle/proof Clippy, 34 Python tests and formatting pass.
- Five-fixture focused gate: **10/18 proof-required APIs recorded**, no nonproof capability rows in this selected group. Original Coracle, original microDAO, original bulletin board and both composite fixtures are covered. `${LOCAL_EVIDENCE}/compact-focused-40279a39/receipt.json`.
- Both integrated Coracle proof paths pass: `${LOCAL_EVIDENCE}/compact-integrated-adr193-proof.log`. Independent 26-case TS/native/recorded/replay evidence includes dual-player Red priority and empty Maybe behavior. Wrapper gas, sum of per-query costs and whole-program replay gas remain separate measurements; budget enforcement is per query in this pinned runtime.
- Exact checked-source inventory `${LOCAL_EVIDENCE}/compact-40279a39-inventory.json`: **355/363 proof-required APIs available, 8 explicit gaps, zero unassessed exports within this corpus**; 210 sources / 732 exports / 189 compiled roots / 369 nonproof exports; no missing/unmatched rows or baseline drift. Five microDAO and three Coracle recorded APIs remain.

#### Newly found gaps outside the aggregate

The composition review found a valid TS Field contract with two nested terminal Let bindings whose Rust native return fails with unknown parameter `after`. This is a native lexical-scope bug outside the existing checked corpus; ADR196 is preparing an explicit return-continuation fix in the native emitter. The 355/363 number does not cover it. ADR195's independent boundary tests also found TS runtime shielded-value encoding limited to u64 while the Compact/ledger type is u128; tracked separately in [#300](https://github.com/MediaNoxLabs/compact/issues/300). Rust retains ledger u128 semantics; wide TS failures must not be reported as parity successes.

Active work: ADR195 receiveShielded on the shared typed planner, ADR194 original advance independent preparation (20 native/oracle cases and keys), ADR196 native return scope. Review requires the new receive admission to audit nested binding expressions and reject hidden ledger reads before delivery. Latest broad full/resumed gate and clean macOS ARM archive still belong to ee912835; later integrations have the focused evidence above. No push, publication or remote CI; user-owned documentation edit preserved.
