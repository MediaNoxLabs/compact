---
id: RUST-ADR-0109
alias: ADR-0109
title: "Record List Field vector head comparisons"
date: 2026-10-05
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["recording", "list", "vector"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: dbc0e1f28c5eed9de82c2cbdccc527e35a55425e971f0fc16e01529eed6f57fa
---
# RUST-ADR-0109 — Record List Field vector head comparisons

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Accepted exact List-head Maybe<Vector<N,Field>> comparison against validated typed expected values. Evidence covers original ListVectorField4 and its proof/application, not Boolean vectors or arbitrary nested composite Lists.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#211 closure](https://github.com/MediaNoxLabs/compact/issues/211#issuecomment-6017587800). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`54d3a8c3`](https://github.com/MediaNoxLabs/compact/commit/54d3a8c3409290c6eed5d2d23b8e9d4bbac49d0d). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
adr: 109
status: accepted
date: 2026-10-05
milestone: rust-backend-v2
issue: https://github.com/MediaNoxLabs/compact/issues/211
```

## Historical decision and amendments

### Problem and source evidence

The TypeScript ADT suite compiles `examples/adt/tests/list_vector_field_4.compact::test`, a proof-required circuit over `List<Vector<4, Field>>`. On the integrated Rust backend at `54d3a8c3`, schema-11 source compilation succeeds and native Rust exists, but recorded capability fails at action 6: `assert(c.head().value == default<Vector<4, Field>>)`. The recorded head-value equality permits Field and Enum (ADR-0106) but not the fixed Field vector already supported as a typed List element and a closed vector value. Later assertions compare `head().value` with `[3, 320, 4, 4]` and the default again. The source has no witness, parameters, or pure circuit calls.

### Before and after generated Rust

Before, native `test` evaluates the typed `List<Vector<4, Field>>` head and compares its `Maybe.value`; `recorded::test` and `Contract::recording.test_call` are omitted by the capability gate.

After, the recorded emitter should retain each head query in source order and compare the concrete value with a validated closed vector:

```rust
let (frame, observed): (_, crate::types::Maybe) =
    crate::ledger_slots::c.record_head::<crate::types::Maybe, _, _>(frame)?;
let expected: runtime::FixedVector<runtime::Field, 4> =
    runtime::FixedVector::default();
if observed.value != expected {
    return Err(runtime::CompactError::AssertionFailed(message.into()));
}
```

The emitted AST can inline the expected expression and uses hygienic identifiers. The literal case retains the four typed Field elements; no VM instruction text is constructed in the emitter.

### Decision and ownership

Keep the schema-11 `Expr::ListHead` plus `Expr::StructField(value)` typed representation. Widen only recorded head-value equality/inequality when the declared List element is a fixed `Type::Vector` whose element is `Type::Field`, the head result is the exact compiler `Maybe<Vector<N,Field>>` type, and `cell_source` validates a same-type closed default, literal, or scoped local. Existing Vector Let lowering, List slot query/record methods, Compact Field codec, and ledger-8 List VM behavior own state, serialization, gas and proof inputs. No Scheme IR or runtime API change is proposed. Head path/type mismatch, non-Field vector elements and dynamic expected expressions remain unavailable.

### Acceptance and limits

Capture a fresh TS oracle and check generated native, recorded and observed calls for result, initial/final serialized state, all four gas dimensions, ordered public VM and private outputs; Verify replay. Run pinned ZKIR 2.1.0 proof generation, verification and ledger-8 application with final empty List. Add a typed renderer negative guard for a non-Field vector head and admit this one source into the checked List cohort only after complete parity. Expected exact inventory movement from integrated `54d3a8c3`: +1 exported proof-required circuit and +1 available; missing and unassessed totals unchanged. Other ADT List source gaps remain separate.

### Tracking

- Predecessors: [ADR-0101 — Record nested List queries in ADT assertions](0101-record-nested-list-queries-in-adt-assertions.md), [ADR-0106 — Record List enum head comparisons](0106-record-list-enum-head-comparisons.md).
- Issue: [#211](https://github.com/MediaNoxLabs/compact/issues/211).
- Delivery: signed GPG/DCO local commit `f26596ef4e6c0c0ac3b4088932cf1260c413ed25` based on integrated `54d3a8c3`; parent integration pending.

### Local validation, 2026-10-05

- Frozen schema-11 frontend SHA-256 `2851285051fcfa9f70e637cb091132ddefdfdd83a50b09d612ad6a3f043580ed` and final Rust `compactc` SHA-256 `2952e130474e25371b544c2b1d4f04e2a5e053c23b14752d69b83a24bffaf877` compiled TS and Rust source. Compiler metadata confirms `test` is `pure:false, proof:true`; generated capability is `recorded:true, observed_call:true`. Schema 11 and runtime ABI 37 are unchanged.
- Fresh TS oracle `runtime-rs/tests/fixtures/adt-list-vector-field-4.json` has 18 queries, 159 ordered public VM operations, zero private outputs and unit result. Aggregate gas is readTime 340,000,000, computeTime 1,425,303,631, bytesWritten 0, bytesDeleted 0. Generated crate test passed native and recorded result, initial/final serialized state, effects, four TS gas dimensions, VM shape/order and Verify replay.
- Pinned ZKIR 2.1.0 emitted `test.bzkir` SHA-256 `a876724ebc3e7eb863268db7e6f9a736167bca1197a785544484d9f60534a1c0`. Proof generation, verification and typed observed call application through ledger-8 passed; final List of `FixedVector<Field, 4>` is empty. Artifacts: `${LOCAL_EVIDENCE}/adr109-vector4-final-proof`.
- A renderer negative guard confirms `List<Vector<4, Boolean>>` remains recorded/observed unavailable. The exact final compiler reproduced the checked-in fixture byte-for-byte. 144 generated fixtures had zero stale/failed; 83 renderer tests and 20 inventory tests passed. The checked singleton source scope accepted TS/Rust/proof metadata; focused deterministic gate `${LOCAL_EVIDENCE}/adr109-final-focused-gate/receipt.json`, targeted generated crate test, rustfmt, Clippy `-D warnings` and `git diff --check` passed.
- Exact integrated-base inventory movement: declarations 933→934, proof-required 303→304, available 231→232, missing 72 unchanged, unassessed exports 37 unchanged. The one added row is `examples/adt/tests/list_vector_field_4.compact::test`.
- No Scheme IR, runtime, ledger package or user documentation was changed. Remaining List source blockers (bytes, opaque, composite structs and nested vectors) are separate decisions.
