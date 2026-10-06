---
id: RUST-ADR-0195
alias: ADR-0195
title: "Record shielded receive with canonical coin identity"
date: 2026-10-05
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["recording", "shielded-receive", "coin-identity"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: 03ba2847c1a1cbbb7f590d257e840c9bdfe17c003a3526beeb7e228c56d932ef
---
# RUST-ADR-0195 — Record shielded receive with canonical coin identity

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. The unchanged receiveShielded wrapper records audited pure commitment computation and a typed ledger CoinInfo intent from the same coin. Original TypeScript refusal above 2^64 was a separate descriptor bug corrected by ADR197; this decision keeps strict external-wallet funding refusal until ADR199.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#298 closure](https://github.com/MediaNoxLabs/compact/issues/298#issuecomment-6017734169). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`4d27092c`](https://github.com/MediaNoxLabs/compact/commit/4d27092ccfac0b9ffc7a817b9e91f180eddc5008) · [`da8be4a2`](https://github.com/MediaNoxLabs/compact/commit/da8be4a2b7a852ca4574b2139ca5833e374e7149) · [`dd85fa81`](https://github.com/MediaNoxLabs/compact/commit/dd85fa8154e69ab35c23dfaf186d9dc0220617e4) · [`ee912835`](https://github.com/MediaNoxLabs/compact/commit/ee912835169d314851b99fc92c3e543ffaa2fd2e). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
type: adr
status: delivered
date: 2026-10-05
milestone: rust-backend-v2
commit: bf330c796a708a128c1bd168938b61183656f50e
```

## Historical decision and amendments

### Problem
The unchanged Compact standard-library receiveShielded helper compiles for native Rust, but a proof-required exported wrapper has no recorded or observed API. The helper reads kernel.self, creates a contract-recipient Zswap output, computes the same coin commitment, and claims that commitment as a shielded receive. Current bounded recording planners cannot compose the address/recipient and computed commitment into one frame with the output intent. Coracle.start and micro-dao vote_commit/set_topic/buy_in use this helper, but their wider action graphs remain separate.

### Before and after
Compact source stays unchanged:
```compact
export circuit accept(coin: ShieldedCoinInfo): [] {
  receiveShielded(disclose(coin));
}
```
Before: ledger_contract::accept(context, coin) executes natively; strict recording rejects the exported wrapper. After: a generated recorded::accept(context, coin) and observed accept_call are available. Illustrative recorded body:
```rust
let (frame, self_address) = frame.kernel_self()?;
let recipient = CoinRecipient::Contract(self_address);
let info = coin_info_from_compact(coin.nonce, coin.color, coin.value.value());
let frame = frame.create_zswap_output(info, recipient.clone())?;
let commitment = info.commitment(&recipient);
let frame = frame.kernel_claim(KernelClaim::CoinReceive(commitment))?;
frame.finish(())
```
Actual emitted syntax may differ. The helper call is inlined into the same ordered RecordingFrame; no synthetic public query or detached native helper call is allowed.

### Decision and ownership
Extend the existing typed declaration-directed helper/plan path after ADR-0191 and ADR-0193 handoff. A shared typed effect leaf receives materialized CoinInfo, recipient and commitment values; the typed planner owns argument evaluation, lexical scope, call graph, type equality, effect order and final Unit. Admit the audited standard-library helper graph structurally, not by wrapper/source/helper name. Use pinned midnight-coin-structure Info::commitment and Info::nullifier with SenderEvidence::Contract for cross-language identity vectors; receive only emits the commitment claim. Keep source IR schema 20 and runtime ABI 48 if existing nodes and APIs suffice; propose an exact versioned change before adding either.

### Runtime and transaction boundary
Reuse CircuitContext and RecordingFrame create_zswap_output, kernel_self and kernel_claim. Preserve one Unit private transcript output from the native intent and the two real public queries. No wallet state is fabricated. ADR-0188 exact offer binding remains unchanged: an external wallet-funded receive has an unmatched wallet input and must be rejected pending a separate typed funding-envelope policy. Probe an exact zero-value contract output with separate Night-backed Dust under unchanged default strictness and actual ledger apply; report acceptance only if it passes. Nonzero wallet-funded receives remain an explicit negative.

### Guards and acceptance
- Compile the unchanged wrapper and renamed control with TypeScript and Rust, strict recording, and compare declared capabilities/inventory with the original source. Reject malformed helper signatures/arity, wrong coin/recipient/result types, escaped scope, recursion, hidden effects, duplicate output commitment and wrong claim kind.
- Independent TS/native/recorded comparison for nonce/color/value boundaries, contract address changes and nonzero output starts: return Unit, serialized state, effects, ordered public VM and per-query gas, one private Unit, exact output intent/cursor/com_indices, replay. Evaluate each argument exactly once.
- Compare TS coinCommitment and coinNullifier with ledger Info::commitment and Info::nullifier for distinct domains, contract/user recipients, changed addresses and high Uint128 values. Do not emit a nullifier on receive.
- Generate local keys, prove/verify a recorded call and test exact offer binding. If the zero-value contract output can be funded only by separate Dust, use default strictness and ledger.apply; retain nonzero external-wallet funding as a precise rejection.
- Run focused source/renderer/runtime/fixture gates and Clippy/fmt using the warm target, then record exact signed-head evidence. Conventional GPG+DCO commit, no push or remote CI.

### Coordination
Research: [ADR-0195 — Shielded receive and offer ownership research](0195-record-shielded-receive-with-canonical-coin-identity.md). ADR-0191 then ADR-0193 own shared typed_plan/stateful/recorded files until signed handoff; independent source/capture/fixture/proof preparation can proceed. Tracking issue will be added before implementation.

Tracking issue: https://github.com/MediaNoxLabs/compact/issues/298 (rust-backend-v2).

### Refined canonical identity decision
Emit the existing declaration-directed, audited pure coinCommitment helper for the Compact claim. Do not replace that helper with a name-based or preimage-shape intrinsic. The intent leaf converts the same typed coin and recipient into ledger CoinInfo/CoinRecipient; create_zswap_output computes its key through pinned ledger Info::commitment. Compare generated pure helper, independent TypeScript helper and ledger commitment bytes before claiming parity. The illustrative snippet above describes semantic equality; actual generated code should call the audited pure helper for the claim. Likewise compare TS coinNullifier with ledger Info::nullifier using Contract sender evidence as a vector only; receive emits no nullifier. Reject altered helper signature/type/unsupported body through the declaration-directed audit. This avoids a duplicate codec and a false source-name privilege.
The pure helper audit must recurse through every CoinPreimage member, recipient branch, persistent-hash opening and nested call. It must bind declared parameter/result types, reject effectful or open helper bodies and recursion, and preserve Compact helper semantics even if a source helper is renamed. These are admission checks before claim lowering, not after-the-fact string inspection.
### Observed TypeScript value boundary
The independent unchanged-source capture accepts `0`, `42`, and `2^64−1`. The generated pure `_coinCommitment_0` also hashes `2^64` and `2^128−1`, but the current TypeScript `createZswapOutput` rejects those two values before the claim because `runtime/src/compact-types.ts` uses the 8-byte `MaxUint8Descriptor` in `ShieldedCoinInfoDescriptor`. Compact declares `Uint<128>` and ledger-8 `Info.value` is `u128`. ADR-0195 preserves ledger/Compact `u128` Rust semantics, compares full TS/native/recorded results only within the executable TypeScript range, and includes wide values as Rust/ledger canonical commitment and proof vectors with explicit TypeScript call rejection. This is tracked separately as [MediaNoxLabs/compact#300](https://github.com/MediaNoxLabs/compact/issues/300). No TypeScript oracle or ABI change belongs to this receive bridge.
### Delivery receipt (2026-10-05)
Signed conventional DCO commit: `bf330c79f...` (`feat(rust-backend): record standard-library shielded receive`), GPG signature verified; no push. Exact full SHA is available from branch `codex/adr195-shielded-receive`. Source schema 20 and runtime ABI 48 unchanged. New source has two proof-required exports, both native and recorded; no Coracle or micro-dao whole-source admission claimed.

The unchanged receive wrapper and renamed control pass independent TypeScript/native/recorded/replay comparison at 0, 42, and 2^64−1; two pinned-ledger identity tests cover contract/user commitments and contract nullifiers through u128 max. Six current-Scheme fixture freshness checks, 16 backend units, malformed structural audit, 25 inventory tests, strict Clippy, and fmt pass. The source proof checks verify zero and 2^64 call proofs and reject altered public binding. The exact zero-value contract output with separate Night-backed Dust passes default-strict ledger apply. An offer with an external wallet input fails ADR-0188 exact input ownership, as intended.

The original TypeScript runtime 0.16.101 descriptor refusal at 2^64 and u128 max remains captured with source/runtime provenance in `runtime-rs/tests/fixtures/shielded-receive-oracle.json`; issue #300 and ADR-0197 separately address that defect. The current Rust proof at 2^64 establishes call validity only, not wallet-funded ledger apply. Shared emitter ownership was released to ADR-0194 after signing.


### Integrated receive recording and TS boundary fixes — dd85fa81 (2026-10-06)

ADR195/[#298](https://github.com/MediaNoxLabs/compact/issues/298) integrated as signed/DCO `4d27092c`; ADR200/[#303](https://github.com/MediaNoxLabs/compact/issues/303) as `dd85fa81`. The former uses shared typed planning and declaration-audited pure identity helpers for the unchanged receiveShielded wrapper; the latter makes signed bigint reduction satisfy its documented Euclidean range. Earlier196/197/198 changes are retained. Schema20/ABI48 unchanged.

- **171 fixtures fresh**, backend16 library +13CLI +153renderer tests,2 pinned-ledger coin identity tests,34Python tests and targeted strict backend/runtime/fixture/proof Clippy pass.
- Six-fixture main gate passes11/23proof-required recording APIs,0nonproof native-only rows in this selected group: `${LOCAL_EVIDENCE}/compact-focused-dd85fa81/receipt.json`. Original Coracle/microDAO and the native lexical regression are included.
- Both zero and2^64 receive call proofs verify (4480bytes each), with changed-binding rejection. **Only the zero-value contract output is ledger-applied**, under default strictness with separate Night-backed Dust and authoritative index0. Real external wallet input still rejects InputMismatch under the existing exact policy. `${LOCAL_EVIDENCE}/compact-integrated-adr195-proof.log`.
- Combined main production+test TypeScript checks, full **70/70runtime tests**, targeted ESLint/format pass. Signed reduction preserves nonnegative behavior and canonical signing-key rejection. Original TSb8refusal capture and separate corrected197capture remain distinct.
- Exact inventory `${LOCAL_EVIDENCE}/compact-dd85fa81-inventory.json`: **358/370 proof-required APIs available,12explicit gaps,zero unassessed within this corpus**;212sources/739exports/191compiledroots/369nonproof, zero baseline drift/missing/unmatched rows. Gaps:5microDAO,4terminal-lexical regression,3Coracle. Corpus availability is not complete language/semantic parity.

Main receipt: `${LOCAL_EVIDENCE}/compact-dd85fa81-integration-receipt.json`. Latest broad/full and clean portable archive remain ee912835; no later full/package claim. ADR194 advance is implementing; ADR199 explicit wallet funding is runtime-only; ADR201 read-only composite Cell observations is approved for preparation and waits for194shared-emitter handoff. Historical capture harness provenance is being tightened separately. No push, publication or remote CI. User-owned documentation edit preserved.


### Historical capture guard — da8be4a2

Signed/DCO `da8be4a2` follows the dd85fa81 integration gate with a capture-only fix: resolve metadata from the actual imported runtime and require the original b8 descriptor before regenerating ADR195 historical evidence. The old runtime reproduces the checked fixture byte-for-byte; the corrected u128 runtime refuses early with a descriptive pointer to the separate197capture. Stored oracle and production sources are unchanged. `${LOCAL_EVIDENCE}/compact-adr195-capture-guard-receipt.json`. Existing dd85fa81 evidence remains scoped to that exact commit; no redundant production gate was run for this harness-only change.
