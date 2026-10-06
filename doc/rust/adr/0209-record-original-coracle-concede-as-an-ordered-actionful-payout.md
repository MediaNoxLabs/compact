---
id: RUST-ADR-0209
alias: ADR-0209
title: "Record original Coracle concede as an ordered actionful payout"
date: 2026-10-06
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["recording", "Coracle", "payout"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: a0e07a7ec9ad420ee52afeac6e4849ad2e2c49146c4a6c400c642f0d6743e59b
---
# RUST-ADR-0209 — Record original Coracle concede as an ordered actionful payout

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. A distinct actionful payout domain records original Coracle concede with branch-local Board witnesses, exactly one winner Cell write and selected qualified send. Seeded red/blue strict payouts pass; no full prior game lifecycle is claimed and the source proof log has a documented retention limit.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#313 closure](https://github.com/MediaNoxLabs/compact/issues/313#issuecomment-6017760585). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`3404c30c`](https://github.com/MediaNoxLabs/compact/commit/3404c30c965248d69f07e759e22902fc4faf5de6) · [`4bc6bf95`](https://github.com/MediaNoxLabs/compact/commit/4bc6bf95b7853be6d4718989ea65b12cb878e815) · [`86cd2dd9`](https://github.com/MediaNoxLabs/compact/commit/86cd2dd9fc93014b99e714117c83eb6035e0aa52) · [`b1469975`](https://github.com/MediaNoxLabs/compact/commit/b146997522fa3ab986a89e3528bd3d26b05f87fa). These include historical prerequisites and probes, not only final delivery commits.

## Historical decision and amendments

Status: approved bounded preparation, 2026-10-06. Milestone: `rust-backend-v2`. Based on main `86cd2dd9` (schema20/runtime ABI48); implementation follows ADR206 and ADR207 shared emitter handoffs. Full read-only source/ledger analysis: [Original Coracle concede — guaranteed actionful payout research — 2026-10-06](references.md#private-note-13).

### Problem

The unchanged original `test-center/test-contracts/coracle.compact::concede` compiles and runs natively, but its exported proof-required Rust API has no recorded/observed call. ADR206's shielded payout profile admits only actionless helper calls. `red_concede` and `blue_concede` each run a second secret witness, a Board witness, four assertions, and one enum winner-state Cell write **before** reading the selected qualified deposit and returning `sendShielded(...).sent`. Dropping the action list, hoisting the write, or evaluating both helpers changes source behavior and private/public order.

### Decision

After ADR206 withdraw and ADR207 merge have released the shared emitter, extend the existing typed payout Plan with a separate bounded actionful helper mode. Audit the full selected value-returning helper structure, and adapt its ordered `actions + StateReturn::Expression` to the existing typed `ReturnPlan` continuation. The shared Plan evaluates each witness, assertion, Cell write, native OwnPublicKey leaf, qualified deposit read and send once in source order. The outer `If` evaluates authentication once and executes only the selected branch. Admission is by typed structural/effect rules, never source path or helper name.

The bounded action tree allows typed Let/Sequence, declared Bytes32 and Board-shaped two-Field witnesses, typed assertions, and exactly one canonical root Enum Cell write before one full qualified send returning ShieldedCoinInfo. Recursively inspect unused bindings, unselected branches, argument expressions and all pure/stateful callee bodies. Reject missing/ambiguous/cyclic calls, hidden collection/extra intent/write effects, wrong slot/type, lexical escape, and malformed witness result. Keep ADR206 withdraw read-only and existing send profiles unchanged. No new runtime method, IR variant or public generated requirement is expected, so schema20/ABI49 after ADR206 should remain; revisit only if implementation demonstrates a real new requirement.

### Before and after for a Rust consumer

Before, the original source offers native execution only:

```rust
let result = contract.concede(context_with_coin_public_key)?;
// No recorded call to prepare, prove or apply.
```

After the bounded profile, the generated recorded/observed call follows the existing ADR206 execution-key and offer-binding API (illustrative names until emission is finalized):

```rust
let observed = observed.with_coin_public_key(recipient_key);
let bound = OfferBackedObservedState::with_options(/* exact prior state and offer */)?;
let call = contract.concede_recorded(bound.circuit_context(private_state))?;
let prepared = bound.prepare(call)?;
```

This does not grant wallet-spend authority: caller-selected key controls the source's `ownPublicKey` recipient; real ledger coin ownership and contract-nullifier proofs remain independently checked. The source itself authenticates the player with its local secret and board commitment.

### Segment and offer policy

Corrected TypeScript source execution with exact serialized pre-call state and captured output commitment map partitions both red/blue 42-op public transcripts under pinned ledger-v8 initial parameters as **guaranteed-only**. Each has one contract-owned qualified input and one user output, no change/transient, and claimed nullifier/spend in segment 0. Use default strict exact-offer binding and segment-0 Input/Output proof preimages. Do not inherit ADR206 withdraw's explicit whole-fallible segment mode. Assert actual partition again at proof time against the precise prestate and parameters. Initial fixture states are explicitly seeded game/coin states; they do not prove funded `start` lifecycle or network finality.

### Evidence and acceptance

Preparation before shared emitter handoff: independent corrected-TS capture of unchanged source, original generated crate native tests/support, and original `concede` proving keys. Preserve source/generated/runtime provenance, red/blue success and selected failure ordering. No recorded success or ledger application claim during preparation.

Delivery after handoff: native/recorded/replay parity for red/blue and both-authenticated red preference; invalid player, wrong board/nonce/turn, alive, missing execution key, witness failure and gas boundaries. Assert exact complete state bytes, returned coin, ordered VM/query operations, all six private output alignments, four-dimensional query gas, one input/output, and error prefixes. Preserve winner-state write before deposit reads/key/send; no successful prepared call after later failure. Check original compiler capability change from recorded unavailable to available without losing prior fixture APIs. Add malformed schema20 IR negatives, including unsupported effects in unused/unselected paths.

Then prove and independently verify one original red and blue call with real contract-owned selected deposit, exact output commitment/owner/index, separate NIGHT-backed Dust, default strict ledger apply, changed binding rejection and spent/nullifier replay refusal. The unselected deposit remains unspent; only the source winner enum Cell changes. No remote CI or push is part of this milestone slice.

### Why cash_out follows later

Original microDAO `cash_out` sends first, then invokes `reset_state(true)` with Counter, Set, Merkle and multiple Cell resets. Its lazy majority assertion also reads Counter and Maybe state. Concede's single pre-send enum write is the smallest reusable actionful payout boundary. Do not broaden this ADR to `cash_out` or infer its transaction partition.

Issue: to link after creation.

MediaNoxLabs issue: https://github.com/MediaNoxLabs/compact/issues/313 (rust-backend-v2).
Preparation checkpoint, 2026-10-06: Signed DCO/GPG commit 37bd4d08aaa9cec3b9c131a55db384a8965c07cf records unchanged original-source corrected-TS capture and native seeded-state parity. Four successful rows and 23 rejection rows cover both player branches, recipient identity, witness and assertion boundaries. Native success matches serialized state, effects, result FAB, witness/private transcript, gas sum, selected Zswap input, output commitment and index. Pinned ledger-v8 independently partitions successful red and blue runs as a 42-operation guaranteed transcript, no fallible transcript, one input and one output. Selective concede ZKIR keys compile at k=16, 54,685 rows; prover SHA-256 3edd4d961061ba39b73c3bef0fdb6838ffc446ee21f5ef9a525d6bb499bdf479, verifier SHA-256 89bd2e8a31a6f750b86a2f0951c131f08a3c46e7581df64e27b9c95ac49a4536 under ${LOCAL_EVIDENCE}/compact-adr209-keys. Shared emitter/runtime and recorded/proof implementation remain pending ADR206 then ADR207 handoff. This checkpoint does not claim recorded replay or ledger application.

### Implemented admission decision, 2026-10-06

The implementation uses distinct `ShieldedPayout` (ADR206 readonly) and `ActionfulShieldedPayout` typed Plan domains. The readonly profile retains its Bytes32 witness, existing Cell types, and zero-write requirement. The actionful profile admits a checked Board witness, typed commitment/Maybe Cell observations, and one Enum winner Cell write in an audited action tree. A recursive path count requires exactly one writing helper on every root execution path; sequential second calls and a branch without a writer refuse recording. Every binding, argument and alternate branch is structurally audited before typed Plan lowering. The Plan evaluates the selected helper action list and return in one lexical scope, preserving write-before-send order. These choices preserve schema20 and runtime ABI49; no generated-facing runtime method was added.

The implementation keeps complete original-source capability inventory honest: `guess`, `concede` and `withdraw` record; `start` remains unavailable. The focused source gate, 39 backend unit tests, 34 Python inventory/gate tests, original 27-row TS/native/recorded/replay fixture, and strict Clippy passed. Both original red and blue call proofs were independently verified and ledger-applied under default strict offer binding with separate NIGHT-backed Dust; a changed binding and selected spent nullifier reject. The fixture begins from seeded prior-game state, so this does not claim full funded game lifecycle.


### Original concede integrated — 4bc6bf95 (2026-10-06)

ADR209/#313 is integrated as signed/DCO `b1469975` (independent capture) and `4bc6bf95` (recording), from37bd4d08/876ae194. The original source now records concede with branch-local witnesses and a winner enum write before the selected qualified send. Shared typed Plan uses distinct readonly/actionful payout policies; actionful paths each contain exactly one writing helper. Sequential extra helpers, missing-helper branches, hidden effects, malformed declarations and scope leakage are refused. Root and independent review have no remaining actionable findings.

Main focused verification passed:39 backend unit +13 CLI +153 renderer tests,34 Python tests, three source fixtures and their generated tests (9/10 proof-required APIs recorded, original Coracle start remains the one gap in that subset), strict three-package Clippy and inventory. Wider inventory is now **381/386 proof-required APIs available, five original-source gaps**,217sources/755exports/196compiledroots/369nonproof/1,024declarations; zero unassessed/missing/unmatched rows or drift. Source delivery has27 corrected-TS/native/recorded/replay cases and both original strict red/blue payout proofs with actual historical input/user output, separate Dust, default-strict apply and replay refusal. Prior game state/coins are seeded.

Root receipt: `${LOCAL_EVIDENCE}/compact-4bc6bf95-integration-receipt.json`; focused: `${LOCAL_EVIDENCE}/compact-focused-4bc6bf95/receipt.json`; inventory: `${LOCAL_EVIDENCE}/compact-4bc6bf95-inventory.json`; consolidated source: `${LOCAL_EVIDENCE}/compact-adr209-delivery-receipt.json` (SHA256 199ed0b70e595822e4b420baf5d09a958a5788448bea3d49954656c9e57b33af). Source proof evidence is explicitly a transcription of successful tool stdout in `${LOCAL_EVIDENCE}/compact-adr209-proof-observation.md`, not a saved command-native log; individual proof bytes were not retained. These limits are preserved rather than presenting the source gate receipt as proof artifacts.

Latest full/portable checkpoint remains **3404c30c** (374commands/176freshfixtures/18portable checks); this is subsequent focused integration, schema20/ABI49 unchanged. ADR211 set_topic signed candidate has passed its two strict paths and is finishing final admission/freshness gate. ADR212 cash_out,213 start,214 vote_commit and215 buy_in all have signed independent native/capture preparation and selective original keys. No push/remote CI; user documentation remains untouched.
