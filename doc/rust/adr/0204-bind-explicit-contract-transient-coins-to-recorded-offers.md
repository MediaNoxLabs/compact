---
id: RUST-ADR-0204
alias: ADR-0204
title: "Bind explicit contract transient coins to recorded offers"
date: 2026-10-06
publication_date: 2026-10-07
decision_status: "accepted-runtime-policy"
topics: ["transient", "Zswap", "offer-binding"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: 758620791eea6aaf3781e762dc8d3f36fcf2e3e2aee54db0364dd293acf56fc4
---
# RUST-ADR-0204 — Bind explicit contract transient coins to recorded offers

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-runtime-policy. An explicit selected transient carrier supports the first receive-to-full-immediate-send flow with actual wallet funding and exact normalized offer coverage. It does not silently admit merge, partial immediate send or arbitrary transient composition.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#308 closure](https://github.com/MediaNoxLabs/compact/issues/308#issuecomment-6017751774). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`a7e14034`](https://github.com/MediaNoxLabs/compact/commit/a7e14034f6153212a356b47a1be73c72182e4fa5). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
status: delivered
date: 2026-10-06
```

## Historical decision and amendments

### Problem and source evidence
The existing exact/ADR0199 wallet-funded offer policies reject transient offers and validate every circuit input against the historical ledger tree. The unchanged standard library uses a different domain: receiveShielded creates a same-transaction output; sendImmediateShielded passes that coin through upcastQualifiedCoin with mt_index=0 into sendShielded. The zero is the upstream singleton proof-tree index, not historical ledger membership. Original microDAO vote_commit has this receive→immediate-send sequence; occupied pot merges and Coracle start are related future uses.

Pinned midnight-zswap8.0.3 Transient::new_from_contract_owned_output consumes a real Output, constructs a singleton leaf0 tree, and builds a real input proof. Transient retains both complete proofs, commitments, nullifier and owner. Offer::new sorts ordinary outputs and transients separately; ledger try_apply allocates ordinary outputs first and then transients, advancing first_free for both. Current CircuitZswapPlan keeps separate input/output arrays and cannot prove their cross-kind source order.

Research: [Generated Rust — Transient offer reconciliation research — 2026-10-06](references.md#private-note-05). Dependency: candidate ADR0205 owns the reusable authoritative commitment→index mapping/count/API contract. ADR0203 owns qualified historical-input send lowering.

### Scope decision
First delivery covers **receive→full-value immediate send only**, using a same-contract transient in the guaranteed segment, a genuine explicit wallet input, a normal final output, frontier above1 and separate Dust fees. Merge reuse is follow-up and is not required to close this ADR. No blanket original microDAO/Coracle export closure claim. Existing exact and with_wallet_funding policies keep their current transient refusals and validation behavior.

### Before / after
Before, a valid native source flow can accumulate output/input intents but offer preparation fails TransientsUnsupported or historical InputIndexMismatch:

```compact
receiveShielded(disclose(coin));
return sendImmediateShielded(disclose(coin), disclose(recipient), disclose(coin.value));
```

After, a consumer explicitly binds the actual upstream transient and wallet inputs in a typed offer-backed policy. Illustrative API names are provisional until205's API contract is approved:

```rust
let received = Output::new_contract_owned(&mut rng, &coin, None, address)?;
let transient = Transient::new_from_contract_owned_output(
    &mut rng, &coin.qualify(0), None, received,
)?;
let funding = WalletFundingInputs::from_inputs(wallet_inputs)?;
let transients = ContractTransientCoins::from_transients(vec![transient])?;
let bound = OfferBackedObservedState::with_transient_coins(
    observed, &ledger, complete_normalized_offer, transients, Some(funding),
)?;
let call = generated.recording().receive_then_send_call(bound.observed(), private, coin, recipient)?;
let prepared = bound.prepare(call, verifier, randomness)?;
```

Generated wrapper availability must be demonstrated from the unchanged standard-library calls. A lower-level primitive fixture can isolate runtime behavior but cannot be represented as completed wrapper support.

### Runtime/domain decision
Retain selected complete upstream Transient<ProofPreimage,D> objects in a private immutable carrier. Require nonempty exact full-object offer membership, same observed contract, unique commitments/nullifiers and guaranteed-segment compatibility. Do not infer funding or transients from absent owners, hash-only identity, or mt_index0.

Add private ordered Input(i)/Output(i) provenance to the existing CircuitZswapPlan; no additional evaluator. Append events only with successful corresponding effects. Seal and compare event order with the public recording's initial/final plan; preserve it across context/constructor conversions and call_local checks. Each transient requires one preceding output to self with matching coin/commitment and exactly one later input with singleton index0. Historical inputs retain actual tree/index/owner/nullifier validation, including genuine ledger index0.

Reconcile an exact disjoint union of historical contract inputs, ADR0199 explicit full wallet inputs, and selected transient input/output pairs. No extra/missing output, implicit wallet input, duplicate/nullifier collision, unmatched transient, empty plan, or changed observation/allocation is admitted. Preserve opaque retained-offer transaction ownership and all upstream claim multisets.

**Reuse ADR0205 mapping; do not add a competing allocation mode.** Its mapping must bind the policy-approved union of ordinary and transient commitments to the exact try_apply indices while execution events remain in source order. Cursor/count and terminal frontier follow205's agreed contract. No runtime edits until205 is approved and its owner hands off the lease.

### Compiler and ABI decision
Reuse shared typed declaration/call/intent lowering after ADR0203; audit only the first full-send wrapper composition needed here. No Compact IR schema addition is currently necessary. Runtime-only proposal: **no additional ABI bump beyond any205 requirement** when the final implementation only adds an explicit consumer API and private provenance, preserving generated create_zswap_input/output/RecordingFrame signatures and result layouts. Old generated crates remain valid against the new runtime; explicit new constructor use naturally requires the runtime revision containing it. Recheck the actual diff at205 handoff: if generated code requires a new runtime entry point/carrier/layout, reserve a new ABI before emission. This compatibility decision must be finalized and recorded before delivery, rather than inferred from the ADR number.

### Parity and proof policy
Raw TypeScript provisional source order and authoritative offer allocation are separate evidence. At frontier2, source outputs received→final receive provisional indices2→3; normalized ledger allocation gives final2 and transient received3. Preserve raw TS/native capture; independently replay its VM operations under the actual ledger map and compare offer-backed Rust and applied state. Do not rewrite TS indices or call raw wrapper state equal when it differs.

Prove both transient sides and the generated contract call, verify independently and reject changed bindings, prove/seal/balance with separate pinned Night-backed Dust, run WellFormedStrictness::default and apply. Assert real indices, frontier, output/contract state, wallet and transient nullifier consumption, and replay rejection. Existing exact/ADR0199 tests must remain unchanged and green.

Negatives: missing/extra/duplicate transient; changed proof_input/proof_output with unchanged hashes; wrong owner/coin/nullifier/index/segment; input-before-output or double consume; historical index0 misclassified as immediate; changed observation/map/final plan; unselected wallet input; canonical/source order disagreement; overflow before mutation; absent/duplicate claims; applied-state replay. Failed calls retain observable prefixes only.

### Delivery coordination
Create linked MediaNoxLabs rust-backend-v2 issue before source preparation. Independent source/TS/proof artifacts may proceed now. Shared runtime ownership remains ADR0205; shared emitter prerequisites remain ADR0203. No push or remote CI, no new Cargo cache workload during the active full gate. Strict proof artifacts must be stored separately from skip-zk output.


### Independent preparation — 2026-10-06

Issue: https://github.com/MediaNoxLabs/compact/issues/308. Signed/DCO checkpoint `0996502a0a14cd2b68457a03d375dae311c1ee09` (GPG verified), not final delivery. The unchanged standard-library receive→full immediate-send wrapper compiles natively, while recording remains explicitly unavailable at `callee[receiveShielded].actions[0] StateAction::Let`. Shared runtime/emitter are unchanged; dependencies remain ADR0203 and ADR0205.

Seven corrected-runtime TS/native cases pass: user/foreign/self recipient, zero, 2^64, u128MAX and a second nonce. Source intent order is output→input(singleton0)→output, three private outputs, logical frontier+2. Raw JSON preserves effects/query order. Ledger effect multisets compare sorted values with multiplicity retained because Rust upstream serialization sorts keys while JavaScript preserves insertion order; exact public op order remains a distinct later recording gate.

User/foreign gas: five queries/24ops; sum compute 5468247995, wrapper-last-query 1102262433, whole-program replay 1170302119. Self has six queries/30ops. No aggregate wrapper gas equality claim.

A pinned upstream construction/allocation test uses a genuine wallet input42, two historical seeded leaves, an actual full contract Transient42 and an ordinary final output42. Upstream try_apply gives final2/transient3/frontier4, while raw source native/TS gives received2/final3/frontier4. Both wallet and transient nullifiers are consumed and repeated offer application fails. This test is allocation evidence, not proof or default-strict transaction application.

**Constructor boundary:** upstream Transient::new_from_contract_owned_output also returns a preimage for wrongly qualified index1, with the same public singleton root/commitment/nullifier. A successful constructor is not validation. Keep explicit runtime intent index0 checks and require a later malformed-preimage proof rejection.

Targeted fixture freshness, exact recapture, both tests and strict package Clippy pass. Keys are preserved separately at `${LOCAL_EVIDENCE}/compact-adr204-proof/keys` (prover 9.5MiB), never overwritten by skip-zk output. Receipt `${LOCAL_EVIDENCE}/compact-adr204-preparation-receipt.json`; full fixture sweep and strict proofs await implementation. ADR0205 options/map/count contract is approved; reuse it after runtime lease handoff.


### Delivered — 2026-10-06

Signed/DCO preparation `0996502a0a14cd2b68457a03d375dae311c1ee09` and implementation `b36b2e8f47ddb4648a8fe5a34b7231bc7f00a2e5`; both GPG verified. Issue [#308](https://github.com/MediaNoxLabs/compact/issues/308), milestone `rust-backend-v2`. Dependency-only local cherry-picks are 3e3bf8f1 (202), f9f594fc (205), 13de44c3 and 341ebd17 (203); they are not additional204 deliveries. No push or remote CI.

#### Final architecture and API

`ContractTransientCoins::from_transients` retains complete upstream `Transient<ProofPreimage,D>` objects. `OfferBindingOptions::with_transient_coins` requires explicit `CanonicalOfferIndices`, composes explicit wallet funding, and validates full-object offer identity, same contract, guaranteed segment, uniqueness and disjoint coverage. Canonical rows reuse205 allocation for persistent and selected transient commitments; allocation never changes after observation.

Private `IntentEvent::{Input,Output}` entries preserve complete monotonic projections of typed intent arrays. Reconciliation requires each transient output-to-self before its unique singleton-index0 input, matching coin/commitment/nullifier. Historical inputs, including real ledger index0, retain ordinary tree checks. Constructor conversions and `call_local` compare the complete plan including events; event-only mutation is explicitly rejected. Default and wallet-only transient refusal and empty-plan behavior remain controls.

The compiler adds an explicit `ImmediateShieldedSend` domain and structural audit module; it reuses shared `Plan`, scopes, argument evaluation, pure-call auditing, nonce/subtraction/intent leaves. No new evaluator or source-name match. The exact receive prefix, same coin/recipient/full-value projection and pure singleton bridge are required. All hidden bindings/branches are audited. Partial amounts, changed projections, nonzero singleton bridges, extra effects and helper cycles refuse admission.

Before: native `contract.receive_then_send(...)` only. After: additionally `ledger_contract::recorded::receive_then_send(...)` and `contract.recording.receive_then_send_call(...)`, producing the existing typed recorded/observed call. Runtime methods emitted by the generated crate are unchanged. **ABI48/schema20 retained**: private event provenance and additive consumer policy require no new emitted runtime entry point.

#### Evidence and precise limits

All seven corrected-runtime TS/native/recorded cases preserve typed result, private output order, raw public VM operations, complete state/effects and replay. TypeScript raw provisional indices remain received2/final3; authoritative execution uses received3/final2. Independent public-program replay under the actual allocation map equals bound Rust state/effects. No raw TS indices are rewritten. Query sum5468247995, wrapper-last1102262433 and whole-program replay1170302119 remain distinct compute-time metrics for the five-query user/foreign cases; self has six queries.

A genuine wallet42 input plus full contract transient42 and final user42 output proves and applies with separate Night-backed Dust and `WellFormedStrictness::default`. Original-source contract proof is4480bytes and rejects a changed binding. Actual wallet/transient/output proofs pass; frontier starts2 and ends4, final leaf2/transientleaf3, both nullifiers consumed, replay rejected. Explicit offline seed is not a proved prior transaction history.

The upstream constructor also accepts a malformed singleton-index1 preimage. The negative now asserts the precise prover diagnostic `Public transcript input mismatch for input 13;`, while the valid index0 case succeeds using the same keys/funding. Constructor success is never treated as proof validity.

Validation: backend29unit+13CLI+153renderer; runtime25; Python34; seven fixture parity cases plus upstream allocation control; strict Clippy across backend/runtime/fixture/proof runner; workspace format; all175fixtures fresh. Exact signed-head focused gate covers4sources with9/9 proof-required recorded APIs,9commands; receipt `${LOCAL_EVIDENCE}/compact-adr204-delivery/receipt.json`. Strict proof artifacts stay at `${LOCAL_EVIDENCE}/compact-adr204-proof/keys`, separate from skip-zk outputs. Logs `${LOCAL_EVIDENCE}/compact-adr204-final-strict-proof.log` and `${LOCAL_EVIDENCE}/compact-adr204-exact-head-strict-proof.log`.

Merge/immediate partial-send and all original microDAO/Coracle exports remain separately scoped. Shared emitter/runtime ownership was handed to ADR206 after signed delivery;206 will independently add execution identity and ABI49.


Final exact-head receipt: `${LOCAL_EVIDENCE}/compact-adr204-delivery/final-receipt.json`, SHA256 `4e9ed22d5dd1c2908091ed4270c51395bbd8194cf30529d5b645dbca11a75373`. Clean signed head b36b2e8f; 9-command focused gate and exact-head strict proof/application rerun both passed. Receipt hashes compiler, proof runner, original source/TS fixture, keys/ZKIR and verification logs.

### Combined local acceptance passed — a7e14034 (2026-10-06)

The full local gate passed at signed/DCO `a7e14034f6153212a356b47a1be73c72182e4fa5`: **371 commands, 175 fresh fixtures**, source/format/refusal checks, workspace and generated tests, strict Clippy, external consumers and proof/ledger checks. The final consumer/proof/ledger stage took 1,064.424 seconds. Its fixture subset has 362 of 369 proof-required APIs available; the wider whole-source inventory has **377 of 384 available**, with the same seven explicit original-source gaps. These are different source sets, not conflicting totals, and neither is a behavioral completeness claim. No unassessed, missing compiler metadata, unmatched exports or baseline drift.

This checkpoint includes ADR202 terminal lexical returns, ADR203 qualified full/partial sends, ADR205 canonical output allocation and ADR204 explicit received transients. The full gate reran the new strict proof cases: full and partial qualified send, both canonical output orders, and wallet receive → transient → full immediate send. The last has final output index 2, transient index 3, frontier 4, complete actual upstream proofs, separate Dust, default-strict application and replay refusal. Historical unbalanced smoke cases retain their original limits. Prior application/coin setup remains seeded where stated.

A clean same-head Nix compiler package also passed 16 relocated archive verification commands on **aarch64-darwin**: default TS, strict Rust, bundled ZKIR keys, exact runtime sources, offline generated consumers, relative installer symlink, paths with spaces, unset runtime/Scheme overrides and no Nix linkage in six binaries. New Field/send/canonical/transient capability checks pass. Archive SHA-256: `6d58b97167faf8306cb2880e671a1450a2cb72e0884886881e93833f2ae9e99e`.

- Consolidated receipt: `${LOCAL_EVIDENCE}/compact-a7e14034-integration-receipt.json`
- Full receipt: `${LOCAL_EVIDENCE}/compact-full-a7e14034/receipt.json`
- Wider inventory: `${LOCAL_EVIDENCE}/compact-a7e14034-inventory.json`
- Portable receipt: `${LOCAL_EVIDENCE}/compact-a7e14034-portable-final/receipt.json`
- Archive: `${LOCAL_EVIDENCE}/compact-a7e14034-portable-final/compactc.zip`

All seven commits since the preceding full checkpoint have valid GPG signatures and DCO trailers. Main tree before/after is unchanged except the preserved user edit to `doc/ledger-adt.mdx`. Schema 20 / ABI 48 remain the integrated baseline. No branch push, publication, remote CI or current-head live-wallet claim.

#### Next delivery lanes

- ADR206 / #310: original Coracle withdrawal, typed execution coin identity and ABI 49. All 27 original recorded parity cases pass; call proof verifies. Strict ledger application exposed guaranteed/fallible segment mismatch. The approved explicit whole-fallible persistent-offer amendment is being tested; no completed strict withdrawal claim yet.
- ADR207 / #311: signed preparation `acf7b6b6` has 20 TS/native merge cases and two real upstream allocation tests. Keys are ready. Shared planner changes and strict proofs wait for ADR206 handoff.
- ADR208: four previously weakly exercised pure literal/hash/scalar exports are receiving direct independent boundary tests; this is behavior evidence, not a proof-API count increase.
- Research saved for per-export behavior evidence, final local production acceptance, original microDAO cash_out and original set_topic/start/buy_in dependencies.
