---
id: RUST-ADR-0113
alias: ADR-0113
title: "Record closed unsigned ternary comparisons"
date: 2026-10-05
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["recording", "ternary", "unsigned-arithmetic"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: e08653925b2f4f5d80238a88a4add2869a8a06f61e3846a6f76100f4b5936284
---
# RUST-ADR-0113 — Record closed unsigned ternary comparisons

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Accepted closed typed unsigned ternary comparisons with validated casts and both compile-time branches, while excluding dynamic or effectful arms. Historical proof and TS cases cover walkerCompareEq both flags and streamCompareEq false, not every stream branch.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#216 closure](https://github.com/MediaNoxLabs/compact/issues/216#issuecomment-6017597035). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Referenced repository commits:** [`45217fdd`](https://github.com/MediaNoxLabs/compact/commit/45217fdd2d8c9f9ebc9903ad8d5793ee336f7aa3). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
adr: 113
status: accepted
date: 2026-10-05
milestone: rust-backend-v2
issue: https://github.com/MediaNoxLabs/compact/issues/216
```

## Historical decision and amendments

### Problem and source evidence

At integrated `45217fdd`, `examples/rust_backend/ternary_cond_oracle.compact` has ten proof-required circuits whose first recorded gap is `StateAction::Let`. Two have the same bounded comparison shape: `walkerCompareEq(c, x: Uint<8>)` binds `b = x == (c ? 1 : 0)` and `streamCompareEq()` reads Boolean `flag` before binding `b = 1 == (f ? 1 : 0)`. Schema-11 IR makes the walker right operand `UnsignedCast<255>(If(Coerce<1>(1), Coerce<1>(0)))`; the stream right operand is the corresponding `If` at maximum 1. Both then convert `b` to Field 1/0 and write `fieldCell`. Rust source and native code compile, but `boolean_expression` handles observed cell comparisons and Boolean literal comparisons, not this closed unsigned ternary equality, so recorded and observed calls are absent. The other eight blockers have distinct typed shapes.

### Before and after generated Rust

Before, native `walkerCompareEq` and `streamCompareEq` evaluate their typed expressions and write `fieldCell`, while the generated `recorded` module lacks these calls.

After, the recorded AST retains a typed Boolean selection and writes the resulting Field in source order:

```rust
let selected: u128 = if c { 1u128 } else { 0u128 };
let b: bool = x.value() == selected;
let value: runtime::Field = if b {
    runtime::Field::from(1u128)
} else {
    runtime::Field::from(0u128)
};
let frame = crate::ledger_slots::fieldCell.record_write(frame, value)?;
```

The stream form first obtains `f` through `flag.record_read(frame)` and compares the literal 1 to the selected value. Identifiers are illustrative; emitted AST names are hygienic.

### Decision and ownership

Add a private typed helper to recorded Boolean lowering for equality or inequality of a validated bounded unsigned operand and a closed ternary of two bounded unsigned literals. It checks the compiler's `UnsignedCast` target, each arm's `Coerce` and literal maximum/value, equal arm types, and the other operand's matching declared Uint type or literal maximum. The condition must resolve to an existing typed Boolean parameter or already recorded local; no branch may carry a ledger query, witness, call, or dynamic arithmetic. Compare the validated numeric values without VM emission and bind the Boolean once. Reuse existing Boolean Let, Field `If` conversion, `CellSlot::record_read/write`, runtime `BoundedUint`, Field conversion, and ledger-8 VM/gas ownership. No schema, runtime API or ledger implementation change.

### Acceptance and limits

Capture fresh TypeScript oracle executions for walker and stream from the same constructor state used by existing ternary fixtures. Check generated native/recorded/observed result, initial/final serialized state, all four gas dimensions, ordered public VM, private outputs and Verify replay for each. Run pinned ZKIR 2.1.0 proof generation, verification and ledger-8 application for both calls. Add negative renderer guards for mismatched width, dynamic/effectful ternary arms and out-of-range literals. Check exact capability inventory: two available proof circuits gained, no denominator change and other eight first blockers unchanged. Run local focused fixture, rustfmt and Clippy checks. Do not edit user-facing documentation.

### Tracking

- Predecessors: [ADR-0101 — Record nested List queries in ADT assertions](0101-record-nested-list-queries-in-adt-assertions.md), [ADR-0111 — Record literal Bytes List assertions](0111-record-literal-bytes-list-assertions.md).
- Issue: pending.
- Delivery: pending.


### Local validation (2026-10-05)

- Frozen schema-11 source compile changed only `walkerCompareEq` and `streamCompareEq` from recording unavailable to recorded plus observed. The other eight ternary Let blockers remained unavailable. Focused gate passed with 13/21 recorded circuits.
- Full repository inventory kept 935 declarations and 306 proof-required exports; proof available rose 237→239 and missing fell 69→67.
- Fresh TypeScript oracle and generated Rust native/recorded executions agree in result, serialized initial/final state, all four gas dimensions, ordered public VM operations, zero private outputs, and Verify replay for walker true/false and stream false. TypeScript reports the final query gas for a multi-query call; the test also compares the sum of captured query gas with the Rust call total.
- Pinned ZKIR 2.1.0 compiled all 21 circuits. Generated observed calls for all three cases proved, verified and applied through ledger-8; typed and manual traces agreed. 87 renderer tests, five ternary fixture tests, 146 fixture snapshots, rustfmt, Clippy, and the focused gate passed.
- A valid dynamic ternary arm stays recording unavailable. The typed IR validator rejects mismatched widths; literal parsing and bounds checks reject out-of-range arms in the new lowering. No new runtime primitive or schema change was needed.
- Milestone issue: https://github.com/MediaNoxLabs/compact/issues/216. Delivery commit: pending signed local commit.

### Delivery

Signed local commit `c2d0a775e3cb3b25324ef597a0eaa4f94225d245` (`%G? = G`, DCO trailer present), based directly on `45217fdd`. This commit is ready for parent integration; no branch push or remote CI was used.
