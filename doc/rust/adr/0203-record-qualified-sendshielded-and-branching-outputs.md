---
id: RUST-ADR-0203
alias: ADR-0203
title: "Record qualified sendShielded and branching outputs"
date: 2026-10-06
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["recording", "shielded-send", "qualified-coin"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: d37f935fd2d5b2a74672cf81da98a2afa532fcb303d92a476889ba82f0b62bb1
---
# RUST-ADR-0203 — Record qualified sendShielded and branching outputs

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Audited qualified sendShielded records historical input/nullifier, checked Uint128 subtraction and source-ordered sent/change outputs. Full and partial same-order strict cases apply; reversed order still rejects under the default exact policy unless ADR205 is explicitly selected.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#307 closure](https://github.com/MediaNoxLabs/compact/issues/307#issuecomment-6017749863). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`1515cc56`](https://github.com/MediaNoxLabs/compact/commit/1515cc5684c198e5aab6d01d4888c06f26c2f389) · [`7a311c9f`](https://github.com/MediaNoxLabs/compact/commit/7a311c9fe80ef588b744407fb39f0130ba6d352f) · [`8376be8e`](https://github.com/MediaNoxLabs/compact/commit/8376be8e145b852d9084cd0aa3f43158245bc1c9) · [`cc5ac0e2`](https://github.com/MediaNoxLabs/compact/commit/cc5ac0e27cf80daab179d7d4d10d5b15d6b8bcfa). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
type: adr
status: delivered-locally
date: 2026-10-06
milestone: rust-backend-v2
schema: 20
runtime-abi: 48
issue: https://github.com/MediaNoxLabs/compact/issues/307
```

## Historical decision and amendments

### Problem
The unchanged standard-library `sendShielded` helper and small self/user wrappers compile natively, but proof-required wrappers expose no recorded or observed-call API. The helper returns a value while sequencing one qualified input, its nullifier claim, checked u128 subtraction, nonce evolution, a sent output and spend claim, optional self-receive, and branch-local change output/claims. Source IR schema20 already expresses this as `StateReturn::Expression` with nested `Expr::Let`, `Sequence` and `If`; the typed recording Plan currently accepts only narrower Unit effect helpers. This blocks microDAO cash_out and Coracle concede/withdraw, whose other actions remain independent gaps.

### Before and after
The original source shape remains unchanged:
```compact
export circuit send_to_user(input: QualifiedShieldedCoinInfo, key: ZswapCoinPublicKey, value: Uint<128>): ShieldedSendResult {
  return sendShielded(disclose(input), left<ZswapCoinPublicKey, ContractAddress>(disclose(key)), disclose(value));
}
```
Before, generated native Rust returns ShieldedSendResult and strict recording reports `unsupported_return` on StateReturn::Expression. After this slice, generated `recorded::send_to_user` and typed observed call replay the same ordered effects and return ShieldedSendResult. Illustrative Rust flow:
```rust
let (frame, self_addr) = frame.kernel_self()?;
let frame = frame.create_zswap_input(input_qualified);
let frame = frame.kernel_claim(KernelClaim::Nullifier(nullifier))?;
let change = subtract_unsigned(input.value, value)?;
let frame = frame.create_zswap_output(sent, recipient)?;
let frame = frame.kernel_claim(KernelClaim::CoinSpend(sent_commitment))?;
let (frame, result) = if change == 0 { (frame, no_change(sent)) } else {
    let frame = frame.create_zswap_output(change_coin, self_recipient)?;
    let frame = frame.kernel_claim(KernelClaim::CoinSpend(change_commitment))?;
    let frame = frame.kernel_claim(KernelClaim::CoinReceive(change_commitment))?;
    (frame, with_change(sent, change_coin))
};
frame.finish(result)
```
The actual generated code also emits the conditional self-receive claim; the example shows only the branch ownership boundary.

### Decision: emitter and runtime
Extend the existing declaration-directed typed Plan for audited non-Unit stateful expression returns with empty top-level actions. Inline exact typed helper arguments once in a lexical scope; reject cycles and unknown nodes. Reuse current Let/Sequence/If, typed struct construction and the shared qualified input/output/kernel claim effect leaf. Add checked Uint128 subtraction through the existing shared unsigned arithmetic syntax, and typed Bytes32/Field transient degrade-hash-upgrade through existing runtime primitives. Pure identity/helper calls remain recursively audited as in ADR-0195; no helper-name intrinsic or duplicate coin codec. Conditions execute before branch effects, and only the chosen branch executes. Reject malformed result/parameter types, hidden ledger or witness effects, unsupported crypto shapes and branch type mismatch. Static syntactic effect totals must not be mistaken for runtime branch counts. No IR/schema or generated runtime ABI change is anticipated; record an exact version change if implementation proves otherwise.

The existing ADR-0188 exact offer binding remains: contract input commitment/index/owner/nullifier must match, every selected circuit output must match the authoritative offer order and index, and transients are rejected. This ADR does not relax normalized output order or admit immediate in-transaction sends/merges. ADR-0199 wallet funding is a separate opt-in path and is not used for qualified contract-owned send.

### Acceptance and honest boundary
Capture independently with the corrected ADR-0197 TypeScript u128 runtime, retaining package provenance and the historical ADR-0195 b8 capture separately. Test full send42/42, partial send42/17 with change25, and underflow42/43; self, foreign-contract and user recipients; renamed wrapper. Compare TS/native/recorded return, state, ordered program/effects, private Unit transcript, per-query gas and replay. Assert that input intent and nullifier claim precede the checked subtraction, underflow returns an error before any output and no prepared call exists, and nonselected change/self-receive branch effects never execute. Independently compare both nonce domains against pinned ledger transient crypto and CoinInfo commitments.

Prove and apply a real qualified contract-owned input42 to one output42 with separate Night-backed Dust at default strictness; check owner/index/nullifier consumption and applied-state replay rejection. For partial send, construct both outputs from independent canonical coin values and test a normalized-order-aligned strict offer if available; explicitly reject a reversed normalized offer without reordering source intents. Do not claim all partial sends settle until an indexed multi-output policy is designed and proved. Keep the full test-center unshielded-tokens source blocked by its independent `kernel.balanceGreaterThan` node; wrapper admission does not close Coracle or microDAO source cohorts. Focused TS, native, recorded, fixture freshness, source inventory, proof, fmt and strict Clippy must pass before a signed DCO delivery.

Research: [ADR-0203 — Qualified sendShielded source and proof boundary](0203-record-qualified-sendshielded-and-branching-outputs.md). The shared planner is leased to ADR-0201 then ADR-0202; only independent source/capture/proof preparation may proceed meanwhile. No push or remote CI.


### Local delivery — 2026-10-06
Signed DCO/GPG commits: corrected TypeScript oracle preparation `12b997d186def262e76dab244371d8bf34dc64d8`, typed emitter and full-send proof `d8618567c7c70d0100c2cc8be87d24c5884f1115`, partial-order proof and source inventory `08491fe0cc2372e1d65d12f0459d1a1ab70857fe`. ADR202 prerequisite was cherry-picked as `2ec5c186`; root integration assigns different commit IDs.

Four unchanged wrapper exports record under schema20/ABI48. Corrected ADR197/198/200 TypeScript capture is byte-identical on rerun; 10 rows compare complete returned coins, state, ordered public program, intent outputs, query/gas/replay and error boundary. Upstream `midnight_transient_crypto` independently matches both nonce domains; ledger CoinInfo commitments match captured vectors. Static audit rejects hidden qualified Set queries in unused bindings, unselected branches and arguments, and rejects impure/recursive/malformed helpers.

On exact signed head `08491fe0`, `${LOCAL_EVIDENCE}/compact-adr203-signed-head-proof.log` (SHA256 `73c3786c76e62dfd258804cf64bf92972bdd63a4cfaf941b0560208e46d865f8`) records full self42→42 and same-order partial42→17+change25 cryptographic call proofs, default-strict Night-backed Dust-funded ledger application and spent-nullifier replay rejection. The reversed normalized/source order receives exact `ZswapOfferOutputMismatch`. Partial fixture grants 10 additional Night-backed Dust units through pinned TestState only; strictness and actual offers are unchanged. An earlier one-unit partial balance panic is a failed fixture-capacity run, not acceptance evidence.

Focused checks passed: 4/4 source/API inventory with no unassessed export; one fixture freshness; 153 renderer tests; native/recorded/replay tests; strict Clippy on backend, fixture and proof crate. The original test-center unshielded-tokens contract remains blocked by independent `kernel.balanceGreaterThan`; immediate/transient composition and alternative partial output orders are separate ADR204/205 boundaries. No remote CI, push or publication.

### Qualified shielded send integrated — 1515cc56 (2026-10-06)

ADR203/[#307](https://github.com/MediaNoxLabs/compact/issues/307) is integrated as signed/DCO commits `cc5ac0e2` (independent corrected TS capture), `8376be8e` (typed recording) and `1515cc56` (partial-send proof controls). The unchanged standard-library helper records qualified input/nullifier, checked u128 subtraction, sent output and optional change in source order. It reuses existing pinned cryptographic runtime primitives and audits helper declarations, arguments, all branches and unused bindings. No schema20 or ABI48 change.

Root and independent review found no remaining admission or helper-scope issue. Root integration retained both the ADR203 and ADR205 workspace/baseline cohorts and README sections. All main checks pass:

- Eight-source gate: **25/32 proof-required APIs available**, retaining the seven existing original-contract gaps. Ten TS/native/recorded/replay cases and independent pinned nonce/commitment vectors are included.
- Backend **26 library + 13 CLI + 153 renderer tests**, 34 Python harness tests, targeted strict Clippy, and all **174 fresh generated fixtures**.
- Two **4480-byte** call proofs verify and reject changed binding. Full 42→42 and same-order partial 42→17+25 pass default-strict ledger validation/application and exact nullifier replay refusal. Partial outputs allocate at indices 3 and 4. Reversed normalized/source output order explicitly rejects under the default exact policy; ADR205 canonical allocation remains separately opt-in. Extra Night-backed Dust funds the larger partial transaction. Previous contract coins are explicitly seeded prerequisites, not prior lifecycle proof.
- Whole-source inventory: **376/383 proof-required APIs available, seven explicit gaps**, across 215 sources / 752 exports / 194 compiled roots / 369 nonproof exports. Zero unassessed, missing or unmatched rows and no baseline drift. Four new APIs enlarge the corpus; they do not close the original microDAO/Coracle gaps.

Receipt: `${LOCAL_EVIDENCE}/compact-1515cc56-integration-receipt.json`; focused gate: `${LOCAL_EVIDENCE}/compact-focused-1515cc56/receipt.json`; inventory: `${LOCAL_EVIDENCE}/compact-1515cc56-inventory.json`. Latest full/portable checkpoint remains `7a311c9f`; this is a focused integration. User documentation changes are preserved. No push, publication or remote CI.

ADR204 is finishing strict transient acceptance and signing. ADR206/#310 has 27 independent original withdraw TS/native cases and prepares typed native-witness identity recording with ABI49 reserved; shared implementation waits for204. A separate read-only audit checks remaining production acceptance beyond corpus API counts.
