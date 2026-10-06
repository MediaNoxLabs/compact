---
id: RUST-ADR-0206
alias: ADR-0206
title: "Original Coracle withdraw and typed execution coin identity"
date: 2026-10-06
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["recording", "Coracle", "identity"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: b53a33653344d9d85314d23c2874fa2e22fcf273c3b031251d5b169160fab250
---
# RUST-ADR-0206 — Original Coracle withdraw and typed execution coin identity

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Original Coracle withdraw records native OwnPublicKey at its actual private-transcript position and binds observed identity and a whole-fallible retained offer. Seeded red/blue default-strict proofs/apply, replay and rollback pass; caller key configuration is not wallet ownership proof or a full game lifecycle.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#310 closure](https://github.com/MediaNoxLabs/compact/issues/310#issuecomment-6017755516). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`5806c197`](https://github.com/MediaNoxLabs/compact/commit/5806c1974d1487c4bd626051f9b4c4a88e0e0474) · [`8e7080f5`](https://github.com/MediaNoxLabs/compact/commit/8e7080f51ff778ff8175ca66b430ef3b3560f574) · [`a7e14034`](https://github.com/MediaNoxLabs/compact/commit/a7e14034f6153212a356b47a1be73c72182e4fa5) · [`e9ef0535`](https://github.com/MediaNoxLabs/compact/commit/e9ef05356d43af401bfc721513a619720a3109bb). These include historical prerequisites and probes, not only final delivery commits.

## Historical decision and amendments

Status: accepted for bounded implementation, 2026-10-06. Runtime ABI49 reserved; schema20 unchanged. Depends on ADR203 qualified send and ADR205 canonical persistent allocation; shared emitter/transaction leases coordinated with ADR204.

### Problem
The original Coracle withdraw graph combines authentication witnesses, read-only qualified Cells, two full sends and native OwnPublicKey. Native execution supports the key but typed recorded lowering lacks its private transcript leaf, and observed construction cannot carry execution identity. Existing send admission intentionally excludes these additional effects.

### Decision
Implement the bounded withdraw profile in the attached approved proposal. Keep source behavior, full branch audits, shared typed evaluator and strict offer binding. A caller-provided public key is execution/recipient configuration, not proof of wallet ownership. Seal initial/final identity and validate generic/observed preparation against public context replacement. No synthetic VM query.

### Developer-facing before and after
Before (available native execution, missing recorded composition):
```rust
let context = context.with_coin_public_key(key);
let output = contract.withdraw(context)?;
// withdraw_recorded was not emitted for this original graph.
```
After (intended API shape; final generated signature will be documented from delivery):
```rust
let observed = observed.with_coin_public_key(key);
let bound = OfferBackedObservedState::with_options(/* existing arguments */, options)?;
let call = contract.withdraw_recorded(bound.circuit_context(private_state))?;
let prepared = bound.prepare(call)?;
```
The generated effect leaf will invoke a new typed RecordingFrame own-key method, so generated runtime ABI advances to49. Existing ledger schema stays20. Exact wrapper signatures above are illustrative until implementation.

### Alternatives
Do not add identity arguments to every generated circuit, encode OwnPublicKey as pure data/declared witness, infer wallet spend authorization, clone a separate evaluator, or weaken offer matching. A typed observed execution setting and shared frame leaf preserve source order without facade signature churn.

### Acceptance and limits
Two strict offline funded original withdraw proofs, red/blue and opposite output commitment sort orders, separate Dust, real input nullifiers and exact replay refusal; all original contract Cells unchanged. Seeded prior game and coin state does not establish the game funding lifecycle or network finality. Declared/native witness ordering, missing key, identity mutation/reconstruction bypasses, malformed typed graph, hidden effects and lexical-scope negatives required. Concede/actionful write and cash_out/reset(true) remain separate.

### Approved proposal
## Next original shielded payout composition — read-only proposal

2026-10-06. No new ADR/issue, source edits, key generation or broad build. ADR205 is delivered as b0dc2af7 and integrated by parent as8e7080f5. ADR203 owns qualified-send lowering; ADR204 owns transient runtime composition after205. This proposal uses the unchanged original sources and frozen202 original IR;203 policy limits were confirmed directly with its owner.

### Recommendation and shared fanout

Deliver original Coracle `withdraw` first, using an explicitly audited payout composition domain and a small typed native-witness/execution-identity boundary. The common `ownPublicKey` leaf and observed execution configuration are needed by all three candidates; qualified Cell reads, typed authentication and send composition then cover `withdraw`, support later `concede`, and prepare `cash_out`. Do not combine all three original entry points in the first slice.

| Original entry | Result and existing control flow | New composition over203 | Persistent offer |
| --- | --- | --- | --- |
| Coracle withdraw | Outer secret witness; red/blue authentication; selected red/blue helper; winner assertion; two full qualified sends; nested WithdrawnCoins result | Bytes32 secret witnesses, native ownPublicKey twice, Bytes32/enum/qualified Cell reads, assertions, nested typed composite helper graph | Two contract inputs and two user outputs. Canonical205 needed when normalized output order reverses source wager/deposit order. No transients or change. |
| Coracle concede | Outer authentication; selected helper with secret+Board witness, honesty/turn/death assertions; write winner state; full deposit send; return sent coin | Above plus Board/commitment helpers and enum write in an actionful composite-return helper (many read/assert leaves overlap193) | One contract input/output; no change or transient |
| microDAO cash_out | ownPublicKey; lazy final/beneficiary/majority assertion; full pot send; reset_state(true); return sent coin | Above plus Counter reads/lessThan, Maybe public-key Cell, actionful reset composition and true pot-clearing branch | One contract input/output; no change or transient |

203 is deliberately insufficient by itself. Its owner confirms `CompositeDomain::ShieldedSend` admits ordered Expression Sequence/Let/If, pure audited calls, actionless stateful Expression helpers, qualified input/output and Kernel leaves, checked u128 subtraction and the exact nonce chain. It explicitly excludes every Cell/collection access, declared/native witness, Action and ReturnPlan helper. Do not claim original withdraw closure merely because sendShielded works.

### Native witness and execution identity boundary (highest shared fanout)

The original IR contains `NativeWitnessCall { builtin: OwnPublicKey }`, distinct from declared witnesses and pure calls. Native lowering already:

1. Reads `CircuitContext::own_coin_public_key()` (caller-configured public key; None returns MissingCoinPublicKey).
2. Appends exactly one `AlignedValue::from([u8;32])` to the private transcript at that evaluation position.
3. Returns the exact typed `ZswapCoinPublicKey { bytes }`, without changing private state or adding a public query/gas charge.

The recorded typed Plan currently lacks this leaf. Generated observed-call helpers also create contexts from `ObservedContractState::circuit_context`, which has no execution coin key. Treating ownPublicKey as a pure value or as a user-defined witness would lose the native private transcript binding/order.

Recommend a clean runtime leaf `RecordingFrame::own_coin_public_key(self) -> Result<(Self,[u8;32]),CompactError>` following the native behavior, then one typed Plan leaf that wraps the bytes using `NativeWitnessBuiltin::OwnPublicKey.result_type()`. No emitted handwritten VM and no cloning of arbitrary Private state. A missing key must fail before appending its private output, at the same position after earlier source effects/witnesses as native execution. The leaf alone creates no public query and must not make an otherwise nonproof/no-query export proof-eligible.

Add an optional typed caller-supplied execution coin key to observed construction, e.g. `ObservedContractState::with_coin_public_key(self, CoinPublicKey) -> Self`, set before wrapping the observation in OfferBackedObservedState. Context creation copies it through the existing context setter. This avoids extra generated per-circuit parameters and works with existing exact/canonical/wallet/transient offer policies. Keep this execution configuration distinct from public observed ledger state/Observation finality metadata and from WalletFundingInputs.

**Identity is not ownership authentication.** The key is the caller's selected execution identity/recipient value. It does not prove control of a wallet secret key and must not authorize wallet funding or select wallet inputs. Coracle authorization remains the source's local secret-key hash checks; it permits the authorized player to choose the outgoing public-key recipient. MicroDAO additionally compares the supplied key to the stored beneficiary. Upstream offer proofs establish spend/output validity separately.

Seal the execution identity against public mutable-context bypasses. Capture initial/final identity in private recording metadata alongside existing intent snapshots; preparation verifies it agrees with the observed execution configuration and final context. This prevents constructing a recorded call under keyB, replacing only its public execution context with keyA and presenting it as keyA-bound. Do not serialize the execution configuration as a fabricated ledger query. Keep declared secret witnesses and builtin outputs in their proper private order. Include direct RecordedCall::new misuse tests, not just generated facade success.

Because new generated code will call a new runtime recording method, **bump the centrally assigned runtime ABI** (likely49 if still free); schema20 should remain. Additive consumer-only205 did not require a bump, but this emitter→runtime requirement does. Do not contort the implementation around call_local merely to avoid the ABI release. Root should assign the exact next number after203/204 dependencies are settled.

### Bounded original withdraw profile

Use the actual203 implementation after signed handoff. Retain shared Plan evaluation/declaration dispatch and existing argument-once/fresh-callee scope/active-call guard. Add a separately audited payout domain; do not globally loosen ShieldedSend or turn on a union of unrelated profile flags.

Admit:

- Root `StateReturn::Expression` and actionless Expression helpers with source Let/Sequence/If/Assert and scalar or recursively typed composite results required by WithdrawnCoins/ShieldedCoinInfo/ShieldedSendResult/Either/Maybe.
- Root Bytes32 secret witnesses with exact declarations/signatures; NativeWitness OwnPublicKey with exact result shape and private output semantics.
- Declared root Cells of Bytes32, the original enum shape and canonical QualifiedShieldedCoinInfo, validating exact physical slot/type. This slice has **no Cell writes** and no Counter/Set/Merkle operations.
- Existing203 send leaves, nonce/hash/arithmetic bounds and pure-helper auditing. Authentication helpers use existing persistentHash over typed tuples/Bytes32; no new pure arithmetic/hash algorithm.
- Both branches, unused locals, helper arguments and the full reachable declaration graph audited. Reject actionful helpers, unknown/missing/ambiguous/cyclic calls, hidden unsupported effects, wrong typed Cells/native witness types and all lexical-scope escapes. No source-path/helper-name identity gate or arbitrary exact operation count.

Where a primitive already exists in the shared Plan, add only the explicit profile admission needed; do not create another evaluator or copy the203 nonce matcher. Prefer reusable value/effect permission helpers beneath the distinct profile audits if needed; inspect actual203 layout first. If qualified Cell read type support is missing in the shared leaf, add its exact canonical type, not arbitrary struct Cells. No newly admitted reset/Counter action belongs to this first profile.

Original source behavior must remain exact:

- Outer `player_is_red` and `player_is_blue` are both evaluated before authentication and selected helper dispatch.
- Selected helper reads a fresh local secret again. Each full send evaluates its qualified Cell input, ownPublicKey recipient, and separate Cell value argument in source order.
- A successful selected withdraw is expected from source inspection to consume two declared secret witnesses, two ownPublicKey native witness outputs and four input/output Unit private outputs. Measure/pin actual order; do not hard-code a proof eligibility count from this expectation.
- Both sends consume full input values, so change is absent. Still audit the complete standard-library send helper branch tree; do not remove its unselected change branch from admission.
- No public Cell is cleared by original withdraw. Replay protection is the real ledger nullifier state, not an invented contract state mutation. Both state and behavior must stay unchanged.
- If one secret authenticates both stored red and blue identities, the source's first red branch wins; retain the existing193 dual-identity style control. Authentication identity and outgoing coin public key are separate test values.

### Smallest meaningful validation

No captures/keygen until approval. Reuse original Coracle source/known helpers and193 setup utilities where applicable.

1. Independent pinned TS/native/recorded cases for red and blue winners; nonplayer, wrong winner/phase, changed secret in selected helper; missing execution key; both-authenticated branch choice; distinct output key; witness failure and gas boundary. Compare exact returned nested coins, unchanged complete contract data, effects, operation sequence/query gas, source input/output order, all private output order and successful replay. Missing-key/error prefixes must match native TS execution where comparable; failed Rust context is consumed, so do not claim returned partial state.
2. Native-witness boundary tests: typed key encoding, repeated key reads produce repeated private outputs at the original positions, interleaving declared witness/Kernel calls, missing key before output append, wrong builtin/result types, explicit no-query preparation/contract-info applicability boundary, and sealed initial/final identity mismatch through public RecordedCall construction.
3. Renderer negatives for Cell owner/type/path, hidden Counter/Set/writes, unused binding/unselected branch effects, ambiguous/missing/cyclic helper declarations, caller/branch/sibling scope escape. Preserve203's standalone refusal of added Cells/witnesses and194's original false-only reset policy.
4. Actual original withdraw proof keys; two distinct real pre-state contract-owned coins (pot and selected deposit), canonical205 complete offer, two user outputs matching returned wager/deposit, separate Dust. At least one strict red and one strict blue application with opposite commitment sort orders; independent verification and changed-output/private-binding refusal; exact nullifiers and both output commitments/owners/frontier; source order preserved; unselected deposit remains unspent. The contract state remains source-identical after application. Repeat the transaction and require exact upstream nullifier rejection.
5. Explicit wrong qualified index/coin/owner/nullifier and missing/extra offer outputs must fail before successful preparation. Use205 canonical policy; never append wallet change or transient selection. Seeded game/prize/deposit state is an offline test prerequisite, not proof of game-start/funding lifecycle or network finality.

### Why concede and cash_out follow separately

Concede is a good second composition after withdraw: it reuses193's honesty/turn checks and Board witness but needs an actionful ShieldedCoinInfo-returning helper with enum mutation. Apply202's existing extracted-return continuation only where relevant; do not assume203's actionless inline_call handles its action list. Exact failed auth/turn/alive/missing-key prefixes and winner-state-before-send semantics need independent evidence. One strict payout for each player is sufficient initial proof scope; original board/state game lifecycle remains seeded.

Cash_out composes send with194's reset helper but invokes literal true, which194 deliberately did not admit. It needs its own accepted branch policy and Counter/Maybe/qualified/Boolean/opaque-string reset scope. Preserve lazy majority assertion order; unlike advance it does not compute no+1. Probe roundMAX reset failure after send intents instead of assuming arithmetic wrapping. Successful application clears pot/pot_has_coin, resets voted structures, increments round, and returns sent. A future cash_out slice must retain194 advance's literal-false restriction; do not weaken that original audit globally.

Neither candidate requires204 transients: they spend previously qualified full-value coins.205 provides persistent multi-output order reconciliation for withdraw. Shared emitter/runtime leases still belong to203/204; this document authorizes no edits or ADR creation.

### Evidence locations

- Original Coracle source lines250–355; microDAO source lines187–218.
- Frozen original IR: `${LOCAL_EVIDENCE}/compact-focused-adr202-final/compiled/{coracle,micro-dao}/contract/compact-rust-ir.json`.
- Extracted read-only helper specimen: `${LOCAL_EVIDENCE}/compact-original-red-withdraw-ir.json`.
- Native own key lowering: `tools/compact-rust-backend/src/stateful.rs:952–969`; builtin typed result in `ir.rs:521–...`.
- Existing execution key API: `runtime-rs/src/context.rs:400–408,567–573`; call_local rejects identity changes in `recording.rs:179–203`.
-205 receipt `${LOCAL_EVIDENCE}/compact-adr205-delivery-receipt.json`;203 policy constraints confirmed by its owner in task messages. No new source compilation was required for this research.


### Independent preparation — 2026-10-06

Implementation branch `codex/adr206-coracle-withdraw` starts from integrated ADR205 `8e7080f5`. ADR204 retains transaction.rs and shared emitter ownership. No edits to those files or ABI fixture regeneration have occurred.

- New `RecordingFrame::own_coin_public_key` follows native getter semantics and appends exactly one bytes32 FAB private output, with no public VM query or gas/private-state mutation. Private trace metadata stores initial and final execution keys.
- Four focused runtime tests pass: interleaved declared/native outputs with Kernel query cost unchanged, missing key, sealed snapshot persistence after public context replacement, and successful key-only execution followed by exact `PrepareCallError::EmptyTranscript`.
- Original TypeScript/native Rust capture: 27 cases, 4 successes and 23 expected failures. Successful red, blue, distinct recipient and dual-auth red preference use 14 queries and eight private outputs `[secret, secret, key, Unit, Unit, key, Unit, Unit]`; all contract Cells remain byte-identical. Inputs/outputs retain source order. Upstream set iteration order is compared as sets, not promoted to query-order equivalence.
- Missing key has two prior secret outputs and five completed TypeScript queries. TypeScript fails at `undefined.bytes`; Rust preserves its existing `MissingCoinPublicKey` error. This is position parity with distinct native error representations.
- Original unchanged withdraw ZKIR key generation completed at k17 / 92,206 rows; prover approximately 37 MiB. This is baseline original circuit complexity, not emitter-induced growth.
- Proof harness is prepared but deliberately unhooked until the new recorded method and observed identity binding exist. No proof or strict ledger acceptance is claimed yet.

Artifacts: `${LOCAL_EVIDENCE}/compact-adr206-withdraw-ts.json`, `${LOCAL_EVIDENCE}/compact-adr206-capture-withdraw.mjs`, `${LOCAL_EVIDENCE}/compact-adr206-capture-provenance.json`, `${LOCAL_EVIDENCE}/compact-adr206-native.log`, `${LOCAL_EVIDENCE}/compact-adr206-recording.log`, `${LOCAL_EVIDENCE}/compact-adr206-keygen.log`, `${LOCAL_EVIDENCE}/compact-adr206-proof`.

Failed attempts are retained: the first runtime test compile shadowed its context helper; the first native support compile omitted a u64→u128 conversion; the first native parity run exposed only differing nullifier-set serialization order. These were corrected before the passing receipts.

Pending: generic/observed/offer identity preparation checks and mutation negatives; separately audited payout compiler domain over actual signed ADR203/204; ABI49 fixture refresh; recorded parity, malformed/scope negatives; strict two-input/two-output proofs with opposite normalized orders and exact nullifier replay refusal.


## ADR206 amendment proposal — explicit whole-fallible persistent offer placement

### Observed boundary
The unchanged original Coracle withdraw call proves and verifies (4480 bytes, changed binding rejected), but default-strict ledger validation rejects `NullifiersNEClaimedNullifiers`: retained offer input nullifiers are in logical segment 0, whereas upstream partition placed the call claims in logical segment 1. The current `OfferBoundPreparedCall::into_transaction` always supplies `Some(offer)` as guaranteed coins and constructs its intent at1. Failure log: `${LOCAL_EVIDENCE}/compact-adr206-proof-first.log`.

This is upstream cost-based transcript partitioning, not a reason to change source assertions or fabricate checkpoints. The current generated program has no checkpoint, so it partitions as a whole; the proposed preparation guard will explicitly verify the actual prototype is wholly fallible.

### Bounded API
Add `OfferPlacement::{Guaranteed (default), Fallible(NonZeroU16)}` to existing composable `OfferBindingOptions`, with a consuming `with_offer_placement` builder. Retain the chosen placement privately in bound observation and prepared offer. Existing constructors/default guaranteed placement and all199/204/205 policies remain unchanged.

For the new fallible option only:

- Require canonical allocation, nonempty native intent plan, no wallet funding selection and no transients. The complete retained offer must contain only this one persistent call's exact inputs/outputs under the existing188/205 bijection and ownership/index checks.
- Validate every full input public return vector equals `[Fr(1), Fr(segment)]` and every full output return vector equals `[Fr(segment)]`, using pinned ledger8 shapes. Reject wrong/missing/extra/overflow tags and mixed segments. `segment()` alone is insufficient because malformed conversions may map to None. These checks are policy binding; final cryptographic proof/ledger validation remains authoritative.
- Callers construct upstream proofs with `Some(segment)` or explicitly create a retargeted offer before binding. Runtime keeps the entire caller-retained offer unchanged and never retargets it.
- After canonical replay/partition, require `guaranteed_public_transcript == None` and `fallible_public_transcript == Some`. Reject mixed/guaranteed programs for this first option, including any guarantee-prefix state/query/index use.
- `into_transaction` installs one intent at the selected nonzero segment, no guaranteed Zswap offer, and the exact retained offer at `fallible_coins[segment]`. No additional segment/offer composition or inference is supported.

### Allocation and failure semantics
Canonical indices are the upstream allocation map from the observed prestate for this sole persistent offer. With no guaranteed Zswap offer and no other fallible segments, actual successful allocation agrees with that map. Source query/private/intent order and logical cursor remain unchanged. The strict harness must assert that separate Dust balancing preserves the one-offer placement and does not add Zswap outputs.

On full success, both claimed inputs are spent and both user outputs are allocated at the canonical indices. On a rejected fallible phase, upstream returns partial success if the guaranteed fee/replay phase succeeded: Dust/guaranteed bookkeeping may persist, while fallible Zswap/contract changes roll back. Test that boundary against pinned ledger rather than claim transaction-wide atomicity or assume indices were committed. Original withdraw itself writes no contract Cells.

### Validation
- Preserve exact old default segment 0-vs1 rejection as a negative control; do not label its valid call proof as accepted transaction.
- Runtime public constructor/prepare tests: wrong/mixed/extra/missing preimage tags, zero via NonZero rejection, wallet/transient/noncanonical/empty-plan restrictions, whole-guaranteed or mixed prototype refusal, and unchanged default guaranteed behavior.
- Strict red and blue full-value withdrawals, opposite output sort orders, inputs/outputs constructed with `Some(1)`, all call and Zswap proofs, separate Dust, unchanged original Cells and untouched third deposit, actual leaf owners/nullifiers/frontier and exact replay refusal.
- Pinned failed fallible application probe: actual ledger outcome, no spend/allocation/contract-state publication, and explicit retained guaranteed fee/replay effects.

ABI49 already reserved for the new recording leaf; schema20 unchanged. This amendment adds opt-in consumer runtime policy and does not add generated methods or change source semantics.

Independent read-only review by source-bridge agent agrees with the whole-fallible restriction, exact tag shapes, unchanged offer retention and sole-segment allocation contract. Root approved this amendment before implementation. Also test a non-1 segment to bind both the physical intent key and sole offer key to caller-selected n without an extra expensive proof.


### Implementation progress and pinned boundary corrections

Shared lowering and runtime implementation now pass all 27 TypeScript/native/recorded scenarios; runtime identity tests7, feature-enabled runtime library controls27, backend32+13+153, four-source cross-tab9/11 and strict Clippy. ABI49 fixture refresh covered175 fixtures; final freshness check is running against the frozen compiler.

The segment7 constructor/transaction test is structural: it verifies retained offer identity and equal selected physical call/offer keys, while the original segment1 proofs establish ledger acceptance. Public preparation separately rejects empty plans and wholly guaranteed programs.

Physical intent keys are nonzero. Upstream fee balancing adds a distinct non-1 Dust-only intent; logical phase0 is guaranteed execution, not physical intent0. The harness requires that separate intent to have no contract actions/unshielded offers, no guaranteed shielded offer, and exactly one fallible shielded offer at1.

Two negative classes are distinct: the original independently constructed segment0 offer with fallible claim transcript failed NullifiersNEClaimedNullifiers; relocating already-proven segment1 coins into guaranteed placement fails earlier as Zswap(InvalidProof), because those proofs bind their segment. Both logs are retained; no source claims or proof tags are silently changed. Strict final proof/rollback checks are in progress.

### ADR206 signed local delivery

Commit: `556ef73e9f3ca7943f12a8e81a3a9a4685018c86` (GPG verified, DCO signed), based on integrated `a7e14034`. No push or remote CI.

Original Coracle `withdraw` now has complete recorded/observed APIs. The bounded payout domain reuses the typed planner and qualified-send leaves. `RecordingFrame::own_coin_public_key` preserves the native private output order; raw/observed/offer preparation seal the initial and final execution identity. This is recipient/execution configuration, not wallet authentication. Runtime ABI **49**, schema **20**.

Before, the original native function was available but this complete recording was refused. The actual consumer shape is now:

```rust
let options = OfferBindingOptions::default()
    .with_output_allocation(PersistentOutputAllocation::CanonicalOfferIndices)
    .with_offer_placement(OfferPlacement::Fallible(NonZeroU16::new(1).unwrap()));
let bound = OfferBackedObservedState::with_options(
    observed.with_coin_public_key(recipient), &ledger, segment_one_offer, options)?;
let recorded = ledger_contract::recorded::withdraw(
    bound.observed().circuit_context(private), &witnesses)?;
let call = RecordedCall::new(bound.observed(), recorded, "withdraw", ());
let prepared = bound.prepare(call, verifier, Fr::from(0))?;
```

The explicit fallible policy requires canonical allocation, exact full upstream segment vectors, one nonempty persistent offer/intent plan, and a wholly fallible transcript. It rejects wallet funding, transients, mixed partitions and extra shielded offers. Existing default guaranteed policies remain unchanged. A structural segment-7 test verifies call/offer physical keys and unchanged offer retention; it is not a segment-7 proof.

#### Evidence

- **27** original TypeScript/native/recorded scenarios; complete state, effects, gas, query replay, raw intent order and eight private outputs on successful calls.
- Runtime identity **7** tests and feature-enabled runtime library **27** tests; includes mutable-context bypasses, exact tag vector/policy rejection, public empty/guaranteed refusal, and existing default/funding/transient/canonical controls.
- Backend **32 + 13 + 153** tests; malformed declarations, hidden effects, branches and lexical scopes stay bounded.
- Four-source real compiler cross-tab/Cargo gate: **9/11** proof exports recorded. Original Coracle `start` and `concede` remain explicit gaps.
- All **175** generated fixtures fresh; strict affected-package Clippy and standalone runtime check pass.
- Both original red and blue full-value withdrawals pass cryptographic proof verification and **unchanged default-strict ledger validation/application**, with real two-input/two-output offers and opposite normalized output orders. Each call proof is **4480 bytes** (original ZKIR k17 / 92,206 rows). All original Cells remain unchanged, the third deposit stays unspent, actual indices/owners match, and replay rejects `NullifierAlreadyPresent`.
- Changed public game state produces exact `Transcript(Execution(ReadMismatch))`. The fallible coins/contract state roll back; guaranteed Dust and replay bookkeeping persist. Logical phase0 is guaranteed execution; physical intent keys are nonzero. Separate Dust uses its own non-1 intent, with no contract actions or extra shielded offer.
- The original independently built segment0 offer failed `NullifiersNEClaimedNullifiers`. Moving already-proven segment1 coins into guaranteed placement instead fails earlier with `Zswap(InvalidProof)`. Both failed logs are retained separately.

Prior game/coin state is explicitly seeded offline; this proves withdrawal admission, not a full funded game lifecycle, network acceptance or finality. The full debug recorded test requires an 8 MiB worker; the proof harness uses its existing 64 MiB worker.

Receipt: `${LOCAL_EVIDENCE}/compact-adr206-delivery-receipt.json` (commit, exact file/CLI/key hashes and logs). Final strict log: `${LOCAL_EVIDENCE}/compact-adr206-strict-final.log`; source gate: `${LOCAL_EVIDENCE}/compact-adr206-focused-final/receipt.json`; original negative: `${LOCAL_EVIDENCE}/compact-adr206-proof-first.log`. Pinned CLI `${LOCAL_EVIDENCE}/compact-adr206-compactc`, reusable original keys `${LOCAL_EVIDENCE}/compact-adr206-proof/keys`.

Final exact implementation run completed successfully (exit0). Signed receipt SHA-256: `f7cefaf23f5f321b709472d9a53eb85355be0fc01b507d4233c49b2b1209cf05`. Root integrated the signed delivery as `e9ef0535` while preserving ADR208. Shared emitter/runtime lease released to ADR207; no more implementation edits.


### Original withdrawal accepted locally — 5806c197 (2026-10-06)

ADR206 / #310 is integrated as `e9ef0535`, followed by signed/DCO `5806c197` correcting the Python cohort expectation to include withdraw. Schema 20 / ABI 49. Root passed 27 runtime library controls, seven identity tests, backend 32 + CLI 13 + renderer 153 tests, six focused source fixtures (13 commands; 11 of 13 proof-required APIs recorded), strict four-package Clippy and actual default-guaranteed send/canonical/transient proof controls. All 34 Python tests pass at the follow-up commit; the original stale-expectation failure log is preserved. The follow-up changes only that test expectation.

The whole-source inventory at implementation head reports **378 of 384 proof-required APIs available**, six explicit gaps: Coracle start/concede and microDAO vote_commit/set_topic/buy_in/cash_out. All 216 sources / 753 exports / 195 compiled roots retain zero unassessed, missing metadata or unmatched exports, with no baseline drift.

Original-source delivery `556ef73e` passes all 27 corrected-TS/native/recorded/replay cases and both strict original withdrawal paths: red/source-normal and blue/reversed output ordering; 4,480-byte call proofs and actual two-input/two-output upstream proofs; unchanged Cells, untouched other deposit, canonical owner/index/value, separate Dust, default-strict application and nullifier replay rejection. The precise negative after relocating a proven segment-1 offer is `Zswap(InvalidProof)`; the separately retained original segment-0 offer/call mismatch is `NullifiersNEClaimedNullifiers`. On concurrent state change, actual fallible `Transcript(Execution(ReadMismatch))` rolls back coin changes while preserving guaranteed Dust/replay effects. Physical Dust intent is nonzero and separate; guaranteed logical phase is 0.

- Consolidated follow-up receipt: `${LOCAL_EVIDENCE}/compact-5806c197-integration-receipt.json`
- Root implementation receipt: `${LOCAL_EVIDENCE}/compact-e9ef0535-integration-receipt.json`
- Focused gate: `${LOCAL_EVIDENCE}/compact-focused-e9ef0535/receipt.json`
- Inventory: `${LOCAL_EVIDENCE}/compact-e9ef0535-inventory.json`
- Original proof delivery: `${LOCAL_EVIDENCE}/compact-adr206-delivery-receipt.json`, SHA-256 `f7cefaf23f5f321b709472d9a53eb85355be0fc01b507d4233c49b2b1209cf05`

The last full/portable checkpoint remains a7e14034; these later focused/source receipts are stated separately. The generated withdrawal debug worker still needs 8 MiB; measured private-frame storage work is being proposed as ADR210 rather than treating this as fixed. Compiler structural-test stack behavior is a separate issue. No full funded game lifecycle or current-head live-wallet claim. User documentation preserved; no push or remote CI. ADR207 merge implementation and ADR209 concede preparation continue in isolated worktrees.
