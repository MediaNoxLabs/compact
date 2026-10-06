---
id: RUST-ADR-0029
alias: ADR-0029
title: "Type Set and Map vector arguments from their declarations"
date: 2026-10-03
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["backend", "recording", "ledger-adts"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: f9c7bb4ab1281245102f4da55db9cc2006b4fd0e44b1030503cc290fac828eca
---
# RUST-ADR-0029 — Type Set and Map vector arguments from their declarations

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Accept the retrospective declaration-directed typing fix for Set/Map Vector operands delivered before this note. Tuple surface syntax must not determine FAB encoding when an ADT declaration requires a Vector. Preserve native source/type evidence separately from later recorded Vector support and avoid treating this note as a new implementation commit.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#93 closure](https://github.com/MediaNoxLabs/compact/issues/93#issuecomment-6017400326). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Referenced repository commits:** [`12760a54`](https://github.com/MediaNoxLabs/compact/commit/12760a5481db282b7afef80cd1ff806739fe2c83) · [`dc527e08`](https://github.com/MediaNoxLabs/compact/commit/dc527e088b24ac811a0ad38422c0c7927288fe49). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
adr: 29
status: accepted-partial
date: 2026-10-03
milestone: rust-backend-v2
issue: 93
```

## Historical decision and amendments

### Problem

Oracle [#93](https://github.com/MediaNoxLabs/compact/issues/93) reports that `Set<Vector<2, Field>>.member([0 as Field, 1 as Field])` can silently encode a two-byte key, while `insert`/Map insertion can emit a Rust array of fields that does not build. This is a declared-type boundary problem: the same Compact tuple syntax denotes a Vector value only when the ADT's type is applied. A false negative from a wrong key is worse than a compile failure.

The exact Compact shape is in `examples/rust_backend/vector_key_adt.compact`; it exercises direct vector literals in Set insert/member/remove and Map insert/member/lookup/remove/insertDefault.

### Before and after

```rust
// Oracle codegen-rust example from #93: value is inferred as bytes or a raw array.
let tmp = [0, 1];
let found = set.member(tmp); // Compiles with the wrong Bytes<2> key.
let inserted = set.insert([Fr::from(0u64), Fr::from(1u64)]); // Does not build.
```

```rust
// Current AST backend emits one declared vector type at both operations.
let key: runtime::FixedVector<runtime::Field, 2> = runtime::FixedVector::new([
    runtime::Field::from(0u128),
    runtime::Field::from(1u128),
]);
let step = crate::ledger_slots::keys.insert(context, key.clone())?;
let query = crate::ledger_slots::keys.member(context, key.clone())?;
```

### Decision and ownership

This ADR records the earlier local decision delivered in signed/DCO `12760a5481db282b7afef80cd1ff806739fe2c83` (2026-10-02). It is retrospective; no new code is introduced by this note. The Scheme front end's `compiler/rust-ir-passes.ss` lowers Set/Map operands using `typed-expression-ir` and the ADT's declared argument types, including value expressions with witnesses. The private typed IR distinguishes `vector` from `tuple`; `stateful.rs` verifies operand type against the declaration and emits `syn` AST through named `SetSlot<T>`/`MapSlot<K,V>`. `runtime::FixedVector<Field,2>` owns the CellValue/FAB conversion. Ledger-8 `QueryContext` and VM operations remain the state authority. No proc macro, new midnight-zk primitive, or public ABI bump was required in the original fix; current generated/runtime ABI is 14 and private IR schema is 8.

Typing only the Rust emitter after untyped IR would leave ambiguous aggregate semantics in intermediate steps and could still choose a wrong FAB key. A dedicated vector macro would obscure the declared type. The chosen boundary preserves the Compact type in IR before rendering.

### Verification and risks

`tests-rust-backend/vector-key-adt/tests/vector_key_adt.rs` compares exact serialized ledger-8 TypeScript state after each Set/Map operation, plus membership and lookup results; the pinned capture is `runtime-rs/tests/fixtures/vector-key-adt.json`. The generated crate compiles. On 2026-10-03 at HEAD `dc527e08`, the exact [#93](https://github.com/MediaNoxLabs/compact/issues/93) three-circuit source compiled with `compactc --target rust --skip-zk`, emitted `FixedVector<Field,2>` for all three uses, and `cargo check` passed. The checked-in vector-key fixture test passed again at that HEAD. Earlier delivery recorded 129 fixture freshness and two rejection probes; the current ABI-14 milestone gate has 132 fresh fixtures and 54 offline proof/application calls, but those 54 do **not** include a recorded vector-key Set/Map call. Do not infer proof readiness from the native state parity test.

The original fix changes compiler IR lowering; it does not add a recorded vector-key Set/Map API. `recorded.rs` currently admits narrower scalar Set/Map shapes. Remote CI and publication of this local branch remain unverified. Keep #93 open until the branch is published and remote compiler-backed parity runs; track recorded/proof expansion under #107/#105 and a focused future ADR if exposed.

### Tracking and delivery

- Oracle issue: [#93](https://github.com/MediaNoxLabs/compact/issues/93), assigned to [rust-backend-v2](https://github.com/MediaNoxLabs/compact/milestone/2).
- Parent acceptance: [#104](https://github.com/MediaNoxLabs/compact/issues/104), [#107](https://github.com/MediaNoxLabs/compact/issues/107), [#105](https://github.com/MediaNoxLabs/compact/issues/105).
- Local conventional GPG-signed/DCO delivery: `12760a5481db282b7afef80cd1ff806739fe2c83`; branch unpushed.
- Revalidation: `dc527e088b24ac811a0ad38422c0c7927288fe49`; exact source compilation and fixture test pass.
- State: accepted for native Set/Map vector argument lowering; recorded proof and remote release acceptance remain open.

### Amendments

Append later remote CI, recorded-proof, or superseding decisions; preserve this provenance.
