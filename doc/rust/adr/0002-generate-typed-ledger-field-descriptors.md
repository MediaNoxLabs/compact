---
id: RUST-ADR-0002
alias: ADR-0002
title: "Generate typed ledger field descriptors"
date: 2026-10-02
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["architecture", "generated-api"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: b013edfc8674702bd50dbfd293ce83e5ff9eee1d04cf716d7cce0a06abde6c57
---
# RUST-ADR-0002 — Generate typed ledger field descriptors

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Accept declaration-derived typed ledger slots and their incremental native/recorded use. Public descriptor constructors aid type safety and reviewability; they are not an authorization boundary. Later collection and emptiness deliveries extend the initial Cell/Counter scope without erasing historical gas and nested-Map evidence limitations.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#108 closure](https://github.com/MediaNoxLabs/compact/issues/108#issuecomment-6017411372). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Referenced repository commits:** [`1e58d5bf`](https://github.com/MediaNoxLabs/compact/commit/1e58d5bfe3f737fd2b6f80d4e1d52dd30074f264) · [`476e3ad8`](https://github.com/MediaNoxLabs/compact/commit/476e3ad838e724b8e70f5e1b9bdb4d3008e7cfbf) · [`4812b3f3`](https://github.com/MediaNoxLabs/compact/commit/4812b3f31b2b7b91c23059e9a4102353ccc82094) · [`4cbca227`](https://github.com/MediaNoxLabs/compact/commit/4cbca22761d6617d7caaa23e3630a05806d5a367) · [`7218d4a9`](https://github.com/MediaNoxLabs/compact/commit/7218d4a9f310a3c8af0784ea85394a23cd9b2a2b) · [`858579a0`](https://github.com/MediaNoxLabs/compact/commit/858579a0a55f199a83758920d68e5bbcdea2e652) · [`b17d2ab0`](https://github.com/MediaNoxLabs/compact/commit/b17d2ab0cbb3b8ccf986a809f2ab7a4687b0c592) · [`b17f43ad`](https://github.com/MediaNoxLabs/compact/commit/b17f43ada7c346451656527ca81a382815b873b0) · [`de3ccbc9`](https://github.com/MediaNoxLabs/compact/commit/de3ccbc9580b2aad99aba99177405c19ba9c94e5). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
adr: 0002
status: accepted-partial
date: 2026-10-02
milestone: rust-backend-v2
issue: 108
```

## Historical decision and amendments

### Problem

Numeric ledger indices conceal the source declaration and allow an emitter edit to pair the wrong value type with a physical path. The compiler IR already knows each field's ID, kind, type, and chunked physical path; generated Rust should expose that model to reviewers and consumers.

### Before and after

```rust
// Before: valid Rust, but the declaration and type are not visible here.
let step = context.write_cell(0, true)?;

// After: generated descriptor and typed runtime method.
pub const flag: runtime::slots::CellSlot<bool> =
    runtime::slots::CellSlot::new(&[0u8]);
let frame = crate::ledger_slots::flag.record_write(frame, true)?;
```

`flag.write(context, 7u64)` fails at Rust type checking because the slot's value is `bool`. The type check does not replace the ledger VM's runtime checks.

### Decision and ownership

Generate a public `ledger_slots` module from the closed ledger-field IR. `CellSlot<T>` and `CounterSlot` hold the physical path, including chunked paths, and delegate native/recorded operations to existing runtime methods. The emitter verifies field references against the IR, generates named constants, and uses those constants in recorded Cell/Counter methods. VM instruction construction remains in the runtime.

The research probe proposed sealed constructors. The delivered constructors are public because generated crates must instantiate cross-crate runtime descriptors. Treat descriptors as type-safety and readability aids, not an authorization boundary. Revisit sealing through generated newtypes if it materially prevents a real mistake; avoid implying that a consumer cannot forge a path today.

### Evidence, consequences, and remaining work

Local commits `4cbca227` and `858579a0`. Seventy-five affected fixture crates regenerated for the descriptor API; 23 recorded bodies later switched to names. A separate Cargo consumer executed and recorded the Counter slot. All 45 renderer tests, full workspace check, and three offline proven call gates passed. The `chunked-ledger-oracle` generated output includes two-segment paths.

Add Set, Map, List, and Merkle descriptors with correct key/value types and operation contracts. Move native emitted bodies to descriptors only after state/gas/transcript parity checks. Add a consumer compile-fail case for wrong Cell value type and measure generated-source readability.

### Tracking

- Parent: [M2 runtime maintenance #106](https://github.com/MediaNoxLabs/compact/issues/106).
- Focused issue: [#108](https://github.com/MediaNoxLabs/compact/issues/108). Milestone: [rust-backend-v2](https://github.com/MediaNoxLabs/compact/milestone/2).
- Delivery: Cell/Counter slice local only; remaining field kinds open.


### Decision history

- 2026-10-02: Created ADR-0002 from the generated-crate research probes with status `accepted-partial`; linked MediaNoxLabs/compact#108 in `rust-backend-v2`. Local delivery evidence is recorded above. Future changes should append a dated amendment or link a superseding ADR.



### Amendment — 2026-10-02: native Cell and Counter bodies use declared slots

**Problem.** Recorded bodies already used `ledger_slots`, but native generated bodies still embedded numeric paths in Cell and Counter operations. Reviewers had to match an index to a ledger declaration by hand, and type/path association remained split across two emitter paths.

**Before (generated native body, simplified):**

```rust
let step = context.write_cell(0, value)?;
let read_step = context.read_cell::<bool>(0)?;
let step = context.increment_counter(1, amount)?;
```

**After (generated native body):**

```rust
let step = crate::ledger_slots::flag.write(context, value)?;
let read_step = crate::ledger_slots::flag.read(context)?;
let step = crate::ledger_slots::round.increment(context, amount)?;
let step = crate::ledger_slots::round.reset(context)?;
```

This keeps the source ledger name in ordinary generated Rust. The slot constant binds `CellSlot<bool>` or `CounterSlot` to its compiler-derived physical path; Counter read retains the Compact `Uint<64>` wrapper at the circuit boundary.

**Emitter and runtime ownership.** `stateful.rs` now emits slot calls for Cell reads in expressions and returns, Cell writes, Counter increment/decrement/reset, and Counter returns after the same IR declaration/index/type checks. The generated `ledger_slots` module and physical path derivation are unchanged. `CounterSlot::reset` delegates to `CircuitContext::write_cell_at_path(path, 0_u64)`; other slot methods already delegated to the existing VM runtime. Constructors and collection/Merkle operations remain on their current paths. No macro or new VM opcode is introduced. The external consumer gate compiles a deliberately wrong `flag.write(context, 7_u64)` and requires Rust's `expected bool, found u64` type error, while ordinary consumers build and run.

**Evidence and limits.** Local conventional GPG-signed/DCO commit `7218d4a9`, toolchain `0.31.126`. Sixty-nine generated fixture libraries changed; all 129 were regenerated and then checked with 0 stale/failed. All 47 renderer tests, runtime tests, full Cargo workspace check, and focused Counter, Counter reset, Cell, nested witnessed, tiny, election, and zerocash oracle tests passed. The packaged `--consumer --proof` gate passed including the external compile-fail case and nine replayed/partitioned, proven/verified, validated/applied ledger-8 call shapes. This demonstrates retained behavior for representative flows but does not settle the known `tiny` gas/query-shape difference or provide complete ADT parity. Set/Map/List/Merkle descriptors, constructor migration, broader native frame work, release and remote CI remain open under [#108](https://github.com/MediaNoxLabs/compact/issues/108) in [rust-backend-v2](https://github.com/MediaNoxLabs/compact/milestone/2).


### Amendment — 2026-10-02: typed Set slots and native emission

**Problem.** Set declarations had no named typed descriptor even after Cell and Counter bodies moved to `ledger_slots`. Generated `SetInsert`, `SetRemove`, `SetReset`, membership, size, and emptiness operations still embedded physical indices. A consumer could accidentally pass a value with the wrong element type to a raw Set operation.

**Before (generated Rust, simplified):**

```rust
let step = context.insert_set(1, item)?;
let read_step = context.member_set(1, item)?;
```

**After (generated crate and consumer):**

```rust
pub const s: runtime::slots::SetSlot<runtime::Field> =
    runtime::slots::SetSlot::new(&[1u8]);
let step = crate::ledger_slots::s.insert(context, item)?;
let read_step = crate::ledger_slots::s.member(context, item)?;
// s.insert(context, true) fails Rust type checking: expected Fr, found bool.
```

**Emitter/runtime ownership.** The schema-6 `LedgerFieldKind::Set { ty }` declaration now emits `SetSlot<T>` with the compiler-derived physical path. `stateful.rs` emits native slot calls for Set insert/remove/reset actions, `Expr::SetMember`, and Set member/size/emptiness returns after checking the IR field identity, index, and element/result types. `runtime-rs/src/slots.rs` provides the typed facade; its methods delegate to existing `CircuitContext` Set VM operations. The VM programs, gas accounting, and constructor emission are unchanged. The descriptor constructor remains public, consistent with ADR-0002's type-safety rather than access-control model.

**Evidence and limits.** Local conventional GPG-signed/DCO commit `476e3ad8`, toolchain `0.31.127`. Seventy-nine generated fixture libraries changed; all 129 generated successfully. All 47 renderer tests, the runtime suite, the full Cargo workspace check, and focused Set Boolean, Set size, Set membership, witnessed Set, and vector-key ADT oracle tests passed. The packaged external consumer compiled a separately generated Set crate alongside Counter, Cell, and pure crates; it inserted a Field, checked membership and the generated `check` circuit, and required Rust E0308 for a Boolean Set element. The packaged `--consumer --proof` gate also retained all nine previously supported offline ledger-8 proven call shapes. It did **not** prove a Set circuit: `RecordingFrame` and the emitter still lack complete Set Verify-program recording. Set recording, Map/List/Merkle descriptors, constructor migration, source-to-proof parity for ADTs, publication, and remote CI remain open under [#108](https://github.com/MediaNoxLabs/compact/issues/108), [#107](https://github.com/MediaNoxLabs/compact/issues/107), and [rust-backend-v2](https://github.com/MediaNoxLabs/compact/milestone/2).

### Amendment — 2026-10-02: recorded Set slot methods

**Problem.** `SetSlot<T>` named the field and enforced its element type for native calls, while the recorder still had no slot operation. Extending the recorder through raw numeric paths would split the generated domain model again.

**Before (native only):**

```rust
let step = crate::ledger_slots::seen.insert(context, true)?;
// No typed descriptor route for a complete recorded call.
```

**After (recorded body):**

```rust
let frame = crate::ledger_slots::seen.record_insert(frame, true)?;
let (frame, present) = crate::ledger_slots::seen.record_member(frame, true)?;
let frame = crate::ledger_slots::seen.record_reset(frame)?;
```

The same `SetSlot<T>` now offers `record_insert`, `record_remove`, `record_reset`, `record_member`, `record_size`, and `record_is_empty`. Wrong element types remain compile errors. These methods delegate to `RecordingFrame`; canonical ledger VM programs remain in `ledger.rs`. The emitter checks declaration identity, physical root path and supported Field/Boolean type before generating a recorded method. No new macro or Set storage representation was added.

**Evidence and limits.** Local signed/DCO commit `de3ccbc9` (toolchain `0.31.128`). The generated Set Boolean and Set oracle crates exercise the slot methods with replay and TypeScript parity, and the packaged ledger-8 gate proves/applies their generated traces. The runtime rejects a tampered Set read and matches native state/effects/gas. All 129 fixtures are current and the workspace checks. Typed Map/List/Merkle descriptors, nested Set recording, more Set element types, and constructor migration remain open in [#108](https://github.com/MediaNoxLabs/compact/issues/108) in [rust-backend-v2](https://github.com/MediaNoxLabs/compact/milestone/2).


### Amendment — 2026-10-02: typed Map key/value slots

**Problem.** Generated native Map code still passed a raw numeric path to generic context methods. That concealed the source declaration and left key/value pairing split between emitter checks and runtime calls. Recorded Map code needed the same domain model.

**Before (generated native body, simplified):**

```rust
let step = context.insert_map(0, key, value)?;
let read = context.lookup_map::<_, Field>(0, key)?;
```

**After (generated declaration and native/recorded bodies):**

```rust
pub const table: runtime::slots::MapSlot<bool, runtime::Field> =
    runtime::slots::MapSlot::new(&[0u8]);
let step = crate::ledger_slots::table.insert(context, key, value)?;
let (frame, value) = crate::ledger_slots::table.record_lookup(frame, key)?;
```

A separately generated consumer can insert, record and look up through this descriptor. Passing a `Field` key or Boolean value to `MapSlot<bool, Field>::insert` fails Rust type checking. As with Cell/Set slots, the public constructor is a readability/type-safety aid, not an authorization boundary.

**Emitter/runtime ownership.** The closed ledger IR provides each Map name, physical path, key and value type. `lib.rs` emits `MapSlot<K,V>` where both types have scalar `CellValue` representations; `stateful.rs` moves native insert/default/remove/reset/member/lookup/size/emptiness to the slot after existing IR validation. `recorded.rs` uses the same slot for complete root Boolean/Field Map traces. `slots.rs` delegates to context/frame methods; VM operations stay in `ledger.rs`. Nested `LedgerMap` values cannot currently be encoded as `CellValue`, so their constructor/runtime support remains and no invalid scalar `MapSlot` is emitted. A renderer regression test and the `nested_map_oracle` fixture guard this boundary.

**Evidence and limits.** Local signed/DCO commit `b17f43ad`, toolchain `0.31.129`. Eighty-five generated fixture libraries changed across the two fixture updates, with all 129 outputs generated successfully; Map Boolean/Field, constructor Map and nested Map fixture tests pass. The packaged external consumer compiles the typed Map path, rejects wrong key/value examples with Rust E0308, and the ledger-8 gate proves/applies 11 Map calls within 29 total calls. Full runtime suite, 47 renderer tests, workspace check and formatting passed. Nested Map value descriptors, List/Merkle descriptors, constructor migration, broader expression recording, published runtime and remote CI remain open in [#108](https://github.com/MediaNoxLabs/compact/issues/108) and [rust-backend-v2](https://github.com/MediaNoxLabs/compact/milestone/2).


### Amendment — 2026-10-02: typed root List slots

**Problem.** Native generated List bodies passed numeric field indices to context methods, while List recording needed a named, typed route. The numeric path concealed which Compact declaration owned the operation and let a direct context call take an unrelated element type.

**Before (generated native body, simplified):**

```rust
let step = context.push_front_list(0, value)?;
let read = context.head_list::<Field, Maybe>(0)?;
```

**After (generated declaration and native/recorded bodies):**

```rust
pub const items: runtime::slots::ListSlot<runtime::Field> =
    runtime::slots::ListSlot::new(0);
let step = crate::ledger_slots::items.push_front(context, value)?;
let (frame, head) = crate::ledger_slots::items.record_head::<Maybe, _, _>(frame)?;
```

An external Cargo consumer compiles a valid `Field` List call and rejects `items.push_front(context, true)` with Rust E0308. The slot constructor remains public, as with earlier Cell/Set/Map slots: it is a type-safety and readability tool, not an authorization boundary.

**Emitter/runtime ownership.** The schema-6 `LedgerFieldKind::List { ty }` declaration emits `ListSlot<T>` with its compiler-derived root index. `stateful.rs` validates declaration identity and types and uses the slot for native push/pop/reset/length/emptiness/head. `recorded.rs` uses the same slot for complete supported Field/Boolean traces. `runtime-rs/src/slots.rs` defines the typed API and delegates to `CircuitContext` or `RecordingFrame`; shared ledger VM programs, state representation, and gas accounting stay in `ledger.rs`. Constructor actions retain their existing compiler-derived path and are checked separately through the constructor List fixture. `Maybe<T>` is the circuit return type, not the stored List element type.

**Evidence and limits.** Local conventional GPG-signed/DCO commit `4812b3f3`, toolchain `0.31.130`. The renderer validates List element and `Maybe<T>` result types; the generated crate exposes named slot calls. The separate shared-runtime consumer compiles valid List calls, rejects the wrong element type, and replays a recorded call. Focused generated List fixtures, the runtime trace test, 47 renderer tests, the full runtime suite, formatting, and the workspace check passed. All 129 compiler fixture outputs are current (0 stale, 0 failed). The packaged gate proved, verified, validated and applied 10 List calls within 39 offline ledger-8 calls, including an empty/present head and constructor seeded mutation. Root typed slots are delivered; nested paths, broader element types, constructor migration to slots, wallet/node submission, publication and remote CI remain M2 work. See [#111](https://github.com/MediaNoxLabs/compact/issues/111), [#108](https://github.com/MediaNoxLabs/compact/issues/108), and [rust-backend-v2](https://github.com/MediaNoxLabs/compact/milestone/2).



### Amendment — 2026-10-02: reuse `CellSlot<T>` for Bytes and enum traces

**Problem.** A generated `CellSlot<FixedBytes<32>>` or `CellSlot<Choice>` already named the field and enforced the value type for native calls, but the recorder's Boolean/Field filter prevented that same descriptor from being used in a complete recorded call. Adding untyped path APIs for Bytes and enum values would split field identity and type checking across two generated APIs.

**Before (generated native call only):**

```rust
let step = crate::ledger_slots::hashCell.write(context, hash)?;
// No recorded call was generated for Cell<Bytes<32>>.
let step = crate::ledger_slots::choice.read(context)?;
// No recorded call was generated for Cell<Choice>.
```

**After (generated recorded bodies):**

```rust
let frame = crate::ledger_slots::hashCell.record_write(frame, hash.clone())?;
let (frame, observed): (_, runtime::FixedBytes<32>) =
    crate::ledger_slots::digest.record_read(frame)?;
let frame = crate::ledger_slots::choice.record_write(frame, Choice::no)?;
let (frame, observed): (_, crate::types::Choice) =
    crate::ledger_slots::choice.record_read(frame)?;
```

These lines come from independent fixtures and circuits. The generated `CellSlot<T>` declaration, Rust type mapping, physical path and runtime API are unchanged. `recorded.rs` now permits `Type::Bytes` and `Type::Enum` for root Cell read/write when the IR declaration and expression/result types match; it retains `CellSlot<T>` as the sole emitted path. Rust ownership requires a clone for non-Copy parameter/local sources, while an enum literal or default is constructed directly. The runtime remains responsible for VM programs and gas; no new primitive or macro is added. The alternative of a generic untyped `record_write(index, value)` remains rejected because it discards declaration identity at the call site.

**Evidence and limits.** Local signed/DCO commit `b17d2ab0`; `recorded_enum_cell`, `constructor_persistent_hash`, and `inline_type_scope_oracle` generated crates pass state, gas and replay checks; TypeScript confirms the enum fixture's serialized state; the packaged offline gate proves/applies its enum write as call 40. All 130 fixture outputs are current, workspace check and formatting pass. This validates root Bytes/enum Cells only. Nested paths, richer ADT values, constructor migration to slots, release and remote CI remain open under [#108](https://github.com/MediaNoxLabs/compact/issues/108), with replay scope under [#107](https://github.com/MediaNoxLabs/compact/issues/107), in [rust-backend-v2](https://github.com/MediaNoxLabs/compact/milestone/2).



**Coverage correction (2026-10-02).** Default-value lowering was added to the emitter but is not executed by this commit’s generated fixture tests. Parameter and literal sources and typed reads are covered.



### Proposed amendment — 2026-10-02: native collection emptiness expressions

**Problem.** Generated native circuit actions already use named typed Set/Map slots, but an `isEmpty()` expression nested inside a Cell write still emits `context.is_empty_set(3)?` or `context.is_empty_map(4)?`. Numeric paths obscure the declared field and bypass the slot type boundary. Nested `Map<K, Map<K2, V>>` has no scalar `MapSlot` because its value is not `CellValue`; that case must retain a direct path until a reviewed nested descriptor exists.

**Before/after Rust.**

```rust
// Before: generated native expression for check_map_empty.
let query = context.is_empty_map(4)?;
// After for a scalar Map or Set: the declaration owns the physical path.
let query = crate::ledger_slots::table.is_empty(context)?;
// Nested Map remains direct until a typed nested value model exists.
let query = context.is_empty_map(&[4u8])?;
```

**Decision.** Use the generated slot for Set and scalar Map emptiness expressions, including chunked paths. Preserve a direct `CircuitContext` query only when a Map key/value type cannot form a scalar `MapSlot`. Reuse the same type eligibility as slot generation. The private IR still supplies field identity, index, type and physical path. The runtime `SetSlot`/`MapSlot` implementations and canonical VM programs do not change; this is an emitter change only. Generated API gains no new public item and runtime ABI stays 4.

**Verification and risks.** Regenerate compiler fixtures, assert generated source uses named slots for the nested query fixture, compile a nested-Map fixture to guard the fallback, compare native result/state/gas with existing TypeScript captures, and run the packaged consumer/proof gate. The main risk is referring to a slot that was deliberately omitted for nested Map values or changing the path passed to the VM. The existing fixture and proof gates should catch this. Focused issue [#108](https://github.com/MediaNoxLabs/compact/issues/108) is in `rust-backend-v2`; append exact delivery evidence and local commit after validation.



### Delivery amendment — 2026-10-02: native collection emptiness uses names

Local conventional GPG-signed/DCO commit `1e58d5bfe3f737fd2b6f80d4e1d52dd30074f264` implements the proposed scalar Set/Map `isEmpty()` expression change. `scalar_map_slot_types` is now shared by slot declaration and native expression emission, so an emitter branch cannot name a Map slot that the crate omitted. The existing `SetSlot` and `MapSlot` methods call the same `CircuitContext` methods with the same physical path; runtime source, VM opcodes, IR schema, generated public item set and ABI remain unchanged. Actual generated before/after in `nested-collection-query-write`:

```rust
// Before
let __compact_query_0 = context.is_empty_set(3)?;
let __compact_query_0 = context.is_empty_map(4)?;
// After
let __compact_query_0 = crate::ledger_slots::seen.is_empty(context)?;
let __compact_query_0 = crate::ledger_slots::table.is_empty(context)?;
```

The proposed nested-Map fallback example used an illustrative slice; the actual root-index output is `context.is_empty_map(1)?`. A renderer test verifies scalar Set and Map use names, while `Map<Field, Map<Field, Field>>` still emits that direct query and no invalid `MapSlot`. Two checked-in generated fixtures changed. Their existing tests retain TypeScript result/state parity and query gas comparison. The full backend suite passed (52 renderer and 4 CLI tests); full Cargo workspace check passed; all 130 fresh compiler fixtures matched. The aarch64 Darwin packaged `compactc --target rust --consumer --proof` gate passed, including external consumers and 44 offline ledger-8 replay/partition/prove/verify/validate/apply calls. Formatting, headers (0 missing across 1,461 files), staged diff, commit GPG signature and DCO passed.

This is a partial #108 delivery. Nested Map typed paths, Merkle descriptors, broader native-body migration, wallet/node submission and remote CI remain open. The M2 branch remains local. The previous proposal and its typo are retained as decision history; this amendment states the actual generated syntax and verified result.



**Evidence clarification, 2026-10-02.** Both affected generated fixtures compare state and returned flags with TypeScript captures. Only `nested-collection-query-write` additionally has a composed query-gas assertion; `set-size-oracle` does not assert gas. The nested-Map fallback was renderer-tested, not compiled from a new Compact source in this slice. These limits remain open for broader acceptance work.
