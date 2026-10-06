---
id: RUST-ADR-0131
alias: ADR-0131
title: "Record a closed conditional Field vector write"
date: 2026-10-05
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["recording", "vector", "ternary"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: 19c358ede6784289371d580ccee5eaab75b66dd5dcbbe60d1e3328423b554103
---
# RUST-ADR-0131 — Record a closed conditional Field vector write

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Accepted the implemented two-element closed conditional Field vector write after an already recorded flag read. Both seeded false/true calls have parity and proof/application evidence; the source-wide API count is separate from those two actual proofs.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#231 closure](https://github.com/MediaNoxLabs/compact/issues/231#issuecomment-6017623220). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`e3eb7fe0`](https://github.com/MediaNoxLabs/compact/commit/e3eb7fe0dc137c322995db35e39d0a1bb9e9351f). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
initiative: compact-rust-backend
milestone: rust-backend-v2
adr: 131
status: accepted
date: 2026-10-05
base: e3eb7fe0
```

## Historical decision and amendments

### Problem and source evidence

The original `examples/rust_backend/ternary_cond_oracle.compact` exports proof-required `streamVectorElement()`. It reads `flag`, constructs `[f ? 1 : 2, f ? 3 : 4]`, and writes `vecCell`. Native Rust compiles, but the schema-12 capability report at integrated base `e3eb7fe0` has no recorded or typed observed-call API. The first unavailable node is `StateAction::Let` at `actions[0].action` after the supported flag Cell read. The value is a single `Vector<Field, 2>` binding from a two-element `Expr::Vector`; each element is an `If` using that already recorded Boolean local. Each true arm coerces a bounded unsigned literal to Field, and each false arm coerces a Field literal to Field. An immediate Cell write consumes the same vector binding. This differs from ADR-0128's scalar Uint annotation and needs its own typed rule.

### Decision and ownership

Admit only a two-element `Vector<Field, 2>` with exactly two closed Field conditional elements and an immediate Cell write of the same binding. Both conditions must resolve from an already recorded Boolean local, and every arm must be a checked small literal: `Coerce<Field>(UnsignedLiteral)` with a bound at most 4 and value within that bound, or `Coerce<Field>(FieldLiteral)` with a value at most 4. Preserve element order in `runtime::FixedVector<runtime::Field, 2>` and use the existing typed `vecCell.record_write`. Reject dynamic elements, unrecorded Cell reads, witnesses, other vector sizes and other element types. The intended change belongs only to the recorded AST emitter and checked generated crate; the runtime, schema 12, ABI 37, ledger-8 and ZKIR remain owned by their existing packages.

### Before and intended generated Rust

Before, the generated crate has native `streamVectorElement`, but no `recorded::streamVectorElement` or `recording.streamVectorElement_call`. The intended recorded sequence is:

```rust
let (frame, flag): (_, bool) = crate::ledger_slots::flag.record_read(frame)?;
let selected: runtime::FixedVector<runtime::Field, 2> = runtime::FixedVector::new([
    if flag { runtime::Field::from(1u64) } else { runtime::Field::from(2u64) },
    if flag { runtime::Field::from(3u64) } else { runtime::Field::from(4u64) },
]);
let frame = crate::ledger_slots::vecCell.record_write(frame, selected)?;
```

Generated identifiers will be compiler allocated. If the native/recorded VM or proof evidence disagrees, revise this decision rather than widening the rule.

### Required acceptance

Use a minimized schema-12 IR fixture and renderer test, including negative cases for a direct unrecorded predicate and witness arm. Capture fresh TypeScript false/true flag calls, seed the true state before call, and compare generated native/recorded initial/final serialized state, unit result, all four gas dimensions, ordered public VM shape, private outputs and replay state/effects. Compile pinned ZKIR 2.1.0 prover/verifier keys and prove, verify and apply both typed observed calls through ledger-8, checking vector values and unchanged flag/Counter. Inventory should move 20/21 → 21/21 recorded APIs for this source if all checks pass. The API count alone is not proof evidence. Use local gates only; no push or remote CI.

### Tracking

Milestone-v2 issue and delivery commit to follow. No implementation or parity result is claimed by this proposed ADR.
### Observed outcome

The emitter implements the guarded two-element Field vector rule. The checked generated Rust follows the proposed structure, with compiler allocated names and a clone at the Cell write because the typed `FixedVector` is retained as a local. [Milestone-v2 issue #231](https://github.com/MediaNoxLabs/compact/issues/231) tracks the change.

Fresh TypeScript false/true captures match generated native and recorded Rust for serialized initial/final state, unit result, four gas dimensions, ordered public VM operations, zero private outputs and replay state/effects. The true flag is seeded before each call and its setup query is excluded from call gas. The TypeScript fixture recaptures deterministically. Pinned ZKIR 2.1.0 generated binary ZKIR and prover/verifier keys; both typed observed calls equaled manual recorded prototypes and proved, verified and applied via ledger-8. The applied vectors are `[2,4]` and `[1,3]`; flag and Counter remain unchanged.

The original-source inventory is now 21/21 proof-required recorded/observed APIs, zero gaps and zero unassessed. This API count is separate from the two actual proofs. Renderer 102/102, generated ternary crate 10/10, targeted Clippy, formatting, and focused local gate `--skip-cargo` pass. Gate receipt: `${LOCAL_EVIDENCE}/adr131-focused-gate/receipt.json`; inventory: `${LOCAL_EVIDENCE}/adr131-ternary-inventory.json`. Full integrated-head local gate is pending parent cherry-pick. No push or remote CI.

### Delivery

Signed conventional GPG+DCO feature commit `3df42bee5594bb044b559996d1aa10f3c7b79cc7` from isolated base `e3eb7fe0` is ready for parent cherry-pick. Signature verified, worktree clean. The focused source receipt and two pinned proofs are local evidence; exact integrated-head package and broad local gate remain pending. No push or remote CI.
