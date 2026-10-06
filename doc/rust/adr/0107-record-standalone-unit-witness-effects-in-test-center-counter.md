---
id: RUST-ADR-0107
alias: ADR-0107
title: "Record standalone Unit witness effects in test-center Counter"
date: 2026-10-05
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["recording", "witness", "counter"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: 2f609a492e66e5ac98707c765322098f9b594bd3834525efd45eddcb598ff801
---
# RUST-ADR-0107 — Record standalone Unit witness effects in test-center Counter

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Accepted direct zero-argument Unit witness expression effects with one private invocation and its empty Unit encoding after the public Counter action. This supports the original Counter export and its proof/application, without admitting arbitrary discarded expressions.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#210 closure](https://github.com/MediaNoxLabs/compact/issues/210#issuecomment-6017586089). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`622fa954`](https://github.com/MediaNoxLabs/compact/commit/622fa9546cf75a3b064e153b751bce4dd3b8ed46). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
adr: 107
status: accepted
date: 2026-10-05
milestone: rust-backend-v2
issue: https://github.com/MediaNoxLabs/compact/issues/210
```

## Historical decision and amendments

### Problem and source evidence

The original `test-center/test-contracts/counter.compact` is TypeScript-positive in `compiler/test.ss` and `tests-e2e/src/tests/compiler/compiler.runtime.e2e.test.ts`. Its exported `increment(): []` first increments a public Counter, then calls `private_increment(): []` as a standalone witness expression. The Rust target compiles and its `contract-info.json` marks `increment` `proof=true`, but capability schema 3 reports recorded and observed-call unavailable at `actions[1]`, `StateAction::Expression`. A native Rust method exists; the recorded emitter lacks this narrow effect form.

The other test-center sources compile for TypeScript but currently reject in Rust for separate reasons: `welcome` constructor fold, `bboard` nested ledger query, `micro-dao` unsupported standard-library expression, and `coracle` nested ledger query. They are not claimed by this ADR.

### Before and after generated Rust

Before, `Contract::recording.increment` and `increment_call` are absent. After, the generated recorded body preserves public and private source order:

```rust
let frame = ledger_slots::round.record_increment(frame, 1)?;
let (frame, ()): (_, ()) = frame.try_witness_metered(|context, meter| {
    witnesses.private_increment(context.witness_context_with(LedgerView {
        state: context.query.state.get_ref(), meter,
    }))
})?;
```

The exact emitter may use a hygienic ignored binding. The public API stays a typed generated Rust method, and recorded execution emits the witness private output, private-state update and read-meter cost through the existing runtime path. It must not silently drop a Unit witness as a no-op.

### Decision and ownership

Admit `StateAction::Expression` in recorded lowering only when its value is a direct `Expr::WitnessCall` with no arguments and the compiler witness declaration has no parameters and `Type::Unit` result. Preserve its position after the Counter operation. Route the call through existing `RecordingFrame::try_witness_metered` and generated `TryWitnesses`, with the declared `LedgerView`; reuse runtime FAB encoding, private-state threading and ledger-8 Counter slot behavior. Leave all other expression actions unavailable with the existing structured reason. No IR/schema or runtime ABI change.

Add the original source to a checked TypeScript/Rust cohort only after generated native and recorded crates compile. Keep original source identity, compiler proof applicability and recording status separate. Build a fresh TypeScript oracle for a witness that increments private state; compare native and recorded final public Counter, private state, four gas dimensions, ordered public VM operations, private/FAB outputs and replay. Run pinned source-to-proof-to-ledger-8 application if local toolchain permits. Add a generated fixture and focused renderer negative guards.

### Expected inventory and scope

The 28 unassessed exported circuits under five `test-center/test-contracts` sources gain only `counter.increment` in this slice: known proof-required +1 and, if recording succeeds, available +1 with missing unchanged; unassessed -1. No lexical declaration changes. `welcome`, `bboard`, `micro-dao`, and `coracle` remain explicitly unassessed and require their own designs. Compiler acceptance alone is not semantic parity.

### Tracking

- MediaNoxLabs issue: pending.
- Delivery: proposed; no code or parity claim yet.

Issue: https://github.com/MediaNoxLabs/compact/issues/210 in `rust-backend-v2`.

### Local delivery, 2026-10-05

Signed GPG/DCO conventional commit `5c7b6cdc` on `codex/adr107-test-center`, base `622fa954`, implements the guarded direct zero-argument Unit witness expression in recorded AST lowering. The original test-center Counter compiles for TS and Rust with proof-required recorded and observed-call APIs; other expression actions retain structured unavailable reasons. No IR schema or runtime ABI change. The new generated crate, checked source cohort, fixture and TypeScript capture keep original source identity.

TypeScript witness callback sees public round 1 and private state 7 after the public increment. Native and recorded Rust match serialized initial/final public state, final private state 8, both query gas sums, three ordered public VM tags, one empty Unit FAB output and Verify replay. Pinned `increment.zkir`/`.bzkir` and proving/verifier keys support source-to-proof-to-ledger-8 application; the typed observed-call prototype agrees with manual recording. Focused receipt `${LOCAL_EVIDENCE}/adr107-focused-gate/receipt.json` passes 1/1 recorded. All 143 generated fixtures are fresh, 81 renderer tests, 20 inventory tests and targeted Clippy pass.

Full branch receipt `${LOCAL_EVIDENCE}/adr107-full-inventory.json` from 622fa954 base has 192 sources, 932 declarations, 303 proof-required, 228 available, 75 missing, 343 nonproof, 36 unassessed, zero baseline identity drift/unmatched rows. Exact delta is proof-required +1, available +1, unassessed -1, missing unchanged. Parent has since integrated ADR-0105/0106, so combined counts require root recomputation. Four other test-center contracts remain unassessed. Root exact-head full gate pending; no push or remote CI.
