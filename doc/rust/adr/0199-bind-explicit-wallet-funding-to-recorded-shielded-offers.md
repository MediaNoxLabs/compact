---
id: RUST-ADR-0199
alias: ADR-0199
title: "Bind explicit wallet funding to recorded shielded offers"
date: 2026-10-05
publication_date: 2026-10-07
decision_status: "accepted-runtime-policy"
topics: ["wallet-funding", "Zswap", "offer-binding"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: 2c09670a89a09e7daff1bdb37bfdc73a3c365bf386e7352350e088181f2e88e6
---
# RUST-ADR-0199 — Bind explicit wallet funding to recorded shielded offers

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-runtime-policy. Explicit private wallet funding binds a complete retained input to exact recorded contract-owned intents and output coverage. The default exact policy remains unchanged; accepted proof cases use actual wallet input, strict fees and replay controls.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#304 closure](https://github.com/MediaNoxLabs/compact/issues/304#issuecomment-6017744876). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`305df36a`](https://github.com/MediaNoxLabs/compact/commit/305df36abb6730b8bb240cfaa46abf38af437a94) · [`34ae31b7`](https://github.com/MediaNoxLabs/compact/commit/34ae31b7fb5f87ec7b3fbd679b74ad1d4e0a8d10) · [`c6ff13c5`](https://github.com/MediaNoxLabs/compact/commit/c6ff13c5c2ad283733cc5a45b9ca1faa734217b0) · [`ee912835`](https://github.com/MediaNoxLabs/compact/commit/ee912835169d314851b99fc92c3e543ffaa2fd2e). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
status: delivered
milestone: rust-backend-v2
issue: https://github.com/MediaNoxLabs/compact/issues/304
schema: 20
runtime-abi: 48
commit: 13386bc31514101046643d1f62fcbd410f2d88a4
```

## Historical decision and amendments

## Explicit wallet funding for recorded shielded receive

### Problem

ADR195 records the unchanged receiveShielded helper, verifies zero and wide-value call proofs, and applies a zero-value contract output under default ledger strictness. A nonzero wallet-funded receive still fails `ZswapIntentError::InputMismatch`: ADR188 correctly requires every offer input to match a contract-owned input intent. Full microDAO/Coracle lifecycles require an explicit funding composition policy.

### Proposed bounded delivery (ADR0199)

Keep `OfferBackedObservedState::new` and its exact policy unchanged. Add an explicit typed wallet-funding envelope to a separate construction path. The envelope enumerates the exact authorized wallet input nullifiers from the wallet's actual upstream offer construction. Reconcile the entire retained offer against contract-owned input intents plus those explicit funding identities, with no unaccounted inputs, overlap, duplicates, or foreign-contract input substitution.

For this first slice, all outputs must still exactly match the circuit outputs and authoritative allocation; no wallet change, transients, output reordering shortcut, or relaxed ledger validation. A `None` input owner identifies public offer shape only; it is not evidence of secret-key authorization. Pinned upstream input proofs, balance checks and default-strict ledger validation remain authoritative. Keep the same immutable observation/offer binding through preparation and transaction assembly.

### Acceptance

- Existing default extra-input rejection remains tested.
- Unchanged receiveShielded with a real wallet-owned input and matching nonzero contract output: native/recorded/replay evidence, call and input/output proofs, default-strict ledger apply, contract commitment ownership/index and wallet nullifier consumption.
- Explicit Night-backed Dust fixture remains separate from shielded funding.
- Wrong/missing/extra/duplicate funding identities, contract-input overlap, foreign owner, altered observation/allocation, extra/change output and transient rejection.
- Applied-state replay rejection; no proof-only or zero-value substitute for funded acceptance.
- ADR with before/after consumer code, runtime invariants, ABI/version decision and exact local receipts. Any API names here are proposals until reviewed.

### Scope and evidence

Reuse ledger8 Offer/Input/Output/proof/state primitives and the shared recording runtime. No emitter shape expansion is needed for the receive wrapper. Design review precedes implementation. No remote CI, push or publication; part of rust-backend-v2 and parent#105. Source research: midnight vault Shielded standard-library delivery slices and generated recording composition review. ADR195/#298 is the preceding emitter slice.

### Before / proposed after

Before: a complete upstream offer with wallet input42 and matching contract output42 reaches `OfferBackedObservedState::new`, but preparing the unchanged receive call rejects InputMismatch because there is no contract input intent. The default behavior is correct for its documented exact policy.

```rust
let bound = OfferBackedObservedState::new(observed, &ledger, offer)?;
let call = contract.recording().accept_call(bound.observed(), (), coin)?;
// Existing exact policy: external wallet input is not a contract input intent.
let prepared = bound.prepare(call, verifier, randomness)?;
```

Illustrative proposed surface, not an implemented API:

```rust
let funding = WalletFundingInputs::from_authorized_inputs(wallet_inputs)?;
let bound = OfferBackedObservedState::with_wallet_funding(
    observed, &ledger, complete_offer, funding,
)?;
let call = contract.recording().accept_call(bound.observed(), (), coin)?;
let prepared = bound.prepare(call, verifier, randomness)?;
let transaction = prepared.into_transaction(&mut rng, network, ttl);
```

The actual constructor/name must make caller authorization explicit without implying that metadata proves secret-key ownership. An explicit list binds allowed identities; pinned input proofs and final ledger validation establish spend validity. No owner-None-only inference, subset matching, or boolean allow-extra-inputs mode.

### Emitter/runtime ownership

No new emission semantics are needed for the receive wrapper. Runtime transaction preparation owns the complete offer/envelope reconciliation; the existing recording frame owns ordered source effects and query transcripts; ledger8 owns commitments, nullifiers, input/output proof construction, allocation and balance validation. Existing exact constructor and tests remain.

### Open review decisions before implementation

1. Exact immutable envelope representation and construction from upstream wallet-authorized inputs; error variants for missing/extra/overlapping identities.
2. ABI compatibility: determine whether this additive consumer-side preparation API changes any generated contract requirement. Do not bump or omit a bump without recording the reason.
3. Scope remains one funded receive with all outputs from the circuit; wallet change, transients and normalized multi-output allocation are later ADRs.
4. Preserve ADR195 zero/wide proof and default wallet-refusal evidence. New funded acceptance gets separate artifacts and strict-policy proof/ledger receipt.

No implementation or successful funded receipt is claimed by this proposed ADR.

### Accepted representation and guards (2026-10-05)
Use a private-field `WalletFundingInputs<D>` carrier of upstream `Input<ProofPreimage,D>` values selected explicitly by the wallet caller, normally returned by `LocalZswapState::spend`. `OfferBackedObservedState::with_wallet_funding` receives that carrier with the complete retained offer and validates full Input equality against offer inputs, distinct nullifiers, and `contract_address == None`. This carrier records caller intent only; it does not prove secret-key ownership. The ordinary `new` constructor retains ADR-0188 exact behavior.

At prepare, the funded mode requires a nonempty circuit intent plan and must reject the empty verify-ops shortcut. For a nonempty plan, validate contract-owned inputs with existing owner/tree/commitment/nullifier rules; require a disjoint exact union of contract-intent nullifiers and explicit wallet-funding nullifiers equal to every complete-offer input, with cardinality equality and duplicate-offer rejection. Keep output allocation/order and transient rejection unchanged. Complete offer input proofs and default-strict ledger apply establish spend validity. No generated runtime API dependency changes: schema 20 and runtime ABI 48 remain. First delivery accepts one wallet-input42 to contract-output42 with separate Dust and no change/transients.

### Delivery receipt (2026-10-06)
Signed conventional DCO commit `13386bc31514101046643d1f62fcbd410f2d88a4` on `codex/adr199-wallet-funding` (GPG verified), integrated by parent as `34ae31b7`. No push or remote CI. Source schema20/runtime ABI48 are unchanged because generated receiver code and required symbols are unchanged; the consumer opts into an additive transaction constructor.

Five focused runtime transaction tests pass, including legacy exact behavior, empty funded plan, wrong/missing/extra/duplicate funding identities, foreign owner, altered upstream proof with the same public fields, contract/funding overlap, changed and extra output, transient and allocation rejection. The real pinned-ledger fixture uses upstream `LocalZswapState::spend` to fund input42, a generated recorded receive call, and a contract output42 at authoritative index1. Native/recorded state, effects and gas agree; replay state/effects agree and its distinct flattened-query gas boundary is asserted. Generated call proof verifies and changed binding fails (4480 bytes); separate Night-backed Dust and default strictness permit `ledger.apply`, which consumes the wallet nullifier. Replaying the same offer on applied state returns `NullifierAlreadyPresent`. The ordinary exact constructor continues to reject that offer with `InputMismatch`. Focused tests, fmt and strict Clippy pass on signed head; proof artifacts are under `${LOCAL_EVIDENCE}/compact-adr195-proof`.

This first funded profile has one wallet input and one circuit contract output; it does not authorize wallet change outputs or transients. The caller-selected envelope is an explicit assertion of funding identity. The retained upstream input proof and final ledger validation establish spend validity; owner metadata alone does not.


### Integrated explicit wallet funding — 34ae31b7 (2026-10-06)

ADR199/[#304](https://github.com/MediaNoxLabs/compact/issues/304) integrated as GPG/DCO-verified `34ae31b7`. `WalletFundingInputs::from_inputs` carries caller-selected upstream Inputs, validates full equality including proof preimage against the retained offer, and requires an exact disjoint union with circuit-owned inputs. This is selection intent; upstream proofs and ledger validation establish spend validity. The default constructor retains its exact policy. Empty funded plans, duplicates, changed proof/data, allocation mismatch, unaccounted inputs/outputs and transient offers reject. Schema20/ABI48 unchanged.

Main gate:5feature-enabled transaction unit tests and strict runtime/proof Clippy pass. A real upstream wallet spend42 funds the unchanged receiveShielded contract output42. Generated4480-byte call proof verifies and changed binding rejects; native/recorded/replay state and effects agree; separate Dust/default-strict ledger apply passes at output index1; spent nullifier is consumed and replay rejects NullifierAlreadyPresent. The original exact constructor rejects the same external input. `${LOCAL_EVIDENCE}/compact-34ae31b7-integration-receipt.json`.

The first runtime test invocation omitted the ledger-transaction feature and selected zero tests; preserved as non-evidence and corrected with a5-test feature-enabled run. Prior c6ff13c5 corpus count remains359/370 available,11gaps; runtime-only work does not add recorded APIs. Full/package checkpoint remains ee912835. No push, publication or remote CI. ADR201 integration waits for strengthened separate-Dust strict proof evidence;202 now owns the shared planner lease;203 shielded-send research starts.


### Full-gate funded receive hook — 305df36a

Signed/DCO `305df36a` now runs ADR199 wallet-funded acceptance directly after the existing receive proof in `check_compactc_target.py --shielded-receive --proof`, reusing the same source compile and proving keys. All34Python harness tests pass. The actual source-to-proof hook also passes: zero/wide call proofs and zero strict application retain their existing scope, followed by funded42/default-strict application and nullifier replay refusal. `${LOCAL_EVIDENCE}/compact-305df36a-proof-hook-receipt.json`. No broad gate claim from this focused hook.
