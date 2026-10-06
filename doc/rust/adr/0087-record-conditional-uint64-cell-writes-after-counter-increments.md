---
id: RUST-ADR-0087
alias: ADR-0087
title: "Record conditional Uint64 Cell writes after Counter increments"
date: 2026-10-05
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["backend", "recording", "typed-slots", "control-flow"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: aae9bbef0018af0799ecd773978bca0ed99faa3dcbde288ec6801f73c2bd11ca
---
# RUST-ADR-0087 — Record conditional Uint64 Cell writes after Counter increments

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Accepted exact Uint64 conditional binding after Counter increment for streamIncrement, with one observed flag and typed Cell write. Both flag values have behavioral parity; the isolated proof used the false flag only, with the true proof later handled separately by ADR0232. This is not generic Uint64 expression admission.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#189 closure](https://github.com/MediaNoxLabs/compact/issues/189#issuecomment-6017550557). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

### Original source metadata

```yaml
adr: 87
status: accepted-partial
date: 2026-10-05
milestone: rust-backend-v2
issue: https://github.com/MediaNoxLabs/compact/issues/189
```

## Historical decision and amendments

### Problem and exact candidate

At local signed ADR-0086 commit `84e7567b` (ABI36/private IR8/report schema3), the unchanged 137-source/333-export corpus has 194 complete recorded/observed APIs among 286 compiler proof-required exports; 92 are unavailable. `ternary_cond_oracle.streamIncrement` is proof-required and its native Rust works, but the recorded path stops at `StateAction::Let` path `actions[0].action.actions[1]`. Source:

```compact
export circuit streamIncrement(): [] {
  const f = flag.read();
  ops.increment(f ? 3 : 4);
  wideCell = f ? 10 : 20;
}
```

Schema-8 IR first reads Boolean Cell `flag`, then ADR-0086 lowers `UnsignedCast<65535>(If(f,3,4))` and Counter increment. The next and only known blocker is `UnsignedCast<18446744073709551615>(If(f,10,20))` inside a second Let, followed by `CellWrite(wideCell,index2,value=tmp_46)`. This is one complete API candidate, not an assumed gain. Other ternary circuits have different blockers.

### Developer-facing before and after

Before, only the native entry point exists:

```rust
let native = ledger_contract::streamIncrement(context)?;
// No ledger_contract::recorded::streamIncrement or typed observed call.
```

After, if the complete trace passes validation:

```rust
let trace = ledger_contract::recorded::streamIncrement(context)?;
let replay = trace.public.initial().query(
    trace.public.verify_ops(), None, &trace.execution.context.cost_model,
)?;
assert_eq!(replay.context.state.get_ref(), trace.execution.context.query.state.get_ref());
let call = contract.recording.streamIncrement_call(&confirmed_state, private_state)?;
```

The emitter should preserve one prior Boolean Cell observation and one source-order conditional selection:

```rust
let selected: u64 = if flag { 10u64 } else { 20u64 };
let value = runtime::BoundedUint::<{ u64::MAX as u128 }>::new(selected as u128)?;
let frame = crate::ledger_slots::wideCell.record_write(frame, value)?;
```

The actual generated Rust may use an equivalent typed expression. It must not evaluate the untaken arm or reread `flag`.

### Decision and ownership

Extend recorded `StateAction::Let` only for an exact Uint64-max cast of a pure Boolean `If` with checked unsigned literal arms and exact coercion wrappers. Retain source-order local binding and pass the typed `BoundedUint<u64::MAX>` through existing `CellSlot::record_write`/`RecordingFrame::write_cell`, which in turn use the current midnight-ledger Cell VM. No new runtime API, macro, VM opcode, ZK primitive, ABI, private IR or capability schema is intended. Native generated Rust stays unchanged. Do not generalize to arbitrary Uint64 expressions, Field ternaries or all streaming circuits.

### Acceptance and risks

1. Rebuild immutable pre/post compilers with an isolated Cargo target, classify all 137 sources, join authoritative compiler proof flags and require a complete API with no losses. Assert the schema-3 false-capability reason invariant and strict diagnostics.
2. Capture TypeScript `streamIncrement` with false and true ledger flag. Compare full ledger state, ordered Verify-op shape, private output count and all four summed query-cost dimensions with native, recorded and replay. The existing TypeScript wrapper may report only its last query cost, so sum VM queries.
3. Generate a strict standalone consumer from a supported minimal source. Use the pinned original ternary source/ZKIR 2.1 to prove, verify, validate and apply at least one `streamIncrement` recording through midnight-ledger 8.0.3. Check final Counter and Uint64 Cell values; prove both branches if feasible.
4. Run focused renderer/consumer tests, all 137 fixture freshness checks and targeted Rust 1.99 Clippy. Parent integration owns the combined full gate. No push/remote CI.

Risks: Compact unsigned max versus Rust `u64` overflow, typed Cell alignment, reordered Counter/Cell effects, duplicated observation, and counting a shifted first blocker as a complete API. Keep issue open for integrated/same-revision CI and any uncovered branch proof gap.

### Tracking

- Predecessor: [ADR-0086 — Record conditional Counter amounts after Boolean observations](0086-record-conditional-counter-amounts-after-boolean-observations.md).
- Focused issue: pending creation in [rust-backend-v2](https://github.com/MediaNoxLabs/compact/milestone/2).


- Focused [#189](https://github.com/MediaNoxLabs/compact/issues/189) is assigned to `rust-backend-v2`; proposed, no code yet.

### Local delivery — 2026-10-05

Conventional GPG-verified/DCO commit `42bb6872db0633d223be86b7f80009b6c1342329` implements the bounded Uint64-max conditional binding on top of ADR-0086 `84e7567b`. The unchanged 137-source/333-export corpus gains one complete recorded/observed API, `ternary_cond_oracle.streamIncrement`; authoritative compiler proof metadata marks it `proof_required=true`. Complete availability moves **194→195/333**, proof-required availability **194→195/286**, and proof gaps **92→91**. No previously available API is lost. Pre-change first blocker was `StateAction::Let` at `actions[0].action.actions[1]`; the post-change capability is available with both `recorded` and `observed_call` true.

The emitter accepts only an exact Uint64-max cast of a Boolean If with checked unsigned literal arms and exact coercion wrappers. It constructs one typed `BoundedUint<18446744073709551615>` local after the existing observed Boolean Cell read and conditional Counter increment, then calls the existing `CellSlot::record_write`. Native generated output, runtime ABI36, private IR8, report schema3, ledger VM and ZKIR semantics are unchanged.

TypeScript false/true flag captures match Rust native/recorded/replay full ledger state, ordered Verify-op shape, effects, zero private outputs and all four summed query-cost dimensions. The 68 renderer tests pass; all 137 fixtures are fresh with zero failures and schema-3 reason invariants. A strict standalone generated consumer with `ledger-transaction` compiles. Pinned original ternary ZKIR 2.1 proof, verification, ledger validation and application pass for the false flag, asserting Counter=4 and Uint64 Cell=20. Targeted Rust 1.99 all-target Clippy with `-D warnings` passes. The worktree is clean.

Remaining: a pinned proof/application for the true seeded flag branch, combined full local integration gate, same-revision remote CI/publication and the separate ternary expression blockers. #189 stays open; no push/remote CI was run.
