---
id: RUST-ADR-0092
alias: ADR-0092
title: "Record direct Boolean witness assertions before ledger writes"
date: 2026-10-05
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["backend", "recording", "witnesses", "assertions"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: 97b4579a4362b4b392a7e57cc8f9e325f99611546d71db6216683d77846e0d44
---
# RUST-ADR-0092 — Record direct Boolean witness assertions before ledger writes

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Accepted direct Boolean witness assertions with typed once-only arguments, metered private transition and assertion-before-write ordering. checked_write gains a recorded API and focused proof; proof-false checked_value remains not applicable. This does not admit arbitrary nested Boolean witness trees.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#194 closure](https://github.com/MediaNoxLabs/compact/issues/194#issuecomment-6017559211). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`0c4d0f11`](https://github.com/MediaNoxLabs/compact/commit/0c4d0f11b12d2892d0a44d106b46ff953e7c1bc8). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
adr: 92
status: accepted
date: 2026-10-05
milestone: rust-backend-v2
issue: https://github.com/MediaNoxLabs/compact/issues/194
```

## Historical decision and amendments

### Problem and measured acceptance source

`examples/rust_backend/assert_witness.compact` declares `echo(Boolean): Boolean`. Its `checked_write` circuit asserts `disclose(echo(flag))` and then writes a `Field` Cell. The pinned compiler emits `Expr::WitnessCall` as the direct `StateAction::Assert` condition and marks `checked_write` `proof_required: true`, yet Rust reports `recording_status: unavailable` at `actions[0]`. The ordinary generated Rust executes in the correct order; it has no recorded circuit or observed-call handle. `checked_value` has the same witness assertion but `proof_required: false` because it has no ledger effect; that circuit is not a proof capability target.

### Before and after

Before, only direct execution is generated:

```rust
let (next_private, allowed) = witnesses.echo(context.witness_context_with(view), flag)?;
context.private_state = next_private;
private_transcript_outputs.push(AlignedValue::from(allowed));
if !allowed { return Err(CompactError::AssertionFailed("write denied".into())); }
let step = ledger_slots::cell.write(context, value)?;
```

After, `ledger_contract::recorded::checked_write` will additionally be emitted from typed IR. Its body should have this shape:

```rust
let frame = RecordingFrame::new(context);
let (frame, allowed) = frame.try_witness_metered(|context, meter| {
    witnesses.echo(context.witness_context_with(LedgerView { state: context.query.state.get_ref(), meter }), flag)
})?;
if !allowed { return Err(CompactError::AssertionFailed("write denied".into())); }
let frame = ledger_slots::cell.record_write(frame, value)?;
Ok(frame.finish(()))
```

The generated `checked_write_call` then accepts an observed ledger state and can be proved and applied. Assertion failure must prevent the write and preserve witness call count/order; the error remains the source string. No handwritten VM operations are permitted in generated code.

### Decision and boundaries

Extend `boolean_expression` in the AST renderer to recognize a direct `Expr::WitnessCall` only when its declared result is `Type::Boolean`, argument count matches, and every argument is already supported by the typed scalar source lowering. Bind each argument once in declaration order before invoking the witness on `RecordingFrame::try_witness_metered`. The frame owns private-state transition, witness ledger read gas, and one aligned private FAB output. The ordinary execution path is unchanged. Unsupported argument shapes retain the structured capability gap. This introduces no new IR node, schema version, runtime public API, or ledger primitive; it reuses the established recorded witness helper and `CellSlot::record_write` path backed by ledger-8 VM primitives. `midnight-zk` is used only in the proof gate.

Do not generalize to arbitrary Boolean expression trees or attempt to claim `checked_value` as proof required. A follow-on can address nested Boolean witnesses if inventory evidence warrants it.

### Validation plan

Freeze the exact compiler and Scheme binaries. Regenerate only `assert-witness` fixture. Assert that `checked_write` changes from unavailable to available and `checked_value` remains `not_applicable`. Compare TypeScript and Rust success/failure for state, four-dimensional gas, ordered public VM trace, private state and FAB outputs, and witness order. Run generated crate tests, a source-to-proof-to-ledger application for passing `checked_write`, and focused Clippy/render tests. On failure, confirm no ledger write or proof call is issued. Check fresh parity inventory for no regressions. Run the broader local gate at integration after concurrent AST slices land.

### Tracking

- MediaNoxLabs issue: pending.
- Branch: `codex/recorded-assert-witness` from `0c4d0f11`.
- Commit and final evidence: pending.


Tracking issue: https://github.com/MediaNoxLabs/compact/issues/194 (rust-backend-v2).

### Local delivery — 2026-10-05

Delivered at local conventional GPG/DCO commit `4ca58a2f98609230c0fbfaaa3ece438717d0f20c` on `codex/recorded-assert-witness`, based on `0c4d0f11`. The exact source `assert_witness.checked_write` moves from proof-required recorded/observed unavailable at `actions[0]` to both available. `checked_value` remains proof-false/not-applicable and `read_cell` remains available. ABI37, typed IR schema10, and capability report schema3 do not change. The generated path evaluates one typed Boolean argument, records one metered witness and private FAB Boolean, checks `write denied`, then records a typed Cell write. The false path records no Cell write.

The TypeScript capture now includes serialized initial/final state, four gas dimensions, ordered public VM shape, private FAB value/alignment, witness order and failure query count. Rust native, recorded and VM replay agree with the TypeScript oracle for the passing write; both Rust paths match TypeScript failure text/order and the passing private state advances 7→8. TypeScript and Rust public VM are `push(false), push(true), ins(false,1)`; gas is readTime `85000000`, computeTime `1233942932`, bytesWritten `38`, bytesDeleted `36`. The failed TypeScript assertion performs zero ledger queries.

Pinned Scheme `${HISTORICAL_NIX_STORE}/g7igz5r3rvdmabz28387fcw7h5y5k6cl-compactc/bin/compactc-scheme` SHA-256 `0032952e62bf60226341f941cae89f98b8e446f6f7452c3547af0367a476c290`, local Rust compactc SHA-256 `e1c4291da98dc9ca74b3af82ea70335694ccffbac3677ab724f23651e7bea406`, and ZKIR 2.1.0 produced `checked_write` proving/verifying artifacts. `compact-rust-proof-smoke --assert-witness ${LOCAL_EVIDENCE}/assert-witness-proof` replayed, proved, verified and applied the guarded Field write in ledger-8. Focused deterministic gate passed at `${LOCAL_EVIDENCE}/adr92-focused-gate/receipt.json`; 73 renderer tests and 3 fixture tests passed; targeted backend/fixture/proof-smoke Clippy `-D warnings`, `cargo fmt --check`, and `git diff --check` passed. Full lexical inventory: 190 sources, 929 declarations, 679 contract exports, 24 module exports, 159 compiled Rust roots, 204/296 proof-required APIs available, 92 gaps, 120 unassessed contract exports, zero declaration baseline drift. Receipt `${LOCAL_EVIDENCE}/adr92-inventory.json`. The root integration will run the combined local full gate after concurrent slices land. No push or remote CI.
