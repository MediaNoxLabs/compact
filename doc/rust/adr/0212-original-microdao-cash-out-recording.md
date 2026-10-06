---
id: RUST-ADR-0212
alias: ADR-0212
title: "Original microDAO cash-out recording"
date: 2026-10-06
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["recording", "microDAO", "cash-out"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: 0b9cb2dc07ca0b4861017418b0cca1816f05db5de49160d7714d8cd20f6f2601
---
# RUST-ADR-0212 — Original microDAO cash-out recording

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. A separate reset-payout domain records original cash_out with send/private outputs before reset(true) and a saved sent-coin return. Seeded strict application and late overflow rollback are covered; partition observations are measured cases, not a theorem.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#316 closure](https://github.com/MediaNoxLabs/compact/issues/316#issuecomment-6017765811). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`53eb9a1e`](https://github.com/MediaNoxLabs/compact/commit/53eb9a1e6e329ca9f224de41396dbe5320921e3d) · [`a7e14034`](https://github.com/MediaNoxLabs/compact/commit/a7e14034f6153212a356b47a1be73c72182e4fa5). These include historical prerequisites and probes, not only final delivery commits.

## Historical decision and amendments

Status: implemented and locally verified; signed delivery ready for integration (see final receipt below).

Date: 2026-10-06.

### Problem and smallest useful delivery

The unchanged original `test-center/test-contracts/micro-dao.compact` already has native `cash_out(): ShieldedCoinInfo`. Its recorded boundary needs to compose a selected recipient key, optional beneficiary and Counter guards, a full-value historical pot transfer, and the actionful `reset_state(true)` helper before returning the previously bound `coin.sent` value. Each individual effect has an existing runtime implementation. The missing piece is a closed structural compiler admission that uses the shared typed Plan to combine them in source order and preserve the caller's terminal lexical bindings across the helper.

Deliver only original `cash_out` and its required helper closure. Reuse ADR206 persistent fallible offers. No merge, wallet input selection, transient, change output, new funding tolerance, whole proposal lifecycle, or generalized DAO admission is required.

Before: native execution exists; recorded cash-out capability is unavailable. After, the ordinary generated recorded facade should expose `cash_out(context) -> Result<RecordedCircuitResult<ShieldedCoinInfo, ...>, ...>` with the existing generated type conventions. This is an expected API sketch, not a claimed emitted signature. It selects the existing caller-configured own coin public key, records the original effect sequence in one frame, and returns the source's saved sent coin. Preparation remains explicit: retained historical-input/user-output offer, canonical indices, and `OfferPlacement::Fallible(nonzero_segment)`. The runtime never moves or retargets an offer implicitly.

No new emitter-to-runtime method is expected. ABI49/schema20 should remain unchanged if implementation confirms the existing leaves suffice. Any newly discovered runtime or type boundary returns for review.

### Source and independent evidence

Reference IR: `${LOCAL_EVIDENCE}/compact-cashout-research-ir.json` and `${LOCAL_EVIDENCE}/compact-cashout-research-a7e14034/contract/compact-rust-ir.json`.

Reused unchanged original generated TypeScript: `${LOCAL_EVIDENCE}/compact-set-topic-research-ts/contract/index.js`, SHA256 `416eb0583adb159e29699ddd234ef036ea8a9b764c1a849018b569cc485a40e0`.

Probe: `${LOCAL_EVIDENCE}/compact-cashout-probe.mjs`; final data: `${LOCAL_EVIDENCE}/compact-cashout-probe-final.json`; final stderr: `${LOCAL_EVIDENCE}/compact-cashout-probe-final.log` (empty). The probe enforces corrected runtime `${LOCAL_EVIDENCE}/compact-adr200-runtime` and imports pinned ledger-v8 from the existing local wallet-live installation. It executes original methods, observing completed queries and native witness calls without replacing their behavior. Declared witness providers throw if called. Partitioning deserializes the original contract state and installs the captured commitment/index map through immutable `QueryContext.insertCommitment` before constructing `PreTranscript`.

This evidence is TypeScript execution plus pinned ledger transcript partitioning, not proof verification or transaction acceptance. Seeded state bypasses the proposal/deposit/voting lifecycle.

The first diagnostic files are retained: `${LOCAL_EVIDENCE}/compact-cashout-probe.json` accidentally counted diagnostic ledger-view queries because arrays were not snapshotted, and `${LOCAL_EVIDENCE}/compact-cashout-probe2.json` inspected the wrapper caller on failure. The final file snapshots immediately after the call and separately observes the actual callee context. Neither instrumentation correction changes source behavior. Only final counts below are evidence.

### Exact guards, branches, and ordering

The source first obtains `ownPublicKey()`, then lazily checks:

1. state equals final;
2. beneficiary.is_some;
3. beneficiary.value equals the selected own key;
4. `no.lessThan(yes)`.

The source does not check participant-set membership or `pot_has_coin`. Missing beneficiary is the relevant absent-recipient negative. The successful empty participant-set and `pot_has_coin=false` controls must remain successful. Caller-selected own-key configuration is not evidence of wallet secret-key ownership: there is no declared secret witness here, and payout is constrained to the stored beneficiary.

The ordinary eligible seed uses final phase, populated topic/key, yes4/no3, round7, nonzero pot99, two tree leaves, two members in each set, and cursor2. Successes have 21 completed source queries, 71 VM operations, three private outputs in order `[own key, Unit input, Unit output]`, no declared witnesses, one historical qualified input, one full-value user output, and raw cursor2→3. The source result is the sent coin saved before reset.

| Query | Source effect |
|---|---|
| 0 | read state |
| 1–2 | read beneficiary separately for tag and value |
| 3–4 | read yes, then compare no against that saved threshold |
| 5–6 | read pot separately for full coin and amount.value |
| 7 | Kernel.self |
| between7–8 | native input intent |
| 8 | claim nullifier |
| between8–9 | native output intent |
| 9 | claim Zswap coin spend |
| 10–14 | setup state, topic none, yes reset, no reset, beneficiary none |
| 15–17 | reset tree/cursor, committed set, revealed set |
| 18 | increment round by1 |
| 19–20 | default qualified pot; pot_has_coin=false |

Do not deduplicate repeated reads, reorder private outputs, move resets ahead of send, or evaluate a later lazy guard early. Cash-out uses direct Counter comparison; it does not need advance's widened `no+1` arithmetic.

The 21 pinned cases comprise 9 successes and 12 expected failures:

| Cases | Observed result / completed source prefix |
|---|---|
| eligible; empty collections/topic; long topic4096; pot flag false; pot0; pot u128MAX; round u64MAX−1; yes1/no0; yes u64MAX/no u64MAX−1 | success,21 queries/3 private outputs |
| phase setup/commit/reveal | exact illegal-cash-out assertion,1 query/1 private output |
| absent beneficiary | same assertion,2 queries/1 private output |
| wrong recipient | same assertion,3 queries/1 private output |
| yes=no; yes<no; both u64MAX | same assertion,5 queries/1 private output |
| missing key, including wrong phase | JS missing `.bytes` error,0 queries/0 private outputs |
| zero gas | exact `Error: ran out of gas budget`,0 completed queries/1 private output |
| round u64MAX | exact `Error: arithmetic overflow`,18 completed queries/3 private outputs |

Rust's typed missing-key error should preserve the failure point; the JS exception text is not a cross-language error-string contract.

At roundMAX, input/output intents and the first eight reset queries have happened inside the callee before increment fails. The observed callee has setup state, empty topic/beneficiary, reset Counters/tree/sets; round remainsMAX and pot/flag remain unchanged. The wrapper caller remains original and no successful result is returned. This instrumentation does not establish a public partial-state return API. The consuming Rust error path should not expose a fabricated successful result.

### Ledger partition and strict scope

All nine tested successes partition as `[None, fallible71]` using pinned initial ledger parameters, including empty/populated collections, short/long topic, and zero/max pot. This is measured for those cases, not a universal partition theorem. Bound preparation must inspect the actual prototype and continue rejecting unsupported guaranteed/fallible mixtures.

The existing ADR206 policy is sufficient in principle: one unchanged, nonempty persistent contract-input/user-output offer; canonical allocation; exact retained proof tag vectors for selected nonzero segment; no wallet funding or transients; wholly fallible public transcript. Separate Dust fee funding is orthogonal. Physical intent keys remain nonzero; logical guaranteed phase is0.

Proposed minimum strict evidence after approval:

- Seed an actual historical contract-owned pot coin and populated final DAO state. Prefer a nonzero historical index with an unrelated prior leaf. Set the beneficiary to the selected execution key.
- Construct the real upstream input and single full-value user output with `Some(1)` segment preimages. Bind the exact retained offer with canonical indices and explicit fallible placement. Do not retarget source claims or silently repair allocation.
- Produce/verify the original call proof and both Zswap proofs, fund Dust separately, then require unchanged default strictness and ledger apply success. Confirm no guaranteed shielded offer or unrelated fallible offer was introduced by fee balancing.
- Assert the returned sent coin and recipient/output commitment/index, consumed historical nullifier, exact replay rejection, all reset state including tree cursor/sets, round increment, and byte-identical organizer/costs. Return value survives pot reset.
- Test rollback with a transaction proved from the valid seed but a current state whose round isMAX while earlier read slots stay equal. Expected behavior is late fallible arithmetic failure with all tentative contract/Zswap changes rolled back and independent guaranteed Dust treatment preserved. The actual typed ledger error and phase results must be measured and asserted; they have not been observed in this research. Retain any earlier refusal if upstream detects it sooner.

One nonzero strict original-source positive and the reused-transaction rollback path are the minimum proof scope. Zero/max-pot TS cases are execution evidence only unless separately proved. No claim of a funded proposal lifecycle, network settlement, or finality follows from seeded acceptance.

### Shared lowering design and ownership

ADR209 owns the emitter now, followed by ADR211. Cash-out preparation can be independent after approval, but shared edits wait for signed handoffs and must build on their actual code.

- Reuse ADR206 own-key leaf, sealed initial/final identity, qualified input/output semantics, and canonical persistent fallible preparation.
- Reuse ADR209 shared actionful-helper declaration dispatch and terminal continuation scopes. Its present action policy admits enum Cell writes, not Counter/Set/tree resets; do not globally expand that profile.
- Reuse ADR211 optional topic/key Cell type handling and audited `none<T>` helper closure. Its wallet/transient policy is not required by cash-out.
- Reuse ADR194 reset emission leaves and field/path/type validation. Keep original advance admission's literal-false reset restriction intact. A distinct payout/reset structural profile may admit a literal-true Boolean→Unit helper call within a ShieldedCoinInfo-returning root.

Use one shared typed Plan/evaluator and its declaration resolution, argument evaluation, active-call guards, fresh callee scopes, and result checks. A small reusable closed reset-action auditor may be shared with194 while each entry profile retains separate admission. Do not create a copied planner, broaden all actionful helpers, or rely on circuit/helper names or arbitrary operation counts.

The new profile should audit the entire reachable declaration closure, all unused binding initializers and both branches, even though this entry supplies literaltrue. Bound leaves are typed Cell writes of the required enum/optional/qualified/Boolean shapes, Counter reset and checked increment, Bytes32 Set reset, and depth10 Bytes32 Merkle reset. Verify physical field paths and slot types. Pure helper closure must remain wholly pure and correctly declared, with cycles/ambiguity rejected. Shared typed expression handling validates every scope/type even when the structural walker admits a leaf. Reject unsupported hidden effects, dynamic/false reset selection in this new entry profile unless explicitly reviewed, and unrelated action or arithmetic forms.

Keep caller's saved send result alive across the Unit helper and return it in the caller terminal scope. Helper locals/parameters must not escape; earlier sibling and branch scopes remain inaccessible. Do not use a broad flag union to accidentally grant every existing profile the combined reset/payout capabilities.

### Verification and decision request

After ADR/issue creation, prepare immutable original TS captures, native parity cases, fixture metadata, and retained keys while211 owns shared files. Then implement the bounded profile on the signed handoff.

Validation: all21 independent cases with complete query/private/intent order and failure prefixes; malformed operand/type/slot and hidden-effect rejection; full unused/unselected branch audit; pure/stateful ambiguity and recursion rejection; caller/helper/branch/earlier-sibling scope negatives; exact offer/identity preparation controls; default-stack original package tests using210; focused backend/render tests and strict Clippy; the strict positive/rollback above. Include194 false-reset,206 read-only payout,209 actionful payout, and211 optional Cell controls. No broad proof suite or new compiler-stack workaround is part of this slice.

Requested approval: original cash_out only, a distinct closed reset-payout structural admission using shared Plan, one strict historical-pot payout plus late rollback evidence, ABI49/schema20 unchanged unless an actual missing boundary requires review. No implementation has started.


### Decision

Approved on2026-10-06. Implement the bounded original cash-out profile and validation described above. Preserve ABI49/schema20 unless a concrete additional boundary is reviewed. Create the milestone issue before source edits. Research measurements remain distinct from future proof/application results.

### API example

```rust
// Existing native execution:
let result = ledger_contract::cash_out(context)?;
// Expected recorded execution after this decision:
let recorded = ledger_contract::recorded::cash_out(context)?;
```

The generated generic signature and exact bound preparation example will be recorded from actual emitted code during delivery. Preparation retains explicit canonical allocation and nonzero fallible placement; no runtime funding or offer retargeting is implied.


Issue: https://github.com/MediaNoxLabs/compact/issues/316 (rust-backend-v2). Created before source edits. Independent preparation branch: codex/adr212-micro-dao-cash-out.

### Independent preparation completed

Signed GPG+DCO commit `9bd8e2258a27163f59aeb5288089eea6d79f9f6c` on `codex/adr212-micro-dao-cash-out` adds the original TypeScript capture,21 cases, reusable seeded support, and native parity test. Exact signed-head test passed all21 cases (9 success,12 expected failure); targeted strict Clippy and formatting pass. The normal debug worker needs no stack override.

Unchanged original cash_out key generation passed at k=15,18,864 rows. Retained source/key artifacts: `${LOCAL_EVIDENCE}/compact-adr212-proof`; preparation receipt `${LOCAL_EVIDENCE}/compact-adr212-preparation-receipt.json`, SHA256 `31960546cf40079f2ea4acee87a45b0ab481a34314161376b2e66ea113231482`. Exact native log `${LOCAL_EVIDENCE}/compact-adr212-prep-final.log`; keygen log `${LOCAL_EVIDENCE}/compact-adr212-keygen.log`.

Initial harness type conversion, JSON map encoding and clone-on-Copy Clippy failures are retained in the receipt; none required source/runtime behavior changes. Recorded lowering and strict transaction proof/application remain pending the signed ADR209/211 emitter handoff. This is preparation evidence, not completion of ADR212 or a funded proposal lifecycle.


### Implementation and measured strict result

The implementation uses `CompositeDomain::ResetShieldedPayout` and a separate closed `reset_payout` admission module, with the existing typed Plan responsible for values, lexical scopes, helper arguments, source order and effects. The whole reset helper is audited, including unused bindings and its unselected branch. This profile admits only its literal-true call; ADR194 retains literal-false admission and ADR209 retains its separate actionful Field-return helper bounds. Counter increment accepts typed literal1 or the compiler’s scoped u16 local initialized to1; shared scope/type validation still applies. No runtime source, ABI49 or schema20 change was needed.

Actual developer-facing change:

```rust
// Before: native cash_out only; capability recorded=false.
let native = ledger_contract::cash_out(context)?;
// After: the same source also exposes a recorded result and observed-call facade.
let recorded = ledger_contract::recorded::cash_out(context)?;
let sent_coin = &recorded.execution.result;
// Bind to the exact retained offer before preparing the recorded call.
let options = OfferBindingOptions::default()
    .with_output_allocation(PersistentOutputAllocation::CanonicalOfferIndices)
    .with_offer_placement(OfferPlacement::Fallible(segment));
```

The options sketch uses the existing typed binding policy; the exact executable construction is in `tools/compact-rust-proof-smoke/src/micro_dao_cash_out.rs`. The generated function preserves the full-value send before all reset effects and returns the saved sent value after resetting the pot. Native and recorded execution match all21 pinned TypeScript cases; successful transcripts replay through the upstream VM. Four structural test groups cover renamed source admission, false/dynamic/repeated/missing reset paths, hidden unused effects, unselected branch audit, malformed slot/type/callee scope, and impure pure declarations.

The first cryptographic proof verified at4480 bytes. Actual upstream Input and Output proofs plus a separate Dust fee spend produced a default-strict accepted transaction. Its seeded historical pot99 was at index1 behind a padding output; the user payout was allocated index2. The exact retained persistent offer was fallible segment1.

Reapplying that same verified transaction against an otherwise unchanged prestate with round=u64MAX reached `Transcript(Execution(ArithmeticOverflow))` in physical contract intent1. Logical guaranteed phase0 and the separate Dust-only intent65261 succeeded. Contract and Zswap state remained at the changed prestate; Dust and replay protection advanced. This is measured late reset rollback, not an assumed ReadMismatch. The unchanged original prestate applied successfully, completed every reset, preserved organizer/costs, and rejected spent-nullifier replay. The first proof log is `${LOCAL_EVIDENCE}/compact-adr212-proof-first.log`; final signed-head evidence follows below.

### Signed delivery and final acceptance

Signed GPG+DCO commit `4bb4faf30793e4e1634ed059c1b1d5c69ff36d98` on `codex/adr212-cash-out-recording`, based on integrated `53eb9a1e`, is clean and ready for parent integration. No push or remote CI was used.

The final source-head run rebuilt the proof runner and proved with the retained original cash_out keys (k15,18864 rows). Its4480-byte call proof, real Input/Output proofs and separate Dust fee transaction passed default ledger strictness and application. The exact late `Transcript(Execution(ArithmeticOverflow))` rollback assertion passed. Contract/Zswap rollback and guaranteed Dust/replay persistence are checked against the changed roundMAX prestate; the original prestate succeeds and spent-nullifier replay fails.

Controls passed:50 backend library tests,13 CLI tests,153 renderer/integration tests;176 fresh generated fixtures;34 Python tests; strict Clippy on backend/original DAO/proof runner; actual four-source Cargo/capability gate13/16 proof-required exports recorded and zero nonproof recorded exports. The three remaining gaps in that selected cross-tab are Coracle start and microDAO vote_commit/buy_in; they belong to separate active deliveries. The exact signed-head four-source source-only rerun also passed. All21 original cash_out TS/native/recorded/replay cases pass.

Receipt: `${LOCAL_EVIDENCE}/compact-adr212-delivery-receipt.json`, SHA256 `b5a08bb86a4dd1a8322c9c3f44cd9de97701ebea66980b90266e3f02cd9b724f`. It records source/CLI/key/log hashes, exact scope and retained failed attempts. Final proof log `${LOCAL_EVIDENCE}/compact-adr212-proof-final.log`; source receipt `${LOCAL_EVIDENCE}/compact-adr212-exact-source/receipt.json`; actual Cargo gate `${LOCAL_EVIDENCE}/compact-adr212-focused/receipt.json`. Proof root `${LOCAL_EVIDENCE}/compact-adr212-proof`, immutable compiler `${LOCAL_EVIDENCE}/compact-adr212-compactc`.

Initial build/type-diagnostic and duplicate-module Clippy failures remain on disk. The support module now reuses one advance module through an alias; no duplicate code or lint suppression was introduced. The first accepted sealed transaction is preserved separately as `${LOCAL_EVIDENCE}/compact-adr212-first-cash-out-sealed.bin`; final proof bytes were freshly generated from the signed source.
