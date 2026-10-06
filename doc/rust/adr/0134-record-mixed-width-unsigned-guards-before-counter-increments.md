---
id: RUST-ADR-0134
alias: ADR-0134
title: "Record mixed-width unsigned guards before Counter increments"
date: 2026-10-05
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["recording", "unsigned-arithmetic", "assertions"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: d102fd9248d2b2b6d8de1843ba6e13113f336cac673eda14c43122671dd802e9
---
# RUST-ADR-0134 — Record mixed-width unsigned guards before Counter increments

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Accepted the two explicit mixed-width unsigned guard domains: checked parameter widening/equality and the typed multiply-by-four pure assertion before Counter increment. Both APIs are proved; switched arguments, altered widths/products/amounts and hidden effects remain rejected. Preserve the historical dedicated 64 MiB proof-thread setup.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#236 closure](https://github.com/MediaNoxLabs/compact/issues/236#issuecomment-6017631924). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`967eee59`](https://github.com/MediaNoxLabs/compact/commit/967eee596f0fe732911c6de1e90210c571dd0543). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
adr: 134
status: accepted
date: 2026-10-05
milestone: rust-backend-v2
issue: https://github.com/MediaNoxLabs/compact/issues/236
```

## Historical decision and amendments

### Problem and typed evidence

At root `967eee59` (IR schema 12/runtime ABI 37), both `mixed_width_operand_oracle.compact::recordMatching` and `::recordPinned` were proof-required, native-runnable, and unavailable to recording/observed calls. `recordMatching` lowered to `Assert(Equal(UnsignedCast<4294967295>(Parameter small: Uint<255>), Parameter big: Uint<4294967295>))` followed by `Let 1: Uint<65535> -> CounterIncrement mixedOps`; the capability gap was `unsupported_action StateAction::Assert actions[0]`. `recordPinned` lowered to `PureCall assertProductLE(Coerce(q), Coerce(y))` followed by the same Counter-one continuation; its gap was `unsupported_action StateAction::PureCall actions[0]`.

The pure Unit helper has one closed assertion: `Let product: Uint<17179869180> = UnsignedMultiply(UnsignedCast(q), 4); Compare less_equal(product, UnsignedCast(y))`. Incorrect widening, truncation, or operation ordering could produce an accepted proof for a call that TypeScript rejects, or a Counter write before rejection.

### Decision

Admit `recordMatching` only for the exact typed public Uint<255> to Uint<4294967295> cast and equality of two public parameters. Emit the runtime's checked `cast_unsigned` and BoundedUint comparison before the Counter action. Preserve the source assertion message and reject effectful operands or other widths. The regular action walker records the later Counter increment.

Admit `recordPinned` only when both public parameters and pure-callee parameters are Uint<4294967295>, both call arguments are typed `Coerce(Parameter)`, the callee body is precisely a Unit sequence containing one checked multiply-by-four, `less_equal` assertion with Uint<17179869180> intermediates, and the second action is ADR-0133's exact Uint<65535> literal-one Let/Counter continuation. Invoke the generated pure helper before the regular Counter action walker. Extra pure steps, changed product, switched arguments, and altered Counter amount remain unavailable.

The AST emitter owns shape admission and typed syntax. The existing generated pure circuit owns the product assertion. The runtime's existing BoundedUint, checked cast, and typed Counter slot own range semantics, gas, ordered VM, replay and FAB input. No new IR schema, runtime ABI, macro or primitive is introduced. This is a follow-on to ADR-0133's Counter-one continuation, with a separate two-Uint32 guard matcher.

### Before and after generated Rust

Before, native `recordMatching` widened `small`, compared it with `big`, and incremented `mixedOps`; native `recordPinned` called `pure_circuits::assertProductLE(q, y)?` then incremented it. Neither circuit emitted a `recorded::` function or typed observed call.

After, the generated recording API has the following readable shape (actual generated names use `__compact_` prefixes):

```rust
pub fn recordMatching<Private>(context: CircuitContext<Private>, small: BoundedUint<255>, big: BoundedUint<4294967295>) -> Result<RecordedCircuitResult<Private, ()>, CompactError> {
    let frame = RecordingFrame::new(context);
    let widened: BoundedUint<4294967295> = cast_unsigned::<255, 4294967295>(small)?;
    if !(widened == big) {
        return Err(CompactError::AssertionFailed("values must match across widths".to_owned()));
    }
    let frame = ledger_slots::mixedOps.record_increment(frame, 1u16)?;
    Ok(frame.finish(()))
}

pub fn recordPinned<Private>(context: CircuitContext<Private>, q: BoundedUint<4294967295>, y: BoundedUint<4294967295>) -> Result<RecordedCircuitResult<Private, ()>, CompactError> {
    let frame = RecordingFrame::new(context);
    pure_circuits::assertProductLE(q, y)?;
    let frame = ledger_slots::mixedOps.record_increment(frame, 1u16)?;
    Ok(frame.finish(()))
}
```

Both also emit `recordMatching_call` / `recordPinned_call` with typed observed state and FAB input `(arg0, arg1)`.

### Local evidence and delivery

- Fresh TypeScript capture from the local Scheme frontend for success and failure of both circuits. Success: Unit result, exact initial/final serialized state and one Counter increment. Failure: original assertion text, unchanged state and zero VM queries.
- Rust native and recorded tests compare result, serialized state, effects, all four gas dimensions (`readTime=170000000`, `computeTime=1323481916`, `bytesWritten=80`, `bytesDeleted=80`), complete ordered VM (`idx` path `01`, `addi 1`, cached `ins 1`), empty private outputs, and replay state/effects. Three fixture tests pass.
- Renderer negative guards reject other cast widths, ledger reads inside the widened operand, extra pure steps, changed product literal, non-one Counter amount, and reordered pure-call arguments. All 107 renderer tests pass.
- Exact compiled artifact from `${LOCAL_EVIDENCE}/compactc-adr134-both` SHA-256 `dd6f337b7c481b2572737d6cdc7294a9541d69bcab254d16ba167532684030df`, using pinned ZKIR 2.1.0. Both generated traces replayed, typed observed calls matched direct prototypes, and ledger-8 proved, verified, applied, and checked Counter=1 for each. The two negative calls were rejected before observed-call construction. The proof runner uses a 64 MiB thread stack, consistent with other proof smoke selectors.
- Focused local gate receipt `${LOCAL_EVIDENCE}/compact-focused-adr134-both/receipt.json`: one source fixture, 2/2 recorded. Full local inventory on this isolated ADR-0133 plus ADR-0134 branch: 274/316 proof-required available, 42 gaps, 25 unassessed (`${LOCAL_EVIDENCE}/compact-adr134-inventory.json`). Full root gate remains the integration gate. No remote CI or push.

Issue: https://github.com/MediaNoxLabs/compact/issues/236. Signed delivery SHA to be appended after commit.

Signed local feature commit: `95e88d060da3e92d94c9fda854ccd56716ec969a` (`%G?=G`, DCO trailer verified). Its local parent is the ADR-0133 cherry-pick; integrate only this feature commit after ADR-0133. Root integrated SHA and exact-head gate receipt pending.
