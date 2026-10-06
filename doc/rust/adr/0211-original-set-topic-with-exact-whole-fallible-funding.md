---
id: RUST-ADR-0211
alias: ADR-0211
title: "Original set_topic with exact whole-fallible funding"
date: 2026-10-06
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["recording", "microDAO", "fallible-offer"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: f6d77d73ff2510e5e26181c5735a10d3a5ad40e1a045d287680cad20e8ba67f2
---
# RUST-ADR-0211 — Original set_topic with exact whole-fallible funding

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Original microDAO set_topic records through a closed profile and an explicit whole-fallible wallet/transient offer policy with exact disjoint coverage. Empty and occupied seeded strict paths and rollback pass; this does not imply a full DAO lifecycle.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#315 closure](https://github.com/MediaNoxLabs/compact/issues/315#issuecomment-6017764068). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`3404c30c`](https://github.com/MediaNoxLabs/compact/commit/3404c30c965248d69f07e759e22902fc4faf5de6) · [`53eb9a1e`](https://github.com/MediaNoxLabs/compact/commit/53eb9a1e6e329ca9f224de41396dbe5320921e3d) · [`57a8d92b`](https://github.com/MediaNoxLabs/compact/commit/57a8d92b5fe4c30a4dacfd0cb5c792d80e89418a) · [`eb7c55ce`](https://github.com/MediaNoxLabs/compact/commit/eb7c55ce61a3d39aecfa2887db32ab76aac67a8e). These include historical prerequisites and probes, not only final delivery commits.

## Historical decision and amendments

### Original set_topic integrated — 53eb9a1e (2026-10-06)

ADR211/#315 is integrated as signed/DCO eb7c55ce and 57a8d92b, followed by the four signed independent preparation commits for ADR212–215. The focused root gate passed at exact head 53eb9a1e6e329ca9f224de41396dbe5320921e3d: runtime all-target tests, 46 backend unit/13 CLI/153 renderer tests, 34 Python tests, 13 focused commands over six sources and strict five-package Clippy. The focused source subset has 16/20 proof-required APIs recorded. Wider inventory is **382/386**, with four explicit gaps: original cash_out, start, vote_commit and buy_in. It retains 217 sources, 755 exports, 196 compiled roots, 369 nonproof exports and 1,024 declarations, with zero unassessed, missing/unmatched metadata or baseline drift.

The original set_topic source delivery separately proves the empty guaranteed and occupied whole-fallible funded paths, using actual wallet/historical/transient/output components, separate Dust, default-strict ledger application, canonical pot indices, replay refusal and exact fallible rollback. Admission now refuses seed shadowing, changed received-coin forwarding and path-local missing/extra qualified stores. Schema20/ABI49 unchanged. Root receipt: ${LOCAL_EVIDENCE}/compact-53eb9a1e-integration-receipt.json; focused: ${LOCAL_EVIDENCE}/compact-focused-53eb9a1e/receipt.json; inventory: ${LOCAL_EVIDENCE}/compact-53eb9a1e-inventory.json; source delivery: ${LOCAL_EVIDENCE}/compact-adr211-delivery/receipt.json (SHA256 0d0f36a58227a7e2b880ef418c87cf0e066a1eefdd23114bab21c21e34a6c508).

The latest complete full/portable checkpoint remains 3404c30c (374 commands, 176 fresh fixtures, 18 portable checks). This later focused checkpoint does not substitute for final same-head full acceptance or live-wallet work. No remote CI or push; user documentation remains untouched.

#### Parallel delivery revision

ADR212 cash_out, ADR213 start and ADR214 vote_commit now implement independent, narrowly scoped profile modules on isolated branches from 53eb9a1e. Their runtime prerequisites are already integrated. Root serializes integration, reviews any shared typed-Plan routing overlaps and regenerates combined source fixtures. Each agent owns a separate warm Cargo target. ADR215 buy_in preparation is integrated and its implementation follows the first available lane. This replaces our internal serial emitter queue; it does not relax source/refusal/proof acceptance.


Status: Accepted for implementation, 2026-10-06. Milestone: rust-backend-v2.

### Problem

The unchanged original microDAO set_topic combines declared ledger Cells, authority witnesses, receive, conditional merge and qualified writes. Existing recording profiles do not compose that export. Independent pinned TypeScript evidence shows empty-pot calls are wholly guaranteed but occupied-pot calls are wholly fallible; existing ADR206 placement explicitly rejects the exact wallet/transient funding the latter needs.

### Before / after

Before, generated `ledger_contract::set_topic(...)` is native-only, with no recorded/observed-call API. `ContractTransientCoins::from_transients(...)` accepts only guaranteed proof halves and `OfferPlacement::Fallible(s)` rejects all wallet/transient selections.

After, the original generated API will include typed `recorded::set_topic(context, witnesses, topic, beneficiary, seed)` and `recorded::Contract.set_topic_call(observed, private, witnesses, ...)`. Consumers opt into an exact retained offer, explicit WalletFundingInputs, placement-aware ContractTransientCoins, canonical output allocation and the observed whole-fallible segment. No VM instruction program is a user API input.

```rust
let selected = ContractTransientCoins::from_transients_for_placement(
    vec![actual_transient], OfferPlacement::Fallible(segment),
)?;
let options = OfferBindingOptions::default()
    .with_output_allocation(PersistentOutputAllocation::CanonicalOfferIndices)
    .with_offer_placement(OfferPlacement::Fallible(segment))
    .with_wallet_funding(actual_wallet_inputs)
    .with_transient_coins(selected);
let bound = OfferBackedObservedState::with_options(observed, &ledger, actual_offer, options)?;
```

The original guaranteed constructors remain compatible. The policy never retargets proofs, fabricates balances or interprets singleton index zero alone as authorization.

### Emitter / runtime ownership

Runtime work is scoped to transaction.rs and transaction/{placement,transients}.rs, independent of ADR210 private RecordingFrame storage. Shared Plan, typed declaration-directed call resolution and the ADR207 merge arithmetic/singleton audit remain the only evaluator; the bounded original-source admission waits for ADR209's emitter handoff. ABI49/schema20 remain unchanged unless actual new emitted runtime requirements arise, which requires explicit review.

### Accepted detailed design and validation

## ADR211 proposal — original microDAO set_topic and whole-fallible selected funding

Status: read-only proposal, awaiting parent review. No ADR, issue, repository code edits or builds yet. This follows delivered ADR207; ADR209 owns the shared emitter and ADR210 owns private recording storage. Runtime work can stay in `transaction.rs`, `transaction/placement.rs` and `transaction/transients.rs` without touching their leases.

### Original-source target and independent evidence

Target remains unchanged `test-center/test-contracts/micro-dao.compact`, original `set_topic` (lines 144–161), not a renamed/reduced wrapper. Existing `${LOCAL_EVIDENCE}/compact-set-topic-probe.json` contains 16 independent corrected-TS cases: five successes and eleven rejections. `${LOCAL_EVIDENCE}/compact-set-topic-proposal.md` preserves source order, query/witness prefixes and pinned JS partition observations. Empty pot is wholly guaranteed (44 operations); occupied pot is wholly fallible (71 operations). A one-export empty-path proof alone does not close the occupied path.

Original operation order: configured seed value/color assertion; secret witness; authority hash/read assertion; setup phase assertion; receive(seed); selected empty writeCoin+Boolean or historical pot read+immediate merge+writeCoin; optional topic and beneficiary writes; phase commit. Costs is a declared two-Uint64 struct Cell; pot is a QualifiedShieldedCoinInfo Cell. A bounded shared-Plan domain must combine these already implemented leaves and helper declarations, including exact merge arithmetic, without enabling arbitrary effects or losing branch-local ordering. All bindings/arguments/branches/helper bodies need audit. Runtime placement must follow upstream prototype partition, never a compiler branch-to-segment rule.

### Exact segment shapes from pinned sources

Pinned crate root: `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/midnight-zswap-8.0.3`.

| Retained upstream component | Public return vector at logical segment s | Ownership boundary |
|---|---|---|
| Wallet persistent Input | `[Fr(1), Fr(s)]` | contract_address is None; exact caller-selected full Input |
| Historical contract Input | `[Fr(1), Fr(s)]` | contract_address equals observed contract; historical commitment/index/path and nullifier checked |
| Persistent Output | `[Fr(s)]` | selected output owner/commitment matches ordered circuit intent and sealed allocation |
| Contract transient input half | `[Fr(1), Fr(s)]` | singleton input index zero; selected full Transient, same observed contract |
| Contract transient output half | `[Fr(s)]` | same coin created earlier in this call, identical owner and full retained output proof |

The leading 1 in Input output is NOT an owner discriminator: `local.rs:200–230` calls the same `Input::new_from_secret_key` constructor for wallet spending that contract inputs use. `construct.rs:210` builds `[true, segment]` for both. Persistent output returns are at `construct.rs:363`. `Transient::new_from_contract_owned_output` (`construct.rs:388–413`) creates its input using the supplied segment but retains the independently supplied output proof, so it does not establish matching segments by itself. The new policy must compare both exact vectors; `segment()` alone is insufficient for malformed lengths or overflow values. No retargeting after offer binding, no rewriting proof vectors to fake a segment, and no owner inference from the leading scalar.

### Proposed public policy and implementation ownership

Keep `OfferBindingOptions`, `OfferPlacement::Fallible(NonZeroU16)` and canonical allocation as the existing independent policy dimensions. Extend whole-fallible placement only when wallet/transient selections are explicitly supplied using existing typed selection options. Existing persistent-only fallible calls remain accepted; implicit extra funding/transients remain refused. There is no new allocation mode, no acceptance of mixed guaranteed/fallible call prototypes, and no automatic branch inference.

Add a placement-aware `ContractTransientCoins::from_transients_for_placement(coins, placement)` constructor (final spelling can follow review). The old `from_transients(coins)` delegates to guaranteed mode and continues to reject nonzero tags. Store the selection's expected placement privately and require it to equal the bound offer placement. Both halves must match `[1,s]` / `[s]`. Even guaranteed binding must reject a newly selected fallible transient carrier. Constructor/selection remains identity validation, not proof verification.

`placement.rs` validates exact input/output/transient tags for the entire retained fallible offer. It must not require all inputs to be contract owned: wallet ownership comes from explicit full-input selection. It continues to require canonical allocation, a nonempty offer, and a nonzero segment. The transient count must be fully selected; options with empty selections cannot bypass validation.

`transaction.rs` passes immutable placement to transient validation; retains full offer equality, normalized-offer check, observed contract equality, sealed allocation and prepare pointer/initial/final context binding. `into_transaction` continues consuming the same retained offer into one fallible map entry at s and the call intent at s. No guaranteed shielded offer is inserted. The separate Dust balancing helper may add a Dust-only physical intent, as in ADR206; assert it has no contract calls or shielded/unshielded offers. This distinction avoids claiming the fee-balanced transaction has literally one physical intent.

No new emitted runtime method is expected. ABI49/schema20 remain valid if only additive consumer construction and private policy storage change. Any actual generated requirement would trigger explicit reassessment before emission changes.

### Complete disjoint coverage and allocation invariants

Let H be historical contract inputs materialized by this call, W caller-selected wallet Inputs, T caller-selected full Transients, O persistent outputs. The retained normalized offer must satisfy:

- Offer persistent inputs are exactly H disjoint-union W; historical entries are matched to actual pre-state tree index/commitment/owner and contract nullifier, while W matches full upstream Input equality (proof preimage included).
- The circuit input intents are exactly H plus one consumed input for every T. No repeated or extra nullifier, wallet/transient overlap, transient/persistent overlap, missing selected entry or implicit extra wallet coin.
- The circuit output intents are exactly O plus one previously created output for every T. Existing private events preserve actual source order and require each transient creation before consumption; singleton input zero is justified by that selected actual transient, not by the integer alone.
- T matches the complete retained upstream values, both proofs/value commitments/ciphertext/owner included. Each T is in the same logical segment and observed contract; a different proof with the same commitment is not equivalent.
- `ledger.zswap.try_apply` on that exact sole offer derives canonical commitment indices from the captured pre-state frontier. Do not relabel source-order provisional indices as authoritative. In occupied set_topic at frontier 2, merged persistent output is index 2 and received transient is index 3, while raw TS provisional order is the reverse.
- All outputs retain their sealed owner/index association and logical progress count. The policy does not fabricate funds, normalize away missing claims or skip ledger validation.

### Minimum meaningful strict original-source vertical

Both cases are necessary for completion, use original set_topic keys, original contract state and default ledger parameters/strictness. Historical inputs are explicitly seeded offline, not a proved DAO lifecycle.

1. Empty pot control: original constructor with organizer secret4, Costs(seed_dust=10,buy_in_dust=3), setup phase and no pot. Seed a genuine wallet coin of native color/value10 plus independent padding so frontier exceeds1. Use a real wallet Input and actual contract Output at guaranteed tags. Original set_topic must partition wholly guaranteed. Bind canonical indices with explicit wallet funding, prove original contract and Zswap components, fund fees separately with Night-backed Dust, verify/apply. Assert topic, beneficiary, phase, pot_has_coin and actual qualified index; reject replay.
2. Occupied pot acceptance: seed historical contract coin17 and wallet coin10 in the same actual tree, install the exact historical qualified coin into original pot, retain setup phase and same costs. At frontier2 construct historical Input and wallet Input with Some(1), received Output with Some(1), Transient with input Some(1) retaining that output, and merged persistent Output27 with Some(1). Original set_topic must partition wholly fallible; retained offer has persistent H+W, one T, and one merged O. Bind it once, record/prove original call, verify all upstream proofs, add only separate Dust funding, run default-strict well_formed and ledger apply. Assert canonical merged index/owner, pot metadata, all three spent nullifiers and replay refusal. At least the occupied case must be a full original-source proof, not only a manual RecordingFrame imitation. Independent TS capture may prepare original proof inputs before emitter handoff, but final acceptance requires generated recorded Rust evidence too.

### Rollback and refusal tests

From a fully proved occupied call, change a relevant public Cell in the application state (e.g. setup phase or costs), preserving the proof reference state for the verification/application concurrency scenario as in ADR206. Require the precise fallible `ReadMismatch` path, not arbitrary failure. Segment0 replay/Dust processing may persist; segment1's whole offer and contract mutation must roll back atomically: Zswap state/frontier/root/nullifier set equal pre-application state, wallet and historical inputs unspent, no transient/merged output installed, changed public pre-state retained. Transaction replay protection can still prevent reusing the same failed transaction; do not claim automatic wallet/private-state rollback. Off-chain witness side effects remain caller-owned.

Focused negatives: wallet `[0,s]`, wrong length/overflow/wrong segment for every component; mismatched transient halves; full selected-proof substitution with same commitment/nullifier; wrong owner; duplicate/missing/extra W/H/T; history path/index mismatch; output-before-input violation; sealed map/events/observation substitutions; no selections with extra offer components; whole guaranteed or mixed prototype under fallible mode; post-proof move to guaranteed or another segment; missing/extra ledger claims; insufficient balance under default strictness. Reuse existing199/204/205/206 negatives when unchanged, adding only fallible cross-policy combinations and actual original-source rollback.

### Delivery boundaries

After approval create ADR0211 and milestone issue before code. Runtime edits can proceed independently of ADR210's recording core and ADR209's emitter; source/capture/fixture/proof preparation is separate. Wait for the ADR209 shared emitter handoff before adding the bounded set_topic admission. Preserve the 16-case raw evidence and separate provisional execution, authoritative replay, proof, application and rollback receipts. No buy_in, Coracle start or general multi-offer/multi-call transaction policy is included.


### Runtime and independent preparation checkpoint

Issue: https://github.com/MediaNoxLabs/compact/issues/315. ADR was created with the vault selector first and its physical file was verified before implementation. ADR210 private storage is included as signed dependency `dbfec345` (original `111fe5f3`).

Scoped transaction runtime changes implement the approved placement-aware transient carrier; its private placement is checked even before the Guaranteed early return. Whole-fallible canonical offers may carry only explicit full wallet/transient selections. Exact vectors are checked for both persistent input owner classes and both transient halves. Nineteen transaction tests pass, including actual historical+wallet+transient construction and an empty-history wallet+transient control at nonzero segment 7. Runtime strict Clippy passed. These are structural/binding tests, not a new cryptographic proof claim.

Original unchanged microDAO set_topic's 16 independent TS/native cases pass. Existing raw research evidence is preserved separately. The retained original set_topic keys are k16 / 41,052 rows at `${LOCAL_EVIDENCE}/compact-adr211-proof`; the new two-path proof module is currently unregistered and has not been compiled/executed. Generated recording remains unavailable until the ADR209 emitter handoff and bounded shared-Plan admission.


### Shared emitter and first strict proof checkpoint

ADR209 preparation and implementation were integrated after its signed handoff. A distinct `GuardedShieldedDeposit` domain reuses shared Plan evaluation and the ADR207 received-merge/singleton declaration audit. New admission covers typed Costs reads (two Uint64 members), exact Uint64-to-Uint128 cost widening, optional String/key writes and one received seed with an optional historical merge. No new evaluator or emitted runtime method; ABI49/schema20 unchanged. All arguments, bindings and branches are audited, with typed lexical scope and declaration-directed pure/helper resolution preserved.

Before: unchanged `set_topic` compiled native Rust but capability metadata rejected recorded/observed access at its first assertion. After: `contract.recording().set_topic_call(observed, private, topic, beneficiary, seed)` records the original body; full-source strict recording still refuses the unrelated vote_commit gap.

First strict run `${LOCAL_EVIDENCE}/compact-adr211-proof-first.log` passed both original empty and occupied paths, each with a 4,480-byte verified original call proof plus complete funded Zswap and Dust proofs, default well_formed and ledger application. Occupied case rejects changed proof placement and checks exact segment-1 ReadMismatch rollback with segment-0 Dust/replay persistence. This is an explicitly seeded prior-state vertical, not a full DAO lifecycle.

Raw provisional replay initially failed with ExpectedCell(null) when the replay context lacked the commitment map; the independent TS capture already installs its provisional map. The recorded fixture now does the same explicitly, while the strict proof retains only actual canonical upstream offer allocations. This preserves, rather than masks, the provisional/authoritative boundary.


### ADR211 local delivery — original set_topic and exact fallible funding

Signed GPG + DCO commits: `c818fde7` (runtime and independent preparation), `154750e7` (recording, guards and strict original-source proof). No push or remote CI.

The unchanged original export now records through the shared typed Plan. Its bounded admission rejects seed/formal shadowing, changed receive-output forwarding, malformed slots/types, hidden effects/cycles, and relocated writes that would leave one branch with zero stores and another with two. Each selected path has one qualified store and at most one merge. Actual owner/commitment/claim and full transient proof identity remain runtime/ledger validation responsibilities.

Runtime policy retains the exact caller-selected upstream offer: full wallet/history/transient disjoint coverage, exact `[1,s]` input and `[s]` output tags for both owner classes and both transient halves, immutable canonical allocations, and whole-fallible placement only. Guaranteed defaults are preserved; no proof retargeting. ABI49/schema20 unchanged.

#### Evidence

- 16 independent corrected-TS/native/recorded/replay cases; exact VM/state/private/effects and query-sum gas, with wrapper/replay gas separately preserved.
- Both original empty (44 ops, guaranteed) and occupied (71 ops, fallible) paths verify 4,480-byte contract proofs plus complete Zswap/Dust proofs, default-strict well_formed and ledger application.
- Occupied path checks exact fallible ReadMismatch rollback with unchanged Zswap/contract state and retained guaranteed Dust/replay effects. Wrong proved offer placement and spent-nullifier replay reject.
- 46 backend unit + 13 CLI + 153 renderer tests; 34 Python and 31 runtime unit tests; strict affected-package all-target Clippy.
- All 176 fixtures fresh. Exact signed-head six-source gate passed 13 commands, 14/18 proof-required recorded APIs.

Keys and both sealed transactions are retained in `${LOCAL_EVIDENCE}/compact-adr211-proof` (sealed files 18,820 and 34,269 bytes). Full receipt: `${LOCAL_EVIDENCE}/compact-adr211-delivery/receipt.json`, SHA256 `0d0f36a58227a7e2b880ef418c87cf0e066a1eefdd23114bab21c21e34a6c508`.

Prior coins and contract state are explicitly seeded offline; this does not claim a complete DAO lifecycle. Original microDAO now records 4/7 proof-required APIs; vote_commit, buy_in and cash_out remain separately assigned gaps. Shared emitter lease has passed to ADR212.
