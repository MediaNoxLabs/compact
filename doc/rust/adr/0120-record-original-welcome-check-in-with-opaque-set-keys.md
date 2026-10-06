---
id: RUST-ADR-0120
alias: ADR-0120
title: "Record original Welcome check-in with opaque Set keys"
date: 2026-10-05
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["recording", "opaque-string", "witness", "original-contracts"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: 64a9a2c687ac720d91cefb9a3b9699b43336376d43206e19d00ec30dc7b0cff6
---
# RUST-ADR-0120 — Record original Welcome check-in with opaque Set keys

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Accepted the structurally audited original Welcome check-in sequence: opaque-key membership assertion, insertion and typed Unit witness. Eligible success is proved; ineligible execution stops after membership and before the witness. Preserve aggregate-versus-last-query gas and the historical 64 MiB constructor proof-stack requirement.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#221 closure](https://github.com/MediaNoxLabs/compact/issues/221#issuecomment-6017605411). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Referenced repository commits:** [`a6bfc04a`](https://github.com/MediaNoxLabs/compact/commit/a6bfc04af726fbf820958a27f731b6b984e7fd9d) · [`ab3b5fa6`](https://github.com/MediaNoxLabs/compact/commit/ab3b5fa6f77852484ab3901399a928452cdd8cbe) · [`c01270c4`](https://github.com/MediaNoxLabs/compact/commit/c01270c4c58dfde843f25c275e35b6bc2098736f). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
adr: 120
status: accepted
date: 2026-10-05
milestone: rust-backend-v2
issue: https://github.com/MediaNoxLabs/compact/issues/221
```

## Historical decision and amendments

### Problem and scope

At exact signed schema-12 root `a6bfc04a`, the original `test-center/test-contracts/welcome.compact` compiles for Rust and its constructor matches the checked TypeScript empty/one-participant state. All three exported circuits require proof and remain unavailable for recording. `check_in(participant: Opaque<"string">)` stops first at `StateAction::Assert(SetMember(eligible_participants, participant))`; its remaining actions insert the same opaque key into `checked_in_participants` and call Unit witness `set_local_id(participant)`. The other two exports also require nested private assertions, public-key hashing and Bytes Set operations and are outside this decision.

### Before and after generated Rust

Before, native `check_in` executes the typed Set member assertion, Set insertion and `set_local_id` witness, but there is no `recorded::check_in` or observed call builder. After, the generated recording body should preserve the same order:

```rust
let (frame, eligible): (_, bool) =
    crate::ledger_slots::eligible_participants.record_member(frame, participant.clone())?;
if !eligible {
    return Err(runtime::CompactError::AssertionFailed("Not eligible participant".into()));
}
let frame = crate::ledger_slots::checked_in_participants.record_insert(frame, participant.clone())?;
let (frame, _) = frame.try_witness_metered(|context, meter| {
    witnesses.set_local_id(context.witness_context_with(super::LedgerView {
        state: context.query.state.get_ref(), meter,
    }), participant.clone())
})?;
```

The actual generated names and retained clones may differ. The observable VM, gas, state and private transcript must match TypeScript, not merely compile.

### Typed ownership and limits

Reuse schema-12 `Type::OpaqueString`, `Expr::Parameter/Coerce`, `Expr::SetMember`, `StateAction::Assert/SetInsert/Expression(WitnessCall)`, existing generated `Witnesses` signatures and ledger-8 `SetSlot<OpaqueString>`. Admit opaque sources only through a typed parameter/local path; permit this key in recorded Set membership/insertion and a single typed Unit witness argument. Require the complete compiler-typed Assert → SetInsert → Unit witness sequence with one opaque parameter and Unit result before advertising a proof capability. This structural admission gate prevents untested opaque Set-only and Map calls from becoming observed APIs; they need separate parity and proof decisions. The witness result remains recorded through `RecordingFrame::try_witness_metered`. Arbitrary opaque expressions, bytes, Set removal, `add_participant`, `add_organizer`, and unsupported action shapes remain unavailable. No IR schema or runtime API change.

### Acceptance

Capture a fresh TypeScript oracle from the exact original Welcome source using a constructor with `alice` eligible and one without. For `check_in("alice")`, compare generated native, recorded and observed result/state, all four gas dimensions, ordered public VM, private witness output, and Verify replay. For an ineligible participant, compare assertion error and unchanged state/private effects; no successful proof is possible for this input. Compile pinned schema-12 ZKIR, then prove, verify, validate and apply the successful generated observed call through ledger-8. Keep the checked Welcome source row and count only this single API gain if all checks pass. Add negative recording guards, focused renderer/runtime/fixture/parity tests, rustfmt and Clippy. Run locally with isolated compiler and target; no push or remote CI.

### Tracking

- Predecessors: [ADR-0112 — Iterate typed constructor vectors in Welcome](0112-iterate-typed-constructor-vectors-in-welcome.md), [ADR-0117 — Record closed pure assertion calls](0117-record-closed-pure-assertion-calls.md).
- Milestone issue: https://github.com/MediaNoxLabs/compact/issues/221.
- Delivery: signed local commit `31aa29353fb51459e0bec994c50a21fd1fb890f2` on `a6bfc04a`, integrated as signed root `c01270c4c58dfde843f25c275e35b6bc2098736f` (proof-module registration/format follow-up `ab3b5fa6`).

### Local validation (2026-10-05)

- Immutable schema-12 `a6bfc04a` compiler and source were used for the baseline. Exact original Welcome TypeScript capture: eligible `check_in("alice")` returns Unit, updates only checked-in Set, changes private state 7→8, emits 10 ordered public VM operations and one empty Unit private output. Its two query charges sum to readTime `340000000`, computeTime `2587430004`, bytesWritten `608`, bytesDeleted `494`; TypeScript's `output.gasCost` exposes the last query only, so the Rust aggregate compares to the sum. The empty-constructor `check_in("bob")` fails `Not eligible participant` after one membership query and before any witness call, with unchanged serialized state and private state.
- Generated native and recorded Rust match TypeScript success result, serialized state, four aggregate gas dimensions, ordered VM shape, private output and witness argument. Recorded Verify replay matches final state and effects. Native/recorded failing calls return the same assertion error; generated observed call rejects the ineligible input before preparation.
- Pinned ZKIR 2.1.0 compiled `check_in.zkir` at k=6, 28 rows. The generated observed successful call matched manual preparation, proved, verified, ledger-8 validated and applied; final ledger Set contains `alice`.
- Full source inventory at this exact branch keeps 195 sources, 935 declarations and 309 proof-required exports. Proof available `249→250`, missing `60→59`; the sole changed row is original Welcome `check_in`. `add_participant` and `add_organizer` remain unavailable at their existing Assert blockers. Checked Welcome source scope and exact generated fixture passed. Renderer negative guards cover non-parameter opaque source, opaque Set removal, OpaqueBytes and incomplete sequence.
- Local focused renderer tests, generated Welcome tests, inventory tests, rustfmt and Clippy pass. Proof smoke requires a 64 MiB main-thread stack on macOS (`ulimit -s 65520`) for the 5000-element constructor. No push or remote CI.

### Delivery

Signed conventional DCO commit `31aa29353fb51459e0bec994c50a21fd1fb890f2` has a good GPG signature and clean worktree. Root integrated it as signed `c01270c4c58dfde843f25c275e35b6bc2098736f` and followed with proof-module registration/format commit `ab3b5fa6`. No push or remote CI was used for the isolated delivery.
