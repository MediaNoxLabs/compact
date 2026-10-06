---
id: RUST-ADR-0194
alias: ADR-0194
title: "Record original microDAO advancement and reset helper"
date: 2026-10-05
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["recording", "microDAO", "reset"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: 5c9cbfc0db259f877ba50712ca6d1dcf40b6bde1039016c8e153c0167d151700
---
# RUST-ADR-0194 — Record original microDAO advancement and reset helper

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. The original microDAO advance export records an audited false-reset helper and checked round/phase branches. Three seeded branches prove/apply; literal-true reset and cash-out are separate profiles.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#299 closure](https://github.com/MediaNoxLabs/compact/issues/299#issuecomment-6017735789). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`334b1475`](https://github.com/MediaNoxLabs/compact/commit/334b147508ab1c3185d5ec23c915dceda399b928) · [`c6ff13c5`](https://github.com/MediaNoxLabs/compact/commit/c6ff13c5c2ad283733cc5a45b9ca1faa734217b0) · [`ee912835`](https://github.com/MediaNoxLabs/compact/commit/ee912835169d314851b99fc92c3e543ffaa2fd2e). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
status: delivered-locally
date: 2026-10-05
adr: 194
```

## Historical decision and amendments

### Problem
The original microDAO `advance` export runs natively but lacks recorded proof preparation. Its internal reset helper combines checked unsigned arithmetic, authorization/phase checks, typed Cell writes, Counter/Merkle/Set resets, and a conditional pot reset.

### Approved scope
Record original `advance` only. Reuse the shared typed planner and same-frame Unit helper dispatch; audit/lower the whole reset helper, but admit only the original literal-false call from this entry. Preserve exact no=u64::MAX cast rejection before the yes comparison and pot preservation. Probe round=u64::MAX against pinned TypeScript/upstream behavior. Existing runtime slot APIs remain authoritative; no ABI/schema change expected. No funding-policy expansion.

### Validation
Independent pinned TypeScript/native/recorded/replay cases for authorization, commit/reveal transitions, final resets, failures and prefixes; malformed type/path/helper admission negatives; three original branch proofs with separate Dust and default-strict ledger application where verified; true source/contract-info cross-tab, focused Cargo/Clippy/freshness gates. Keep failed attempts and exact evidence.

### Coordination
Shared emitter lease ADR193 → ADR195 → ADR194. Independent captures/tests may proceed; inspect incoming code before planner changes. No push or remote CI. Signed GPG+DCO delivery.

ADR: `ADR-0194 — Record original microDAO advancement and reset helper` in the midnight Obsidian vault. Approved detailed proposal: `${LOCAL_EVIDENCE}/compact-adr194-design-proposal.md`. Depends on ADR193 helper infrastructure. Related #296 (ADR191), #297 (ADR193).

### Approved design detail



Make the original `test-center/test-contracts/micro-dao.compact` export `advance(): []` recordable. Preserve its organizer authorization, phase transitions, final-vote guard, complete reset sequence, and unchanged pot when the original call passes `false`. `reset_state(Boolean)` is an internal helper, not an additional exported API or proof obligation.

Authority: original source and `${LOCAL_EVIDENCE}/compact-focused-adr192-micro-dao-reveal/compiled/micro-dao/contract/compact-rust-ir.json`. Do not rewrite the source, substitute a lookalike contract, or count emitted API rows without the compiler contract-info proof cross-tab.

Before (existing native API):

```rust
let result = contract.advance(context)?;
assert_eq!(result.result, ());
// No recorded advance API; existing native execution remains available.
```

After (illustrative generated API, exact signature to be checked at delivery):

```rust
let recorded = ledger_contract::recorded::advance(context, &witnesses)?;
// One RecordingFrame spans witness, assertions, phase reads/writes,
// and the selected internal reset_state(false) call.
assert_eq!(recorded.execution.result, ());
// Existing observed-state preparation consumes this recorded result.
```

The public result stays Unit. There are no Zswap intents or Kernel shielded claims in this entry. No runtime ABI or schema change is expected (ABI48/schema20).

### Exact source behavior

1. Call `local_secret_key` once and compute `public_key` through the existing pure persistentHash helper.
2. Assert phase is not setup, then assert the derived key equals organizer. Witness invocation occurs before either assertion; wrong authorization in setup must still fail the phase assertion first.
3. For commit or reveal, write `successor(state)` (commit→reveal, reveal→final). `successor` is a pure enum If helper, with its original setup→commit and fallback→final behavior retained in the audited body.
4. For final, read no, compute the original widened unsigned `no + 1`, checked-cast to Uint64, then query `yes.lessThan(threshold)`. A rejected vote guard does not execute resets.
5. On accepted final reset, in order: state=setup; topic=None; yes reset; no reset; beneficiary=None; committed Merkle tree reset; committed-participants Set reset; revealed-participants Set reset; round increment1; then evaluate reset_pot (false in advance), leaving pot and pot_has_coin unchanged.

The actual arithmetic IR has operand max `18446744073709551616`, addition result max `36893488147419103231`, then cast max `18446744073709551615`. At no=u64::MAX, addition succeeds with 2^64 and the final cast returns `UnsignedOutOfRange` before the yes comparison. Do not collapse this to `yes <= no`, use wrapping arithmetic, or read yes early.

### Reusable planner boundary

Extend the existing typed Plan and the ADR193 same-frame actionful Unit helper dispatch. Resolve declarations first, audit each reachable body under an explicit reset/phase profile, evaluate call operands once in caller scope, enter a fresh callee scope with cycle guards, and return the same frame. Do not copy scopes, typed value lowering, dispatch, or VM programs into another planner.

Admission should describe typed operations and declaration roles, not literal source filenames/helper names. It remains separate from the composite-intent and token-query profiles. Do not enable composite_values merely to gain unsigned casts, and do not broaden existing profile acceptance as a side effect.

Known missing compiler boundaries requiring review:

- `UnsignedAdd` plus `UnsignedCast` for this checked unsigned Counter-threshold expression. Reuse existing `unsigned_cast_syntax` and native checked unsigned arithmetic syntax/runtime primitives, preserving declared maxima and evaluation order. This does not add Field arithmetic or arbitrary dynamic integer operations.
- CounterReset → the existing typed slot `record_reset(frame)`.
- MerkleResetToDefault for the declared fixed-depth Bytes32 tree and validated physical path → existing `record_reset_to_default(frame)`.
- SetReset for Bytes32 Sets → existing `record_reset(frame)`; current shared planner reset admission is limited to qualified-coin Sets.
- Cell writes of the exact Maybe<ZswapCoinPublicKey> type, Boolean, and QualifiedShieldedCoinInfo required by the complete reset helper. Existing Enum/Maybe<OpaqueString> admission handles state/topic. Preserve declared field type/index/path equality; do not add arbitrary Struct Cell writes.

`none<OpaqueString>` and `none<ZswapCoinPublicKey>` are pure helpers. The existing pure-body auditor already admits their StructLiteral/Default forms and can invoke the generated pure helper with no frame effects; no new opaque-string evaluation rule is needed unless the post193 dispatch shape requires inlining. That would be reported before broadening.

The complete reset helper includes a conditional true branch assigning default QualifiedShieldedCoinInfo and false to pot_has_coin. Recommendation: audit and lower both branches using existing typed Cell writes, preserve the conditional, but initially admit calls from the new entry only with the literal Boolean false. Do not delete or ignore the other branch during admission. This preserves sound whole-body validation without claiming that cash_out or reset_state(true) has proof coverage. A mutated unsupported effect even in the unselected true branch must fail admission.

No runtime addition is expected: Counter reset uses canonical Cell writes; Counter increment uses canonical counter_program; Merkle reset and Set reset already have authoritative runtime programs. Generated crates must contain typed runtime/slot calls, never copied opcode literals.

### Independent preparation after approval

Reuse the original full microDAO fixture/support introduced for vote_reveal and the same source identity. Add an independent pinned TypeScript capture for advance using runtime0.16.101 and the actual compiled original source. Capture witnesses/private outputs, ordered public query operations/results, total gas, final entire ledger state, and exact failure prefix. Keep seeded prior contract state explicit; these tests do not prove a full deposited/voted DAO lifecycle.

Prepare fixture tests and proof setup without touching shared emitter files until ADR193→ADR195 lease handoff. Check the actual posthandoff planner first so changes already supplied by those slices are not duplicated.

### Proposed validation matrix

Positive TS/native/recorded/replay:

- Authorized commit→reveal and reveal→final, unchanged unrelated fields.
- Authorized final with yes=no=0, yes=no>0, and yes<no, using nonempty Merkle/Set data, Some topic/beneficiary, nonzero round, and a distinctive populated pot. Assert every reset target and unchanged pot value/index/flag, not merely root equality.
- Both pot_has_coin true/false, empty/nonempty collections, None/Some option values; output is Unit in all successes.
- Threshold boundary no=u64::MAX−1 with yes=u64::MAX−1 succeeds and yes=u64::MAX fails the original vote assertion.

Exact negative/prefix cases:

- Setup with correct and incorrect secret; organizer mismatch in every other phase; final yes>no. Preserve original error message and witness/query order.
- no=u64::MAX: exact checked-cast failure after no read and before yes.lessThan; no reset queries.
- Witness failure and malformed witness result; bounded gas exhaustion before/within reset; no later effects after failure.
- Probe round=u64::MAX separately in pinned TS/native runtime before specifying an expected result. Existing Counter increment delegates to upstream Addi; do not assume it shares the local threshold cast's u64 overflow semantics.

Renderer/admission negatives:

- Wrong field kind/index/path/depth; malformed Maybe/key/coin Cell type; invalid numeric maxima/cast operands; malformed helper argument/result type; missing/ambiguous/recursive declarations; callee local escaping; unsupported effect hidden in reset_pot's unselected branch.
- Do not admit unrelated dynamic/true reset helper callers, unsupported shielded operations, or additional microDAO exports by accident. Retain all existing profiles' negative cases.

Proof/ledger:

- Pin the original `advance` proving keys once. Verify and ledger-apply the commit transition, reveal transition, and populated final-reset branch; check returned Unit binding, before/after state, transcript, and modified-binding refusal.
- Prefer the established separate Night/Dust fee setup and default strictness for ledger admission. No token offer or synthetic public operation is needed. Report actual fee/ledger outcomes; never label a proof-only result as ledger acceptance.
- Internal reset_state(true) is not proof-covered by these cases; cash_out remains a separate shielded-helper task.

Integration gates:

- Real local_parity_gate for original microDAO plus affected191/192/193/195 peers at the same ABI, including Cargo when fixtures match.
- Backend renderer/unit tests, relevant native/recorded fixtures, targeted Clippy, formatting, fixture freshness, and inventory/contract-info cross-tab. Expected delta is exactly one newly available proof API (`advance`), no new source identity; report actual numbers rather than assuming the prediction.
- No remote CI/push. Signed conventional GPG+DCO commit with explanatory problem/change/tests body, ADR194/issue links, exact source/key/log hashes and validation limits.

### Review decisions requested

Approve the bounded compiler additions above, whole-helper audit/lowering with literal-false entry admission, and three original advance branch proofs. Any additional missing primitive revealed after ADR193/195 integration must be reported before expanding scope. Upon approval, create ADR194 and its rust-backend-v2 issue before repository edits; then independent captures/tests can proceed while shared files remain leased.


### Approval and tracking
Parent approved this bounded design on 2026-10-05 before implementation. Issue: https://github.com/MediaNoxLabs/compact/issues/299 (rust-backend-v2). The proposal review decisions above are accepted. Shared emitter lease remains ADR193 → ADR195 → ADR194.


### Independent preparation checkpoint (ADR194)

Base: integrated191 `334b147508ab1c3185d5ec23c915dceda399b928`; branch `codex/adr194-micro-dao-advance`. Shared emitter lease remains ADR193 → ADR195 → ADR194.

- Original-source pinned TypeScript capture:20 scenarios,7 successes and13 expected refusals. Native Rust test passes all20 with full serialized before/after state, effects, witness/private transcript, cumulative query gas, and unchanged pot value/index/flag.
- `no == u64::MAX`: exact checked-cast refusal of18446744073709551616 against18446744073709551615, after4 successful TS queries and before yes.lessThan. Rust exact `UnsignedOutOfRange` agrees.
- `round == u64::MAX`: pinned TS `Error: arithmetic overflow` after13 successful queries; native exact upstream `ArithmeticOverflow`. Prior reset writes precede this failure in the TS prefix; the failed Rust API consumes context, so no returned partial-state claim is made.
- A malformed31-byte witness succeeds at no Rust typed boundary: FixedBytes<32> excludes that return statically. The test preserves the TS dynamic type error and models an explicit Rust witness rejection separately.
- Original advance proving keys generated once: k13,4492 rows, `${LOCAL_EVIDENCE}/compact-adr194-proof`. Three default-strict separate-Dust branch proofs are prepared in an unhooked module; no recorded/proof/ledger acceptance is claimed yet.

Evidence: `${LOCAL_EVIDENCE}/compact-adr194-preparation-receipt.json`, `${LOCAL_EVIDENCE}/compact-adr194-native-first.log`, `${LOCAL_EVIDENCE}/compact-adr194-keygen.log`. Capture script and20-case JSON are new repository files, alongside independent support/native tests. First capture is retained at `${LOCAL_EVIDENCE}/compact-adr194-ts-first.json`; its Set fixture values were corrected to canonical Null before the passing native parity capture. No runtime/schema changes, push, or remote CI.


### ADR194 delivered locally

Signed GPG+DCO commit: `733819bd32374e6a917d82354d0ab22f2208c4a7`. `git verify-commit` passes and the worktree is clean. No push or remote CI. Schema 20 / runtime ABI 48 remain unchanged.

#### Developer-facing change

The original `contract.advance(context)` native method remains available. The previously missing recorded path now supports:

```rust
let recorded = contract.recording().advance(context)?;
let prepared = contract.recording()
    .advance_call(&observed, private_state)?
    .prepare(verifier, communication_randomness)?;
```

The new phase-reset audit uses the existing typed Plan, scopes, once-only operands and same-frame internal helper dispatch. Counter/Merkle/Set resets call existing slots. Both branches of the internal reset helper are audited/lowered, but the exported entry is admitted only with its original literal-false call. No opaque-string default lowering expansion was needed.

#### Accepted evidence

- 20 pinned original TS/native/recorded cases: 7 successes with full state/effect/transcript/gas/replay comparison and 13 expected refusals. Populated and empty reset cases preserve pot value, index, and flag.
- Exact no-max checked-cast failure before the yes comparison; round-max upstream ArithmeticOverflow after the reset prefix. Failed Rust calls consume context; no returned partial-state claim. Malformed dynamic TS Bytes31 and explicit typed Rust witness rejection remain distinguished.
- 16 renderer mutations reject invalid scopes, types, maxima, paths/depth, helper recursion/declarations, true/dynamic helper calls, and hidden effects in the unselected true branch.
- Three original branch proofs (commit, reveal, final): 4,480 bytes each, independently verified, changed-binding refusal, separate Night-backed Dust, unchanged default strictness and successful ledger apply. Prior DAO state **including pot data is explicitly seeded**. This is not a funded proposal/deposit/vote lifecycle or coverage of reset_state(true)/cash_out.
- Real six-source Cargo/capability gate passes: 13/20 proof APIs, 3 nonproof native-only, 7 explicit proof gaps. Exact-head six-source compiler/cross-tab repeats successfully with Cargo skipped. Original microDAO contributes 3/7 recorded proof APIs, with vote_commit/set_topic/buy_in/cash_out still missing.
- Backend 17 + 13 + 152 tests; targeted seven-package Clippy (all targets/features, warnings denied); Python 25 + 4; six fixtures fresh; formatting passes.

Receipt: `${LOCAL_EVIDENCE}/compact-adr194-delivery-receipt.json`
SHA-256: `bf23bca57ab6de4ee0dcc1dea77fa471cc7bd56c3119c152d6ccf1e180331762`
Exact-head gate: `${LOCAL_EVIDENCE}/compact-adr194-exact-head/receipt.json`
Actual Cargo gate: `${LOCAL_EVIDENCE}/compact-adr194-local-gate/receipt.json`
Proof log: `${LOCAL_EVIDENCE}/compact-adr194-proof-first.log`
Pinned original keys/ZKIR: `${LOCAL_EVIDENCE}/compact-adr194-proof` (k=13, 4,492 rows).

The receipt hashes changed sources, keys, compiler and logs, and retains the initial seed/refresh/registry failures. Shared emitter lease has been released to ADR201; no further source edits planned.


### Integrated microDAO advancement — c6ff13c5 (2026-10-06)

ADR194/[#299](https://github.com/MediaNoxLabs/compact/issues/299) integrated as signed/DCO `c6ff13c5`. The shared typed plan records the original advance circuit and audited false-reset helper. All 20 TS/native/recorded/replay cases pass, including both overflow boundaries. Schema20/ABI48 unchanged.

- Three original branches (commit, reveal, populated final reset) each verify a 4480-byte proof, reject changed public binding, and apply with separate Dust under default ledger strictness. Prior DAO/pot state is explicit test setup; this does not establish a funded DAO lifecycle.
- Combined main: six-fixture focused gate passes (12/23 proof-required APIs available in that group), backend17 library +13CLI +153renderer tests, all171fixtures fresh,34Python tests and targeted strict Clippy pass.
- Inventory: **359/370 proof-required APIs available,11explicit gaps,zero unassessed** across212sources/739exports/191compiledroots/369nonproof. No missing/unmatched proof rows or baseline drift. Gaps:4microDAO,4terminal lexical,3Coracle. These are corpus/API availability counts, not full semantic or production parity.

Receipt: `${LOCAL_EVIDENCE}/compact-c6ff13c5-integration-receipt.json`; inventory `${LOCAL_EVIDENCE}/compact-c6ff13c5-inventory.json`; focused `${LOCAL_EVIDENCE}/compact-focused-c6ff13c5/receipt.json`. Prior historical receive capture guard remains included. Latest broad/full and clean portable package still belong to ee912835; no later package/full claim. ADR199 wallet funding is under final review, ADR201 owns shared planner changes for readonly Field composites, ADR202 terminal return adapter is approved for ADR/issue and independent preparation. No push, publication or remote CI. User-owned doc edit preserved.
