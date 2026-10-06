---
id: RUST-ADR-0202
alias: ADR-0202
title: "Record terminal lexical return continuations"
date: 2026-10-06
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["recording", "lexical-scope", "ReturnPlan"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: 4ac0101d34c4e35367674072e4c6752b3f27944905364f97a2af0047974e5d2d
---
# RUST-ADR-0202 — Record terminal lexical return continuations

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. A structural terminal-return adapter keeps extracted values inside final Sequence/Let scopes and reuses typed ReturnPlan evaluation. It does not broaden earlier statements, branches or helper caller scope and does not establish all-language parity.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#306 closure](https://github.com/MediaNoxLabs/compact/issues/306#issuecomment-6017748111). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`70251e9a`](https://github.com/MediaNoxLabs/compact/commit/70251e9a902f28beea6f41ecaf7207787c87b9d5) · [`7a311c9f`](https://github.com/MediaNoxLabs/compact/commit/7a311c9fe80ef588b744407fb39f0130ba6d352f) · [`c6ff13c5`](https://github.com/MediaNoxLabs/compact/commit/c6ff13c5c2ad283733cc5a45b9ca1faa734217b0). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
status: delivered-locally
date: 2026-10-06
adr: 202
```

## Historical decision and amendments

### Problem
The existing terminal_lexical_return_oracle source compiles natively after ADR196, but two/three/nested/observed remain recorded gaps. Extracted return expressions reference bindings in terminal nested Let suffixes; actionful Field-returning helpers need the same lexical continuation treatment.

### Approved design
A structural adapter maps actions plus an extracted expression into the existing ReturnPlan. Only terminal Sequence/Let suffixes enclose the return; earlier sibling, branch and caller/helper scopes remain isolated. Existing typed Plan methods evaluate everything. Audit the full reachable typed domain, including unused bindings and unselected branches. No source-name matcher or arbitrary exact operation counts. Admission requires the bounded Field Cell read/write effects. No runtime ABI or schema change.

Reuse the existing thirteen TS/native cases. Add recorded/replay comparisons, branch/sibling/caller scope negatives, and original four-export cryptographic proofs/default-strict ledger application with separate Dust. Prior state and strict proof scope remain explicit.

### Emitted API
Before: native `ledger_contract::two(context)` returns `CircuitResult<Private, Field>`; only echo has a recorded method. After: `ledger_contract::recorded::two(context)` returns `RecordedCircuitResult<Private, Field>`, with typed observed `two_call`; witnessed observed gains its recorded and observed-call forms using the generated witness handle. Exact emitted signatures will be copied into the delivery ADR.

### Alternatives and limits
Do not flatten/hoist every local, export a mutable local-scope accumulator, or add a second evaluator. The adapter may clone a bounded IR suffix; a borrowed structural view can replace that representation later. Existing explicitly scoped Effectful ReturnPlans remain unchanged. No proof coverage beyond actual executed cases.

Shared planner lease remains ADR201; independent fixture/proof preparation may proceed after this issue/ADR exists. Integrate actual201 policy before emitter edits. Signed explanatory GPG+DCO delivery; no push or remote CI.

ADR202: `ADR-0202 — Record terminal lexical return continuations` in midnight Obsidian. Approved proposal: `${LOCAL_EVIDENCE}/compact-adr202-design-proposal.md`. Prerequisite ADR196/#301, related ADR194/#299.

### Accepted design detail



Close the four recorded gaps in existing `terminal_lexical_return_oracle.compact`: `two`, `three`, `nested`, `observed`. Preserve the already-recorded `echo` as a control. No new fixture source, source identity, runtime API, ABI, schema or TypeScript implementation change.

The source has one Field Cell, Field addition, typed local bindings, a Field witness with one Field argument, a Boolean assertion parameter, and a no-argument internal Field-returning helper. Every success reads and writes the same Field Cell. `observed` reads, invokes its witness, asserts, then writes and returns its local result. Its rejecting branch must preserve the read/witness prefix and omit the write.

The existing13 TypeScript/native cases already cover all five exports at seeds0/4, observed delta0, assertion rejection after the witness, and zero gas before the witness. Reuse this capture unchanged rather than recapturing unrelated behavior.

### What is missing

The compiler's schema20 representation separates the extracted `StateReturn::Expression` from ordered actions. In `two`, that return references `after`, bound under:

```
final Let(before) -> final Sequence child -> Let(after) -> CellWrite
extracted return: after
```

`three` adds another terminal binding. `observed` puts an assertion before the last Let inside its suffix. `nested` calls an internal circuit with the same action/return representation. These are valid lexical continuations, not locals from arbitrary preceding statements.

ADR196 fixed native lowering using a `returns_from_scope` marker propagated only through terminal Sequence/Let suffixes. It deliberately stops at branches and independent ReturnPlans. The current generic recorded Plan still retains only the outer final top-level Let scope; its action evaluator properly closes nested scopes, so the extracted return subsequently cannot resolve `after`. Its stateful-call path also lacks an admitted Field-returning actionful helper for this domain.

### Recommended reusable boundary

Add a small **structural adapter from extracted actions plus expression to the existing ReturnPlan**. It performs no evaluation, type inference, name resolution, frame manipulation or opcode generation. The existing `Plan::return_plan`, `Plan::action`, `Plan::bindings`, `Plan::expression` and typed declaration dispatch remain the sole evaluators.

Adapter semantics:

1. An empty action sequence becomes `ReturnPlan::Value(return_expression)`.
2. For a nonempty sequence, earlier actions remain ordinary actions of `ReturnPlan::Sequence`; only its last action can enclose the continuation.
3. A terminal `StateAction::Let(bindings, action)` becomes `ReturnPlan::Let(bindings, attach(action, continuation))`.
4. A terminal `StateAction::Sequence` repeats rule2.
5. Any other terminal action, including If, becomes an ordinary action followed by the return in the surrounding scope. Branch-local bindings never enclose that return.
6. A pre-existing `StateReturn::Effectful` ReturnPlan is already scoped explicitly and remains unchanged. Do not apply this extraction adapter to the ordinary actions inside its Sequence nodes.

This matches196's native continuation semantics and makes a terminal three-binding return structurally identical to an explicitly nested ReturnPlan. A temporary owned ReturnPlan requires cloning a bounded action suffix; this is a compiler-only cost. Prefer this simple adapter to a second evaluator or a mutable exported-scope accumulator. If avoiding clones is necessary later, a borrowed structural view can replace it without changing the evaluation contract.

Entry lowering should use this adapter under an explicit bounded Field Cell continuation admission policy. Shared `inline_call` should reuse it for an admitted extracted Field-returning helper after evaluating arguments once in caller scope and establishing its existing isolated callee scope/active-call guard. It must not merge caller locals into that scope or expose helper locals to its caller. Existing pure helper dispatch remains declaration-directed and audited; no new pure primitive is needed by this source.

After201 lands, use its actual policy structure rather than adding a competing policy representation. Keep194 phase_reset,195 receive and191 composite-intent admission independent. For existing Unit helpers, expression-only helpers, and explicitly scoped effectful ReturnPlans, preserve current lowering behavior and tests. The old generic root-Let logic can delegate to the same adapter where its existing admission already allows an extracted expression; do not independently widen its value/effect domains.

### Bounded admission and changes

- Entry/result: Field result with Field/Boolean parameters; extracted expression after audited actions, or an audited call to a same-domain internal Field-returning circuit. Keep actual signature/type validation in the shared Plan.
- Leaves: the existing Field Cell read/write, Field addition, Boolean conditions/equality, typed Let/Sequence/If/Assert, witness whose parameters/result are Field, and typed helper calls required by the source. Reject unrelated ADTs, Kernel/Zswap effects, and unsupported operations even in unused bindings or unselected branches.
- Retain one declared Field Cell identity across the bounded body/helper graph; require actual recorded reads and writes for this new profile. Do not generalize proof applicability from emitted method presence: source contract-info remains authoritative.
- Calls: audit the reachable declaration graph with cycle/missing/ambiguity rejection; match formal/actual and declared return types; arguments execute once and in source order before callee scope. Reuse the existing purity guard if an already-supported pure call is encountered; no new pure-call arithmetic or hashing domain is part of this task.
- Actual missing boundary needing approval: a Field-returning stateful helper with ordered actions **and** an extracted local return (the existing Unit-action and expression-only helper domains are insufficient). This is the same terminal-continuation mechanism, not a new interpreter.

No stateful.rs edits should be necessary after bringing in196. No hand-written VM sequence in the generated crate. Runtime recording/transaction methods remain authoritative.

### Developer-facing result

Before: `contract.two(context)` and the other native methods work; only `echo` has the recorded API.

After, for example:

```rust
let recorded = ledger_contract::recorded::two(context)?;
assert_eq!(recorded.execution.result, expected_after);
let observed_call = contract.recording().two_call(&observed, private_state)?;
```

The exact facade form will follow the generated witnessed/unwitnessed handles; the observed witness-bearing method uses `contract.recording().observed(context, reject)`. Delivery documentation will copy the emitted signatures. Recorded results must contain the returned local value, not a reread of the written Cell or the caller's similarly named local.

### Validation

1. Extend the existing13-case fixture test to run native, recorded and replay for successes. Compare full serialized state, returned Field/public output, query operations/results, gas sums, private transcript and witness/event order. Keep echo(91) returning91 even though the stored Cell advances to a different value.
2. Preserve rejection-after-witness and zero-gas-before-witness behavior. Record the limitation that failed Rust calls consume context; the TypeScript capture retains exact failed-query/prefix evidence.
3. Negative IR cases: earlier top-level Let local returned later; earlier Sequence sibling local escape; locals from either If branch leaking after it; helper reading a caller local; helper local escaping to caller; wrong binding/result/formal type; missing/ambiguous/cyclic callees; unsupported effects in unused bindings/unselected branches. Include shadowing and caller-argument evaluation controls. Preserve existing196 native scope-escape tests and existing explicitly scoped effectful-return tests.
4. Pin original proving keys after approval. Prove and ledger-apply one success for each newly admitted export (`two`, `three`, `nested`, `observed`) with independent proof verification and changed-binding refusal. Prefer separate Dust and default strictness using194's existing helper pattern. Assert the exact returned Field/output binding and updated Cell. Existing source-seeded state is local test setup, not proof of a prior on-chain lifecycle.
5. Focused cross-tab should become5/5 recorded for this existing source (actual result to be measured). Run relevant191/193/194/195/201 peer regressions, renderer tests, targeted Clippy/freshness, and current-version source guard. No baseline source identities should change; report actual inventory rather than claiming a predicted global total.

### Coordination and next step

Request approval for the adapter plus the bounded actionful Field-returning helper domain above. After approval, create ADR202 and a rust-backend-v2 issue before edits; prepare independent existing-capture extensions and keys while201 retains the emitter lease. Integrate196 and201 before shared changes and inspect their actual implementation first. Signed explanatory GPG+DCO delivery; no push or remote CI.


### Approval
Parent approved the proposal on 2026-10-06. The review decisions above are accepted; no arbitrary operation counts or source identity requirements are to be added.


Issue: https://github.com/MediaNoxLabs/compact/issues/306

### Independent ADR202 preparation

ADR and issue were created before edits. Worktree branch `codex/adr202-terminal-lexical-recording` is based on integrated194 `c6ff13c5` and includes native196. Shared emitter lease remains ADR201; no typed_plan/recorded/stateful edits have been made.

The existing 13-case TypeScript capture is unchanged. Extracted reusable witness/seed support and reran all 13 native cases successfully. Prepared an unhooked proof module for the four original exports; no recorded or proof acceptance is claimed yet.

Pinned original keys: `${LOCAL_EVIDENCE}/compact-adr202-proof`; all k=6, with rows two=27, three=28, nested=27, observed=55. Key generation log `${LOCAL_EVIDENCE}/compact-adr202-keygen.log`. Preparation hashes `${LOCAL_EVIDENCE}/compact-adr202-preparation-receipt.json`.

The planned strict proof setup uses existing separate Night/Dust helpers. State seeded through native echo calls remains explicit local prior state, not proof of earlier ledger transactions. After201 signed handoff, integrate its actual CompositeDomain policy before implementing the approved terminal-continuation adapter.


### Implementation and review — 2026-10-06

Integrated ADR201's signed policy migration before emitter edits. `CompositeDomain::TerminalReturns` is independent of the existing composite-value/intent/receive/Field-observation combinations; phase reset stays independent. `terminal_returns::adapt` only builds existing ReturnPlan nodes. Shared Plan owns argument order, typed values, effects, lexical scopes and helper recursion. No native emitter, runtime, ABI48 or schema20 change.

Exact emitted borrowed API examples:
```rust
pub fn two<Private>(&self, context: CircuitContext<Private>)
    -> Result<RecordedCircuitResult<Private, Field>, CompactError>;
pub fn observed<Private>(&self, context: CircuitContext<Private>, reject: bool)
    -> Result<RecordedCircuitResult<Private, Field>, CompactError>
where W: TryWitnesses<Private>;
pub fn observed_call<'observed, Private>(&self,
    observed: &'observed ObservedContractState, private_state: Private, reject: bool)
    -> Result<RecordedCall<'observed, Private, Field>, CompactError>
where W: TryWitnesses<Private>;
```
Before202 only echo was recorded; now all five original proof-required exports are recorded. Source contract-info is the authority for this claim.

Recording witnessed observed exposed a borrowed facade compile defect: `echo(context, echo)` called the same-named parameter. The bounded correction qualifies the call when emitted parameter identifiers (including `$` normalization/raw identifiers) or the fixed `context` binding shadow the circuit name. After: `crate::ledger_contract::recorded::echo(context, echo)`. Independent review also reproduced noarg `context(context)`; its correction and both witness-mode regressions are retained. Failed compilation log `${LOCAL_EVIDENCE}/compact-adr202-parity-first.log` is preserved.

Two old admission expectations changed explicitly: zero-argument Field witnesses now fit the new Field witness domain while the old profile-local refusal stays tested; a terminal Sequence wrapping a final Let now records in an already admitted profile, matching native196. Earlier sibling/If/caller/helper escapes and hidden effects remain rejected. `let_return_oracle` also uses the shared planner for its existing read/write; its exact behavior is included in the peer gate.

Validation so far: all13 unchanged original TS/native cases now match recorded execution and successful replay; 23 unit +13 compiler +153 renderer tests pass; targeted backend/proof/terminal Clippy and34 Python tests pass; eight-source real local parity gate passes21/28 proof-required APIs,0 extra recorded nonproof exports. The source-specific strict recording/Cargo guard confirms5/5. Four original-key proofs (two/three/nested/observed) are2912bytes each, independently verified, changed bindings rejected, and separately Dust-balanced transactions pass unchanged WellFormedStrictness::default plus ledger.apply. Exact Field outputs and full resulting Cell state are checked. Prior Cell state is native-seeded test setup, not a proved earlier ledger lifecycle. Proof log `${LOCAL_EVIDENCE}/compact-adr202-proof.log`; keys `${LOCAL_EVIDENCE}/compact-adr202-proof`, all k6. Independent scope/admission review reports no remaining actionable findings after the facade correction.

Final signed delivery SHA, full freshness result and receipt follow below. No push or remote CI.


### Signed local delivery

Commit `c4608d30b88fa60e857e223f15295c3a720ce515` is GPG verified, carries the explanatory conventional body and DCO, and leaves the worktree clean. The shared emitter lease is released. Cherry-pick only this commit after integrated201; local201 prerequisite1951d60e is not a separate delivery.

Final full freshness:172 fixtures,0 stale,0 failed. Exact-head local parity gate:2 touched sources,6/6 proof-required recorded,0 extra recorded nonproof exports; broader eight-source gate21/28. All13 original parity cases,23+13+153 backend tests, targetedClippy,34Python tests and formatting pass. Four strict proof/application cases passed with the exact limits above.

Receipt: `${LOCAL_EVIDENCE}/compact-adr202-delivery-receipt.json`
SHA256: `7b52b53bb75cd97588b8975af494e3d981a7ef9c39b9ef28331c0fe3d7caf959`
Exact-head gate: `${LOCAL_EVIDENCE}/compact-focused-adr202-exact-head/receipt.json`
Broader peer gate: `${LOCAL_EVIDENCE}/compact-focused-adr202-final/receipt.json`
The receipt pins all changed files, original source/capture, compiler, keys, ZKIR and logs, including the initial failing compile/test evidence. No push or remote CI.


### Terminal lexical returns integrated — 70251e9a (2026-10-06)

ADR202/[#306](https://github.com/MediaNoxLabs/compact/issues/306) is integrated as signed/DCO `70251e9a`. A structural adapter places extracted returns inside the final Sequence/Let continuation and reuses the existing typed ReturnPlan evaluator. Earlier statements, branches and helper callers retain their own scopes. The bounded Field Cell profile supports typed witnesses and actionful Field-returning helpers. Schema 20 and ABI 48 remain unchanged.

Independent review found and resolved a generated facade collision for a circuit named `context`. Qualification also covers emitted parameter names, including normalized and raw Rust identifiers. The fix is covered alongside the original `echo` parameter collision. Review found no remaining actionable scope or admission issue. The README merge retained both the prior strict-Dust instructions and the new ADR202 section.

Main verification passed:

- Eight selected sources: **24/31 proof-required APIs available**, with seven existing original-contract gaps preserved.
- All **13** original TS/native/recorded cases pass; backend **23 library + 13 CLI + 153 renderer tests** and targeted strict Clippy pass.
- Four **2912-byte** proofs (`two`, `three`, `nested`, `observed`) verify and reject changed bindings. Separate Dust funding and default-strict ledger application check exact returned Fields and updated Cell state. Prior Cell state is explicitly seeded; no prior lifecycle claim.
- All **172 fixtures** are fresh. The extra `let-return-oracle` change only replaces an old temporary with shared planner bindings; its behavior test is included.
- Whole-source inventory: **370/377 proof-required APIs available, seven explicit gaps** across 213 sources / 746 exports / 192 compiled roots / 369 nonproof exports. Zero unassessed, missing or unmatched rows, and no baseline drift. Remaining gaps: four original microDAO and three Coracle exports.

Receipt: `${LOCAL_EVIDENCE}/compact-70251e9a-integration-receipt.json`; focused gate: `${LOCAL_EVIDENCE}/compact-focused-70251e9a/receipt.json`; inventory: `${LOCAL_EVIDENCE}/compact-70251e9a-inventory.json`. Latest full gate and clean portable archive remain the preceding **7a311c9f** checkpoint; this is a focused integration receipt. Source-corpus API counts do not establish complete language or production parity.

Active pipeline: ADR203 qualified send owns the emitter; ADR205 canonical persistent allocation owns runtime changes; ADR204 prepares the dependent receive-to-immediate-send transient test. All have ADRs and milestone issues. No push, publication or remote CI. User-owned documentation edit remains unchanged.
