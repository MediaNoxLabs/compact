---
id: RUST-ADR-0132
alias: ADR-0132
title: "Record nested Uint4 widening to Uint64 Cell"
date: 2026-10-05
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["recording", "unsigned-arithmetic", "ternary"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: ce93788b5747eea50e1cc812444cde52fef3bccc8b95ef1ce5b813a77df3788c
---
# RUST-ADR-0132 — Record nested Uint4 widening to Uint64 Cell

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Accepted the exact nested Uint4 selection widened through the existing checked cast to a Uint64 Cell, followed by a literal-one Counter increment. Both branches are proved; alternate widths, amounts and effectful conditions remain refused.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#234 closure](https://github.com/MediaNoxLabs/compact/issues/234#issuecomment-6017628224). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`b0c28f84`](https://github.com/MediaNoxLabs/compact/commit/b0c28f8419e77019317085363dbfd557b9470f83). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
initiative: compact-rust-backend
milestone: rust-backend-v2
adr: 132
status: accepted
date: 2026-10-05
base: b0c28f84
```

## Historical decision and amendments

### Problem and source evidence

`examples/rust_backend/nested_stateful_ternary.compact` has one proof-required export, `run(c)`. Native Rust compiles and the existing TypeScript state fixture passes, but at base `b0c28f84` there is no recorded or typed observed-call API. Schema-12 IR first rejects `StateAction::Let` at `actions[0]`. The local `x: Unsigned<4>` is an outer Boolean `If` whose arms each contain another Boolean `If` over bounded literals 1–4. The next action is a two-action `Sequence`: first a `Let` widens exactly `Parameter x` via `UnsignedCast<18446744073709551615>` into a Uint<64> Cell write to `stored`; second a closed Uint<16> literal-1 Counter increment of `ops`.

ADR-0121 already admits this exact closed nested Uint<4> selector when its immediate projection is `FieldCast` into a Field Cell. It does not admit the Uint<64> projection. The first rejection is therefore structural, not a missing ledger primitive.

### Proposed decision and ownership

Reuse the existing `closed_nested_uint4_source` guard and admit only an immediate `UnsignedCast` from the same Uint<4> binding to the exact Uint<64> maximum, followed by a Cell write of the projected binding. Preserve the `Sequence` order by lowering the Cell write before the existing typed Counter increment rule. Materialize `runtime::BoundedUint<4>` and widen it through the existing runtime `cast_unsigned::<4, 18446744073709551615>` primitive. Require exactly one binding at each nested Let, one projected Cell write, and one continuation. Dynamic/effectful arms, other widths, unrelated projections and reordered effects remain unavailable. This belongs to the recorded AST emitter and checked generated crate; runtime, schema 12, ABI 37, ledger-8 and ZKIR remain unchanged.

### Before and intended generated Rust

Before, the generated crate has only native `run(c)` and no `recorded::run` or `recording.run_call`. The intended recorded sequence is:

```rust
let selected: runtime::BoundedUint<4> = runtime::BoundedUint::<4>::new(
    if c { if c { 1u64 } else { 2u64 } }
    else { if c { 3u64 } else { 4u64 } } as u128,
)?;
let widened: runtime::BoundedUint<18446744073709551615> =
    runtime::cast_unsigned::<4, 18446744073709551615>(selected)?;
let frame = crate::ledger_slots::stored.record_write(frame, widened)?;
let frame = crate::ledger_slots::ops.record_increment(frame, 1u16)?;
```

Generated names and exact formatting will differ. If ledger traces or proofs disagree, revise the decision instead of widening the rule.

### Required acceptance

Use a minimized schema-12 IR fixture with a renderer positive test and a negative test for an effectful or dynamic nested arm. Capture fresh TypeScript true/false calls and compare generated native/recorded serialized initial/final state, unit result, four gas dimensions, ordered public VM shape, private outputs and replay. Compile pinned ZKIR 2.1.0 prover/verifier keys; prove, verify and apply both typed observed calls through ledger-8, checking stored 1 or 4 and Counter 1. The source inventory should move 0/1 to 1/1 recorded API availability only after implementation; availability is separate from proof evidence. Run local checks only, no push or remote CI.

### Tracking

Milestone-v2 issue and signed feature commit to follow. This proposed ADR claims no implementation or parity result yet.
### Observed outcome

The guarded sibling projection is implemented. The checked generated crate now exposes `recorded::run` and typed `recording.run_call`, materializes `BoundedUint<4>`, widens via `cast_unsigned::<4, 18446744073709551615>`, records `stored` Cell write, then records `ops` increment. [Milestone-v2 issue #234](https://github.com/MediaNoxLabs/compact/issues/234) tracks this slice. The renderer negative test leaves alternative projection width 255 and Counter amount 2 unavailable.

Fresh TypeScript true/false capture matches generated native/recorded Rust for serialized initial/final state, unit result, all four gas dimensions, ordered public VM operations, zero private outputs and replay state/effects. The TypeScript wrapper reports the final query gas separately; the test sums both per-query costs. Pinned ZKIR 2.1.0 compiled binary ZKIR and prover/verifier keys. For both branches, the typed observed call equals the manual recorded prototype and proves, verifies and applies through ledger-8; the stored Uint<64> is 1 or 4 and Counter is 1.

Original-source inventory is 1/1 proof-required recorded/observed APIs, zero missing/unassessed. This availability count is distinct from the two actual proofs. Renderer 103/103, nested fixture 2/2, targeted Clippy, formatting, deterministic TypeScript recapture and focused `--skip-cargo` gate pass. Receipt: `${LOCAL_EVIDENCE}/adr132-focused-gate/receipt.json`; inventory: `${LOCAL_EVIDENCE}/adr132-inventory.json`. Exact integrated-head broad local gate remains pending parent cherry-pick. No push or remote CI.

### Delivery

The final renderer guard also rejects a dynamic witness predicate in the outer conditional. Signed conventional GPG+DCO feature commit `c1360f720e1c088a594442fdf7dfaf5025f0a3b3` from isolated base `b0c28f84` is ready for parent cherry-pick; signature verified and worktree clean. The exact integrated-head package and broad local gate remain pending. No push or remote CI.
