---
id: RUST-ADR-0014
alias: ADR-0014
title: "Represent nested Map slots as typed shapes"
date: 2026-10-02
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["architecture", "generated-api"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: 4607dda2088cefc917fb2d1dfc49f775ef5bbd354e266026963050534672b9eb
---
# RUST-ADR-0014 — Represent nested Map slots as typed shapes

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Accept typed structural slots for nested Map nodes without treating inner ledger nodes as ordinary CellValue values. The delivered emptiness/query slice and later independent TypeScript capture close that exact evidence gap. Populated nested-Map value operations and broader collection semantics are not implied by this slot representation.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#115 closure](https://github.com/MediaNoxLabs/compact/issues/115#issuecomment-6017423026). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Referenced repository commits:** [`04cb992d`](https://github.com/MediaNoxLabs/compact/commit/04cb992de6fb0bf659493feab438786ef416739f) · [`fa97aa45`](https://github.com/MediaNoxLabs/compact/commit/fa97aa457c877282c119946d66104dd217e6a50c). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
adr: 14
status: accepted-partial
date: 2026-10-02
milestone: rust-backend-v2
issue: https://github.com/MediaNoxLabs/compact/issues/115
```

## Historical decision and amendments

### Problem

A Compact ledger declaration `Map<Field, Map<Field, Uint<64>>>` is omitted from generated `ledger_slots`: the inner Map is a ledger state node, not a FAB `CellValue`. Generated native `isEmpty()` therefore falls back to `context.is_empty_map(1)?` and an external Rust consumer cannot use a named, typed descriptor. Treating the inner Map as an ordinary value would incorrectly expose scalar `insert`/`lookup` methods. The public generated crate should express the shape without claiming a value codec.

### Before and after

```rust
// Before: only flag has a descriptor; nested Map query has a raw path.
let step = context.is_empty_map(1)?;
// After: the nested type appears in the generated API.
pub const users_by_org: runtime::slots::MapSlot<
    runtime::Field,
    runtime::slots::MapNode<runtime::Field, runtime::BoundedUint<18446744073709551615>>,
> = runtime::slots::MapSlot::new(&[1u8]);
let step = crate::ledger_slots::users_by_org.is_empty(context)?;
```

`MapNode<K, V>` is a zero-sized type marker for ledger structure, not a Compact value. Shape reads (`is_empty`, `size`, `member`) and their recorded forms require a typed key and a physical path. Scalar value operations (`insert`, `lookup`, and their recorded forms) remain available only for `V: CellValue`. Keep value-changing `remove` and `reset` on scalar slots until nested transition parity is proved. A consumer should get a Rust type error when it tries to insert or lookup a `MapNode` as a value.

### Emitter, runtime, and compatibility

The renderer recursively lowers `Type::LedgerMap` only for generated slot types; general Compact-to-Rust value lowering still rejects it. Share this eligibility rule between slot declaration and native `MapIsEmpty` emission, so the emitter never refers to an omitted slot. Runtime `MapSlot` separates shape methods from scalar value methods and adds the public marker; no VM program or ledger serialization changes. Bump both runtime and generated ABI assertion from 4 to 5 because new generated source requires the marker and new method bounds. Keep private JSON IR schema 8. Regenerate all checked-in fixture libraries and update compatibility docs.

### Alternatives and rationale

A raw numeric fallback leaves the declaration hidden. Implementing `CellValue` for nested Map would misrepresent a state container as a serializable scalar. A body-wide macro would hide method availability and ledger path ownership. A type-level marker gives readable generated source and lets ordinary Rust bounds prevent unsupported scalar operations.

### Verification and risks

Test emitted nested slot syntax and scalar/nested method availability from separate consumer crates; run native/recorded shape reads against ledger-8, compare their state and gas, and compile-fail a nested scalar lookup or insert. Preserve the 37-source oracle inventory by testing the existing nested-Map fixture without changing its Compact source. Run all 130 fixture comparisons, backend/workspace tests and the packaged compiler/consumer/proof gate. Main risks are an ABI mismatch, accidentally exposing a value method on the marker, or changing path/VM behavior. Record exact gates and limitations in a dated delivery amendment.

### Tracking and delivery

- Focused MediaNoxLabs issue: pending, linked to #108 and #106.
- Milestone: rust-backend-v2.
- Local commit: pending.
- Delivery state: proposed.

### Amendments

Append dated evidence, corrections, or a superseding decision without rewriting this proposal.


#### 2026-10-02 — Local delivery (ABI 5)

Focused issue [#115](https://github.com/MediaNoxLabs/compact/issues/115) is in [rust-backend-v2](https://github.com/MediaNoxLabs/compact/milestone/2), with parent [#108](https://github.com/MediaNoxLabs/compact/issues/108) for typed slots and [#106](https://github.com/MediaNoxLabs/compact/issues/106) for the runtime ABI. Conventional GPG-signed/DCO commit `fa97aa457c877282c119946d66104dd217e6a50c` delivers the local implementation on `codex/rust-backend-ast`; Git signature status `G` and sign-off were verified.

The generated crate now exposes `ledger_slots::users_by_org: MapSlot<Field, MapNode<Field, BoundedUint<{ u64::MAX as u128 }>>>`. Native `check_nested_empty` compiles to `ledger_slots::users_by_org.is_empty(context)?`, and a Rust consumer can read `member`, `size`, and `is_empty` (or record those operations) through that named shape. Trying `users_by_org.lookup(context, key)` fails at compile time because `MapNode` does not implement `CellValue`.

The emitter recursively lowers ledger Map values only for slot declarations; ordinary value lowering still rejects nested Maps. The runtime moves shape reads to `MapSlot<K: CellValue, V>` and retains scalar value operations in `MapSlot<K: CellValue, V: CellValue>`. This is an API and generated-source change, so both sides assert ABI 5. The private IR stays schema 8. Ledger path encoding, state serialization and VM programs are unchanged.

Evidence: 52 backend renderer and four CLI tests, the runtime suite and workspace check; 131/131 generated fixture outputs current; 37/37 pinned oracle sources; two rejection probes; external shared-runtime consumer including the nested shape and compile-fail lookup; native/recorded shape reads with matching gas, state and replay; generated `check_nested_empty` native/recorded fixture; packaged compiler gate with ZKIR and keys, trace replay/partition, proof, verification, validation and application for 45 offline ledger-8 calls including the new circuit. Both `.crate` archives packaged and compiled (runtime archive 225 entries), and the saved release manifest was verified. Formatting, diff checks and headers passed (0 missing across 1,465 files).

Limits: nested Map lookup/insert/remove/reset still lack proved nested value semantics; only shape reads are exposed. The new circuit has offline proof/application evidence, not a separate TypeScript capture. The 37 pinned oracle source inventory is unchanged. Branch publication, clean remote CI, registry publication, a real candidate tag and wallet/node submission remain open. The focused issue remains open for remote CI.


Post-commit release check: `target/rust-runtime-abi5-clean.json` was written and verified after commit; its source record binds commit `fa97aa457c877282c119946d66104dd217e6a50c`, tree `56af2f71237cdcf6d4ace6d16968a51a3994b92c`, and `dirty: false`. This local ignored artifact is a rehearsal manifest, not a published release. The proposal-time “pending” tracking lines above remain as history; this amendment supplies the delivered IDs and state.


#### 2026-10-02 — Cross-target evidence amendment

The first ABI-5 delivery had native/recorded and offline proof evidence for `check_nested_empty`, but no TypeScript capture from that exact new source. That left result, state and gas parity inferred from similar collection fixtures. Local conventional GPG-signed/DCO commit `04cb992de6fb0bf659493feab438786ef416739f` adds a reproducible TypeScript capture and a Rust comparison test under [#115](https://github.com/MediaNoxLabs/compact/issues/115).

```rust
// Before: a native-only truth assertion.
assert!(native.result);
// After: the pinned TypeScript result, state bytes and each gas dimension.
assert_eq!(native.result, oracle["result"]);
assert_eq!(state_hex(native.context.query.state.get_ref().clone()), oracle["afterCall"]);
assert_eq!(native_gas["computeTime"].as_u64().unwrap().to_string(), oracle["gas"]["computeTime"]);
```

`capture-nested-map-shape.mjs` compiles the same Compact source with `compactc --target ts --skip-zk` and captures serialized initial/final `ContractState`, Boolean result, four-dimensional reported gas and private output count. The Rust fixture compares all of them, then checks recorded/native gas/state/effects and replay. It passes; running the capture again produced byte-identical JSON. TypeScript and Rust both return `true`, retain identical serialized state, and report read/compute/write/delete costs `170000000/1252757404/0/0` for this one-query circuit. Runtime and emitter code, IR schema and ABI are unchanged by this test-only commit. Formatting, header validation (0 missing in 1,466 files), diff check, signature and DCO pass.

This closes the previously noted dedicated TypeScript capture gap for this exact circuit. It does not establish populated nested Map value operations or broader oracle coverage. Remote CI, release and wallet/node submission remain open.
