---
id: RUST-ADR-0205
alias: ADR-0205
title: "Opt-in canonical persistent output allocation"
date: 2026-10-06
publication_date: 2026-10-07
decision_status: "accepted-runtime-policy"
topics: ["allocation", "Zswap", "offer-binding"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: 76d3fb537101f9660df8b5f5a0a2da7c5099107d05d3cdbc7c988408e3f200ad
---
# RUST-ADR-0205 — Opt-in canonical persistent output allocation

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-runtime-policy. An opt-in canonical allocation policy binds source-ordered intents to upstream normalized output commitments and actual indices while retaining exact private source order. Default/wallet exact policies continue rejecting reversed order; no silent reordering is introduced.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#309 closure](https://github.com/MediaNoxLabs/compact/issues/309#issuecomment-6017753847). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`7a311c9f`](https://github.com/MediaNoxLabs/compact/commit/7a311c9fe80ef588b744407fb39f0130ba6d352f) · [`8e7080f5`](https://github.com/MediaNoxLabs/compact/commit/8e7080f51ff778ff8175ca66b430ef3b3560f574). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
status: delivered-locally
date: 2026-10-06
adr: 205
```

## Historical decision and amendments

Parent approved the read-only proposal before implementation. Keep ABI48/schema20: additive typed options explicitly select the new allocation behavior; existing constructors/default and ADR199 exact-order behavior remain unchanged. No transient admission. Runtime ownership is205 until signed handoff to204.

### Problem and approved result
Source-ordered send/change outputs may be opposite the normalized upstream offer order. Existing exact-order calls reject this safely. Canonical opt-in must bind each source intent to its real upstream index without reordering execution, changing commitment maps, or repairing transcripts afterward. Exact persistent output equality, duplicates, ownership metadata and context/allocation integrity remain checked.

Before: `OfferBackedObservedState::new(...)` only admits matching source/offer output order. After: an additive `with_options(..., OfferBindingOptions::default().with_output_allocation(PersistentOutputAllocation::CanonicalOfferIndices))` binds canonical indices while retaining source order. Actual final names/signatures will be recorded at delivery.

Two-output+writeCoin source admission is probed before runtime implementation. Two real funded strict proofs, both sort orders and stored qualified index, are mandatory. If the existing emitter cannot handle the source, report before any shared planner edit. No remote CI or push.

## ADR205 candidate — opt-in canonical persistent output allocation

Read-only proposal, 2026-10-06. No205 issue, ADR, source edit or runtime implementation has been made. ADR202 is delivered as c4608d30; parent main7a311c9f is frozen during its full checkpoint. This proposal examines the integrated199 runtime and pinned ledger8.0.3. ADR203 supplies the qualified-send helper source; ADR204 owns the later transient policy and waits for this allocation contract.

### Concrete boundary and evidence

`Offer::new` normalizes complete inputs, outputs and transients. `Offer::normalize` sorts the original values; it does not deduplicate. `Output` ordering compares `coin_com` first. Upstream ledger `State::try_apply` processes ordinary outputs in retained offer order, then transient outputs, returning the authoritative commitment→index map. Its allocation advances first_free for each output and rejects a repeated commitment. `offer_well_formed_common` separately rejects non-normalized offers. `try_apply` is not cryptographic proof verification.

Current `OfferBackedObservedState::new_inner` already obtains that complete map before execution and installs it in the observed call context. The two blockers are local policy checks: `CircuitContext::create_zswap_output` requires the next source intent to equal `outputs[position]` at logical next_index, and `reconcile` zips source intents with normalized offer outputs. The map itself is available and must not be rewritten.

ADR203's captured42→17 partial send shows both orderings already at the raw TypeScript level (this is not yet strict funded application evidence):

- send_to_self: sent commitment4468b42f…ee63, change79553540…8b18; source and commitment order agree.
- send_to_user: sentd929db03…51ae, change79553540…8b18; source order is sent/change, but normalized offer order is change/sent.
- Both raw TS rows start at1, record sent index1/change index2 and end at3. The reverse case's authoritative ledger allocation must instead bind sent index2/change index1. This is an inference from the captured commitments plus the pinned derived Output ordering; an actual complete Offer/try_apply reproduction will be required in implementation tests.

Reference capture: `${COMPACT_SOURCE}/runtime-rs/tests/fixtures/shielded-send-oracle.json`; source adjacent `examples/rust_backend/shielded_send_oracle.compact`. Runtime descriptor provenance is corrected ADR200 runtime0.16.101/16-byte Uint128 as captured. No new raw TS behavior is inferred beyond these rows.

### Recommended public policy shape

Keep the documented default and ADR199 exact-order policy. Add one composable typed options entry point, with existing constructors as compatibility wrappers:

```rust
pub enum PersistentOutputAllocation {
    ExactIntentOrder,       // default, existing behavior
    CanonicalOfferIndices,  // explicit205 opt-in
}
pub struct OfferBindingOptions<D: DB = DefaultDB> { /* private fields */ }
// Default: exact order, no wallet funding.
// with_output_allocation(enum) and with_wallet_funding(WalletFundingInputs<D>)
// consume self and retain typed selections.

let options = OfferBindingOptions::default()
    .with_output_allocation(PersistentOutputAllocation::CanonicalOfferIndices);
let bound = OfferBackedObservedState::with_options(observed, &ledger, offer, options)?;
let call = contract.recording().transfer_call(bound.observed(), private, /* ... */)?;
let prepared = bound.prepare(call, verifier, randomness)?;
```

`new` delegates to default options. `with_wallet_funding` delegates to the existing exact-order policy plus the supplied exact upstream input selection. Callers can compose canonical allocation and the same explicit wallet input selection through options; no separate canonical+wallet constructor. Fields remain private, and neither policy infers funding from owner=None. No wallet change outputs or arbitrary extra inputs are admitted. Future204 can add its typed transient selection to the options after approval, without introducing a second allocator. Do not add a transient option or dormant admission now.

### Private allocation and execution contract

Create one private immutable bound-allocation value from the retained complete offer and upstream try_apply result. Each approved persistent output row binds commitment, actual index and public owner metadata. Keep the upstream map in `com_indices` unchanged. Seal the selected output policy in the observation and plan, alongside start/frontier and rows. Retain the complete exact Offer, not a reconstructed or sorted circuit-intent projection. For canonical mode, refuse a non-normalized input offer (compare an upstream-normalized clone with the supplied offer, or reuse an upstream normal-form check); never silently normalize after allocation.

In provisional native mode nothing changes: output indices and `com_indices` are populated in source order, including its existing duplicate behavior. Ordinary observations remain locked. Exact-order bound mode keeps its current positional refusal.

In canonical bound mode, each `create_zswap_output`:

1. Computes the commitment with upstream CoinInfo::commitment and typed CoinRecipient.
2. Looks up one matching sealed persistent row, rejects absent or already-produced commitment, and checks the immutable context map equals that row's actual index. It checks output owner metadata is None for a user recipient or exactly the supplied contract address for a contract recipient.
3. Checks cursor progress arithmetic before any mutation.
4. Appends the original typed coin/recipient to the output intent vector in source order, with the row's actual index, and increments logical produced-count progress. It never rewrites `com_indices`, reorders VM queries, reorders outputs, or changes witness/private Unit output order.

The legacy public field `CircuitZswapOutput.provisional_index` already documents exact indices in bound mode; retain it for compatibility and document its canonical meaning. `next_index` remains start + number of emitted intents: logical allocation progress, not the next source intent's physical Merkle slot. For two outputs at start7, source-order actual indices may be[8,7] while progress goes7→8→9. This distinction must be explicit in docs/tests. A future nicer read accessor is optional, not necessary for205.

Qualified Cell/Set coin operations already ask the upstream VM for the index through `com_indices`. With the sealed map installed before execution they capture the real assigned index. Do not patch a returned qualified coin, query result, transcript or state after execution. Public input remains the actual pre-state qualified input index; canonical allocation only affects newly emitted outputs.

### Exact reconciliation and rejection boundaries

Canonical mode is an intent-bearing closed-offer policy. At preparation retain all existing context/observation pointer checks, empty-initial-plan checks, public transcript initial/final snapshot comparisons, final plan/allocation equality, contract address checks and complete `com_indices` equality. Preserve existing generic EmptyTranscript refusal and default legacy empty-plan offer calls. Do not extend the default empty-plan bypass to new canonical mode: it must account for every output explicitly (the no-query preparation case still cannot become proof-eligible).

For nonempty canonical plans, derive a seen-commitment set from source-ordered intents and require an exact bijection with approved persistent output rows: same count, each commitment once, correct actual index and typed recipient/public owner. Check next_index=start+count with checked arithmetic. This is equality of the complete output set with identities and allocation, not subset matching. Missing/trailing wallet change/extra foreign outputs fail. Repeated commitments fail even when the repeated intent has a different position. The source intent sequence is retained as execution evidence; it is not zipped or sorted for execution.

Contract input commitment/tree owner/nullifier validation remains exactly188. Wallet input selection and equality remain exactly199. Transients are rejected in205's canonical policy; default legacy behavior remains as documented. Do not treat a0 index as transient or read leaf0 for an immediate coin.

Additional owner checking is an early rejection for inconsistent public output metadata. Current proof/default-strict ledger validation already checks output proof ownership; this is not a claim that previously invalid ownership could yield an accepted transaction. A correctly bound foreign-contract recipient is not itself malformed allocation; missing foreign receive-claim composition may still make its transaction invalid. Test the owner mismatch, not an invented prohibition on all external contract recipients.

### Validation and smallest source scope

Runtime tests must cover both source/normalized orderings at a nonzero start, same commitments emitted in different source order, exact source private output sequence, and immutable initial/final maps. Default and199 wrappers must still reject the reversed order with their existing error, while explicit canonical options accept it. Test canonical+explicit-wallet options without extra outputs to show composition preserves199 exact inputs.

A small two-output Unit fixture derived from188's existing transfer primitives can establish the critical stored-index property without waiting for new compiler support: one funded contract input42, two distinct outputs17+25, correct nullifier/spend claims and self receive claim for the change, followed by `pot.writeCoin(change, self_recipient)`. Supply the existing typed commitment/recipient operands as parameters, as188 does; no new evaluator, hashing primitive or emitter admission is needed. Capture its provisional native/TS execution unchanged. Run the same source/proof key twice with nonce/recipient vectors exercising both commitment sort orders. The two cases must independently prove, verify, reject changed bindings, balance separate Dust, pass unchanged default strictness and ledger.apply. Inspect both actual output leaves/owners, spent nullifier, first_free, exact unchanged source intent order and qualified pot index from the authoritative allocation. A no-store two-output proof alone would miss the index-sensitive query boundary.

After203 lands, its partial-send composite can reuse the canonical options and pinned keys for an additional helper-level application test. Do not make the runtime foundation depend on widening203's emitter to add qualified storage; use the188-style fixture for that property. The existing203 same-order/reverse-order captured rows are independent primitive/intent evidence, not substitutes for the proposed funded proof gate.

Negatives: duplicate commitment in complete offer (exact upstream CommitmentAlreadyPresent), duplicate output intent (no plan/map/private output mutation for refused append), missing output, trailing change/extra foreign output, wrong coin/recipient, owner metadata changed without changing commitment, wrong actual index, duplicate allocated index in internal mutation tests, changed public context map/frontier/address, changed final plan/snapshot/allocation policy, binding to a different observation or offer, unnormalized retained offer, and transients. Preserve188/199 wrong input index/coin/owner/nullifier/selection negatives and184/188/191 legacy empty-plan controls. Strict proof scope is explicit offline funded genesis plus separate Dust, not network/finality.

Raw TS/provisional and authoritative bound execution have deliberately different index maps for reverse order. Do not claim byte-for-byte qualified state/query-result parity between those contexts. Compare source operation order/coin identities/private sequence; verify index-sensitive bound query results by upstream VM replay against the sealed map and by strict ledger application. No handwritten VM or transcript patching.

### Ownership, compatibility and ABI recommendation

Runtime files: zswap.rs, context.rs, transaction.rs and transaction/zswap_tests.rs; new proof/fixture files and tiny harness registrations. No shared typed planner edits expected. Coordinate main.rs/Cargo entries mechanically with203.204 waits for the runtime lease and reuses the same sealed row/count contract; its separate source event provenance and ordinary/transient partition are not included here. Ledger allocates all persistent outputs before transients, so later204 must derive physical indices from this map and never equate source ordinal or transient singleton proof index0 with global allocation.

Recommend keep schema20 and ABI48 for this first opt-in additive runtime API, consistent with199: old constructors, default semantics and generated method signatures stay unchanged, and attempting to call the new options API with an old runtime fails at compile time. If parent policy instead treats a new explicit allocation capability as an ABI milestone, reserve the next ABI centrally and refresh mechanically there; do not assume a new number while203/204 run. Any later generated code that depends on canonical mode without an explicit new public API would need a capability/ABI gate.

### Primary code references inspected

- Integrated runtime: `runtime-rs/src/transaction.rs:159–310`, `context.rs:493–541`, `zswap.rs:19–69`, `ledger/cell.rs:254–312`.
- Pinned registry `midnight-zswap-8.0.3/src/construct.rs:431–453`, `structure.rs:299–310,478–480,564–579`, `ledger.rs:94–120,176–208`, `verify.rs:293–308`.
- Pinned VM `midnight-onchain-runtime-3.0.0/src/context.rs:850–880` exposes the immutable commitment map to canonical query programs.
- Original standard library `compiler/standard-library.compact:161–196`, raw `runtime/src/zswap.ts:247–269`.

Await design review before205 ADR/issue or implementation.


### Implementation and strict evidence — 2026-10-06

Issue: https://github.com/MediaNoxLabs/compact/issues/309

Created ADR205/issue before code and based the isolated branch on root70251e9a. The two-output Unit fixture passed strict recorded source admission before runtime edits; no shared planner/emitter changes were needed. Both source exports (distribute/read_coin) remain proof-required and recorded.

#### Exact public API
```rust
let options = OfferBindingOptions::default()
    .with_output_allocation(PersistentOutputAllocation::CanonicalOfferIndices);
let bound = OfferBackedObservedState::with_options(observed, &ledger, offer, options)?;
let call = contract.recording.distribute_call(bound.observed(), private_state, /* inputs */)?;
let prepared = bound.prepare(call, verifier, randomness)?;
```
`options.with_wallet_funding(exact_upstream_inputs)` composes the existing typed selection. Existing new/with_wallet_funding constructors keep ExactIntentOrder. ABI48/schema20 and generated call signatures are unchanged.

Private CanonicalOfferBound rows seal each ordinary output's commitment, upstream actual index and public owner. The caller's normalized complete offer and full map remain immutable. Source intent/private/query order stays unchanged; append selects the row's actual index and rejects duplicate/unknown commitments or inconsistent recipient/owner/map before mutation. Reconciliation requires exact output cardinality and unique complete matching, source-plan snapshots, unchanged allocation/maps/address, and existing input commitment/owner/nullifier/funding checks. Canonical mode refuses transients and unnormalized offers. Added owner checks are early rejection; proof/ledger ownership validation remains authoritative.

#### Empty-plan boundary clarified during review
Canonical mode explicitly returns CanonicalEmptyPlan for empty plans, including empty offers and no-query calls; it cannot use the legacy fast path. Default no-query calls retain Prepare(EmptyTranscript); default funded mode retains WalletFundingEmptyPlan. Tests exercise public prepare, query/no-query, and actual constructor-created empty/nonempty offers. Default legacy empty-plan offer calls remain supported. This accepted precedence follows the explicit199 policy boundary; no fabricated public query is inserted.

#### Independent and authoritative evidence
The new raw TS capture uses corrected runtime0.16.101, pins its compact-types.js hash, and retains provisional source-order indices. Two TS/native/recorded rows compare full state, exact public operations, intents, private Unit order and gas. Claimed-shielded-spend effects are an upstream set: only that effect field is compared as a set, while exact query order is separately asserted. Initial serialization-order test failure is retained at ${LOCAL_EVIDENCE}/compact-adr205-parity-first.log.

Two distribute proofs (k14,9991 rows,4480 bytes each) independently verify and reject changed bindings. Funded contract input42 creates user17 and self-change25. Source order sent/change gives actual indices[1,2] in one case and[2,1] in the other; logical progress remains3 and qualified pot stores change index2/1 respectively. Both transactions use separate Night-backed Dust, unchanged WellFormedStrictness::default and real ledger.apply. Full state, both leaf owners/commitments, frontier, stored coin contents, nullifier consumption and exact replay refusal are asserted. Seed coin is an explicit offline genesis prerequisite, not proof of a prior transaction/network finality. Raw provisional TS indices are not falsely equated with the reversed canonical context.

Keys ${LOCAL_EVIDENCE}/compact-adr205-proof; compiler ${LOCAL_EVIDENCE}/compact-adr205-compactc; proof log ${LOCAL_EVIDENCE}/compact-adr205-proof.log. Test-only borrowed-owner compilation failure retained in ${LOCAL_EVIDENCE}/compact-adr205-proof-build-first.log. Runtime ledger-transaction lib22 tests, standalone native check, targeted Clippy,34Python tests,7-source local parity20/20 proof APIs+3 native-only exports, source-specific Cargo guard2/2, and full173-fixture freshness pass. Parent and independent204 review found no remaining actionable issue. Final legacy controls/signature/receipt follow below. No push or remote CI.


### Signed local delivery

Commit `b0dc2af72ab94fc5e7a50289584164b8c0ad7d46` is GPG verified, DCO signed with explanatory conventional body, and clean. Runtime lease released directly to204;204 can cherry-pick this commit after root70251e9a.

All final controls pass:199 wallet-funded receive,188 transfer,180 funded Set/Cell,184 funded mint (positive default strictness), plus their existing explicit negative/smoke boundaries. Exact-head source gate2/2; broader7-source20/20 proof APIs+3 native-only;173 fixtures fresh;22 runtime lib tests; standalone native check; targeted Clippy;34Python; formatting. Canonical source capture2 cases and strict proof2 sort orders retain the distinct scopes above.

Keys: `${LOCAL_EVIDENCE}/compact-adr205-proof`
Compiler: `${LOCAL_EVIDENCE}/compact-adr205-compactc`
Receipt: `${LOCAL_EVIDENCE}/compact-adr205-delivery-receipt.json`
SHA256: `78f66e536442e503a37f6f48f3817e9d2cd3c8d4879adcf96d99c6958573ddb9`
Exact-head gate: `${LOCAL_EVIDENCE}/compact-focused-adr205-exact-head/receipt.json`
Broader gate: `${LOCAL_EVIDENCE}/compact-focused-adr205/receipt.json`

Receipt pins all changed source/capture/generated files, keys, ZKIR, compiler and logs including failed attempts. No push or remote CI.


### Canonical persistent allocation integrated — 8e7080f5 (2026-10-06)

ADR205/[#309](https://github.com/MediaNoxLabs/compact/issues/309) is integrated as GPG/DCO-verified `8e7080f5`. `OfferBindingOptions` adds an explicit canonical allocation policy alongside the existing wallet funding selection. Source-ordered output intents bind to immutable upstream commitment/index/owner rows. Their physical Merkle indices can differ from source order; logical cursor progress remains start plus the number of outputs. Existing constructors retain exact-order behavior. Schema 20 and ABI 48 remain unchanged.

Root and independent agent reviews found no remaining actionable issue after the canonical empty-plan guard was added. Main exact-commit verification passed:

- Ten affected transaction tests, a seven-source parity gate (20/20 proof-required APIs and three native-only exports), and targeted strict runtime/fixture/proof Clippy.
- Two 4480-byte call proofs verify and reject changed bindings. Separately Dust-funded 42→17+25 transactions pass default-strict validation and ledger application in both normalized output orders. Stored qualified change indices are checked as 2 and 1 respectively; output ownership, complete state, input nullifier consumption and replay rejection are checked. The previous contract-owned input is an explicit offline genesis prerequisite, not a claimed earlier transaction.
- All 173 generated fixtures are fresh. The independent delivery also ran all 22 runtime library tests, 34 Python tests, and legacy funded wallet receive, transfer, Set, Cell and Kernel mint controls.
- Whole-source inventory: **372/379 proof-required APIs available, seven explicit gaps**, across 214 sources / 748 exports / 193 compiled roots / 369 nonproof exports. Zero unassessed, missing or unmatched rows, and no baseline drift. The two new APIs expand the tested corpus; the seven original microDAO/Coracle gaps remain.

Main receipt: `${LOCAL_EVIDENCE}/compact-8e7080f5-integration-receipt.json`; focused gate: `${LOCAL_EVIDENCE}/compact-focused-8e7080f5/receipt.json`; inventory: `${LOCAL_EVIDENCE}/compact-8e7080f5-inventory.json`. Full workspace and portable package acceptance remain at **7a311c9f**; current evidence is a focused integration. No push, publication or remote CI. The user-owned documentation edit remains unchanged.

ADR203 has released its reviewed emitter for ADR204 while finishing partial-send proof controls. ADR204 now owns the shared emitter and runtime for the bounded receive→full immediate-send transient slice. The next original-contract composition research covers Coracle concede/withdraw and microDAO cash_out.
