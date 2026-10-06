---
id: RUST-ADR-0191
alias: ADR-0191
title: "Record Unit-valued Zswap composite results"
date: 2026-10-05
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["recording", "Zswap", "composite"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: 980e448e27c5e6cd78b83438133d9a9211e46f2efc78b1394890096752a1a3b4
---
# RUST-ADR-0191 — Record Unit-valued Zswap composite results

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. The existing typed plan records Unit-valued Zswap composite results using a shared intent leaf; original planned and a separate transfer are covered. Zero-value strict application and funded transfer have their specific retained evidence; arbitrary composite helpers are not admitted.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#296 closure](https://github.com/MediaNoxLabs/compact/issues/296#issuecomment-6017730897). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`334b1475`](https://github.com/MediaNoxLabs/compact/commit/334b147508ab1c3185d5ec23c915dceda399b928) · [`5b86371a`](https://github.com/MediaNoxLabs/compact/commit/5b86371a0b3e2f124823261e609255c2eb3ad45c) · [`ee912835`](https://github.com/MediaNoxLabs/compact/commit/ee912835169d314851b99fc92c3e543ffaa2fd2e). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
status: delivered-locally
```

## Historical decision and amendments

Status: approved for bounded implementation (2026-10-05). ADR192 owns typed_plan.rs/stateful.rs; planner edits wait for its signed delivery and explicit handoff. Independent fixtures, captures, tests and docs may proceed. Base: integrated 5b86371a, ABI48/schema20.

### Smallest useful scope

Close the one remaining `stateful_struct_oracle.planned` proof-API gap using the existing typed composite planner. Admit an explicitly audited composite profile with Unit-valued native intent members, exact ShieldedCoinInfo/QualifiedShieldedCoinInfo/Either operands, existing scalar witnesses, and Kernel.self in the same frame. Preserve declared member evaluation order and call argument order. Use ADR192's declaration-directed pure/stateful helper dispatch rather than a new call resolver.

No general shielded standard-library promise follows. `planned` is an output-only circuit with no input or mint claim. Nonzero output42 is unbalanced under ADR188's exact matching policy; adding an unrequested wallet funding input is rejected by reconciliation. The original contract-recipient capture also lacks an explicit receive claim. Neither case should be repaired by fabricated claims or silent offer rewriting.

### Shared ownership and type boundary

Keep `Plan`, `TypedValue`, Scope, temporary bindings, recursive struct evaluation and exact result-type equality in typed_plan. Add an explicit composite-intents admission profile (or narrow lower entry point configuring the existing Plan), rather than broadly enabling Unit/coin operands in every existing profile. Its auditor admits only declared structures/scalars, exact intent operands, bounded existing witnesses and supported canonical Kernel operations; the typed evaluator rechecks actual operand types and evaluates each argument once, left to right.

A small effect-leaf emitter shared with zswap_plan may accept already materialized Rust operands and canonical IR types, returning a Unit-producing frame step. It should contain coin/recipient conversion and frame-method selection only: no Scope, witness dispatch, recursive evaluator, or call graph. This avoids a third typed value/scoping implementation. Do not migrate unrelated existing profiles in this slice. If extracting this small helper would enlarge the diff unnecessarily, first land shared lowering in typed_plan and make a separately bounded zswap_plan delegation; never copy the complete planner.

ADR192's typed declaration dispatch remains authoritative for helper identity, signature/arity, result types and recursion checks. Stateful calls with effects must inline into the same RecordingFrame. `call_local` is inappropriate because it cannot absorb Kernel.self public queries or native intent effects.

### Before and after generated behavior

Before: native `planned(context, witnesses, coin, recipient)` executes successfully and returns Planned, but there is no recorded/observed planned API. The composite auditor excludes Unit members, coin/recipient parameters and CreateZswapOutput.

After (illustrative sequence, not API-exact source):

```rust
let frame = RecordingFrame::start(context);
let (frame, first_u64) = frame.try_witness_metered(next_value(3))?;
let first = widen_checked(first_u64);
let frame = frame.create_zswap_output(to_coin(coin), to_recipient(recipient))?;
let emitted = ();
let (frame, address_bytes) = frame.kernel_self()?;
let (frame, after_u64) = frame.try_witness_metered(next_value(4))?;
let after = widen_checked(after_u64);
frame.finish(Planned { first, emitted, address: ContractAddress { bytes: address_bytes }, after })
```

The runtime frame already appends the Unit private output for CreateZswapOutput. The compiler must not add a second Unit, add public VM operations for intents, or record witness outputs after reordering fields. Kernel.self remains its canonical three-operation query in the same frame. Actual generated code will use existing checked bounded types and witness adapters.

### Runtime and preparation boundaries

Expected runtime ABI remains 48; schema remains 20. The necessary context/frame methods, private allocation state, sealed initial/final plans and offer-bound preparation exist from ADR188. No runtime API addition is currently justified.

Generic unbound preparation rejects a nonempty plan. Offer-backed preparation must match exact contract-owned inputs and exact normalized ordered outputs/indices, reject transients/extra wallet inputs/change, and verify final cursor/index map against the initial binding. Preserve EmptyTranscript precedence and the ADR188 correction that compiler-marked nonproof intent-only exports are native-only. Composite intent admission must have a structurally recorded public query; `planned` has Kernel.self. No synthetic query may be inserted.

### Evidence obtained before implementation

Pinned original TS artifact: ${LOCAL_EVIDENCE}/compact-adr179-ts/contract/index.js. Pinned ledger-v8 8.0.3. Script/log: ${LOCAL_EVIDENCE}/compact-adr191-planned-zero-probe.mjs and .log. Zero-value user recipient executes with witness order [3,4], private outputs [Uint64,Unit,Uint64], one output intent, cursor1, Kernel.self three public operations, result first30/emitted[]/address9/after41. Ledger Zswap allocation accepts the output at index0, first_free1. Default strict preflight without fee funding/proving rejects Dust deficit -686850000000001. Value42 rejects exact shielded token deficit -42. These preflight checks alone are not proof or transaction acceptance.

A temporary external Rust scratch consumes the original TS serialized proof preimage, uses pinned ${LOCAL_EVIDENCE}/compact-adr187-proof planned keys, reuses unchanged ADR180 Night-backed Dust setup, and attempts prove/seal/balance/default-strict/apply. It touches no repository source or planner. The first complete run passed proofs, separate Dust balancing, unchanged default strictness and ledger application; the final run also checks the exact allocated commitment and unchanged contract state. This proves an original TS/ledger zero-value case, not yet the missing Rust generated recorded API or ADR188 bound preparation for a composite return.

### Validation proposed after approval

1. Actual local_parity_gate joins compiler contract-info, schema/capability identities and Cargo behavior. Only planned's capability changes in the existing source; original produce/consume/witness_order remain native-only. Keep source hashes/baseline identities stable.
2. Original TS/native/recorded parity for both recipient variants, value0/value42, nonzero start index, and distinct witness state. Assert full result, public and private transcript order, intent order, gas, cursor/map and unchanged public state.
3. Original raw nonzero refusal: output-only exact offer retains -42 token deficit; extra funding input is rejected by reconciliation. Contract-recipient missing receive is a separate exact upstream negative. Never claim funded acceptance for these cases.
4. The scratch establishes zero-user-output acceptance: prove original planned with exact offer binding and separate Dust under unchanged default strictness, independently verify its call proof, check returned composite statement/tamper rejection and actual output allocation/ledger state. Reconcile runtime output plans, not only TS transaction construction.
5. Add a separate composite transfer fixture for nonzero42 acceptance, reusing ADR188's contract-owned seeded input and distinct output. Prefer an explicit composite with ordered Unit members for create input / claim nullifier / create output / claim spend and an address member from Kernel.self; witnesses before/after establish same-frame ordering. It has no extra wallet input/change/transient. Exact real claims are supplied as parameters as in ADR188, then checked against upstream offer effects.
6. Malformed IR rejection: wrong coin/recipient/Unit types, wrong witness arity/type, escaped binding, malformed declaration-dispatched helper, recursion, unmatched result field type, unsupported effect and missing structural public query. Runtime legacy180/184/188 controls remain unchanged; rerun only if runtime changes unexpectedly become necessary.

### Why full microDAO/Coracle shielded helper support is separate

receiveShielded = Kernel.self + create output + coinCommitment + claimReceive. mintShieldedToken adds tokenType(domain,Kernel.self), mint claim, output, commitment/spend and conditional self-receive. sendShielded adds qualified input/nullifier claim, checked amount subtraction, transient nonce hash, output/spend/self-receive and a conditional change output; result contains optional coin. sendImmediateShielded and mergeCoinImmediate deliberately use a placeholder input index and require authoritative transient normalization, currently rejected by ADR188. mergeCoin adds two input intents, color assertion, sum/nonce derivation and a self output with spend/receive claims.

microDAO vote_commit/set_topic/buy_in include immediate helpers and therefore transient composition. Coracle start also combines receive/mergeImmediate; concede/withdraw combine sendShielded and composite results. Supporting arbitrary helper graphs now would mix arithmetic/hash admission, optional results, state mutation and a new offer-normalization policy. Retain the exact ADR188 policy for this slice; propose later audited helper profiles and transient/change composition separately with pinned SDK/ledger evidence.

### Implementation order and approval request

Recommended order after ADR192 handoff: (1) narrow composite-intents profile and shared Unit effect-leaf lowering, original planned parity and exact negative admission tests; (2) generated original planned offer-bound preparation plus zero-user-output strict proof using the demonstrated fixture setup; (3) separate nonzero composite transfer, reusing ADR188 funding and exact offer policy. Keep the runtime ABI at48 unless an actual missing API is demonstrated. Do not reserve ADR191 or create its issue until this proposal is approved.

The first two steps alone close the original source gap and establish a real applied transaction. The third is the recommended additional evidence for nonzero intent/Kernel/composite composition; it is not a way to make the original output-only42 call balanced. Existing counters, snapshot/reverse/nested and the ADR192 helper graph should remain the ownership/regression controls.

### Final pre-implementation probe
Original TS planned zero-valued user output plus separate Dust: call/native proofs, exact output commitment at index0, unchanged original contract state, unchanged default strictness and ledger apply all PASS. Log ${LOCAL_EVIDENCE}/compact-adr191-zero-scratch-final.log. Receipt ${LOCAL_EVIDENCE}/compact-adr191-research-receipt.json SHA256 c80ae66b3efecbfea95a7f094339e98b39317e2536732d4e08f3901a5e41b9f7. Source/keys/helpers are individually hashed there. No claim yet for Rust generated composite preparation.


Issue: https://github.com/MediaNoxLabs/compact/issues/296 (rust-backend-v2). Before planner handoff, independent composite_zswap_transfer_oracle source compiles TypeScript/native Rust; four original planned cases and two transfer cases match native Rust return values, witness/private output order, Kernel effects, intent order/cursor/index map and gas query sums. TypeScript captures include both recipient variants, zero/nonzero original values, distinct cursors and seeded private state. New transfer proof keys generated at ${LOCAL_EVIDENCE}/compact-adr191-transfer-proof; no handwritten VM. Planner files remain untouched pending ADR192.


### Implementation and validation (2026-10-05)
Shared planner ownership was handed off by signed ADR192 6364a444 (local cherry-pick83c2d55d). The existing Plan now has an explicit composite-intents profile, using the same TypedValue/Scope/member evaluation and declaration-directed helper dispatch. intent_effect.rs is a shared leaf emitter for already materialized typed operands; zswap_plan delegates to it. The composite profile requires at least one native intent and one structural public query and rejects mint composition. No runtime/stateful/schema/ABI edits.

Actual generated API: `let contract = ledger_contract::Contract::from(witnesses); let call = contract.recording().planned_call(offer_bound.observed(), private_state, coin, recipient)?; let result = &call.recorded().execution.result; let prepared = offer_bound.prepare(call, verifier, communication_randomness)?;`. Result fields are first, emitted:(), address and after. Before this slice only the native ledger_contract::planned(context,&witnesses,coin,recipient) API existed.

Four original planned and two transfer TS/native/recorded/replay cases pass. Existing original struct corpus now includes planned recording. Backend13units+152renderer tests pass, including malformed effect operand/member/scope, no-public-query and unsupported-mint rejection. Original188 native-only exports remain native-only, and both188 generated libraries remain byte-for-byte fresh.

Two strict proof cases PASS: original planned zero-user output at index0 and composite transfer42 at index1, each with independent2912-byte call proof and changed-binding rejection, separate Night-backed Dust, unchanged WellFormedStrictness::default and ledger.apply. Exact output commitment and unchanged contract state are checked. Negatives PASS: user-output42 BalanceCheckOverspend(-42), extra wallet input InputMismatch, contract recipient CommitmentsNEClaimedShieldedReceives under default strictness, transfer spent-nullifier replay. No fabricated claim, VM op, output reindex or extra input. Log ${LOCAL_EVIDENCE}/compact-adr191-proof-final.log. An earlier harness used TestState::apply after privileged seed insertion and failed convenience-wallet replay; it was corrected to the authoritative direct ledger.apply path used by188. Failed logs are preserved.

Actual local_parity_gate with Cargo:4fixtures,9/9 proof-required capability rows recorded,3 nonproof rows native-only; the separate lexical inventory additionally includes pure_snapshot, for13 exported circuits total and4 nonproof. No unassessed exports. Receipt ${LOCAL_EVIDENCE}/compact-adr191-local-gate/receipt.json. Four-fixture freshness,6-package all-target/all-feature Clippy -Dwarnings,25inventory+4local-gate Python tests, three source gates and formatting pass. ABI48/schema20 stay unchanged. Baseline adds exactly the new transfer circuit and its witness; no old identity changes/removals.


### Signed delivery
Commit 36522c7275ce7b0fe2cd2ef31a610045a84fdbc1, GPG verified + DCO, explanatory problem/change/tests body. Worktree clean. Receipt ${LOCAL_EVIDENCE}/compact-adr191-delivery-receipt.json SHA256 7c66a5c2d800ec35b2eafd38a33434f6eab8e8cae3037cf0ed5dcb33876dfc39. No runtime/ABI/schema change, no push/remoteCI. Shared planner ownership released to ADR193. Final explicit discard of an unused original ledger return value is semantically unchanged from the successful proof run and is covered by final Clippy.

### Integrated composite-intent delivery — 334b1475

ADR191/#296 is integrated as GPG/DCO-verified `334b1475`. Private schema 20 / runtime ABI 48 unchanged. The typed composite planner reuses the shared effect leaf and existing frame/runtime primitives. It records the original `planned` output and a new distinct funded composite transfer without adding a separate scoped planner.

- All **168 fixtures** are fresh; backend 13 library + 13 CLI + 152 renderer tests and strict six-package Clippy pass.
- Five-fixture focused gate passes **11/16 proof-required recorded APIs**, with 3 separate native-only nonproof exports: `${LOCAL_EVIDENCE}/compact-focused-334b1475/receipt.json`. The five explicit gaps here are the remaining original microDAO operations.
- Both combined-branch composite call proofs verify, reject changed bindings and pass unchanged default ledger strictness/application. Original zero-value user output is index 0; funded transfer42 is index 1. Both retain explicit Night-backed Dust and unchanged public contract state. Exact -42 deficit, extra wallet input mismatch, missing contract-receive claim and spent-nullifier replay negatives pass. `${LOCAL_EVIDENCE}/compact-integrated-adr191-proof.log`.
- Exact inventory `${LOCAL_EVIDENCE}/compact-334b1475-inventory.json`: **354/363 proof-required APIs available, 9 explicit gaps, zero unassessed**; 210 sources, 732 exports, 189 compiled roots and 369 nonproof exports. No missing/unmatched compiler rows or declaration baseline drift. Compared with89bd7764, one existing gap closes and one new proof-required transfer fixture is added; this explains both numerator and denominator changes.

Remaining corpus gaps are five microDAO and four Coracle operations. ADR193 Coracle guess is implementing; ADR194 microDAO advance is in design; ADR195 receiveShielded prepares independent fixtures before its shared-emitter handoff. Real external wallet-funded receive requires a separate explicit runtime policy; ADR188 exact input/output/change/transient restrictions remain enforced.

Latest full broad gate and clean aarch64-darwin portable archive remain at ee912835. The later192/191 deliveries have focused combined checks above; they are not part of that older full-run/package claim. User-owned documentation edit untouched. No push, publication or remote CI. Corpus API coverage does not establish complete semantic or production readiness.
