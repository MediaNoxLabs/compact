---
id: RUST-ADR-0030
alias: ADR-0030
title: "Record Vector-key Set and Map circuits through typed slots"
date: 2026-10-03
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["backend", "recording", "ledger-adts"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: 132d7baf60dd4b36d282cc0b3a8fb4e18ec33745820370f971ac4b58dc516d49
---
# RUST-ADR-0030 — Record Vector-key Set and Map circuits through typed slots

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Accept recorded Vector-key Set/Map slices using declaration-typed operands, existing slots and ordered typed bindings. Preserve the narrowed negative evidence: the raw-byte key fails Rust typing, while an attempted stateful-element source was rejected by the frontend. The record does not establish arbitrary effectful Vector expressions.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#129 closure](https://github.com/MediaNoxLabs/compact/issues/129#issuecomment-6017447837). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Referenced repository commits:** [`52687129`](https://github.com/MediaNoxLabs/compact/commit/5268712930dbf5808a73b4a5aa873d1248c04086). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
adr: 30
status: accepted-local
date: 2026-10-03
milestone: rust-backend-v2
issue: 129
```

## Historical decision and amendments

### Problem

The native AST backend fixes oracle #93 by preserving `Vector<2, Field>` in Set/Map operands, but `recorded.rs` omits the corresponding public recorded entry points. A generated crate can run eight vector-key circuits natively yet cannot create a complete public VM transcript, prove it, or apply it through ledger-8. The issue fixture is `examples/rust_backend/vector_key_adt.compact`.

### Before

```rust
let step = Contract::default().setInsert(context)?;
// This public recorded method is absent even though a typed SetSlot exists:
// let call = Contract::default().recording.setInsert(context)?;
```

### Decision and after

Proposed API and implementation, subject to compiler-backed parity:

```rust
let inserted = Contract::default().recording.setInsert(context)?;
let member = recorded::setMember(inserted.execution.context)?;
let replay = member.public.initial().query(
    member.public.verify_ops(), None, &member.execution.context.cost_model,
)?;
assert_eq!(replay.context.effects, member.execution.context.query.effects);
```

Use the existing `FixedVector<Field, 2>` CellValue and generic `SetSlot<T>` / `MapSlot<K,V>` recording operations. Accept only declaration-typed vector operands. For pure vector bindings, emit a typed local once, preserving evaluation order and cloning only when a key is reused. Lower Set member, Map member and Map lookup into typed recorded reads before subsequent Cell writes. Keep unsupported effect shapes absent from the recorded API.

### Alternatives and rationale

No vector-specific macro or second VM encoding is needed. The ledger-8 VM builder already owns the FAB key and operation program. Reconstructing vector keys from raw bytes would repeat the #93 false-negative bug. Emitting native-only methods leaves proof/application incomplete.

### Emitter and runtime ownership

The Scheme front end already lowers the Compact Set/Map arguments under the ADT declaration to schema-8 `Expr::Vector` and `Type::Vector`; preserve that. In `tools/compact-rust-backend/src/recorded.rs`, admit typed vector values in `cell_source`, `scalar_expression`, `StateAction::Let`, Set insert/remove/member, Map insert/remove/insertDefault/member/lookup, and expression-local `Expr::Let` where required by the fixture. Use `syn` AST and existing typed slot methods. `runtime-rs` retains the canonical `FixedVector` CellValue/FAB and generic recorded slot methods; no new runtime operation builder is proposed. Do not broaden recorded witness/result types without a separate decision. Public generated recorded methods would expand; propose generated/runtime ABI 14→15 with fixture regeneration, preserving schema 8.

### Verification and risks

Acceptance requires independent ledger-8 TypeScript capture of each operation's result, exact serialized state, four-dimensional gas, and ordered full VM program; replay equality; one-dependency generated consumer; at least one offline proof/verify/validation/application call using a vector key; compile-fail negative check for an unsupported recorded shape; fixture freshness; all-target workspace; clean-source ABI-15 archive. Compare generated source size and public methods. Record actual outcomes and any narrower delivered slice in an amendment. The existing native eight-operation parity test is not proof evidence. Exact witness-driven vector keys, nested ADTs, remote CI, registry release and wallet/node submission remain later acceptance work unless measured here.

### Tracking and delivery

- Issue: pending focused MediaNoxLabs/compact issue; assign to [rust-backend-v2](https://github.com/MediaNoxLabs/compact/milestone/2) before coding.
- Parent issues: [#93](https://github.com/MediaNoxLabs/compact/issues/93), [#105](https://github.com/MediaNoxLabs/compact/issues/105), [#107](https://github.com/MediaNoxLabs/compact/issues/107), [#108](https://github.com/MediaNoxLabs/compact/issues/108).
- Local commits: pending; branch `codex/rust-backend-ast`, currently unpushed.
- Delivery state: proposed, with no recorded vector-key result claimed.

### Amendments

Append the issue link, exact signed/DCO commit and measured tests after delivery; preserve this proposal.


### Issue assignment — 2026-10-03

Focused [#129](https://github.com/MediaNoxLabs/compact/issues/129) is assigned to rust-backend-v2 before implementation. It carries the acceptance gates above; the proposal is unchanged.


### Local delivery — 2026-10-03

Conventional GPG-signed/DCO `5268712930dbf5808a73b4a5aa873d1248c04086` delivers this proposal on local unpushed `codex/rust-backend-ast` and bumps generated/runtime ABI 14→15; private IR stays schema 8. `recorded.rs` now validates declaration-typed `Expr::Vector` through `expression_with_calls`, materializes pure `StateAction::Let` and expression-local `Expr::Let` vector bindings once as typed `FixedVector`, clones non-Copy keys when referenced, routes Set insert/member/remove and Map insert/member/lookup/remove/insertDefault through existing typed slots, and records Map query effects before Cell writes. `runtime-rs` changes only its ABI constant; `FixedVector` CellValue/FAB and generic slot recording operations are reused. No new macro, derive, or VM builder was needed. Unsupported stateful elements in a vector literal remain ineligible for recorded emission; the Scheme compiler itself rejects a tested `keys.insert([current, 1 as Field])` stateful-vector expression. This slice does not claim arbitrary vector expressions or witness-driven keys.

Representative emitted code (line wrapping abbreviated from `tests-rust-backend/vector-key-adt/lib.rs`):

```rust
let __compact_recorded_vector_0: runtime::FixedVector<runtime::Field, 2> =
    runtime::FixedVector::new([runtime::Field::from(0u128), runtime::Field::from(1u128)]);
let __compact_recorded_key_1 = (__compact_recorded_vector_0).clone();
let (frame, __compact_recorded_member_2): (_, bool) =
    crate::ledger_slots::keys.record_member(frame, __compact_recorded_key_1)?;
let frame = crate::ledger_slots::present.record_write(frame, __compact_recorded_member_2)?;
```

The independent ledger-8 TypeScript capture `tools/compact-rust-backend/oracles/vector_key_capture.mjs` reproduces all previously pinned state/result fields and adds eleven query sequences. It pairs TypeScript's omitted `popeq.result` with the corresponding VM read event to obtain the same verification-mode Op representation Rust serializes; this pairing is explicit and order-checked. The fixture test compares full ordered Op values, four gas dimensions, native gas/effects, serialized state, replayed state, and observable Cell results after all eleven calls covering eight circuits. Two focused tests pass. The standalone shared-runtime consumer compiles and replays the vector-key calls; its raw-byte key negative example fails at Rust's `FixedVector` type boundary. The packaged ledger-8 gate proves, verifies, validates and applies the new `setInsert` call, then checks the exact typed key and Set size in the applied contract. All 55 packaged proof/application calls pass. The target/consumer/proof gate, 132 fresh fixtures, 54 renderer tests, 4 compactc tests, all-target workspace check, and clean-source ABI-15 archive manifest verification pass. The archives contain 8 macro and 229 runtime entries. The user-owned `doc/ledger-adt.mdx` edit was preserved byte for byte and is not in the commit.

Generated `vector-key-adt/lib.rs` grows from 305 to 500 lines (+195 net); the new recorded facade provides eight circuit methods, so no source-size win is claimed. The public API is more capable but still low-level in its generated method bodies; ADR-0016/#117 tracks helper reuse and ergonomics separately. No compile-time performance comparison was measured for this slice. Registry publication, remote compiler-backed CI, wallet/node submission, and generalized vector-expression recording remain open. Keep [#129](https://github.com/MediaNoxLabs/compact/issues/129) and oracle [#93](https://github.com/MediaNoxLabs/compact/issues/93) open until publication and remote acceptance.


#### Review tradeoffs and provenance

The generated body currently clones a vector local even for a single consuming use. This is semantically safe and keeps non-Copy locals reusable, but it costs an allocation/copy opportunity; a borrow/move optimization should be measured under ADR-0016/#117 rather than assumed here. The proposed unsupported-effect-shape compile-fail example was narrowed in #129 to a raw-byte key type rejection. The Scheme front end rejects the attempted stateful-element vector source before Rust emission, so no generated recorded omission example was available from that source. Future vector-expression support should add a dedicated negative fixture when the language accepts such a shape. The clean-source ABI-15 archive records commit `5268712930dbf5808a73b4a5aa873d1248c04086`, tree `c743016fb6367d3d04b91424e122590b0ebc6ece`, dirty=false.
