---
id: RUST-ADR-0098
alias: ADR-0098
title: "Record closed pure Vector keys in ADT Set calls"
date: 2026-10-05
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["backend", "recording", "pure-calls", "collections", "vectors"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: 26632f09dedfcb6a862a87a9f7f92101908f674bdf97acdcf7d876147e3eab39
---
# RUST-ADR-0098 — Record closed pure Vector keys in ADT Set calls

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Accepted a zero-argument pure helper only when its exact declared Field vector body consists of matching literals, then reused existing typed Set recording. One original Set export and representative proof/application are evidenced; arbitrary vector calls, hashes, witnesses and context-dependent helpers are excluded.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#201 closure](https://github.com/MediaNoxLabs/compact/issues/201#issuecomment-6017570949). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

### Original source metadata

```yaml
adr: 98
status: accepted
date: 2026-10-05
milestone: rust-backend-v2
issue: https://github.com/MediaNoxLabs/compact/issues/201
```

## Historical decision and amendments

### Problem and measured source

`examples/adt/tests/set_vector.compact` compiles to native Rust under private IR schema 11/ABI 37. Compiler `contract-info.json` marks exported `test` proof-required, but the capability report has `recorded:false`, `observed_call:false`, first gap `StateAction::Let` at `actions[0].action`. The first binding `zero: Vector<2, Field> = default<Vector<2, Field>>` already records through `static_bindings`. The second is `one: Vector<2, Field> = getVector()`. Typed IR identifies `getVector` as an internal zero-argument pure circuit with exact result `Vector<2, Field>` and body `Expr::Vector` of Field literals 2 and 12. Native Rust already calls the generated pure function once. The remaining actions are typed Set insert/remove/member/size/isEmpty/reset and assertions that the recorder already handles.

A fresh TypeScript capture of the source has 14 ledger queries, 64 ordered public VM operations, and no private FAB outputs (`${LOCAL_EVIDENCE}/adr95-set-vector-ts-research.json`). This is the next one-row proof capability opportunity after ADR-0095's enum Set slice, not a general Vector-call implementation. `witness_vector_action.compact` has three distinct witness/expression recording gaps, while `list_vector.compact` still rejects at a nested ledger query in the compiler.

### Before and after generated Rust

Before, direct execution exists but no recorded/observed call:

```rust
let one: runtime::FixedVector<runtime::Field, 2> = crate::pure_circuits::getVector()?;
// ledger_contract::recorded::test and contract.recording.test_call are absent.
```

After, recorded execution evaluates the same closed pure helper exactly once before the first use of `one`, then records each Set operation in source order:

```rust
let one: runtime::FixedVector<runtime::Field, 2> = crate::pure_circuits::getVector()?;
let frame = crate::ledger_slots::c.record_insert(frame, one.clone())?;
let (frame, present) = crate::ledger_slots::c.record_member(frame, one.clone())?;
// Size, isEmpty, remove, reset and assertion order follows the source.
```

Names are illustrative; the AST emitter may use hygienic temporaries. The developer still sees a typed generated `Contract` and observed `test_call` method, not VM instructions.

### Decision and ownership

In the `StateAction::Let` Vector branch of the recorded AST emitter, allow `Expr::Call` only when it names a registered pure circuit with zero parameters and arguments; its result exactly matches the binding's declared `Type::Vector { element: Field, length }`; and its entire body is an `Expr::Vector` with matching Field element type, matching length, and only `Expr::FieldLiteral` elements. Bind `crate::pure_circuits::<name>()?` once to the exact Rust vector type and reuse the existing owned-local handling. All other Vector bindings continue through the existing static literal/default path or report a definite recording gap. No arbitrary pure call, hash, witness, effectful helper, or argument evaluation is admitted.

Reuse `FixedVector<Field,N>`, `SetSlot<FixedVector<Field,N>>`, existing `record_insert/remove/member/size/is_empty/reset`, and the pinned ledger-8 VM/serialization mapping. No new private IR variant, schema/ABI increment, runtime API or ledger primitive is planned. ADR-0095's one-lint generated manifest allowance already covers explicit source Boolean assertions; #200 separately tracks typed simplification.

### Acceptance and limits

Freeze one schema-11 Scheme and Rust `compactc`. Require `set_vector.test` to change from proof-required unavailable to recorded+observed available and no other compiler inventory row to change. Add a generated source fixture and renderer guard test for literal zero-argument callee acceptance, nonliteral callee body refusal, and argument-bearing call refusal. Compare TypeScript, native and recorded serialized state, all four aggregated query gas dimensions, all 64 ordered VM operations, zero private FAB output and Verify replay. Generate pinned ZKIR/proving artifacts and prove, verify and apply an observed typed call through ledger-8. Run the focused deterministic gate, checked fixtures, generated crate test, backend/fixture/proof-smoke Clippy and rustfmt; root runs the combined full gate after integration. Distinct Vector witness, List and complex pure-call forms remain open.

### Tracking

- Predecessors: [ADR-0088 — Record nested Set size assertions through typed slots](0088-record-nested-set-size-assertions-through-typed-slots.md), [ADR-0095 — Record local enum keys in Set assertions](0095-record-local-enum-keys-in-set-assertions.md).
- Cohort: [#188](https://github.com/MediaNoxLabs/compact/issues/188).
- Focused issue: [#201](https://github.com/MediaNoxLabs/compact/issues/201).
- Delivery: local signed/DCO commit `504cc95554edc8ca3b457e7311e16e841d035698`, stacked after `efbe5772`; root integration and combined full gate pending. [Issue evidence comment](https://github.com/MediaNoxLabs/compact/issues/201#issuecomment-5984543716).

### Local validation, 2026-10-05

The AST emitter now accepts exactly the closed zero-argument `getVector()` definition described above. It emits a typed local call once, then retains the existing Set-slot recorder. It also narrows the preexisting Bytes-call `Let` arm to Bytes bindings so Vector calls reach their own typed arm. There is no private IR/schema, runtime, ledger, or ZK API change. The renderer guard rejects a nonliteral pure body and an argument-bearing call.

- A frozen schema-11 Scheme (`SHA-256 369ee19b62cba4d6e5f13b0a40599d6b7a38f42e12216ec01ed9ed270747a3d4`) and isolated Rust `compactc` (`56a2027c43737b171960f7d65ad0861a3d541ad324956caa59ed8d64f4b98cbd`) compiled source and generated fixtures.
- Full source inventory changed exactly `examples/adt/tests/set_vector.compact::test`: recorded and observed false→true. Proof availability is 206→207 of 296 required, missing 90→89; all 930 other declaration rows remained identical. Inventory receipt: `${LOCAL_EVIDENCE}/adr98-full-inventory.json`.
- Fresh TypeScript oracle: 14 queries, 64 ordered public VM operations, zero private outputs. Generated Rust fixture checks initial/final serialized state, native-versus-recorded state and effects, aggregate TypeScript gas in `readTime`, `computeTime`, `bytesWritten`, `bytesDeleted`, exact ordered VM shape, zero private output, and Verify replay. Test passed.
- Pinned `zkir` 2.1.0 compiled the source proof artifacts (`test.bzkir` SHA-256 `49fc7c43d06048f90edce53a9cfd9e9bee07719d1515a5b99497101def4bd13b`). Proof smoke passed typed observed-call preparation, proof generation, verification and ledger-8 transaction application; final Set is empty. Artifacts: `${LOCAL_EVIDENCE}/adr98-set-vector-proof`.
- Focused deterministic gate passed (`${LOCAL_EVIDENCE}/adr98-focused-gate/receipt.json`); 141 checked fixtures have zero stale/failed; 77 renderer tests pass; generated fixture test passes; workspace and standalone generated crate Clippy `-D warnings`, rustfmt, and diff whitespace checks pass. Root will run the combined full gate after integration.
- Explicit follow-ups: #200 for source Boolean comparison lint, distinct Vector witness, List and more general pure-call forms.
