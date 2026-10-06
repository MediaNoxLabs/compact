---
id: RUST-ADR-0094
alias: ADR-0094
title: "Record typed tuple-to-vector witness arguments in Let bindings"
date: 2026-10-05
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["backend", "recording", "witnesses", "coercions"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: ed236e590902b487937620c482aa26b9e01be7e8557bb5d2cbbe116946c40686
---
# RUST-ADR-0094 — Record typed tuple-to-vector witness arguments in Let bindings

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Accepted exact literal Field tuple-to-declared-vector witness arguments, evaluated once before metered witness and typed writes. An explicit same-shape amendment includes keepResult/reuseResult; discarded witness expressions remain separate. Three API gains use one representative witnessConst proof rather than proving every admitted wrapper.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#195 closure](https://github.com/MediaNoxLabs/compact/issues/195#issuecomment-6017560902). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`aabf9241`](https://github.com/MediaNoxLabs/compact/commit/aabf924127553afa156ebbe7e24e6e4a1ced92dd). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
adr: 94
status: accepted
date: 2026-10-05
milestone: rust-backend-v2
issue: https://github.com/MediaNoxLabs/compact/issues/195
```

## Historical decision and amendments

### Problem and measured source

`examples/rust_backend/call_arg_declared_type.compact` exports `witnessConst(): []`, which binds `const w = sumWitness([0 as Field, 1 as Field])` and writes `w` to a Field Cell. The compiler marks it `proof_required: true`, but schema-11 typed IR reports `Expr::WitnessCall` unavailable at `actions[0].bindings[0].value`. The witness call itself is already supported by `field_expression`; its single declared `Vector<2, Field>` argument is represented as `Coerce(Tuple(FieldLiteral(0), FieldLiteral(1)), Vector<2, Field>)`. The recorder's `cell_source` discards the typed coercion and cannot lower the bare tuple. Native Rust and TypeScript can execute this source, but the generated Rust crate lacks `recorded::witnessConst` and its observed-call/proof API. `witnessBare` is a distinct discarded-expression action and remains outside this decision.

### Before and after Rust code

Before, direct generated execution constructs a `FixedVector<Field, 2>`, invokes `sumWitness` once, adds one aligned private FAB output, and writes the result; no recorded API is emitted:

```rust
let argument = runtime::FixedVector::new([runtime::Field::from(0u128), runtime::Field::from(1u128)]);
let (next_private, w) = witnesses.sumWitness(context.witness_context_with(view), argument)?;
private_transcript_outputs.push(runtime::fab::AlignedValue::from(w));
ledger_slots::fieldCell.write(context, w)?;
```

After, the AST recorder will emit a typed argument binding before one metered witness call and one recorded Field Cell write:

```rust
let argument: runtime::FixedVector<runtime::Field, 2> =
    runtime::FixedVector::new([runtime::Field::from(0u128), runtime::Field::from(1u128)]);
let (frame, w) = frame.try_witness_metered(|context, meter| {
    witnesses.sumWitness(context.witness_context_with(LedgerView { state: context.query.state.get_ref(), meter }), argument)
})?;
let frame = ledger_slots::fieldCell.record_write(frame, w)?;
```

The developer API gains `ledger_contract::recorded::witnessConst` and the generated `witnessConst_call` observed-state handle; existing native code remains the same.

### Decision and boundary

Recognize only a typed `Expr::Coerce` from a tuple of side-effect-free Field literals to its declared `Vector<N, Field>` witness formal. Validate target type, tuple length and each element's exact Field type; render it through the existing typed expression AST, bind it once, then invoke the existing `field_expression` witness path. Preserve source order, private-state transition, FAB alignment, witness read metering and public VM effect. Reuse runtime `FixedVector`, `RecordingFrame::try_witness_metered`, and typed CellSlot; no handwritten VM operation, new runtime API, ABI, IR variant or schema change. Reject arbitrary tuple coercions, nested effects and nonliteral vector elements until separate evidence and ADR justify them. Avoid a false proof capability claim for `witnessBare`.

### Validation plan

Freeze Rust compactc and schema-11 Scheme SHA-256. Regenerate only the `call-arg-declared-type` fixture. Require exactly `witnessConst` to gain recorded/observed availability in that source, with no losses. Capture TypeScript state, four-dimensional gas, ordered public transcript, private FAB output and witness order. Compare native and recorded Rust to that oracle, including a witness that advances private state so duplicate or missing evaluation is visible. Run source-to-proof-to-ledger proving and verification for `witnessConst`, focused renderer/generated tests and Clippy, and a fresh parity inventory. Root integration runs the combined full local gate after concurrent slices land.

### Tracking

- Issue: pending.
- Branch: isolated `witness-call-let` worktree from `aabf9241`.
- Commit/evidence: pending.


Tracking issue: https://github.com/MediaNoxLabs/compact/issues/195 (rust-backend-v2).

### Same-shape fixture discovered during freshness check

The full fixture freshness check found one additional source with the exact guarded IR argument shape: `examples/rust_backend/witness_vector_action.compact`. Its proof-required `keepResult` and `reuseResult` both bind a Field witness result from `Coerce(Tuple(FieldLiteral(0), FieldLiteral(1)), Vector<2, Field>)`; both gain recorded/observed methods. `discardResult` remains a separate unsupported `StateAction::Expression`. The generated fixture is updated individually, and focused native/recorded/TypeScript tests verify both one-write and two-write cases, including one witness evaluation, private state 7→8, FAB output and final ledger state. The `witnessConst` proof gate exercises the same argument shape end to end through ledger-8.


### Local delivery — 2026-10-05

Delivered on isolated branch `codex/adr94-witness-call-let` from `aabf9241` at conventional GPG-signed/DCO commit `c95d77be78779f9970a123737d7410a978349769`. Focused issue [#195](https://github.com/MediaNoxLabs/compact/issues/195) remains in `rust-backend-v2` for integration. No push or remote CI. The compiler is local `target/debug/compactc` SHA-256 `9a52bdf08f50e532b5baf3c8d17a9b1934fae18d33a18fa5aae55d8aec41c6c3`, schema-11 Scheme wrapper `${LOCAL_EVIDENCE}/adr89-schema11-scheme-root/compactc-scheme` SHA-256 `369ee19b62cba4d6e5f13b0a40599d6b7a38f42e12216ec01ed9ed270747a3d4`, ZKIR 2.1.0 SHA-256 `9b45827ac4786e383b3bfd93c870afe9a2cbb69af753949fd53549d146c548bc`.

Exactly three proof-required APIs gain recorded and observed-call support: `call_arg_declared_type.witnessConst`, `witness_vector_action.keepResult`, and `witness_vector_action.reuseResult`. Across those two sources, proof availability rises 4→7 of 19, gaps fall 15→12, no losses or unmatched compiler circuits. `witnessBare` and `discardResult` remain unavailable as distinct discarded-expression actions. The guarded emitter change is 17 changed lines in `recorded.rs`; the existing typed coerce renderer produces `FixedVector<Field,2>`, then `RecordingFrame::try_witness_metered` owns one ordered private transition/FAB, and `CellSlot::record_write` owns public effects. No IR schema/runtime/ABI change.

The TypeScript `witnessConst` capture checks exact `[0,1]` argument, one invocation, private state 7→8, FAB atoms/alignment, full state bytes, four gas dimensions, and ordered public VM `push(false), push(true), ins(false,1)`. Native and recorded Rust agree. The second source checks one witness invocation and private state 7→8 for both one-write and two-write reuse. Generated fixture tests: 4/4 call-arg and 2/2 vector-action; renderer tests 75/75. `compact-rust-proof-smoke --witness-vector-let ${LOCAL_EVIDENCE}/adr94-call-arg-proof-zkir` replayed, proved, verified, and applied `witnessConst` through ledger-8; emitted keys and ZKIR came from the pinned source compiler. Focused backend/fixture/proof-smoke Clippy `-D warnings`, `cargo fmt --check`, JSON/JS syntax and diff checks passed. Freshness checker scanned all 139 fixtures and found only `witness-vector-action` stale after the first fixture refresh; its exact fresh compiler output was copied and byte-compared, as was the call-arg fixture. Root integration will run the combined full local gate.
