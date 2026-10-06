---
id: RUST-ADR-0215
alias: ADR-0215
title: "Record original microDAO buy_in with checked price and mint composition"
date: 2026-10-06
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["recording", "microDAO", "mint"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: c6f951593c9a0c92765a0fb36b5af820db835b033c4890999ad85171e5aed2f8
---
# RUST-ADR-0215 — Record original microDAO buy_in with checked price and mint composition

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. A closed FundedShieldedMint domain records original buy_in with checked price and audited mint/receive composition. Empty and occupied seeded branches prove/apply; no claimed buy_in-to-vote_commit chain or full DAO lifecycle.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#319 closure](https://github.com/MediaNoxLabs/compact/issues/319#issuecomment-6017770863). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`7daac7f6`](https://github.com/MediaNoxLabs/compact/commit/7daac7f62e6911bc0cc903dc958af5326b041a0d) · [`db853a2b`](https://github.com/MediaNoxLabs/compact/commit/db853a2b822050699f79822a994819bc889a849c). These include historical prerequisites and probes, not only final delivery commits.

## Historical decision and amendments

Status: accepted bounded design; preparation pending, 2026-10-06. Milestone: rust-backend-v2. Research: [Original microDAO buy_in — guaranteed mint funding research — 2026-10-06](references.md#private-note-15). This decision precedes implementation. Root remains frozen at3404c30c for the current full gate; independent code preparation begins after that gate.

### Problem

Unchanged original microDAO buy_in already executes natively but cannot record its complete price-check, pot funding/merge and mint operation. A developer needs the same ordinary generated Rust recorded/observed APIs used by other circuits, with genuine ledger-8 monetary acceptance. Receiving funds and running a mint VM instruction alone do not establish a balanced, provable transaction.

### Before and after

Current generated native signature consumes CircuitContext<Private>, ShieldedCoinInfo and BoundedUint<18446744073709551615>; it returns CircuitResult<Private, ShieldedCoinInfo>. There is no declared witness provider for this call. The recorded form should preserve all source arguments and result type:

```rust
// Before: existing native execution, with a configured execution coin key.
let execution = ledger_contract::buy_in(context, coin, amount)?;

// After: proposed ordinary generated API, same arguments and selected key.
let recorded = ledger_contract::recorded::buy_in(context, coin, amount)?;
// Existing observed/offer-bound preparation then binds original artifacts,
// complete proof inputs and the retained explicitly selected ledger offer.
```

Verify the concrete emitted signature at delivery. The execution key selects the minted output recipient; setting it is not proof of wallet authorization. Original Costs and supplied amount determine the input price. No witness implementation or helper-specific consumer API is introduced.

### Decision

Compose the existing typed Plan application path with exactly the declared unsigned price multiplication and the audited original mint helper. Preserve existing domains and use a distinct closed admission policy. Reuse existing ledger-8 primitives, checked unsigned lowering, canonical allocation, own-key sealing, mint/output/claim leaves and pure nonce/token helpers. Do not copy a second evaluator or recognize source/helper names.

The product is exactly (2^64−1)^2 =340282366920938463426481119284349108225 at the maximum operands. Follow the actual IR coercion/result bounds and checked conversion, without wrapping or implicit Field arithmetic. Audit both multiplication operands, every helper value/argument/unused binding and every unselected branch, complete source declarations, pure/stateful ambiguity, recursion, physical ledger paths and lexical scopes. A compiler implementation must not gain unrelated subtraction/reset/mint powers merely because another profile needs them.

Source order: require nonzero amount; read configured cost and check exact native-token coin value/color; receive coin; select empty pot store/flag or occupied historical+transient merge/store; obtain own key and mint exactly amount of the instance voting token with the source nonce evolution; return the minted coin. Do not move key access earlier or erase failure-prefix effects. Source permits zero configured cost; preserve that execution behavior. A separate monetary proof is needed before claiming zero-value transaction acceptance.

### Runtime boundary and placement

Independent corrected-TypeScript probe has18 cases:8 successes and10 failures. Empty successes partition wholly guaranteed54 operations, occupied successes also wholly guaranteed81 operations. Both differ from occupied set_topic and vote_commit. Reinspect actual prototypes at preparation; these observations are not a universal partition theorem.

Use existing default guaranteed placement with canonical indices and explicit full wallet Inputs. Occupied additionally needs genuine historical pot input and explicitly selected full received Transient; both persistent outputs (native pot and newly minted voting token) must be bound with exact owner/index/commitment identity. No fallible-policy inference, implicit funding, offer retargeting, extra output, disabled balance check or changed ledger parameters. All retained components are proved by the upstream ledger implementation. Separate Night-backed Dust pays fees.

No new emitter-facing runtime method, schema20 or ABI49 change is currently justified. Reassess only if actual implementation identifies a missing boundary. Shared emitter queue is211 set_topic,212 cash_out,213 Coracle start,214 vote_commit, then215 buy_in; independent capture/native/key preparation may run sooner.

### Acceptance

1. Promote the independent18-case probe to a deterministic original-source capture with source/generated/corrected-runtime provenance. Compare complete result/state/effects/private outputs/query programs and failure ordering against native and then recorded Rust. Preserve raw capture and distinguish unordered upstream effect sets from ordered source/query/private transcripts.
2. Cover empty/occupied normal amount1/2, maximum Uint64 amount and product, zero configured price, amount0 before key access, wrong value/color, merge color-before-overflow, missing key at each late failure point. Typed Rust rejection may differ from malformed JavaScript error wording.
3. Keep original source and all exported circuit identities; record only supported structural closure. Add meaningful negative structural tests for wrong arithmetic/type bounds, hidden/unused effects, invalid field declarations, ambiguous/cyclic helpers, argument/callee/branch scope leaks. Preserve earlier profile refusals.
4. Produce original buy_in proofs for both empty and occupied nonzero funding paths. Seed genuine wallet funds and historical pot at an actual index where applicable; construct/prove all upstream components; use the complete unchanged canonical offer; add separate Dust; require default-strict verification/application. Assert exact native pot and minted instance token balance, amount, nonce, user recipient, qualified indices/owners, nullifiers and replay refusal. No synthetic wrapper or balance-disabled smoke closes this requirement.
5. Run focused compiler/native/recorded/replay/refusal tests, fixture freshness and strict Clippy. Reuse keys and warm target; full workspace/portable gate belongs to the combined final original-source checkpoint.

### Limits and delivery record

Prior wallet coins and application state are explicit seed prerequisites unless an actual lifecycle is demonstrated. A later buy_in→vote_commit ledger chain would need proof that the actual minted output funds the next call; current research makes no such claim. No live-wallet submission, remote CI, push, registry publication or production completion is established here.

Research files: ${LOCAL_EVIDENCE}/compact-buy-in-proposal.md, ${LOCAL_EVIDENCE}/compact-buy-in-probe.mjs, ${LOCAL_EVIDENCE}/compact-buy-in-probe.json. Implementation/receipt/issue link to append after creation and measured delivery.


MediaNoxLabs milestone issue created before implementation: https://github.com/MediaNoxLabs/compact/issues/319.


### Independent preparation checkpoint, 2026-10-06

Signed GPG/DCO conventional commit `0a663cae8200ff0c92fb6a21decbda3ef4536b57` promotes the unchanged original-source corrected-TypeScript capture script and its 18-case fixture (8 successes, 10 expected failures). Rerunning the promoted script against the pinned generated contract, corrected ADR200 runtime and ledger-v8 module reproduces the fixture byte-for-byte. Fixture SHA-256 `4e36e2d3a1cf410f11ed22f3bebc2a5a0c46f8f32351a9ab7be6a452187de440`; source/generated/runtime/ledger hashes are inside it. Direct Rust native parity passes for complete pre/post state, minted result, private outputs, per-query gas sums, exact coin input/output contents, recipients and captured commitment/index maps, mint amounts/colors, and bounded error precedence. Only upstream set-like claim arrays are sorted without deduplication, and TS bigint mint-map values are compared as exact decimal values; ordered public/query/private data stays raw. This is native parity only, not recorded or ledger proof.

The original `buy_in.zkir` compiles at k=16, 47,139 rows. Local keys: `${LOCAL_EVIDENCE}/compact-adr215-keys/buy_in.prover` SHA-256 `a4655a8341abefb148d30ec6e39c0c063e2b2d6b724c0b4fd2521d65fe240f18`, verifier SHA-256 `fcae07694ba9a140dd05645cdde9d15ce2c096ebf58cd285b38008272d53ce60`. Focused native fixture test and strict Clippy passed. Shared emitter/runtime remain untouched pending the ADR211→212→213→214 handoffs.

### Implementation review — 2026-10-06

The candidate uses separate FundedShieldedMint admission with shared typed Plan evaluation. guarded_deposit::prefix exposes common receive, optional merge and exactly-one-qualified-store-per-path checks. ADR211 passes no price amount and retains its previous expression domain. Both root coin and amount names are protected against shadowing. Only the new domain gains exact unsigned product checks, unsigned non-equality, sealed own-key and existing KernelMintShielded effect. Pure token/nonce closure, unused effects and lexical types remain audited.

Concrete generated API: ledger_contract::recorded::buy_in(context, coin, amount), or ledger_contract::recorded::Contract.buy_in_call(observed, private_state, coin, amount). The typed result is ShieldedCoinInfo. Schema20/ABI49 and runtime remain unchanged.

Four structural tests and18 captured TS/native/recorded/replay cases pass, including exact result/state/effects/private outputs, public VM, execution/replay gas and error ordering. Independent emitter review: ${LOCAL_EVIDENCE}/compact-adr215-independent-emitter-review.md, no actionable findings. Initial inline expression emission caused an existing recursive compiler test to overflow its default worker stack. Extracting mint/multiply/non-equality methods fixed the regression: all50 library tests pass without increasing a stack limit. Final strict three-package Clippy passes after correcting test-module placement and trivial unit/Boolean assertions.

Both initial original proof cases passed 4,480-byte call verification, changed-binding rejection, actual upstream monetary components, separate Dust, default-strict apply, wallet recovery of minted tokens and nullifier replay refusal. Final signed proof strengthens historical pot to index1, distinct from received singleton index0; that run remains pending. The original source ZKIR SHA256 is0e2c8de735a1beb42c76104a05c655d87bb0b67269cddcca4be2110bdeaa44e6; matching retained binary IR SHA256 is7184135a7c73a4554eff7f787ed32c65877ed60f9c16b5383c1876a12069d08b. Initial missing-binary harness setup failures are retained. No corresponding monetary claim for zero price or maximum product, funded full DAO lifecycle, live wallet, remote CI or push.

The internal serial queue has been replaced by independent profile branches from53eb9a1e with root-serialized integration. Final signed delivery receipt will supersede this candidate report.

### ADR215 signed local delivery

Signed/GPG-verified/DCO commit `7daac7f62e6911bc0cc903dc958af5326b041a0d` records the unchanged original buy_in through a distinct funded-mint domain and shared typed Plan. Eighteen corrected-TypeScript/native/recorded/replay cases pass, plus50 backend library,13 CLI,153 renderer tests,34 Python tests,176 fresh fixtures and strict three-package Clippy. The exact signed-head five-source gate passes11 commands,18/20 proof-required APIs in its subset. Wider inventory at this head is383/386, with cash_out, vote_commit and start then remaining.

Both original 4,480-byte proofs pass, changed binding is rejected, and full transactions use actual wallet Inputs, persistent contract/user Outputs and occupied historical Input at index1 plus received Transient. Both54/81-operation calls are wholly guaranteed. Separate Night-backed Dust pays fees; default-strict well-formedness/application passes, canonical output owners/indices and exact minted token color/quantity/nonce are checked, the actual wallet recovers the minted output, and spent nullifiers cannot replay. Prior funds/application state are explicit offline seeds. Zero-price/max-product captures are execution-only controls; no full DAO lifecycle/live-wallet claim.

Delivery receipt: ${LOCAL_EVIDENCE}/compact-adr215-delivery-receipt.json; SHA256 `1c3db084f8af69de6059f62b2485ddc977a3e9f6db3e26e2cbcd59238d34a440`. Exact focused receipt: ${LOCAL_EVIDENCE}/compact-adr215-exact-gate/receipt.json. Final strict log: ${LOCAL_EVIDENCE}/compact-adr215-strict-final.log. Original keys/binary IR/sealed transactions and their hashes are retained in the receipt. Independent emitter review found no actionable findings. Initial stack/lint/missing-binary setup failures and fixes remain linked.

Subsequent combined integration includes cash_out and vote_commit and reconciles the exact seven-export positive manifest. Its first gate caught a stale isolated buy_in-refusal assertion after the compiler, inventory, parity and cohort checks passed; signed db853a2b corrects that harness expectation. Final combined checks are pending. Schema20/ABI49 unchanged. No push/remote CI; issue remains open for milestone acceptance.

### Combined microDAO checkpoint db853a2b — 2026-10-06

ADR212/214/215 integrated;58 backend unit+13CLI+153renderer,34Python, six-source parity, exact seven-export strict microDAO cohort, inventory and strictClippy pass. Whole inventory385/386; only original Coracle start remains. See [Milestone 2 — Current local checkpoint](references.md#private-note-09) for evidence and limits. Root receipt ${LOCAL_EVIDENCE}/compact-db853a2b-integration-receipt.json. Latest full/portable remains3404c30c. Live wallet observation gap tracked by ADR217/#321; direct behavior test gaps tracked by216/#320 and next218. No production-completion, push or remote CI claim.
