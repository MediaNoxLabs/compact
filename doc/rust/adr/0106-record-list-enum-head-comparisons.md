---
id: RUST-ADR-0106
alias: ADR-0106
title: "Record List enum head comparisons"
date: 2026-10-05
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["recording", "list", "enum"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: 67650f6d5fc6abe5de970a097aa59b0bf59dee726541831288e9de111d7ebf01
---
# RUST-ADR-0106 — Record List enum head comparisons

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Accepted matching enum List-head comparisons through correctly typed Maybe projections and scoped expected values. Wrong enum identities, paths and element types remain refused; proof and parity evidence covers the original ListEnum export.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#209 closure](https://github.com/MediaNoxLabs/compact/issues/209#issuecomment-6017584378). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`1320388d`](https://github.com/MediaNoxLabs/compact/commit/1320388dacb5a5ed5c0da1d80397f03d75678aa0). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
adr: 106
status: accepted
date: 2026-10-05
milestone: rust-backend-v2
issue: https://github.com/MediaNoxLabs/compact/issues/209
```

## Historical decision and amendments

### Problem and source evidence

The checked TypeScript ADT suite compiles `examples/adt/tests/list_enum.compact::test`, a proof-required stateful circuit. The integrated Rust compiler accepts schema-11 source and emits typed native List operations, but declines a recorded/observed call at the assertion `c.head().value == one`. Its recorded comparison admits `List<Field>` only, although ADR-0101 already records List head and ADR-0095 already retains scoped enum locals. This source has one ledger field, `List<Names>`, and zero-argument `test(): []`; no new Compact syntax or ledger representation is needed.

### Before and after generated Rust

Before, the generated crate has a native `test` and a `recording` module without `test_call`; proof-backed application cannot use the source through the observed path.

After, native behavior remains its existing typed `List<Names>` query and assertion. The recorded emitter produces the equivalent of:

```rust
let __compact_recorded_enum_0: crate::types::Names = crate::types::Names::bill;
let (frame, __compact_recorded_head_1): (_, crate::types::Maybe) =
    crate::ledger_slots::c.record_head::<crate::types::Maybe, _, _>(frame)?;
if __compact_recorded_head_1.value != __compact_recorded_enum_0 {
    return Err(runtime::CompactError::AssertionFailed(message.into()));
}
```

The type name above is illustrative; the generated AST must use the concrete schema-11 `Maybe<Names>` representation and hygienic identifiers.

### Decision and ownership

Retain the schema-11 typed `Expr::ListHead` and `Expr::StructField` projection from ADR-0101. In the recorded AST emitter, admit head `value` equality/inequality when the declared List element is the exact `Type::Enum` of the head result and the other operand is a validated same-type enum local, parameter, or variant. Reuse `cell_source` and existing scoped enum Let lowering from ADR-0095. Keep List VM serialization, enum codec, gas, and proof inputs in the existing runtime and ledger-8 primitives; no new runtime API or parallel List model. Reject mismatched enum types, head paths, and element types other than Field or a matching Enum.

### Acceptance and limits

Create a fresh TS oracle and compare generated native and recorded results, initial/final serialized state, four gas dimensions, ordered public VM operations, private outputs, and Verify replay. Compile source with pinned schema-11 frontend and run pinned ZKIR 2.1.0 proof generation, verification and ledger-8 application. Add a focused renderer negative guard for mismatched enum typed head, and a checked positive-source inventory row only after full source parity. The denominator should grow by one proof-required export and the available count by one; unrelated List source gaps remain.

### Tracking

- Predecessors: [ADR-0095 — Record local enum keys in Set assertions](0095-record-local-enum-keys-in-set-assertions.md), [ADR-0101 — Record nested List queries in ADT assertions](0101-record-nested-list-queries-in-adt-assertions.md).
- Issue: [#209](https://github.com/MediaNoxLabs/compact/issues/209).
- Delivery: signed GPG/DCO local commit `eebc40cc90597ffc8808480a24fbe335b9bef899` based on integrated `1320388d`; parent integration pending.
### Local validation, 2026-10-05

- Schema-11/ABI 37 frozen frontend accepted the TS and Rust target for `list_enum.test`, with compiler `pure:false, proof:true`; generated capability is `recorded:true, observed_call:true`.
- Fresh TS capture has 16 queries, 160 ordered public VM operations, zero private outputs and result `[]`; aggregate gas is readTime 85,000,000, computeTime 1,234,450,028, bytesWritten 212, bytesDeleted 408. The generated crate test passed typed unit result, initial/final serialized state, native/recorded effects, all four TS gas dimensions, ordered VM shape and Verify replay.
- Pinned ZKIR 2.1.0 compiled `test.bzkir` SHA-256 `2a6d7868cc95f2d69776ffc684a1f98f3e3959dc8c657e78f0bbed3aac34f99a`; proof generation, verification, observed call preparation and ledger-8 application passed with final empty `List<Names>`. Isolated artifacts: `${LOCAL_EVIDENCE}/adr106-list-enum-proof`.
- Checked ADT List scope: 2 TS-positive and Rust-positive proof-required sources. 143 generated fixtures, 0 stale/failed; 81 renderer tests, 18 inventory tests, targeted generated-crate test, focused deterministic gate `${LOCAL_EVIDENCE}/adr106-focused-gate/receipt.json`, Clippy `-D warnings`, rustfmt and `git diff --check` passed.
- Exact integrated-base inventory delta: 932→933 declarations, 297→298 proof-required, 222→223 available, missing 75 unchanged, unassessed exports 43 unchanged. The one added row is `examples/adt/tests/list_enum.compact::test`.
- No Scheme IR, runtime, ledger package, or user documentation was changed; the recorded AST emitter widened only the exact typed List head comparison. Other ADT List element and expression gaps remain separate.
