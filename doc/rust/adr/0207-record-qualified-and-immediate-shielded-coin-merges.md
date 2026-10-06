---
id: RUST-ADR-0207
alias: ADR-0207
title: "Record qualified and immediate shielded coin merges"
date: 2026-10-06
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["recording", "merge", "shielded-coin"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: 1f427d16de24e8d1e860188044f98e29bcd462420cd2f41377df52c31ebede6d
---
# RUST-ADR-0207 — Record qualified and immediate shielded coin merges

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. The shared typed plan records qualified historical merge and received-right immediate merge with checked Uint128 arithmetic and source order. Two original wrappers and their strict seeded proofs are covered; no global generic merge admission or new offer-policy relaxation.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#311 closure](https://github.com/MediaNoxLabs/compact/issues/311#issuecomment-6017757308). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`3404c30c`](https://github.com/MediaNoxLabs/compact/commit/3404c30c965248d69f07e759e22902fc4faf5de6) · [`5806c197`](https://github.com/MediaNoxLabs/compact/commit/5806c1974d1487c4bd626051f9b4c4a88e0e0474) · [`61dc7a9d`](https://github.com/MediaNoxLabs/compact/commit/61dc7a9db50bd476d11e7d0d5a3ad8fcfd674c64) · [`a7e14034`](https://github.com/MediaNoxLabs/compact/commit/a7e14034f6153212a356b47a1be73c72182e4fa5) · [`fbe42d3d`](https://github.com/MediaNoxLabs/compact/commit/fbe42d3db3dbc6d142ce3b46c65868cfbc7f792c). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
status: accepted
date: 2026-10-06
```

## Historical decision and amendments

Status: accepted for independent preparation; shared emitter/runtime changes wait for ADR206 signed handoff.
Date: 2026-10-06.

### Recommendation

Deliver two unchanged-standard-library wrappers through the existing typed `Plan`:

```compact
export circuit merge_qualified(a: QualifiedShieldedCoinInfo, b: QualifiedShieldedCoinInfo): ShieldedCoinInfo {
  return mergeCoin(disclose(a), disclose(b));
}
export circuit receive_then_merge(a: QualifiedShieldedCoinInfo, b: ShieldedCoinInfo): ShieldedCoinInfo {
  receiveShielded(disclose(b));
  return mergeCoinImmediate(disclose(a), disclose(b));
}
```

The first establishes the reusable merge helper with two real historical contract inputs. The second composes one historical input with one explicitly selected received transient, using ADR204 unchanged. This is a compiler recording slice with two real strict proof paths, not closure of whole application exports. A new runtime ABI/schema is not expected: native checked wide addition/casts, shared recording intent/Kernel methods, authoritative mapping and transient reconciliation already exist. Reassess only if implementation discovers a genuine new runtime entry point.

### Current evidence and reproducibility

Own clean head `b36b2e8f47ddb4648a8fe5a34b7231bc7f00a2e5` contains ADR204 plus 203/205 dependencies. Root integrated equivalent delivery at `a7e14034` and is running full gates. Research used immutable `${LOCAL_EVIDENCE}/compact-adr204-compactc`, frozen Scheme `${HISTORICAL_NIX_STORE}/1d5x83y7dgg6abzv0fzvna893na3nfjj-compactc-scheme-local-slice/bin/compactc-scheme`, and corrected TS runtime `${LOCAL_EVIDENCE}/compact-adr200-runtime`. No Cargo target or repository files changed.

- Source: `${LOCAL_EVIDENCE}/compact-adr207-merge.compact`.
- Rust generation: `${LOCAL_EVIDENCE}/compact-adr207-merge-rust/contract/{lib.rs,compact-rust-ir.json,rust-capabilities.json}`. Compiler succeeds; no Cargo acceptance claim was made during this read-only task.
- TS generation: `${LOCAL_EVIDENCE}/compact-adr207-merge-ts/contract/index.js`.
- Independent TS probe: `${LOCAL_EVIDENCE}/compact-adr207-merge-probe.mjs`; raw evidence `${LOCAL_EVIDENCE}/compact-adr207-merge-ts-probe.json` (16 cases).
- Extracted unchanged original microDAO IR: `${LOCAL_EVIDENCE}/compact-adr207-mergeCoin-ir.json`, `${LOCAL_EVIDENCE}/compact-adr207-mergeCoinImmediate-ir.json`, `${LOCAL_EVIDENCE}/compact-adr207-upcastQualifiedCoin-ir.json`, from root's compiled original source.

Both exports are proof-required and currently recorded=false/observed=false. Qualified wrapper reports unsupported `StateReturn::Expression` at `return_value`; received wrapper reports unsupported `StateAction::Let` at `callee[receiveShielded].actions[0]`. These generic report reasons do not identify the entire new policy requirement.

### Standard helper semantics and exact typed IR

`compiler/standard-library.compact:207–233` defines these unchanged helpers. `mergeCoin` has no extracted actions; its return is `Expression(Let(selfAddr = KernelSelf, Sequence(...)))`. `mergeCoinImmediate` is an action-free expression call to `mergeCoin(a, upcastQualifiedCoin(b))`. The pure upcast preserves nonce/color/value and adds Uint64 literal index zero, the singleton proof-tree index.

`mergeCoin` evaluates in this order:

1. Metered `kernel.self()`.
2. `createZswapInput(a)` then claim the nullifier of downcast(a), self.
3. `createZswapInput(b)` then claim the nullifier of downcast(b), self.
4. Assert byte-exact color equality, with message `Can only merge coins of the same color`.
5. Build a coin in normalized member order. Its nonce is the first input nonce evolved through `degradeToTransient`, the existing `midnight:kernel:nonce_evolve` two-Field transient hash, then `upgradeFromTransient`. Color is a.color. Value is the checked sum narrowed to Uint128.
6. Create exactly one output to self, derive its upstream commitment, claim spend, then claim receive, then return that coin.

The sum IR is precise and must not be simplified into modular Field arithmetic:

- Input fields: max `340282366920938463463374607431768211455` (u128MAX).
- Each operand cast: max `680564733841876926926749214863536422910` (2*u128MAX).
- `UnsignedAdd` result: max `680564733841876926926749214863536422911` (2^129−1).
- Final checked `UnsignedCast`: u128MAX.

The current native emitter already renders `WideUint` and checked arithmetic. Shared recording currently admits `UnsignedAdd` only under the independent phase-reset policy for its Uint64-derived bounds. That guard must stay intact. The merge policy should authorize this exact declared operand/result type chain and call the existing `unsigned_arithmetic_syntax`/`unsigned_cast_syntax`, with actual materialized types checked. No new evaluator, arithmetic implementation or loose global add flag is needed.

### Independent TypeScript observations

For each wrapper the 16-case probe includes normal17+25, swapped inputs, zero+zero, u128MAX−1+1, 2^64+1, u128MAX+1 overflow, wrong color and duplicate coin identity.

- Qualified success: five queries, 27 VM operations, events input→input→output, three private Unit outputs.
- Received success: seven queries, 36 VM operations, events output(receive b)→input(a)→input(b singleton0)→output(merged), four private Unit outputs.
- Qualified wrong-color and overflow rejection: three successful query prefixes/15 operations, input→input events, no output event.
- Received wrong-color and overflow rejection: five successful query prefixes/24 operations, output→input→input events, no merged output event.
- Rejected calls expose no returned execution context/private transcript; do not invent post-error state or private-output counts. The probe records only successful intercepted query/intent prefixes before the exception.
- Swapping inputs preserves the sum but changes the output nonce because it depends on a.nonce. Preserve call argument order and input order; do not canonicalize source evaluation.
- Duplicate identity is accepted by raw TS execution. It is not a valid funding/proof claim: upstream offer/nullifier uniqueness must reject double consumption in strict preparation or validation. Preserve this distinction rather than silently changing raw source semantics.
- Compute gas: qualified summed queries 5489780080, received summed queries 7672772861, TS wrapper last query 1102262433 for both. Future fixture must also retain separate whole-program replay cost; the research probe did not collect it.

### Reusable admission design

Use an explicit shielded-merge policy with an input shape enum, such as `HistoricalPair` and `ReceivedRight`, under the current composite domain representation. Preserve ShieldedSend, ImmediateShieldedSend, ShieldedReceive, composite-intent, phase-reset and ADR206 routing distinctly. Reuse declaration-directed resolution, isolated helper scopes, evaluate-once arguments, all-branch/all-binding audit and cycle detection.

Share the already audited pure coin closure checks and singleton bridge inspection where structurally identical; do not copy the entire send planner. Merge-specific admission allows the typed coin return and the exact wide-add chain. The root for `ReceivedRight` requires exactly the audited receive prefix on the same second coin and then the immediate bridge on the same first/second values. A qualified root has no receive prefix and both parameters remain historical qualified inputs. Restrict the resulting policy to the evidenced Kernel/intent/pure helper capabilities and expected counts; unsupported ledger writes, witnesses, extra prefixes, recipient redirection or arbitrary helper effects remain rejected.

Before: native methods only. After: existing developer-facing typed `recorded::merge_qualified`, `recorded::receive_then_merge`, and observed-call facade methods returning generated `ShieldedCoinInfo`. Public users still supply actual upstream offers/options, not VM instructions. No post-generation code patch or facade-specific special type is proposed.

### Does ADR204 already support the mixed union?

Inspection says yes, subject to actual integration tests and strict proof:

- `ContractTransientCoins::reconcile_events` identifies the exact singleton transient input by matching output-to-self, full coin, commitment/nullifier and output-before-input order; it returns only those input positions.
- Remaining inputs go through existing historical tree/index/owner/nullifier checks. Therefore a real historical input at ledger index0 remains distinct from a different transient's singleton index0.
- Final ordinary-input count is historical circuit inputs + explicit wallet funding; selected transient inputs are subtracted only after proving their event positions. All upstream offer identities remain retained.
- Outputs are an exact bijection of ordinary outputs plus selected transient outputs against the immutable canonical rows. Source order receive→merged can map to actual transient3→merged2 at frontier2; logical cursor still advances twice.
- Existing plan equality seals the complete ordered events and allocations. No new allocation mode, wallet-state overloading or transient inference should be introduced.

### Strict proof and regression plan

1. Qualified case: seed two distinct same-contract historical coins17 and25 at indices0/1, frontier2; construct two real contract-owned inputs and one real self output42. Record, prepare, prove all inputs/output plus original contract keys, independently verify changed-binding rejection, pay separate Night-backed Dust, validate at default strictness and apply. Check output ownership/value/index and both spent nullifiers; reject replay.
2. Mixed case: seed historical contract17 and wallet25 (frontier2), construct genuine wallet input, actual received contract output25 plus its full transient carrier, and final self output42. Select existing wallet funding + canonical indices + actual transient. Prove/apply with default strictness and separate Dust; check historical, wallet and transient nullifiers, actual final2/transient3/frontier4, and replay. Retain raw TS provisional receive2/merged3 evidence separately and replay its public program under the actual map.
3. Include full native/recorded/TS result/state/effects/private-order/program/replay equality, sum/last/replay gas and wrong-color/overflow successful prefixes. Add color+overflow combined case to prove color assertion wins; add max+max checked rejection and gas limits derived from captured query costs if a real later-query boundary is useful.
4. Structural negatives: changed receive coin, changed bridge member, index1, wrong add/cast bounds, result/member type mismatch, hidden effects in unused bindings/unselected branches/both commitment operands, helper cycles, extra receive/prefix, non-self output, arbitrary witness or ledger mutation. Keep old phase-reset guard negatives.
5. Preparation/proof negatives: duplicate identity/nullifier, missing selected transient/wallet input, swapped/mismatched full proof carrier, wrong historical index/owner, malformed singleton path, changed map/final plan and replay. Reuse ADR204 tests rather than duplicate every unrelated policy case.

Proof artifacts must be separate from skip-zk outputs; source flags join authoritative contract-info for both proof-required exports. Use affected fixture checks first, then full freshness once to identify actual changes. No remote CI or new Cargo target.

### Application dependencies deliberately left open

- microDAO `set_topic` lines149–158: authority/phase guards, receive, optional empty/occupied qualified pot, then optional topic/beneficiary Cells and phase write. The occupied path uses merge immediate; full export needs composition with those existing domains.
- microDAO `buy_in` lines166–180: amount/cost/native-color guards, receive/optional pot merge, then mintShieldedToken to ownPublicKey. Mint/funding/identity composition is additional scope.
- Coracle `start` lines174–201: receives wager AND deposit; occupied pot merges wager while deposit remains an ordinary persistent output. Board/key witnesses, branches and qualified Cell writes remain. Existing transient/persistent union can represent this shape, but this proposed two-wrapper slice does not claim its full recording/proof acceptance.

Root approved this bounded design. ADR206 currently owns the shared emitter/runtime lease; independent source/capture/key/proof preparation may proceed. Final shared changes must wait for its signed handoff. Root temporarily owns the warm consumer Cargo target; coordinate before any Cargo use. Do not create a cold target.


Issue: https://github.com/MediaNoxLabs/compact/issues/311 in rust-backend-v2. Independent preparation branch codex/adr207-shielded-merge starts from signed ADR204 b36b2e8f. Shared emitter/runtime remain ADR206-owned; consumer Cargo target remains root-owned until release.

### Independent preparation checkpoint — 2026-10-06

Issue [#311](https://github.com/MediaNoxLabs/compact/issues/311). Signed/DCO commit `acf7b6b6456f570a9fc182a87d340eee77de933a`, GPG verified and clean. This is preparation only: both proof-required exports remain recording-unavailable, and the strict proof module is unregistered and not yet compiled or executed.

The checked source calls the unchanged standard helpers. Twenty independent TS/native cases pass: normal 17 + 25, swapped sender order, zero + zero, exact u128 maximum, values above u64, two overflow cases, wrong colors, combined color-and-overflow failure, and duplicate identity. Wrong color wins before overflow. The first input's nonce determines the evolved output nonce. Rejected calls expose successful query/intent prefixes only; no post-error returned context/private transcript is asserted.

Two upstream construction scenarios pass. Qualified merge consumes two distinct historical contract inputs. Immediate merge combines historical contract 17 with wallet 25 received as an actual contract transient. Both create merged contract 42. Frontier starts at 2; the mixed path allocates merged output 2 and transient 3, while raw source order uses received 2 then merged 3. Raw duplicate execution succeeds; an actual upstream offer containing the repeated input rejects the exact repeated nullifier. This allocation/identity evidence is separate from future default-strict proof application.

Pinned proving artifacts are preserved at `${LOCAL_EVIDENCE}/compact-adr207-proof/keys` and never overwritten by skip-zk output. Qualified merge: k = 15, 19,606 rows. Receive-then-merge: k = 15, 25,367 rows. Both prover files are approximately 10 MiB. No keys are committed.

Two fixture tests, strict package Clippy, targeted fixture freshness, exact independent TS recapture and workspace formatting pass. Shared emitter/runtime files are untouched. The unregistered proof module prepares two actual offer paths with explicit offline seed state and separate Night-backed Dust; registering and executing it waits for recorded APIs after ADR206's signed handoff. ABI48 preparation output will be regenerated against ADR206's eventual baseline.

Receipt `${LOCAL_EVIDENCE}/compact-adr207-preparation-receipt.json`, SHA256 `5d429b3e1ce90fa10dde5d539ea2c95ed6479f0b97e73fb37d4c894dc486553a`. This appended checkpoint preserves and clarifies the earlier research evidence; it does not replace the historical notes or claim application-export closure.


### Implementation and verified delivery, 2026-10-06

The shared emitter lease arrived in signed ADR206 `556ef73e` (local dependency `04b7d6e7`), followed by the root Coracle cohort test correction `5806c197` (local `0847a8c3`). Merge keeps ABI49/schema20 and changes no runtime API or policy.

Before: both unchanged source exports had native implementations but recording/observed-call unavailable. After: `recorded::merge_qualified(context, a, b)` and `recorded::receive_then_merge(context, a, b)`, plus typed Contract observed-call methods, return the same generated ShieldedCoinInfo and use existing recording primitives. No low-level VM program is exposed as the developer input API.

Emitter changes: `ShieldedMerge(HistoricalPair | ReceivedRight)` is a distinct domain. The shared Plan owns lexical scope, declaration-directed calls and ordered materialization; the new policy only audits signatures/provenance and complete reachable effects. Singleton qualification is reused from immediate send. Unsigned addition uses the existing checked two-limb syntax, with exact operand maximum `2 * u128::MAX`, result maximum `2^129 - 1`, and checked u128 narrowing. Send subtraction remains confined to its previous profiles. Root receive prefix and forwarding are checked structurally, without source/helper name gates. No copied evaluator or generic enabled-flags path is introduced.

Evidence: all 20 TS/native/recorded scenarios pass, including color-before-overflow precedence, maximum values, swapped nonce order and duplicate raw coin identities. Success compares full public operations, private outputs, state, effects, ordered intent plan and summed-query gas, plus upstream whole-program replay with its separately captured gas. Failure captures retain only TS successful prefixes, not invented returned contexts. Four renderer test groups cover exact arithmetic bounds, unsupported arithmetic, provenance, singleton index, changed coin forwarding, extra/missing prefix, hidden branches/bindings, missing local and cycles; renamed declarations remain admitted.

Both 4,480-byte circuit proofs verify and changed binding rejects. Both full transactions use actual upstream historical inputs; the immediate case adds wallet funding and an actual transient. Separate Night-backed Dust funds fees; default WellFormedStrictness is retained. Both apply successfully and check canonical output index/owner, nullifiers and replay refusal. Frontier is 2. Original historical coins are seeded offline, not a proved deposit history. Raw TS provisional allocation remains distinct from offer-backed authoritative allocation. Proof keys and pinned ZKIR are retained under `${LOCAL_EVIDENCE}/compact-adr207-proof`; proof log `${LOCAL_EVIDENCE}/compact-adr207-proof.log`.

Additional gates: 176 fresh fixtures, backend 36 unit + 13 CLI + 153 renderer tests, 34 Python tests, strict Clippy for backend/merge fixture/proof runner, source contract-info/capability join and strict recording CLI pass. Initial local proof-draft compilation used an incorrect RecordedResult type name and was corrected to existing RecordedCircuitResult; original failure log retained. The inherited Coracle Python list failure was corrected by the separate signed root dependency, and the declaration baseline adds only the two new merge exports.

Limits remain: original microDAO set_topic/buy_in and Coracle start need separately audited application composition. Whole fallible wallet/transient funding is not admitted by this delivery. The independent set_topic partition note records why ADR206 whole-fallible persistent-only policy does not cover occupied pots.


### Signed receipt

### ADR207 delivered locally

Signed GPG+DCO implementation `97151d37973cf319c61358c5f7e666571a4c339c` follows preparation `acf7b6b6456f570a9fc182a87d340eee77de933a`. ABI49/schema20 are inherited from ADR206; this slice changes no runtime API or allocation policy.

- Both unchanged standard-library wrappers now expose recorded and observed-call APIs via the shared typed planner.
- 20 independent TS/native/recorded/replay scenarios preserve color-before-overflow, exact u128/129-bit bounds, nonce order, full operations/private output/state/intent/gas evidence and rejection diagnostics.
- Both 4,480-byte circuit proofs verify; changed binding rejects. Actual historical inputs, wallet input/transient where required, and merged output proofs pass default-strict validation and ledger application with separate Dust. Canonical index/owner and nullifier replay refusal are checked. Initial historical coins are explicitly seeded offline.
- 176 fresh fixtures; backend 36 unit + 13 CLI + 153 renderer tests; 34 Python tests; strict Clippy for backend, fixture and proof runner. Exact-head four-source gate passes 9 commands, 10/14 proof-required recording capabilities; four original microDAO gaps remain explicit.

Keys/ZKIR remain `${LOCAL_EVIDENCE}/compact-adr207-proof`. Final local receipt `${LOCAL_EVIDENCE}/compact-adr207-delivery/receipt.json`, SHA256 `d83f9c2aa29dcea64303d0318e5224e0e5622bff5f2fc4b5deffb03f0dc5bd24`.

Original microDAO/Coracle application composition and whole-fallible transient funding remain separate work. ADR0207 and the register are maintained in the midnight vault. No push or remote CI.

Correction to the earlier implementation note: the initial RecordedResult naming error was in the local parity test return annotation, not in the independent proof module. The proof module compiled and both strict paths passed after registration.


### Qualified and immediate merge accepted locally — fbe42d3d (2026-10-06)

ADR207 / #311 is integrated as signed/DCO `61dc7a9d` (preparation) and `fbe42d3d` (recording). Both unchanged standard-library merge paths reuse the typed Plan and existing ledger-8 runtime primitives: historical pair and received-right transient. Checked widening/addition/narrowing preserves exact u128 limits and source failure order. Schema 20 / ABI 49 remain unchanged.

Root checks passed: backend 36 + CLI 13 + renderer 153 tests, 34 Python tests, six-source focused gate (13 commands, 14 of 20 proof-required APIs recorded, six explicit original-source gaps), strict three-package Clippy. Whole inventory: **380 of 386 proof-required APIs available**, 217 sources / 755 exports / 196 compiled roots / 369 nonproof exports; 1,024 unique baseline declarations; zero unassessed, missing metadata, unmatched exports or drift. Availability is not complete behavioral coverage.

Signed source delivery carries 20 independent corrected-TS/native/recorded/replay cases and both actual 4,480-byte strict proofs. Historical 17+25 produces contract-owned 42 at frontier 2. Historical 17 + wallet 25 receives a transient then merges to 42, with final index 2, transient index 3, frontier 4 and three nullifiers. Both use real upstream component proofs, separate Dust, default-strict ledger application and replay refusal. Prior coins are seeded offline; no funded application lifecycle or live wallet claim. All 176 generated fixtures were fresh at source handoff.

Root receipt: `${LOCAL_EVIDENCE}/compact-fbe42d3d-integration-receipt.json`; focused receipt: `${LOCAL_EVIDENCE}/compact-focused-fbe42d3d/receipt.json`; inventory: `${LOCAL_EVIDENCE}/compact-fbe42d3d-inventory.json`; strict source delivery: `${LOCAL_EVIDENCE}/compact-adr207-delivery/receipt.json`. Last complete same-head full/portable acceptance remains **a7e14034**. Current root has subsequently integrated ADR210 as `3404c30c`; its runtime checks are running, so this completed checkpoint remains fbe42d3d until they pass.

Next: ADR209 original Coracle concede owns the emitter. Approved ADR211 original microDAO set_topic adds explicitly selected whole-fallible wallet/transient funding, then composes original recording after209. ADR210 removes the measured debug-frame stack overflow with one retained private allocation. Original cash_out is being researched independently. Issues remain open under final acceptance policy. No push or remote CI; user documentation remains untouched.
