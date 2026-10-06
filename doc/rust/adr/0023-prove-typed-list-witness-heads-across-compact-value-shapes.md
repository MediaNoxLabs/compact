---
id: RUST-ADR-0023
alias: ADR-0023
title: "Prove typed List witness heads across Compact value shapes"
date: 2026-10-02
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["runtime", "witnesses", "gas"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: c3ef950f6680255e1a3f2e7c8828f0eb3d9be1563e17f53f40898e4e1dbe4ee5
---
# RUST-ADR-0023 — Prove typed List witness heads across Compact value shapes

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Accept the targeted cross-value List-head evidence and accompanying typed recorded-write support for Boolean, bounded Uint, Bytes, enum and record elements. This extends ADR0018, preserving exact empty/default Maybe, FAB, gas and operation-order checks. It does not establish all elements, paths or expression combinations.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#123 closure](https://github.com/MediaNoxLabs/compact/issues/123#issuecomment-6017437220). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Referenced repository commits:** [`26205ca2`](https://github.com/MediaNoxLabs/compact/commit/26205ca2fb91f1a203f8c19fdcb713a25e2847a4). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
adr: 23
status: accepted-partial
date: 2026-10-02
milestone: rust-backend-v2
issue: 123
extends: 18
```

## Historical decision and amendments

### Problem and evidence

ADR-0018 / #119 established `MeteredListView<Field>` parity for head, emptiness, and length. The generated view is generic, but a passing Field test does not establish Compact alignment, default absent payload, VM Maybe decoding, and gas parity for other element shapes. A fresh `List<Choice>` compiler probe confirms the generated crate and a consumer call to `head()` compile: `CompactEnum` already derives `Default` to the first variant. That disproves an initial suspicion that enum head lacked a default. The open problem is behavioral evidence, especially empty and populated heads.

### Before and after developer code

```rust
// Before: this API compiles, but only List<Field> has pinned cross-target behavior.
let first = context.ledger.items()?.head()?; // Option<Field>
```

```rust
// After: the same typed API is exercised across generated element types.
let flag: Option<bool> = context.ledger.flags()?.head()?;
let count: Option<BoundedUint<65535>> = context.ledger.counts()?.head()?;
let tag: Option<FixedBytes<3>> = context.ledger.tags()?.head()?;
let choice: Option<Choice> = context.ledger.choices()?.head()?;
let packet: Option<Packet> = context.ledger.packets()?.head()?;
```

Each call returns `Result` and charges exactly one canonical ledger-8 head query. Empty heads decode to `None` with a type-aligned dummy value; populated heads decode to the declared element type. `is_empty` and `length` remain VM reads.

### Decision and ownership

Create one unchanged Compact contract with root Lists of Boolean, Uint<16>, Bytes<3>, unit enum, and record values. Capture fresh TypeScript empty and populated reads, ordered four-dimensional costs, private FAB, public transcript, and exact state. Run generated Rust witnesses through the same calls and compare each result, state, FAB, query-cost prefix and aggregate. Add an external generated-crate consumer compile check. Use the current typed emitter and runtime `MeteredListView<T>` and canonical `head_list<T, (bool,T)>` program; no ABI or IR change is expected. If any shape fails, amend this ADR with the proven failure and smallest domain-correct emitter/runtime change before delivery.

### Alternatives and risks

Testing only the generic runtime method would miss generated type mapping and witness signatures. Testing only one scalar would miss different alignment and serialization shapes. Separate contracts for every type would make state and query ordering harder to review. Risks include assuming `Default` encodes Compact's absent payload, misinterpreting enum ordinal zero or struct field alignment, and comparing TypeScript wrapper gas instead of QueryContext per-query costs. The probe must keep these observations separate.

### Verification and delivery gate

- Compile the unchanged source with ledger-8 TypeScript and the Rust target; preserve source and oracle capture.
- Compare empty and populated head values, emptiness/length, private FAB, public transcript, exact serialized states and ordered four-dimensional query costs for all five types.
- Check generated crate use from a separate one-dependency consumer, plus typed failure propagation where relevant.
- Pass focused tests, 131 fixture freshness, backend/runtime, all-target workspace, packaged proof/application and clean-source package gates.
- Record a conventional GPG-signed/DCO local commit, issue, milestone and delivery amendment. Branch publication and remote CI remain separate production gates.


### Design amendment: recorded List writes are missing (2026-10-03)

The first proof/application attempt found a concrete emitter gap: the generated crate exposes native `push_count`, `push_tag`, `push_choice`, and `push_packet`, but its `recorded::Contract` exposes only `push_flag`. The recorded emitter's `scalar_expression` accepts only Field and Boolean for `ListPushFront`, even though `RecordingFrame::push_front_list<T>` and typed `ListSlot<T>::record_push_front` already support any `CellValue`. The fresh TypeScript oracle and generated native Rust witness tests pass for all five types; the missing recorded methods prevent these four calls from entering the ledger transaction proof gate.

```rust
// Before: native works, but this recorded method is absent.
let native = contract.push_packet(context, packet.clone())?;
let recorded = contract.recording.push_packet(context, packet)?; // no method

// Proposed: keep the value typed and record the same canonical List program.
let frame = crate::ledger_slots::packets.record_push_front(frame, packet)?;
```

Extend the recorded emitter's typed parameter/default lowering for List element values (`Uint<16>`, `Bytes<3>`, enum, and struct). Reuse the runtime List slot and recording frame; do not create a parallel VM path or change the ABI/IR. Verify the complete trace, proof, ledger application and decoded state for each type. This changes the delivery from a coverage-only slice to a focused generated API and replay fix, still under #123 and rust-backend-v2.


### Delivery evidence (2026-10-03)

Local conventional GPG-signed/DCO commit `26205ca2fb91f1a203f8c19fdcb713a25e2847a4` delivers the amended decision on `codex/rust-backend-ast`. Generated/runtime ABI remains 11 and private Rust IR schema remains 8. The recorded emitter now accepts parameter/default values of `Uint`, fixed `Bytes`, unit enum and struct types for List pushes, cloning non-Copy struct values at the source boundary. It calls the existing typed `ListSlot<T>::record_push_front` and `RecordingFrame::push_front_list<T>`; the runtime VM program is unchanged.

The unchanged `witness_list_shapes.compact` source has five root Lists: Boolean, Uint<16>, Bytes<3>, Choice and Packet. A fresh ledger-8 TypeScript compile reproduces the saved 23,903-byte oracle exactly. Ten generated Rust witness calls match empty/populated head values, `Option<T>` presence, private state and FAB, public transcript, exact tagged initial/populated state, and all three ordered query-cost prefixes in four dimensions for each call. TypeScript's witness wrapper reports zero gas while its QueryContext reports the compared VM costs; Rust reports their sum. A rejected packet-head read propagates `CompactError`.

A separate one-dependency generated-crate consumer calls all five typed witness views. The packaged compiler emits ZKIR and proving/verifying keys for the five push circuits; each generated recorded trace is proved, validated and applied to offline ledger-8, and the resulting typed List head and length are checked. The packaged application gate now has 50 calls, up from 45. Backend 53 renderer/four CLI tests, the full runtime suite, focused List tests, all-target workspace compile, 132/132 fixture freshness, 37 pinned oracle inventory, two rejection probes, formatting and scoped diff checks pass. Clean-source `target/rust-runtime-abi11-clean.json` was written/verified from `26205ca2` with `dirty: false`; macro/runtime archives contain 8/229 entries. The unrelated `doc/ledger-adt.mdx` edit was preserved byte for byte and excluded.

Focused [#123](https://github.com/MediaNoxLabs/compact/issues/123) and parent [#119](https://github.com/MediaNoxLabs/compact/issues/119) remain open for branch publication and remote CI. Nested physical List paths, other element forms and expression shapes, registry release and wallet/node submission remain wider M2 gates; this slice does not claim general List completeness.
