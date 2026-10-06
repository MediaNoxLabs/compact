---
id: RUST-ADR-0001
alias: ADR-0001
title: "Record circuit execution as a replayable ledger trace"
date: 2026-10-02
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["architecture", "generated-api"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: 998481859d037cdaeab9f54a70c77311d1a0bdd6d9170a2326d479237eda0f45
---
# RUST-ADR-0001 — Record circuit execution as a replayable ledger trace

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Accept complete recorded execution as an ordered ledger trace alongside native execution. Later amendments add witnessed, nested, collection and Cell slices; they do not authorize partial traces or reconstruct Verify operands from final state. Preserve each slice's historical eligibility and proof limits, including the default-write evidence correction.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#107 closure](https://github.com/MediaNoxLabs/compact/issues/107#issuecomment-6017409514). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Referenced repository commits:** [`02bd0be3`](https://github.com/MediaNoxLabs/compact/commit/02bd0be3bd117daed751a235a77d372cd327002a) · [`077d386b`](https://github.com/MediaNoxLabs/compact/commit/077d386beca9584440ed0e8f626be80c48a3e38f) · [`1ed45da6`](https://github.com/MediaNoxLabs/compact/commit/1ed45da67b0530c0d32e6eef2e736d2855dbfdd2) · [`4812b3f3`](https://github.com/MediaNoxLabs/compact/commit/4812b3f31b2b7b91c23059e9a4102353ccc82094) · [`5ca0cb61`](https://github.com/MediaNoxLabs/compact/commit/5ca0cb612be8e6a5fe856ea626431fa721c7393e) · [`858579a0`](https://github.com/MediaNoxLabs/compact/commit/858579a0a55f199a83758920d68e5bbcdea2e652) · [`99020541`](https://github.com/MediaNoxLabs/compact/commit/990205411c5d050961e88302670a3114846342f6) · [`9e784a90`](https://github.com/MediaNoxLabs/compact/commit/9e784a9057b3ef021e24d2e92c37f88b90e0cf36) · [`b17d2ab0`](https://github.com/MediaNoxLabs/compact/commit/b17d2ab0cbb3b8ccf986a809f2ab7a4687b0c592) · [`b17f43ad`](https://github.com/MediaNoxLabs/compact/commit/b17f43ada7c346451656527ca81a382815b873b0) · [`b36ab0c8`](https://github.com/MediaNoxLabs/compact/commit/b36ab0c8c98a4ac9e15875feac2c581cdc352b17) · [`de3ccbc9`](https://github.com/MediaNoxLabs/compact/commit/de3ccbc9580b2aad99aba99177405c19ba9c94e5) · [`e816df33`](https://github.com/MediaNoxLabs/compact/commit/e816df333c0ff7a4273eb5b458f77cc753fb9bd6) · [`fe41c6c1`](https://github.com/MediaNoxLabs/compact/commit/fe41c6c1c39a6d269b2ee64d07ba888e75315d06). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
adr: 0001
status: accepted-partial
date: 2026-10-02
milestone: rust-backend-v2
issue: 107
```

## Historical decision and amendments

### Problem

Native `CircuitResult` retained the successor context and witness outputs but discarded the ordered verifying VM program. A final state cannot reconstruct a ledger-8 `PreTranscript`, and hand-assembling one in a consumer would let native execution and proof inputs drift. Generated code also repeated context, gas, and private-transcript plumbing.

### Before

From the generated witnessed Cell fixture (simplified):

```rust
let (next_private, secret) = witnesses.secret(context.witness_context_with(view), seed);
context.private_state = next_private;
private_transcript_outputs.push(AlignedValue::from(secret.clone()));
let step = context.write_cell(0, secret)?;
let context = step.context;
total_cost += step.gas_cost;
```

The native result held no ordered `Op<ResultModeVerify, D>` sequence.

### Decision and after

Keep native execution, and add an explicit `RecordingFrame` only for circuits whose *complete* operations can be recorded. It owns the initial `QueryContext`, ordered Verify ops, private FAB outputs, current context, and observed gas. A generated recorded call looks like:

```rust
let frame = runtime::recording::RecordingFrame::new(context);
let frame = crate::ledger_slots::flag.record_write(frame, true)?;
Ok(frame.finish(())) // RecordedCircuitResult, not an unlabelled CircuitResult
```

For reads, capture the actual Gather event's `AlignedValue` and use it in Verify `Popeq`; do not reconstruct it from a decoded Rust value. Replay and ledger partitioning establish transcript effects and gas. The sum of native step gas remains diagnostic and is not substituted for partitioned transcript gas.

### Emitter and runtime ownership

- Emitter: `stateful.rs` exposes `ledger_contract::recorded` and `Contract<W>::recording` only for recognized complete Counter and root Boolean Cell shapes. Other methods remain native.
- Runtime: `recording.rs` owns trace order, witness outputs, and state transitions. `ledger.rs` owns exact VM program construction; it must match Compact ZKIR operations, not merely final state.
- Proof bridge: `transaction.rs` consumes a complete recorded result in a separate optional feature (ADR-0006).

### Evidence, consequences, and remaining work

Local delivery commits: `e816df33`, `077d386b`, `99020541`, `fe41c6c1`, `b36ab0c8`, `1ed45da6`, `858579a0`. Counter increment and Boolean Cell write/read replay, prove, validate, and apply in the offline ledger-8 gate. The Cell proof initially failed on a semantically equivalent but different opcode sequence; aligning the program fixed it.

Generated witness calls, nested stateful calls, collections, and more Cell types still lack complete emitted recording. Before expanding `.recording`, compare result, state bytes, private FAB order, public Verify ops, partitioned gas/effects, error short circuit, and TypeScript/ledger behavior. A native `CircuitFrame` that also removes ordinary emitter plumbing remains an open design/implementation step.

### Tracking

- Parent: [M2 source-to-proof-to-ledger #105](https://github.com/MediaNoxLabs/compact/issues/105).
- Focused issue: [#107](https://github.com/MediaNoxLabs/compact/issues/107). Milestone: [rust-backend-v2](https://github.com/MediaNoxLabs/compact/milestone/2).
- Delivery: partial, local only; no M2 branch push or remote CI result.


### Decision history

- 2026-10-02: Created ADR-0001 from the generated-crate research probes with status `accepted-partial`; linked MediaNoxLabs/compact#107 in `rust-backend-v2`. Local delivery evidence is recorded above. Future changes should append a dated amendment or link a superseding ADR.


### Amendment — 2026-10-02: witnessed Field Cell trace

**Problem.** The `witness_cell_write.compact` fixture executed two `secret(seed)` calls and two Cell writes correctly, but its replay test hand-built a `RecordingFrame`. Consumers had no generated route from `Contract<Secret>` to a complete ledger call, so proof parity depended on manually duplicating the compiler's operation order.

**Before (consumer test, simplified):**

```rust
let frame = RecordingFrame::new(context);
let (frame, first) = frame.witness(|ctx| secret_logic(ctx.private_state, cell(ctx), seed));
let frame = frame.write_cell(0_u8, first)?;
let (frame, second) = frame.witness(|ctx| secret_logic(ctx.private_state, cell(ctx), seed));
let recorded = frame.write_cell(0_u8, second)?.finish(());
```

**After (generated consumer API):**

```rust
let recorded = Contract::from(Secret)
    .recording() // borrows the witness implementation
    .write_twice(context, seed)?;
let call = prepare_call(recorded, CallSpec::new("write_twice", verifier, seed, rand))?;
```

The generated body calls `frame.witness` for each IR `Let` witness binding and immediately calls the typed `ledger_slots::cell.record_write` for the corresponding `CellWrite`. The second witness receives the context after the first write. The older field API `contract.recording` remains for witness-free recorded calls; `recording()` is emitted only for a crate with a witnessed recorded circuit.

**Emitter and runtime.** `stateful.rs` recognizes the typed schema-6 `WitnessCall` binding, Field/Boolean arguments (including a same-type `Coerce`), root Field/Boolean Cell write, and Field read. It checks witness arity and result type and withholds a recorded method for unsupported shapes. `lib.rs` emits `recorded::BorrowedContract<'_, W>` and the borrowing facade. No runtime primitive was added: `RecordingFrame::witness`, `CellSlot::record_write`, `recording::RecordedCircuitResult`, and `transaction::prepare_call` are reused. This limits the new proof surface to VM programs already owned by the runtime.

**Evidence and delivery.** Local signed/DCO commit `02bd0be3` (toolchain `0.31.123`). The TypeScript oracle fixture's private transcript values, private state, final Cell, and replay effects match; two root writes emit six Verify operations. All 129 generated fixtures were refreshed/checked, 45 renderer tests passed, the full Cargo workspace checked, and the packaged `--proof` gate proved, verified, and applied `write_twice` in ledger-8 with the expected final Field Cell value `10`. This covers a witnessed Field Cell slice. Nested stateful calls, collections, and other result types still require complete trace support; remote CI is not available before pushing. Issue [#107](https://github.com/MediaNoxLabs/compact/issues/107) remains open in [rust-backend-v2](https://github.com/MediaNoxLabs/compact/milestone/2).


### Amendment — 2026-10-02: nested stateful calls

**Problem.** `bump_twice()` and `add_twice(amount)` called other stateful Compact circuits. Native execution produced the correct ledger state, but the recorder exposed only the leaf `bump`/`add` methods, leaving the exported call without a complete trace. A nested witness call also needed its private outputs and ledger writes in the same order as native execution.

**Before (generated native body, simplified):**

```rust
let call_step = add(context, amount)?;
let context = call_step.context;
let call_step = add(context, amount)?;
let context = call_step.context;
// No recorded::add_twice entry point; consumers could only hand-build a trace.
```

**After (consumer and generated recording, simplified):**

```rust
let recorded = Contract::default().recording.add_twice(context, amount)?;
// Generated from two typed CircuitCall actions:
let __compact_recorded_arg_0 = amount.value() as u16;
let frame = crate::ledger_slots::count.record_increment(frame, __compact_recorded_arg_0)?;
let __compact_recorded_arg_1 = amount.value() as u16;
let frame = crate::ledger_slots::count.record_increment(frame, __compact_recorded_arg_1)?;
```

For private calls, `Contract::from(Secret).recording().write_nested_twice(context, seed)?` borrows the witness implementation. Each inlined callee records its witness FAB result before its Field Cell write, so the second witness sees the first write.

**Emitter and runtime ownership.** `stateful.rs` walks typed schema-6 `CircuitCall` actions, binds each supported Field/Boolean/Uint16 argument once, then expands the callee's supported actions into the caller's recording steps. It tracks the call stack to reject recursion and uses the call graph to propagate `Witnesses<Private>` bounds. A callee must return Unit; an unsupported argument or action withholds the exported recorded method. A negative renderer test confirms nested Counter reset and non-Unit callee shapes do not leak an incomplete API. The runtime and VM program builder did not change: generated steps continue to use `RecordingFrame` and typed `ledger_slots`, then `prepare_call` partitions the complete trace. This expansion can duplicate generated source for repeated calls; generated size/compile-time measurements and a helper-function alternative remain open under ADR-0005/#110.

**Evidence and delivery.** Local signed/DCO commit `9e784a90`, toolchain `0.31.124`. The nested Counter fixture matches TypeScript serialized state and independently replays `bump_twice` and `add_twice`. The nested witnessed Field Cell fixture matches the two-write oracle outcome and preserves two private outputs. The packaged proof gate emitted ZKIR/keys, replayed, partitioned, proved, verified, and applied `bump_twice`, parameterized `add_twice`, and `write_nested_twice` in ledger-8; final Counter values were `2` and `6`, and the Field Cell was `10`. All 129 fixtures were current, 46 renderer tests and focused crate tests passed, the full Cargo workspace checked, and the external consumer gate passed. Broader return expressions, collections, and nested ADTs still need complete recording; branch publication and remote CI remain outstanding. [#107](https://github.com/MediaNoxLabs/compact/issues/107) and parent [#105](https://github.com/MediaNoxLabs/compact/issues/105) stay open in [rust-backend-v2](https://github.com/MediaNoxLabs/compact/milestone/2).



### Amendment — 2026-10-02: nested Field expression effects

**Problem.** The Compact `nested_witness_call_oracle` exports `outer` and `outerValue` could execute natively but had no generated recorded API. Their typed IR combines a Cell read used as a call argument, `Add(WitnessCall(secret), Parameter(next))` inside a nested writer, and a value-returning internal `Call(innerValue)` whose return expression invokes a witness. A recorder that scans only top-level actions would omit ordered private/public effects.

**Before (consumer, simplified):**

```rust
let native = Contract::from(Secret).outer(context.clone())?;
// No recording().outer or recording().outerValue; a proof consumer
// would have to duplicate the read, witness, addition, and write order.
```

**After (generated consumer API and representative body):**

```rust
let recorded = Contract::from(Secret).recording().outer(context)?;
let call = prepare_call(recorded, CallSpec::new("outer", verifier, input, rand))?;

// The typed emitter evaluates the Field argument before entering inner:
let (frame, next) = ledger_slots::value.record_read(frame)?;
let (frame, secret) = frame.witness(|ctx| witnesses.secret(ctx.witness_context_with(view)));
let combined = secret + next;
let frame = ledger_slots::value.record_write(frame, combined)?;
```

`outerValue` likewise records the witness in `innerValue` before the exported Cell write. These snippets show effect order; generated binding names and exact witness call signatures are compiler owned.

**Emitter/runtime changes.** New `tools/compact-rust-backend/src/recorded.rs` owns recorded method emission, separated from native `stateful.rs`. Its effect-aware Field expression lowering handles same-type coercion, parameters, root Field Cell reads, witness calls, addition in left-to-right order, and internal value-returning calls, with cycle rejection and conservative API exposure. The `Let`, nested Unit-call argument, and Field Cell write paths use this same lowering. Unsupported expressions, including Field multiplication, remain native-only; a negative renderer test enforces that boundary. No new runtime primitive was needed: the emitter composes `RecordingFrame`, typed `ledger_slots`, and the existing `prepare_call` adapter. The generated witnessed Cell fixture changed only to bind witness arguments once before recording.

**Evidence and limits.** Local conventional signed/DCO commit `5ca0cb61`, toolchain `0.31.125`. The nested fixture compares both recorded exports with the TypeScript oracle for state and private output order and independently replays their public effects. The packaged `--consumer --proof` gate compiled emitted ZKIR/keys, replayed and partitioned each trace, proved, verified, validated, and applied both `outer` and `outerValue` in ledger-8; final Field Cell was `7`. All 129 generated fixtures were current, 47 renderer tests passed, the full Cargo workspace checked, and the packaged external consumers passed. Broader expression types, collections/ADTs, wallet/node submission, published runtime, and remote CI remain open. This is a local Milestone 2 slice under [#107](https://github.com/MediaNoxLabs/compact/issues/107) and parent [#105](https://github.com/MediaNoxLabs/compact/issues/105), not issue closure.

### Amendment — 2026-10-02: recorded Set circuits

**Problem.** Native Set methods and typed Set slots executed against ledger-8, but the generated crate exposed no complete recorded trace for Set insert, member, remove, size, emptiness, or reset. A consumer could inspect state but could not make a compiler-matching proof from those circuits. Final state equality alone is insufficient: a first reset implementation produced the right empty Set but a different public VM transcript and failed the emitted ZKIR.

**Before (generated consumer API):**

```rust
let native = Contract::default().add(context, true)?;
let member = Contract::default().contains(native.context, true)?;
// There was no recording.add/contains and no generated Set call to prepare_call.
```

**After (generated consumer API):**

```rust
let recorded = Contract::default().recording.add(context, true)?;
let call = prepare_call(recorded, CallSpec::new("add", verifier, true, rand))?;
let observed = Contract::default().recording.contains(context, true)?;
```

The emitted body calls the typed `ledger_slots::seen.record_insert` or `record_member` method. `RecordingFrame` appends each canonical Verify operation in execution order; read operations place the actual Gather event's aligned value in `Popeq`. The generated API is exposed only when the whole supported circuit can be recorded. A negative renderer probe confirms `Set<Bytes<32>>` does not receive an incomplete recorded method.

**Emitter and runtime ownership.** `tools/compact-rust-backend/src/recorded.rs` lowers root Boolean/Field Set insert, remove, reset, member, size and emptiness through the existing AST path. It also lowers Boolean Set membership used in a Cell write, as in the Set oracle fixture. `runtime-rs/src/slots.rs` supplies typed `SetSlot<T>::record_*` methods. `runtime-rs/src/recording.rs` owns frame state, observed gas and ordered trace; `runtime-rs/src/ledger.rs` owns VM programs shared with native operations. No handwritten VM steps are emitted into contract bodies.

**Opcode decision.** For compiler-declared nonempty Set paths, reset uses keyed replacement, `Push(field index), Push(empty Set), Ins(cached: false, n: 1)`, with parent traversal and reinsertion for nested paths. The earlier `Idx(path), Pop, Push(empty Set), Ins` was state-equivalent but failed the generated proof's public transcript check. This difference is explicitly covered by a proven `reset_fields` call.

**Evidence and limits.** Local conventional GPG-signed/DCO commit `de3ccbc9`, toolchain `0.31.128`. Recorded/native state bytes, effects and gas match for the Set sequence; replay and partition pass, tampering with a Set read is rejected, and remove of an absent element replays. The Set Boolean and Set oracle fixtures match their TypeScript oracles. All 129 fixture outputs are current, 47 renderer tests and the full runtime suite pass, the Cargo workspace checks, and the packaged `--consumer --proof` gate proved, verified, validated and applied 18 offline ledger-8 call shapes, nine involving new Set traces. This is local proof/application evidence. Root Field/Boolean Set shapes are covered; other Set element types, complex control flow, Map/List/Merkle recording, wallet/node submission, release and remote CI remain open under [#107](https://github.com/MediaNoxLabs/compact/issues/107) and parent [#105](https://github.com/MediaNoxLabs/compact/issues/105) in [rust-backend-v2](https://github.com/MediaNoxLabs/compact/milestone/2).


**Example clarification.** The two `recording` calls above are independent demonstrations; a real consumer must give each call its own context. For example:

```rust
let read_context = context.clone();
let recorded_add = Contract::default().recording.add(context, true)?;
let add_call = prepare_call(
    recorded_add,
    CallSpec::new("add", add_verifier, true, rand),
)?;
let recorded_contains =
    Contract::default().recording.contains(read_context, true)?;
```


### Amendment — 2026-10-02: recorded Map circuits and constructor seeded lookup

**Problem.** The compiler could emit native Map mutations and queries, but a generated Rust consumer could not obtain a complete ordered Verify trace for them. A stateful Map lookup needs its actual Gather read in `Popeq`; reconstructing it from a decoded Rust value would risk an alignment or transcript mismatch. Map reset and remove must also use the compiler's VM sequence, even where another sequence reaches the same state.

**Before (generated Rust, simplified):**

```rust
let step = context.insert_map(0, key, value)?;
let context = step.context;
// Contract::recording had no put/get path for this Map.
```

**After (generated consumer API and body):**

```rust
let recorded = Contract::default()
    .recording
    .put(context, true, Field::from(42_u64))?;
let call = prepare_call(
    recorded,
    CallSpec::new("put", verifier, (true, Field::from(42_u64)), rand),
)?;

// Generated body keeps the declaration name and types:
let frame = crate::ledger_slots::table.record_insert(frame, key, value)?;
```

The `get` path uses `table.record_lookup(frame, key)` and places the exact observed aligned value in Verify `Popeq`. Root Boolean/Field Map insert, insertDefault, remove, reset, member, lookup, size and emptiness receive recorded methods only when the complete action/return shape is recognized. Unsupported nested Map values and control flow do not receive an incomplete recorded method.

**Emitter and runtime ownership.** `recorded.rs` lowers the typed schema-6 Map actions and returns to named slot calls. `stateful.rs` uses the same slot for native calls. `MapSlot<K,V>` in `slots.rs` keeps key/value types with the compiler-derived path. `RecordingFrame` owns ordered operations, observed gas and state; `ledger.rs` owns Map insert and lookup VM programs shared by native and recording, and the existing Set-compatible member/remove/size/empty/reset programs. The transaction adapter from ADR-0006 is reused without an emitter-owned proof protocol.

**Evidence and limits.** Local conventional GPG-signed/DCO commit `b17f43ad`, toolchain `0.31.129`. The runtime test compares native and recorded Map state bytes, effects and gas, replays and partitions the trace, and rejects a tampered lookup read. The generated Map Boolean/Field and constructor Map fixtures match the TypeScript oracle and replay. All 129 compiler fixture outputs generated; 47 renderer tests, the full runtime suite, focused generated-crate tests, formatting and the Cargo workspace check passed. The packaged `--consumer --proof` gate built separate consumers, rejected wrong Map key/value types, and replayed, partitioned, proved, verified, validated and applied 29 offline ledger-8 calls, 11 new Map calls. Constructor seeded `get_true` and default lookup prove reads from compiler-generated initial state. This is local offline evidence; nested Map values, complex expressions/control flow, List/Merkle traces, wallet/node submission, release and remote CI remain open under [#107](https://github.com/MediaNoxLabs/compact/issues/107) and [#105](https://github.com/MediaNoxLabs/compact/issues/105) in [rust-backend-v2](https://github.com/MediaNoxLabs/compact/milestone/2).


### Amendment — 2026-10-02: replayable root List circuits

**Problem.** A generated Rust List circuit could run natively but had no complete `recording` method. List head reads must record the exact aligned Gather value in Verify `Popeq`; final state equality alone does not prove transcript parity. The first proof probe exposed two concrete gaps: computing `Concat` size from the serialized default zero produced `4` where Compact ZKIR required the Field alignment maximum plus two (`38`), and root reset included an empty-path `Idx` and zero-length cached `Ins` that Compact suppresses.

**Before (generated consumer and first failed trace):**

```rust
let native = Contract::default().prepend(context, Field::from(7_u64))?;
// No recording.prepend or recording.first_item call was available.
// A first head trace used Concat { n: 4 }; Compact's ZKIR uses n: 38.
```

**After (generated consumer and representative recorded body):**

```rust
let recorded = Contract::default()
    .recording
    .prepend(context, Field::from(7_u64))?;
let call = prepare_call(
    recorded,
    CallSpec::new("prepend", verifier, Field::from(7_u64), rand),
)?;

let (frame, head): (_, Maybe) =
    crate::ledger_slots::items.record_head::<Maybe, _, _>(frame)?;
```

`recording` is emitted for supported complete root Field/Boolean List push, pop, reset, length, emptiness, and head circuits. The result of `head` keeps Compact's `Maybe<T>` shape. A constructor seeded List supplies the present-head and pop/reset proof cases; an empty List supplies absent-head and push/reset cases. The collector records actual Gather reads, gas and state, then replays Verify operations before `prepare_call` partitions them.

**Emitter and runtime ownership.** `recorded.rs` checks the schema-6 List declaration, physical root index, element/result types, and whole circuit shape before emitting named `ListSlot<T>::record_*` calls. `stateful.rs` emits native calls through the same slot. `RecordingFrame` in `runtime-rs/src/recording.rs` owns the ordered Verify trace and observed reads; `runtime-rs/src/ledger.rs` owns shared native/recorded List VM programs. The head program now uses `2 + T::alignment().max_aligned_size()` from the pinned Midnight primitive, exactly matching `compiler/midnight-ledger.ss`'s `2 + rt-max-sizeof(value_type)`. Root reset uses `Push(field index), Push(empty List), Ins(cached: false, n: 1)`; Compact's `suppress-null` and `suppress-zero` remove the other two operations. No List VM sequence is handwritten in generated contract bodies.

**Evidence and limits.** Local conventional GPG-signed/DCO commit `4812b3f3`, toolchain `0.31.130`. The runtime List test compares native/recorded state bytes, effects and gas, replays and partitions the trace, asserts the compiler's `Concat` operand `38`, and rejects a tampered head read. Generated List and constructor List tests compare native/recorded outcomes and preserve the existing TypeScript constructor state oracle. The packaged `--consumer --proof` gate built standalone/shared consumers, emitted ZKIR and keys, and replayed, partitioned, proved, verified, validated and applied 39 offline ledger-8 calls, including 10 List calls. The 47 renderer tests, full runtime suite, focused List fixtures, formatting, and workspace check passed. All 129 compiler fixture outputs are current (0 stale, 0 failed). This is local offline evidence. Nested List fields, richer element/expression types, complex control flow, wallet/node submission, published runtime, and remote CI remain open. Track [#111](https://github.com/MediaNoxLabs/compact/issues/111), [#107](https://github.com/MediaNoxLabs/compact/issues/107), and parent [#105](https://github.com/MediaNoxLabs/compact/issues/105) in [rust-backend-v2](https://github.com/MediaNoxLabs/compact/milestone/2).



### Amendment — 2026-10-02: replayable Bytes and enum Cell circuits

**Problem.** The compiler could generate native root `Cell<Bytes<N>>` and `Cell<Choice>` calls, but the recorder recognized only Boolean and Field Cell values. A Rust consumer therefore could not prepare a proven call for a complete typed Bytes write or enum write/read, even though the runtime's typed `CellSlot<T>` and canonical VM operations already supported those values. We need to extend the closed, whole-circuit recorder without emitting a partial method for the more complex `tiny` contract.

**Before (generated Rust, simplified):**

```rust
let native = Contract::default().choose(context, Choice::no)?;
// No Contract::recording.choose for an enum Cell.
let native = Contract::default().setHash(context, bytes)?;
// No Contract::recording.setHash for a Bytes Cell.
```

**After (generated Rust and consumer API):**

```rust
let recorded = Contract::default().recording.choose(context, Choice::no)?;
let call = prepare_call(recorded, CallSpec::new("choose", verifier, Choice::no, rand))?;

// Generated body uses the same typed slot as native emission.
let frame = crate::ledger_slots::choice.record_write(frame, (__compact_param_0).clone())?;
let (frame, observed): (_, crate::types::Choice) =
    crate::ledger_slots::choice.record_read(frame)?;
let frame = crate::ledger_slots::hashCell.record_write(frame, bytes.clone())?;
```

The read and write lines illustrate separate generated circuits. `Choice::no` literals and `<FixedBytes<32> as Default>::default()` are accepted only where the IR type matches the declared Cell. Parameter/local clones preserve owned Rust values across the generated frame. The whole-circuit recognition gate remains in place: a contract with unsupported witness, assertion, conditional, or `Maybe` steps gets no incomplete `recording` method.

**Emitter/runtime ownership and alternatives.** `tools/compact-rust-backend/src/recorded.rs` extends typed Cell source, write and read lowering for `Type::Bytes` and `Type::Enum`, after matching the ledger declaration, physical root path, source expression and result type. `runtime-rs/src/slots.rs`, `runtime-rs/src/recording.rs`, and `runtime-rs/src/ledger.rs` already own typed slot calls, ordered Verify operations, observed Gather values, state and gas; this slice changes none of them. Emitting raw VM instructions in generated Rust or reconstructing an enum read from its decoded result would duplicate runtime logic and risk transcript divergence, so both remain rejected. No new macro is justified for these ordinary typed calls.

**Evidence.** Local conventional GPG-signed/DCO commit `b17d2ab0` on `codex/rust-backend-ast`, compiler 0.31.133 / private IR schema 8 / runtime ABI 4. New `recorded_enum_cell` fixture compares native and recorded state to a byte-reproducible TypeScript oracle, matches gas and result, replays the complete Verify trace for enum parameter/literal writes and a read, and checks the observed read. Existing `constructor_persistent_hash` and `inline_type_scope_oracle` fixtures now exercise recorded Bytes read and write with native/recorded state, gas and replay equality. The packaged `compactc --target rust` gate emits ZKIR and keys, replays and partitions `choose`, and proves, verifies, validates and applies the call in an offline ledger-8 transaction. This extends the proof gate from 39 to 40 call shapes. All 130 fixture outputs are current (0 stale/failed); 51 renderer and four CLI tests, focused generated-crate tests, workspace check, formatting and staged diff checks passed.

**Limits and tracking.** This is root scalar Cell trace support, not full `tiny` recording: Bytes witness outputs, enum equality/assertions, pure calls, conditional `Maybe` returns, and whole-circuit public transcript comparison remain open. Nested Cell paths, wallet/node submission, published packages and remote CI also remain open. Track [#107](https://github.com/MediaNoxLabs/compact/issues/107), typed-slot [#108](https://github.com/MediaNoxLabs/compact/issues/108), and parent [#105](https://github.com/MediaNoxLabs/compact/issues/105) in [rust-backend-v2](https://github.com/MediaNoxLabs/compact/milestone/2). The branch has not been pushed.



**Coverage correction (2026-10-02).** The emitter now has a typed `Expr::Default` lowering branch, but this commit’s executed generated fixtures cover enum parameter and literal writes, enum and Bytes reads, and a Bytes parameter write. They do not exercise a recorded default-value write. That case remains an explicit regression-test follow-up.
