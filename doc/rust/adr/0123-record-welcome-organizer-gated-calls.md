---
id: RUST-ADR-0123
alias: ADR-0123
title: "Record Welcome organizer-gated calls"
date: 2026-10-05
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["recording", "authorization", "witness", "original-contracts"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: e711b8e2c2d4fd4d91162253198772c961baaee1932aecabbb11d28d366be843
---
# RUST-ADR-0123 — Record Welcome organizer-gated calls

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Accepted the exact original Welcome Maybe-key witness, pure public-key hash, organizer membership and single-insertion sequence. Both successful organizer calls are proved; missing-key and unauthorized-key failures preserve their distinct prefix and messages. Hash/helper mutations remain outside admission.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#223 closure](https://github.com/MediaNoxLabs/compact/issues/223#issuecomment-6017608965). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Referenced repository commits:** [`c01270c4`](https://github.com/MediaNoxLabs/compact/commit/c01270c4c58dfde843f25c275e35b6bc2098736f) · [`c6071253`](https://github.com/MediaNoxLabs/compact/commit/c6071253dbf6e35a75ca549bf2f7c0dcd29f4fe1). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
adr: 123
status: accepted
date: 2026-10-05
milestone: rust-backend-v2
issue: https://github.com/MediaNoxLabs/compact/issues/223
```

## Historical decision and amendments

### Problem and exact source

On signed schema-12 root `c01270c4`, original `test-center/test-contracts/welcome.compact` compiles and `check_in` is proved through ADR-0120. `add_participant` and `add_organizer` remain proof-required and recording-unavailable at their first `StateAction::Assert`. Both assert `organizer_pks.member(public_key(local_sk_or_error()))` before inserting into a Set. Schema-12 IR makes the condition a typed Bytes32 `Expr::Let`: internal `local_sk_or_error` calls the private `local_sk(): Maybe<Bytes<32>>` witness, asserts `is_some`, returns `value`; exported pure `public_key` performs a persistent hash of a fixed Bytes32 prefix and that key; then ledger-8 observes Bytes32 Set membership. `add_participant` inserts OpaqueString; `add_organizer` inserts Bytes32. These operations must retain witness, hash, membership and assertion order, including two distinct failure messages.

### Before and after generated Rust

Before, native Rust runs the internal helper, generated pure hash, organizer membership assertion and Set insertion. Recorded/observed methods for both calls are absent. The intended recorded prefix is:

```rust
let (frame, maybe_sk) = frame.try_witness_metered(|context, meter| {
    witnesses.local_sk(context.witness_context_with(LedgerView { state: context.query.state.get_ref(), meter }))
})?;
if !maybe_sk.is_some {
    return Err(CompactError::AssertionFailed("No secret key found".into()));
}
let organizer_pk = crate::pure_circuits::public_key(maybe_sk.value)?;
let (frame, is_organizer) = organizer_pks.record_member(frame, organizer_pk)?;
if !is_organizer {
    return Err(CompactError::AssertionFailed("Not an organizer".into()));
}
```

Only after this prefix may `eligible_participants.record_insert(frame, participant)` or `organizer_pks.record_insert(frame, organizer_pk_argument)` run. Generated names can differ; TypeScript VM, gas, state and private transcript are the contract.

### Decision and limits

Reuse typed schema-12 Let/Call/Coerce/SetMember/Assert/SetInsert IR, generated pure `public_key`, `MaybeCompact1` and witness traits, runtime `RecordingFrame` witness metering and ledger-8 `SetSlot`. The emitter parses a closed compiler-typed witness → Bytes32 hash → Set authorization → single Set insertion sequence before emitting `syn` statements. It validates the zero-argument internal helper's exact one-Maybe-witness binding, `is_some` assertion and Bytes32 return; the exported pure circuit's one-Bytes32 persistent-hash body; both typed Set declarations and the Unit return. The witness result is recorded once, both assertion messages are retained in source order, and only the selected insertion follows authorization. The prior ADR-0120 opaque `check_in` shape remains admitted. Mutating the pure hash body or adding an internal assertion leaves both organizer APIs recording-unavailable. No arbitrary pure hash, nested stateful call, Bytes/Opaque expression or unrelated Set API is admitted by fallback. No IR schema or runtime API change.

### Acceptance

Capture original Welcome TypeScript success for both calls and failures for missing key and non-organizer key. Compare generated native and recorded Unit result/serialized state, all four aggregate gas dimensions, ordered public VM, private witness outputs and Verify replay; failures must preserve assertion messages and stop before Set insertion. Compile pinned ZKIR 2.1.0 and prove, verify, ledger-8 validate/apply every newly admitted successful observed call. Compare exact checked inventory before/after, retain `check_in`, and leave unproved APIs unavailable. Add renderer negative guards, focused fixtures/scope/inventory tests, rustfmt and Clippy; work only locally without push or remote CI.

### Tracking

- Predecessor: [ADR-0120 — Record original Welcome check-in with opaque Set keys](0120-record-original-welcome-check-in-with-opaque-set-keys.md).
- Milestone issue: https://github.com/MediaNoxLabs/compact/issues/223.
- Delivery: signed local commit `33aa766078e616561137aa2264548c9fa50a77db` on `codex/welcome-organizer-gate-adr123` (base `c01270c4`); root integration pending.

### Local validation (2026-10-05)

- Exact original Welcome TypeScript capture from a one-`alice` constructor: `add_participant("bob")` and `add_organizer(Bytes32[7;32])` each return Unit, emit ten ordered public VM operations and one private `Maybe<Bytes<32>>` output, and call `local_sk` once. Two query charges aggregate to readTime `340000000`; computeTime `2587462152` and bytesWritten `734` for participant, computeTime `2587501460` and bytesWritten `796` for organizer; both delete `494` bytes. Generated native and recorded Rust match result, serialized state, all four aggregate gas dimensions, ordered VM shape and private output; Verify replay matches final state/effects.
- Missing-key calls fail `No secret key found` after the private witness and before a ledger query. A different key fails `Not an organizer` after one membership query and before Set insertion. Generated native and recorded failures match TypeScript messages and witness counts; TypeScript captured unchanged state/private state. Successful observed calls match manual preparation.
- Pinned ZKIR 2.1.0 compiled `add_participant` at k=13/4175 rows and `add_organizer` at k=13/4452 rows. Both generated observed calls proved, verified, ledger-8 validated and applied; final eligible/organizer Set members matched expected keys. Existing `check_in` proof still passes.
- Exact local full inventory after this slice: 195 sources, 935 declarations, 316 proof-required exports, 254 available and 62 missing. The latest immutable packaged root baseline is pre-ADR-0120 `c6071253` at 251/316; signed integrated ADR-0120 adds one, so the projected exact `c01270c4` baseline is 252/316 and this slice adds the two organizer calls to 254/316. Root must verify the integrated delta with its next packaged receipt. The checked Welcome source has 3/3 proof-required calls recorded and observed. The renderer negative test mutates the pure hash body and inserts an extra helper assertion; both organizer APIs become unavailable while `check_in` stays available.
- Focused renderer, generated Welcome and inventory tests, checked source scope, rustfmt, Clippy and pinned proof smoke pass locally with isolated targets. No push or remote CI.
