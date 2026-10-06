---
id: RUST-ADR-0101
alias: ADR-0101
title: "Record nested List queries in ADT assertions"
date: 2026-10-05
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["compiler", "ir", "backend", "recording", "collections"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: c466b9fc292da2daa4902803acbcd946c3f33ecf244dbed59873a2c600cad022
---
# RUST-ADR-0101 — Record nested List queries in ADT assertions

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Accepted typed nested List length/isEmpty/head queries and bounded Maybe<Field> comparisons for original list_field, delegating to existing native/recorded slots. One added source has TS/replay/proof evidence; eleven other List sources and context-dependent vector helpers remain outside this slice.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#204 closure](https://github.com/MediaNoxLabs/compact/issues/204#issuecomment-6017575956). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`f3142ad5`](https://github.com/MediaNoxLabs/compact/commit/f3142ad5f6a3b5d624f55e6081c431c5d6576f88). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
adr: 101
status: accepted
date: 2026-10-05
milestone: rust-backend-v2
issue: https://github.com/MediaNoxLabs/compact/issues/204
```

## Historical decision and amendments

### Problem and source evidence

The checked TypeScript ADT compiler suite compiles `examples/adt/tests/list_field.compact`; its exported `test(): []` is compiler-marked proof-required. The frozen schema-11 Rust target rejects the source at line 42, `assert(c.length() == 0)`, with “Rust backend does not yet support this nested ledger query.” The diagnostic comes from the `stateful-expression-ir` nested-query fallback, which admits Set/Map queries but omits List `length`, `isEmpty`, and `head`. All twelve `list_*.compact` ADT tests hit that first diagnostic. This ADR scopes the initial vertical slice to `list_field.test`.

### Before and after generated Rust

Before, no Rust crate is generated for the source. Top-level List return queries and typed List slots already exist, while an assertion such as `assert(c.length() == 1)` has no typed expression form.

After, the generated native and recorded paths evaluate each query once at its source position:

```rust
let query = crate::ledger_slots::c.length(context)?;
context = query.context;
total_cost += query.gas_cost;
if query.result != 1 { return Err(runtime::CompactError::AssertionFailed(message.into())); }
```

```rust
let (frame, observed) = crate::ledger_slots::c.record_length(frame)?;
if observed != 1 { return Err(runtime::CompactError::AssertionFailed(message.into())); }
```

`c.isEmpty()` uses the Boolean List slot methods; `c.head()` uses the compiler's concrete `Maybe<Field>` type, preserving `is_some` and `value` projections. Examples are illustrative; emitted AST identifiers are hygienic. The public generated API remains typed `Contract::recording.test_call`.

### Decision and ownership

The Scheme private IR pass emits typed `Expr::ListLength`, `ListIsEmpty`, and `ListHead` only for zero-argument public ledger queries on a known List field and fixed valid path. The Rust IR retains field/index discriminants and the precise `Maybe<Field>` result type for head, rather than normalizing to an untyped VM value. The native AST emitter charges each query in source order and threads the updated circuit context. The recorded AST emitter calls existing `ListSlot::record_length/is_empty/head` in the same order, threads the recording frame, and only accepts typed comparisons/projections it can verify. Existing runtime `ListSlot`, `Maybe`, ledger-8 List VM programs, and ZK mapping own serialization and gas. Add runtime support only if the parity proof exposes a real gap; do not create a parallel List implementation.

The first source may expose a second, later diagnostic after the current compiler rejection is removed. Keep unsupported query shapes unavailable with a typed diagnostic or capability gap. In particular, `list_vector` has a separate `getVector()` call that reads `kernel.self()` and is outside ADR-0098's closed literal-vector rule.

### Acceptance and limits

Before source admission, capture a fresh TS oracle and the compiler's `proof:true` metadata. Add `list_field.compact` to the checked positive inventory only after Rust compile, native execution, recorded/observed API and source proof pass. Exact integrated-root expected inventory movement: declarations 931→932, proof-required 296→297, available 218→219 while missing stays 78. The isolated branch directly based on ADR-0098 measures 207→208/297 and 89 gaps unchanged; the root baseline includes eleven other previously integrated availability gains. Check initial/final serialized state, all four gas dimensions, ordered public VM operations, private outputs, Verify replay, and pinned ZKIR proof generation, verification, and ledger-8 application. Add focused renderer/compiler negative guards for unsupported List query shapes and run local fixture, rustfmt and Clippy checks. The other eleven List ADT sources require separate measured expansion.

### Tracking

- Predecessors: [ADR-0095 — Record local enum keys in Set assertions](0095-record-local-enum-keys-in-set-assertions.md), [ADR-0098 — Record closed pure Vector keys in ADT Set calls](0098-record-closed-pure-vector-keys-in-adt-set-calls.md).
- Issue: [#204](https://github.com/MediaNoxLabs/compact/issues/204).
- Delivery: signed GPG/DCO local commit `8e2ea1467a1f853338c6d353a143ead8034956ba` stacked on ADR-0098 `504cc955`; root integration and combined exact-head gate pending.
### Local validation, 2026-10-05

- Scheme source snapshot `compiler/rust-ir-passes.ss` SHA-256 `f4e4c3f081a2759413557195b9c4893e37d376f52bc558dda29e3385197f6c14` matched the isolated frontend snapshot; Rust `compactc` SHA-256 `1d3e3e435170f5932b9008b60836579ac22a10188cafeed7c29201a073d19280`. Schema 11/ABI 37 remained unchanged. No runtime or ledger API changed.
- The source compiles for TS and Rust, with `test` compiler-marked `pure:false, proof:true`; generated capability is `recorded:true, observed_call:true`. The checked one-source List cohort and lexical baseline add precisely `list_field.test`.
- Fresh TS oracle `runtime-rs/tests/fixtures/adt-list-field.json` has 18 queries, 159 ordered public VM operations, zero private output, and aggregate gas `readTime=340000000`, `computeTime=1425303631`, `bytesWritten=0`, `bytesDeleted=0`. The generated crate test passed initial/final serialized state, native/recorded effects and state, all four TS gas dimensions, VM shape/order, zero private output, and Verify replay.
- Pinned ZKIR 2.1.0 emitted `test.bzkir` SHA-256 `dfd50d003ad6f867b601c0ff50d347a1ac2c00899362ca171d8b5c66ce8b3372`. Source proof was generated, verified, and applied through ledger-8; typed observed call matched manual recording and final List was empty. Artifacts: `${LOCAL_EVIDENCE}/adr101-list-field-proof`.
- Focused deterministic gate passed at `${LOCAL_EVIDENCE}/adr101-focused-gate/receipt.json`; 142 checked fixtures had zero stale/failed; 78 renderer tests and 16 inventory tests passed; checked positive-source scope accepted TS/Rust/proof flags. Targeted workspace and standalone generated crate Clippy `-D warnings`, rustfmt, and whitespace checks passed.
- Exact isolated-branch inventory change from ADR-0098: one added row, no changed or removed rows; declarations 931→932, proof required 296→297, available 207→208, missing 89 unchanged, unassessed 120 unchanged. Integrated root f3142ad5 predicts 218→219 available with 78 missing unchanged after cherry-pick. Root combined gate remains pending.
- Scope limit: the other eleven `list_*.compact` sources and context-dependent `list_vector.getVector()` call have not received executing parity or proof claims.
- GitHub local delivery evidence: [issue #204 comment](https://github.com/MediaNoxLabs/compact/issues/204#issuecomment-5987174997).
