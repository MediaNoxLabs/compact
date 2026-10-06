---
id: RUST-ADR-0045
alias: ADR-0045
title: "Record composite Set keys as typed observed calls"
date: 2026-10-03
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["backend", "recording", "ledger-adts"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: de67039bea92277b737622abe16b53f181e9f1aa74c84758c4b1a176b0449d42
---
# RUST-ADR-0045 — Record composite Set keys as typed observed calls

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Accept tuple/struct Set keys through existing typed codecs and recorded slots, with observed-call input identity. Later member/remove and chunked-path evidence extend the root insert slice. Physical chunking is distinct from a Set nested as another collection's value; neither type breadth nor proof breadth is universal.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#144 closure](https://github.com/MediaNoxLabs/compact/issues/144#issuecomment-6017473564). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`3125f60c`](https://github.com/MediaNoxLabs/compact/commit/3125f60cbf6e21c2085c6145d0da7bdfb4ab4c93) · [`4afca58e`](https://github.com/MediaNoxLabs/compact/commit/4afca58eb3e7357d18f897e3de4c7eb07a9cc28a) · [`66348cd7`](https://github.com/MediaNoxLabs/compact/commit/66348cd7a8f3b29c20bcf383c17b05ce5b0d56fb) · [`805c1fc6`](https://github.com/MediaNoxLabs/compact/commit/805c1fc6e1ba751fccc3f3f1dfadd1226af915f2). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
adr: 45
status: accepted-partial
date: 2026-10-03
milestone: rust-backend-v2
commit: 66348cd7a8f3b29c20bcf383c17b05ce5b0d56fb
issue: https://github.com/MediaNoxLabs/compact/issues/144
```

## Historical decision and amendments

### Problem

ABI 22 can emit a typed `*_call` for a complete recorded export, and a Vector-key Set insert is recordable. A Set keyed by a tuple or a generated struct still has only its native method: the AST recording emitter rejects those key types before it can produce a replayable trace. A Rust consumer cannot build a checked observed call for such a circuit through the generated crate. This is a recording eligibility gap, not a need for a new ledger VM or an untyped text template.

The concrete source is `examples/rust_backend/observed_composite_keys.compact`. It has three Set fields keyed by `Vector<2, Field>`, `[Field, Boolean]`, and `CompositeKey { vector: Vector<2, Field>; pair: [Field, Boolean] }`. The struct deliberately nests both a vector and a tuple, so a struct label alone cannot mask a flat scalar-only probe.

### Before and after in a consumer

Before ABI 23, the tuple and struct circuits return native `CircuitResult` and have no generated recorded or observed method:

```rust
let native = contract.insert_struct(context, key)?;
// No contract.recording.insert_struct or insert_struct_call exists.
```

After ABI 23, the generated crate exposes source-typed recording and observed-call methods:

```rust
let key = CompositeKey {
    vector: FixedVector::new([Field::from(3), Field::from(5)]),
    pair: (Field::from(42), true),
};
let call = contract.recording
    .insert_struct_call(&observed, (), key)?
    .prepare(verifier, Field::from(0))?;
```

The same surface exists for `insert_vector` and `insert_tuple`. A separate Cargo consumer depends directly only on the generated contract crate; a tuple with `(bool, Field)` in place of `(Field, bool)` fails Rust compilation with E0308.

### Decision and ownership

The `syn`/`quote` AST emitter extends its declared-type eligibility for root Set recording to tuples and generated structs. The typed value source accepts an IR parameter or default of exactly the declared type and clones a composite parameter before the recording step. The generated method calls the existing `SetSlot<T: CellValue>::record_insert` and constructs its observed input with upstream `AlignedValue::from(key.clone())`. The `CompactCellValue` derive supplies the generated struct's upstream FAB `Aligned` and `Value` implementations; `FixedVector` and tuples use their pinned ledger-8 primitives. There is no body-wide string append or new VM interpreter.

The implementation also makes tuple/struct keys eligible in the existing root Set member and insert/remove recording branches when their expression can be lowered; the committed proof cases exercise insertion. More complex expression forms remain ineligible rather than emitting an incomplete trace. The runtime changes only `RUST_RUNTIME_ABI` from 22 to 23 because the generated API expands. Private Rust IR schema remains 8; transaction, VM, proof, and wallet logic are unchanged.

### Independent parity evidence

Fresh TypeScript compilation and `capture-observed-composite-keys.mjs` record:

| Input | Public FAB atoms | Alignment |
|---|---|---|
| Vector `[3,5]` | `[3]`, `[5]` | Field, Field |
| Tuple `[42,true]` | `[42]`, `[1]` | Field, one-byte Boolean |
| Nested struct | `[3]`, `[5]`, `[42]`, `[1]` | Field, Field, Field, one-byte Boolean |

Each TypeScript call has the same five-operation Set insertion shape: `Idx(cached=false,pushPath=true,pathLength=1)`, `Push(storage=false)`, `Push(storage=true)`, `Ins(cached=false,n=1)`, `Ins(cached=true,n=1)`. The committed Rust fixture checks FAB atoms/alignment, exact operation shape, native/recorded gas and effects, state, and replay against that independent capture.

The complete ABI-23 Nix-packaged `compactc` gate passes with 63 offline proof/application calls. For the three new cases, manual `CallSpec` and generated `*_call` have equal complete prototypes and exact tagged pre-proof bytes of 572, 539, and 615 respectively. Both paths independently prove, verify, validate, and apply to equal ledger-8 state. A separate applied call confirms a one-element Set containing the declared Vector, tuple, or struct key. Independent sealed proof bytes can differ under the pinned PLONK blinding; the deterministic boundary is the pre-proof transaction (ADR-0043).

The local 57 renderer tests, two focused composite fixture tests, 133 fresh generated fixtures, one-dependency consumer and wrong-type rejection, two compiler rejection probes, formatting, and all-features Cargo workspace check pass. Local conventional GPG-verified/DCO commit `66348cd7a8f3b29c20bcf383c17b05ce5b0d56fb` contains this slice. ABI-23 package `${HISTORICAL_NIX_STORE}/j6q1mp2dgjj393qdsq6a6j085d8ldnr2-compactc` was built from a dirty worktree. The exact new source grows from 229 generated Rust lines under ABI 22 (native only) to 305 under ABI 23 (+76); no isolated compile-time or speed claim is made.

### Limits and follow-up

This decision proves three concrete Set insertion shapes. It does not establish arbitrary nested Compact values, tuple/struct Set member or remove proof parity, nested Map/List/Cell calls, an actual signature above 11 parameters, wallet/node submission for this source, compile-time impact, remote CI, registry publication, or clean signed release provenance. The current branch is local and unpushed; the only unstaged tracked file is the user-owned `doc/ledger-adt.mdx`. Keep the focused issue and milestone open until their release gates pass. ADR-0044/#143 remains the broader observed-input compatibility decision.

### Tracking

- Focused MediaNoxLabs issue: to be linked after creation in `rust-backend-v2`.
- Related: [ADR-0044 — Encode multi-parameter observed Rust calls from typed IR](0044-encode-multi-parameter-observed-rust-calls-from-typed-ir.md) / [#143](https://github.com/MediaNoxLabs/compact/issues/143), parent [#103](https://github.com/MediaNoxLabs/compact/issues/103) and [#105](https://github.com/MediaNoxLabs/compact/issues/105).



### Tracking established — 2026-10-03

Focused MediaNoxLabs [#144](https://github.com/MediaNoxLabs/compact/issues/144) is open and assigned to [rust-backend-v2](https://github.com/MediaNoxLabs/compact/milestone/2). This supersedes the pending-link line above without erasing the creation sequence. The issue contains the local delivery evidence and explicit closure gates; the signed/DCO commit remains local and unpushed.



### Follow-up research — 2026-10-03: member/remove paths

The ABI-23 eligibility change also admitted tuple and generated-struct keys to the root Set member and remove recording branches, while ADR-0045 initially proved only insertion. To test that exact residual risk, the same real source now adds `roundtrip_tuple(key)` and `roundtrip_struct(key)`: each inserts the declared key, removes it, and returns `member(key)`. Before this follow-up, consumers could type-check those recorded/observed methods but had no independent proof that their multi-operation transcript and result were correct. After acceptance, the intended Rust call is `contract.recording.roundtrip_struct_call(&observed, (), key)?.prepare(verifier, randomness)?`, returning a Boolean output with an empty final Set.

A fresh TypeScript compile records the same tuple/struct input atoms and alignment as the insertion cases, a `false` result, and 14 ordered operations: `Idx Push Push Ins Ins`, `Idx Push Rem Ins`, `Dup Idx Push Member Popeq`. The committed Rust fixture has been refreshed and its focused tests pass native/recorded result, gas, effects, state, replay and the exact normalized operation shape. The full ledger proof/application gate and standalone generated-crate consumer for these new methods are running; this paragraph does not claim their result. No emitter/runtime/ABI/schema change is proposed for the follow-up, because it exercises the already accepted ABI-23 path. Preserve the independent proof outcome in a dated delivery amendment rather than silently changing this research checkpoint.

Research wording correction: the refreshed Rust fixture is tracked in the repository but its roundtrip changes were not committed at this checkpoint; only the focused local tests had passed.


### Delivery amendment — 2026-10-03: prove member and remove on composite keys

**Decision:** retain ABI-23 root Set member/remove eligibility for tuple and generated-struct keys after exercising those paths through an actual Compact circuit and ledger proof. The preceding research paragraph was intentionally provisional. Local conventional GPG-verified/DCO commit `3125f60cbf6e21c2085c6145d0da7bdfb4ab4c93` delivers the evidence without changing emitter/runtime algorithms, ABI 23, or private IR schema 8.

#### Before and after Rust

Before this follow-up, only insertion had an independently proved generated call for a composite key:

```rust
let inserted = contract.recording
    .insert_struct_call(&observed, (), key.clone())?
    .prepare(verifier, randomness)?;
// The generated member/remove path had no proof/application gate.
```

After, the real `roundtrip_struct(key: CompositeKey): Boolean` source inserts, removes, and queries the same key, with a generated typed call:

```rust
let cleared = contract.recording
    .roundtrip_struct_call(&observed, (), key)?
    .prepare(verifier, randomness)?;
```

`roundtrip_tuple` uses `(Field, bool)` identically. The return is `false` and the applied Set is empty. A generated-crate-only Cargo consumer type-checks both methods using only the generated crate as its direct dependency. No new macro, string template, typed DSL, runtime transaction adapter, VM interpreter, or prover was added; the existing typed `SetSlot<T>` and upstream ledger primitives perform the work.

#### Independent oracle and ledger evidence

Fresh TypeScript compilation and the reproducible `capture-observed-composite-keys.mjs` capture preserve tuple atoms `[42]`, `[1]` with Field/one-byte Boolean alignment, and nested-struct atoms `[3]`, `[5]`, `[42]`, `[1]` with Field/Field/Field/one-byte Boolean alignment. Each `false`-returning roundtrip records 14 ordered operations: `Idx Push Push Ins Ins`, `Idx Push Rem Ins`, `Dup Idx Push Member Popeq`. The focused Rust fixture checks the exact normalized TypeScript operation shape, result, native/recorded gas, effects, state, and replay. The capture reproduces its committed JSON byte for byte.

The rebuilt 65-call offline gate generates all five composite proving artifacts. For `roundtrip_tuple` and `roundtrip_struct`, manual `CallSpec` and generated observed calls match complete prototypes and exact tagged pre-proof transactions of 694 and 906 bytes. Both paths independently prove, verify, validate and apply to equal ledger-8 state. A separate applied call confirms the relevant Set has size zero and does not contain the key. The independent TypeScript `false` result agrees with Rust. As before, independent sealed proofs may differ under pinned PLONK blinding; exact pre-proof bytes are the deterministic parity boundary.

The two focused fixture tests, generated-crate-only consumer, 133 fresh fixture outputs, formatting, syntax check, and current local ABI-23 compiler/proof gate pass. The exact generated source grows 305→456 lines (+151) because two exported circuits were added; no isolated compile-time or speed measurement is claimed. Local worktree has only the user-owned unstaged `doc/ledger-adt.mdx`; the branch is unpushed.

#### Remaining production limits

This closes the explicit tuple/struct root Set member/remove parity gap in [#144](https://github.com/MediaNoxLabs/compact/issues/144). It does not prove other nested ledger containers, arbitrary nested expressions, actual >11-parameter runtime calls, wallet/node admission for this source, compile-time impact, remote CI, clean signed registry release, or branch publication. Keep #144 and `rust-backend-v2` open for those exit gates.



### Related arity checkpoint — 2026-10-03

ADR-0044 / [#143](https://github.com/MediaNoxLabs/compact/issues/143) now proves one real twelve-Field observed call in local signed/DCO `4afca58e`, including independent TypeScript FAB/transcript, exact 546-byte manual/generated pre-proof parity, generated-crate-only consumer and the 66-call proof/application gate. This closes the earlier **actual >11-argument execution** caveat for that bounded all-Field signature. It does not extend the composite-key recording support of this ADR to nested ledger locations or establish a high-arity mixture of tuple/struct/Vector keys. ABI 23 and schema 8 remain unchanged.



### Related chunked Set-path checkpoint — 2026-10-03

ADR-0046 / [#145](https://github.com/MediaNoxLabs/compact/issues/145) now extends the typed tuple-key Set call through a compiler-chunked physical path `[1,14]` in local signed/DCO `805c1fc6`. Independent TypeScript state/FAB/gas/two-segment transcript and 70-call proof/application gate cover insert, insert/remove/member, size and isEmpty; exact manual/generated pre-proof bytes are 557/719/491/512. This closes the earlier **chunked physical ledger path** caveat for this Set shape. A Set nested as another collection's value, other ADTs, remote CI and release remain separate. ABI 24/schema 8; branch local/unpushed.
