---
id: RUST-ADR-0127
alias: ADR-0127
title: "Record closed OpaqueString Map Field calls"
date: 2026-10-05
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["recording", "opaque-string", "map"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: 805a694cacbdbe88abaf13db62e50d5da47d4f6c0521bb72f846a88b36afeebf
---
# RUST-ADR-0127 — Record closed OpaqueString Map Field calls

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Accepted exact Map<OpaqueString,Field> parameter insertion and membership-guarded lookup/return. Both success APIs are proved and missing keys fail before lookup. Opaque strings remain input keys encoded as Compress atoms, not variable-length decoded Map values.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#228 closure](https://github.com/MediaNoxLabs/compact/issues/228#issuecomment-6017618296). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Referenced repository commits:** [`1e9ed682`](https://github.com/MediaNoxLabs/compact/commit/1e9ed682d378aa8e4a5e346aa69aa7e2ea718424) · [`e3eb7fe0`](https://github.com/MediaNoxLabs/compact/commit/e3eb7fe0dc137c322995db35e39d0a1bb9e9351f) · [`e41ad486`](https://github.com/MediaNoxLabs/compact/commit/e41ad4869ede55236b5b23e161b1faa69aea3958). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
adr: 127
status: accepted
date: 2026-10-05
milestone: rust-backend-v2
issue: https://github.com/MediaNoxLabs/compact/issues/228
```

## Historical decision and amendments

### Problem and exact typed source

Original `examples/rust_backend/opaque_string_map_query_oracle.compact` compiles for TypeScript and native Rust, but its two proof-required exports have no replayable observed call. Schema-12 first reports `put` at `StateAction::MapInsert actions[0]` and `ensure` at `StateReturn::Expression return_value`. `put(key: OpaqueString, value: Field)` inserts into `Map<OpaqueString, Field>`. `ensure(key)` first asserts Map membership with `missing key`, then binds Field `value = MapLookup(entries, key)` and returns that Field.

### Before and after generated Rust

Before, `put` and `ensure` have only native methods. After, their recorded bodies use the existing typed Map slot in source order:

```rust
let frame = crate::ledger_slots::entries.record_insert(frame, key.clone(), value)?;
let (frame, present) = crate::ledger_slots::entries.record_member(frame, key.clone())?;
if !present { return Err(CompactError::AssertionFailed("missing key".into())); }
let (frame, observed): (_, Field) = crate::ledger_slots::entries.record_lookup(frame, key.clone())?;
```

These statements belong to separate calls. The generated crate also exposes `put_call` and `ensure_call` for ledger-backed observed state.

### Decision and limits

Admit only a Unit two-parameter `(OpaqueString, Field)` circuit with exactly one MapInsert of those parameters, and a Field one-parameter `(OpaqueString)` circuit with exactly one Assert(MapMember) action followed by exactly one typed Field Let binding from MapLookup returned unchanged. Match one declared `Map<OpaqueString, Field>`, physical index, parameter names, Map field/index and source order. Reuse the existing `syn` emitter, runtime `MapSlot<OpaqueString, Field>`, ledger-8 map VM and `RecordingFrame`. Add only a narrow Field Let return bridge. No IR schema or runtime API change. Arbitrary Map/opaque expressions, altered assertions, extra actions, and OpaqueString values remain unavailable.

The variable-length OpaqueString is only an input key. It is encoded as one Compress atom. `record_lookup` decodes the fixed Field value with `CellValue`, so OpaqueString `FromFieldRepr` with field width zero is never used. This codec fact must be verified with fresh TypeScript/native/recorded state, gas, VM and proof evidence before availability is claimed.

### Acceptance

Capture missing-key failure, successful put, successful ensure and a second key from original TypeScript. Compare generated native and recorded result/state, four gas dimensions, ordered public VM, private effects, failure order/message and Verify replay. Compile pinned ZKIR 2.1.0 and prove, verify, ledger-8 validate/apply both generated observed calls; check final Map value and Field result. Add renderer negative guards, generated fixture, checked source inventory and focused local formatting/tests/Clippy. No push or remote CI.

### Tracking

- Predecessor: [ADR-0125 — Record closed OpaqueString Set operations](0125-record-closed-opaquestring-set-operations.md).
- Milestone issue: https://github.com/MediaNoxLabs/compact/issues/228.
- Delivery: signed local commit `30842fbf3e1eed683481bcfd6a37b618554c1e73` on `codex/adr127-opaque-map` (base `e41ad486`); integrated as signed root `1e9ed682d378aa8e4a5e346aa69aa7e2ea718424`; exact packaged receipt at `e3eb7fe0`: 266/316 proof-required APIs available, 50 missing, 25 unassessed, and 147/147 fixtures fresh. Full gate pending.


### Local validation (2026-10-05)

- Fresh original TypeScript capture matches generated native and recorded Rust on `put("asset-1", 42)` and `ensure("asset-1")`: Unit/Field result, serialized state, four aggregate gas dimensions, ordered public VM, zero private outputs and Verify replay. `put` has five VM operations and readTime 170000000, computeTime 1335175991, bytesWritten 164, bytesDeleted 34. `ensure` has nine VM operations over membership then lookup, with aggregate readTime 425000000, computeTime 2591256716 and zero writes/deletes.
- Before insertion and on a different key, TypeScript and native/recorded Rust fail `missing key`. TypeScript records exactly one membership query before the assertion, leaves serialized/public and private state unchanged, and never performs lookup. The emitter puts the generated assertion before its lookup step.
- Pinned ZKIR 2.1.0 compiles `put` at k=6/47 rows and `ensure` at k=6/50 rows. Both generated observed calls prove, verify, ledger-8 validate and apply. The proven `put` stores Field 42; proven `ensure` returns Field 42 and retains that Map value.
- Exact checked source scope passes one TypeScript-positive source and two proof-required calls. Local full inventory after this slice is 195 sources, 935 declarations, 316 proof-required exports, 262 available and 54 missing; this Map source changes 0/2 to 2/2. The local e41 baseline includes other integrated slices, so the root packaged receipt should verify the global delta.
- Renderer negative guards reject an extra insert, a changed inserted Field source and a changed Let return. Focused renderer 98, Map fixture 2, inventory 23, source scope, rustfmt, Clippy and proof smoke pass locally. No push or remote CI.
