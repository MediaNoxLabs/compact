---
id: RUST-ADR-0099
alias: ADR-0099
title: "Record closed unsigned equalities in streaming assertions"
date: 2026-10-05
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["backend", "recording", "assertions", "unsigned", "control-flow"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: a072028fe9db09d881847a71be762ced5978b2eafa7b7542a40dc6a8c73fa30f
---
# RUST-ADR-0099 — Record closed unsigned equalities in streaming assertions

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Accepted compile-time equality of validated unsigned literals with identical maxima inside an observed conditional assertion, retaining the prior Cell read and lazy control order. Both successful flag cases were proved. The false-assertion source variant is a local probe plus retained renderer guard, not a separately checked-in source fixture.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#202 closure](https://github.com/MediaNoxLabs/compact/issues/202#issuecomment-6017572690). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

### Original source metadata

```yaml
adr: 99
status: accepted
date: 2026-10-05
milestone: rust-backend-v2
issue: https://github.com/MediaNoxLabs/compact/issues/202
```

## Historical decision and amendments

### Problem and exact typed IR

`examples/rust_backend/ternary_cond_oracle.compact` exports `streamAssertEq`. It reads the Boolean `flag` Cell, asserts `f ? (1 == 1) : (2 == 2)`, and writes `fieldCell`. Schema-11 typed IR places the read in a `StateAction::Let`, followed by `StateAction::Assert` over `Expr::If`; each branch is `Expr::Equal` of two `Expr::UnsignedLiteral`s with identical typed maximum (1 in the true arm, 2 in the false arm). ADR-0097 admits pure Boolean scalar arms but not this equality. Current proof-required capability is `recorded=false`, `observed_call=false`; exact first gap is `actions[0].action.actions[0]`, `StateAction::Assert`.

### Decision and bounds

Recognize Boolean `Expr::Equal` only when both operands are closed, validated unsigned literals and have the same typed maximum. Produce the corresponding Boolean literal from their values. Leave all effectful operands, mixed maxima, broad arithmetic, composite equality, and other operators unavailable. The existing Boolean `Expr::If` lowering records the `flag` condition once before selecting the arm. The current `StateAction::Assert` and typed `CellSlot::record_write` retain source order. No schema-11 IR, ABI, capability schema, runtime API, or midnight-ledger/zk primitive change.

### Before and after generated Rust

Before:

```rust
let native = ledger_contract::streamAssertEq(context)?;
// No recorded::streamAssertEq or typed observed streamAssertEq_call.
```

After, if the entire call graph records:

```rust
let (frame, f): (_, bool) = ledger_slots::flag.record_read(frame)?;
let _ = f; // retain the observed condition; both literal comparisons fold to true
let asserted: bool = true;
if !asserted {
    return Err(CompactError::AssertionFailed("stream ternary assert".into()));
}
let frame = ledger_slots::fieldCell.record_write(frame, Field::from(1u128))?;
let call = contract.recording.streamAssertEq_call(&confirmed, private)?;
```

An intentionally false closed-literal branch must return the same assertion error without the Cell write. Accepting this shape is a bounded compile-time evaluation of typed literal equality; it does not move a ledger read or witness out of its branch.

### Validation plan

Use the immutable schema-11 Scheme snapshot and isolated Rust compiler/target. Capture both flag values from TypeScript and compare Rust native/recorded state, private state/FAB, all four gas dimensions, ordered VM, and replay. Check an intentionally false closed-literal branch for assertion failure and no write, plus an effectful equality operand that must remain unsupported. Compile pinned ZKIR and prove, verify, validate, and ledger-apply both successful branches. Run focused renderer/fixture tests, strict fixture freshness, Clippy/fmt/diff, and full capability inventory; root integration owns the combined gate. No push or remote CI.

### Tracking

- MediaNoxLabs issue: [#202](https://github.com/MediaNoxLabs/compact/issues/202), `rust-backend-v2`, created before code.
- Local branch: `codex/m2-ternary-let` stacked after signed/DCO ADR-0097 `1b5eae23`.
- Signed/DCO local delivery: `39e07e817e1d214cc0c6779ead388ff20f297f9f`.

### Local validation evidence

The final generated call reads `flag` once, evaluates the observed scalar once, and binds `true` because both validated same-type literal comparisons are true. This avoids an identical-arm Rust `if` while retaining VM effects and gas. Only `streamAssertEq` changes capability relative to ADR-0097: proof-required recorded and observed availability rises from 208/296 to 209/296, leaving 87 gaps; the 191-source identity baseline has no additions or removals.

The deterministic TypeScript capture adds `streamAssertEqFalse` and `streamAssertEqTrue`. Each has one `dup,idx,popeq` read and one `push,push,ins` Cell write, zero private FAB outputs, unchanged private state, and exact four-dimension query gas checked against native and recorded Rust. Both full serialized states and replayed effects match. The generated observed calls for both flag states matched manual replay and were proved, verified, validated, and applied through ledger-8 with pinned ZKIR; the flag and written Field Cell were checked.

The source assertion cannot fail for a Boolean input, so failure behavior was checked with a temporary source variant replacing its true arm by `((1 as Uint<2>) == (2 as Uint<2>))`. The typed IR preserved direct `UnsignedLiteral(1,max=3)` and `UnsignedLiteral(2,max=3)`, and the generated recorded API remained available. With `flag=true`, TypeScript and generated Rust native/recorded calls returned `failed assert: stream ternary assert` before the Cell write. The TypeScript query and a typed Rust read-prefix replay each contained only `dup,idx,popeq` and gas `(readTime 170000000, computeTime 1249914853, bytesWritten 0, bytesDeleted 0)`; the state and private state were unchanged and no private FAB was produced. A committed renderer regression test retains the false-literal branch and rejects an effectful Cell-read operand inside an equality arm. The variant itself is a local probe rather than a checked-in fixture.

The renderer's 76 tests, ternary fixture tests, strict Clippy, 139/139 fixture freshness, format and diff checks passed. Root integration owns the combined local full gate.
