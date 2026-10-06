---
id: RUST-ADR-0103
alias: ADR-0103
title: "Record nested conditional Set actions"
date: 2026-10-05
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["recording", "conditional-actions", "set"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: 0c2393fd93c0eb773cbb6f786f195d24f1519e42216fa1ae9a79859bb8407ef9
---
# RUST-ADR-0103 — Record nested conditional Set actions

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Accepted recursive validation of both conditional Set action branches and ordered execution of only the selected branch in the shared typed scope. The original three-operation sequence has TS/native/recorded/replay and proof/application evidence; other effect leaves remain governed by their own bounded domains.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#206 closure](https://github.com/MediaNoxLabs/compact/issues/206#issuecomment-6017579494). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Referenced repository commits:** [`b00de52c`](https://github.com/MediaNoxLabs/compact/commit/b00de52c6d083038add290852e3b207dc333624d). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
adr: 103
status: delivered-locally
date: 2026-10-05
milestone: rust-backend-v2
issue: https://github.com/MediaNoxLabs/compact/issues/206
```

## Historical decision and amendments

### Problem and evidence

`examples/rust_backend/set_boolean.compact::choose(value, insert)` is a proof-required export with nested `if` actions. Schema-11 IR has `StateAction::If` over Boolean parameters. Its three leaves are the already-supported typed `SetInsert<Boolean>` and `SetRemove<Boolean>`, followed by a `SetMember<Boolean>` return. The b00de52c Rust compiler emits a native `choose` but marks recording and observed call unavailable at `actions[0]` (`unsupported_action`, `StateAction::If`). The existing TS oracle covers insert true, remove true, and insert false, with serialized state only.

### Before and after generated Rust

Before, the generated crate exposes `ledger_contract::choose` but no `recorded::choose` or `Contract::recording.choose_call`. A developer cannot submit its proof-required call.

After, the recorded function keeps the selected branch's frame and emits no VM operations from the other branch. The shape is:

```rust
let frame = if insert {
    if value {
        let frame = ledger_slots::seen.record_insert(frame, true)?;
        frame
    } else {
        let frame = ledger_slots::seen.record_insert(frame, false)?;
        frame
    }
} else {
    let frame = ledger_slots::seen.record_remove(frame, value)?;
    frame
};
let (frame, observed) = ledger_slots::seen.record_member(frame, value)?;
```

This is illustrative emitted Rust; the AST emitter supplies hygienic names and a typed `RecordingFrame`. `recorded::choose` and `Contract::recording.choose_call` then become available.

### Decision and ownership

Keep the schema-11 `StateAction::If` and typed Set primitives; do not add a new IR instruction or duplicate ledger semantics. In `recorded.rs`, recursively lower both branches into separate statement vectors using the same read-only typed scope, inspect both results before emitting the enclosing conditional, and return the selected branch's frame. Compile-time rejection of either unsupported branch must retain a precise `actions[0].then` or `.otherwise` gap; runtime selection cannot hide unsupported code. Evaluate the Boolean condition once before branch execution. Reuse `SetSlot::record_insert/remove/member`, ledger-8 VM programs, and existing runtime gas accounting. Native lowering already supports this source.

### Validation and limits

The immutable schema-11 Scheme frontend compiles the source before and after. The targeted inventory moves from 8/9 to 9/9 proof-available exports, with `choose` changing from unavailable to recorded plus observed-call. The full inventory against exact b00de52c moves 220/296 to 221/296 proof-available, missing 76 to 75, with no other capability change. Three sequential cases (insert true, remove true, insert false) pass TS/native/recorded comparison for serialized state, Boolean result, four gas dimensions, ordered public VM operations, null/Unit private state, empty private transcript, and Verify replay. Pinned ZKIR 2.1.0 emits `choose` proving/verifying keys; three calls have been proved, verified through ledger `well_formed`, and applied through ledger-8. A renderer regression test confirms that even a statically unselected unsupported branch keeps the circuit unavailable and reports `actions[0].otherwise`.

The generated frame shadowing is intentionally visible in Rust. Its branch blocks produce the chosen `RecordingFrame`; only the emitted branch statement receives a Clippy `let_and_return` allowance because erasing those bindings would obscure the ordered VM effect boundary. Other `StateAction::If` sources remain subject to typed leaf support and separate parity evidence. The parent full gate has not yet run on this branch.

### Tracking

- Milestone: `rust-backend-v2`.
- Issue: [#206](https://github.com/MediaNoxLabs/compact/issues/206).
- Delivery: signed DCO commit `329f5fe1ab5b7876c4f735e3d830d0bc7b7ebe03` on `codex/m2-next`; awaiting parent integration and exact-head full gate.
