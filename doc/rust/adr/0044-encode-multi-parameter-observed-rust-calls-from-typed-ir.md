---
id: RUST-ADR-0044
alias: ADR-0044
title: "Encode multi-parameter observed Rust calls from typed IR"
date: 2026-10-03
publication_date: 2026-10-07
decision_status: "accepted-bounded"
topics: ["architecture", "generated-api"]
assessed_implementation_revision: 03c03a39a2d39c43c91143b9526ca1c2ffae706c
delivery_status: milestone-closed-with-recorded-scope
source_sha256: 2a5b9777aba9b543c624c47ed9eaf469b7c07e14c2140ac6b47c306a41a2ba20
---
# RUST-ADR-0044 — Encode multi-parameter observed Rust calls from typed IR

[ADR register](README.md) · [Evidence and publication conventions](references.md) · [Rust documentation](../README.md)

## Reviewed disposition — 2026-10-07

**Decision:** accepted-bounded. Accept declaration-ordered typed public FAB input composition for the delivered multi-argument observed-call paths, including witnessed and arity 12 cases. Preserve the corrected three-argument source/capture and the distinction between synthetic signature checks and executing evidence. High arity alone does not establish all mixed/nested alignments.

**Delivery:** the `rust-backend-v2` milestone is closed at implementation revision [`03c03a39`](https://github.com/MediaNoxLabs/compact/commit/03c03a39a2d39c43c91143b9526ca1c2ffae706c). Focused records: [#143 closure](https://github.com/MediaNoxLabs/compact/issues/143#issuecomment-6017471810). This documentation publication does not rerun or extend that acceptance. Earlier “pending”, “local only” and ABI/schema statements below describe their recorded dates and revisions.

**Historical evidence:** machine-local paths below use symbolic roots; linked unpublished notes are identified in the reference register. Their bytes are not included here. These references are not public downloads or new reproducibility claims.

**Referenced repository commits:** [`3125f60c`](https://github.com/MediaNoxLabs/compact/commit/3125f60cbf6e21c2085c6145d0da7bdfb4ab4c93) · [`4afca58e`](https://github.com/MediaNoxLabs/compact/commit/4afca58eb3e7357d18f897e3de4c7eb07a9cc28a) · [`66348cd7`](https://github.com/MediaNoxLabs/compact/commit/66348cd7a8f3b29c20bcf383c17b05ce5b0d56fb) · [`70880458`](https://github.com/MediaNoxLabs/compact/commit/708804585fdcb594394586cea443330ae3e18a42) · [`947a40df`](https://github.com/MediaNoxLabs/compact/commit/947a40df818083eea36f9c3574f9a3d3ca63a87f) · [`bd2415e1`](https://github.com/MediaNoxLabs/compact/commit/bd2415e156e0f63189aebba8258aff43eeba6545). These include historical prerequisites and probes, not only final delivery commits.

### Original source metadata

```yaml
adr: 44
status: accepted-partial
date: 2026-10-03
milestone: rust-backend-v2
issue: https://github.com/MediaNoxLabs/compact/issues/143
```

## Historical decision and amendments

### Problem

ADR-0043 gives generated observed-state call methods to complete recorded exports with zero or one parameter. The AST emitter explicitly suppresses methods with two or more parameters because public FAB input parity has not been established. A Rust developer can record such a circuit, but must again choose the entry-point string and encode its input through `CallSpec`. This reintroduces the circuit/input mismatch that ADR-0043 removed for simpler signatures.

### Before and proposed after

Illustrative two-parameter circuit; its exact Compact source and TypeScript oracle fixture must be selected before implementation:

```rust
let recorded = contract.recording.combine(
    observed.circuit_context(private), left.clone(), right.clone())?;
let input = /* caller must determine the exact public FAB layout */;
let call = prepare_call(recorded,
    CallSpec::new("combine", verifier, input, randomness))?;
```

Proposed generated surface after independent input evidence:

```rust
let observed = ObservedContractState::decode(address, indexed_bytes, observation)?;
let call = contract.recording
    .combine_call(&observed, private, left, right)?
    .prepare(verifier, randomness)?;
```

The proposed method owns entry-point identity and clones typed arguments before recording. The emitter must derive the exact public input from ordered declared IR parameters. A Rust tuple conversion is a candidate, not an accepted encoding until the TypeScript oracle and ledger-8 pre-proof bytes agree. Do not flatten nested tuples, vectors or structs by intuition.

### Evidence at proposal time

- `render_observed_call_method` currently returns `UnsupportedStatefulCall` for more than one input, and `lib.rs` only invokes it for `parameters.len() <= 1`.
- Pinned `midnight-base-crypto 1.0.0` FAB conversion has tuple `Value` and `AlignedValue` support; this proves Rust representation availability, not parity with the Compact compiler's circuit argument encoding.
- ADR-0043 establishes exact tagged pre-proof transaction equality as the appropriate adapter invariant. Independently proved sealed bytes can differ because pinned `midnight-proofs 0.7.0` uses `OsRng` during quotient blinding.

### Emitter/runtime ownership

- AST emitter selects complete exported recorded circuits with two declared parameters, derives method name, Rust parameter types and ordered public FAB input from `StatefulCircuit.parameters`, and keeps current collision and witness-borrowing rules. Use `syn` items and the existing recorded method; no body-wide text append or proc macro.
- Runtime should reuse `ObservedContractState`, `RecordedCall`, exact verifier decoder and `prepare_call`. Add runtime logic only if a typed encoding helper is required by measured parity; no new local prover, wallet policy or VM interpreter.
- Reuse upstream ledger-8 FAB, verifier, contract and transaction primitives. Any expanded public generated API needs an explicit generated/runtime ABI decision; private IR schema 8 should remain if parameters suffice.

### Acceptance before delivery

1. Choose an existing or minimal real ledger-8 Compact source with two parameters of distinct shapes. Capture the TypeScript public input and ordered transcript; independently build the manual Rust `CallSpec` input. Compare complete `ContractCallPrototype` and exact tagged pre-proof transaction bytes, then independently prove, verify, validate and apply both paths to equal upstream state.
2. Include at least one witnessed borrowed generated method, an external consumer whose only direct dependency is the generated crate, wrong-typed-argument rejection and an exported `<name>_call` collision case. If the first slice supports only two parameters, keep larger arities suppressed and document that boundary.
3. Keep 132 fixtures fresh, renderer and runtime tests, separate consumer, all existing 58 offline proof/application calls, local package rehearsal and same-head wallet gate green if this changes the live path. Report source-size and compile-time measurements without claiming an optimization from source shape alone.

### Risks and remaining production gates

Tuple flattening, alignment boundaries and cloned argument semantics may vary with nested value types. Caller-supplied observation metadata remains subject to trusted node/indexer and concurrent state changes. Remote CI, clean signed release provenance, registry publication and branch publication belong to the wider rust-backend-v2 milestone. This is a research proposal: no implementation, test result, ABI change or branch push is claimed.

### Tracking

- Focused MediaNoxLabs issue: [#143](https://github.com/MediaNoxLabs/compact/issues/143) in `rust-backend-v2`.
- Related: ADR-0043 / #142; parent generated crate #103 and ledger flow #105.
- Append dated amendments for the chosen source, before/after actual code, emitter/runtime compatibility, exact gates, signed/DCO commits, and unresolved limits.


### Research amendment — 2026-10-03: select a real first circuit

The first measured slice will use `examples/rust_backend/map_boolean_field.compact` `put(key: Boolean, value: Field): []`. It is already recorded in the generated Rust fixture and has emitted proof artifacts in the current compiler/proof gate. The local ABI-20 `compactc --target ts --skip-zk` output for this exact source constructs `partialProofData.input.value` by concatenating the Boolean descriptor's `toValue(key)` with the Field descriptor's `toValue(value)`, and concatenates the matching descriptor alignments in the same order. This establishes the TypeScript order for this circuit, but not yet equality with a Rust tuple's `AlignedValue`, full prototype or pre-proof bytes. The generated TypeScript output is an ignored local research artifact at `target/compact-rust-adr44-map-ts/contract/index.js`; it is not a committed fixture or delivery.

The next test should compare `AlignedValue::from((true, Field::from(42)))` with the independent TypeScript input capture, then compare the current manual `put` call and the proposed generated `put_call` at the ledger prototype/pre-proof boundary. A witnessed two-argument circuit such as `boolean_logic.witnessed_both` is a separate borrowing check; it should not substitute for the Map proof case. No emitter/runtime edit or ABI change has occurred at this checkpoint.


### Delivery amendment — 2026-10-03: measured Boolean/Field slice

**Decision:** accept a bounded two-argument observed-call method for complete recorded exports after direct TypeScript and ledger-8 parity evidence. This amends the proposal above; its original acceptance list remains the wider target. Status is **accepted-partial**, not a claim of arbitrary arity or shape support. Local conventional GPG-verified/DCO commit: `708804585fdcb594394586cea443330ae3e18a42` (`feat(rust-backend): emit typed two-argument observed calls`). No branch push or remote CI.

#### Actual developer surface

Before, a caller records `Map.put` and supplies the entry point and public input manually:

```rust
let recorded = contract.recording.put(
    observed.circuit_context(()), true, Field::from(42_u64))?;
let input = AlignedValue::from((true, Field::from(42_u64)));
let manual = prepare_call(recorded,
    CallSpec::new("put", verifier, input, Fr::from(0_u64)))?;
```

After, the generated crate owns the typed identity and input construction:

```rust
let typed = contract.recording
    .put_call(&observed, (), true, Field::from(42_u64))?
    .prepare(verifier, Fr::from(0_u64))?;
```

The actual generated method takes `bool` then `runtime::Field`, clones each in declared IR order into `runtime::fab::AlignedValue::from((arg0, arg1))`, records using the existing `put` method, and returns `RecordedCall::new(observed, recorded, "put", input)`. `prepare` checks the installed verifier, observed address and state through the existing runtime adapter. A separate consumer with only the generated crate as its direct dependency invokes this method; swapped Field/Boolean arguments fail Rust compilation with E0308.

#### Encoding and ledger evidence

The reproducible independent TypeScript capture `runtime-rs/tests/fixtures/capture-map-boolean-put-input.mjs` compiles `examples/rust_backend/map_boolean_field.compact` and writes `map-boolean-put-input.json`. For `put(true, 42n)`, public FAB atoms are `[1]`, `[42]`; alignments are a one-byte Boolean atom followed by a Field atom. The TypeScript five-operation transcript shape is `Idx(cached=false,push_path=true,path_len=1)`, `Push(storage=false)`, `Push(storage=true)`, `Ins(cached=false,n=1)`, `Ins(cached=true,n=1)`. The Rust runtime encoding test compares the tuple FAB with those atoms and alignments. The proof smoke checks Rust's typed upstream VM operations against that independent shape.

The generated `put_call` and manual `CallSpec` paths have equal complete `ContractCallPrototype` values and exactly equal 522-byte tagged pre-proof transaction payloads. Each path is independently proved, verified, validated and applied to equal ledger-8 state. This is the deterministic parity boundary; independent sealed proofs may differ because pinned PLONK quotient blinding draws from `OsRng` (ADR-0043). The rebuilt compiler/consumer/proof gate passes all existing 58 offline call shapes plus the second independent Map path.

#### Emitter, runtime and compatibility

The syn/quote AST emitter enables observed methods when `StatefulCircuit.parameters.len() <= 2`; `recorded.rs` makes a two-element tuple expression from the typed parameter expressions. More than two remain suppressed, as do names colliding with exported `<name>_call` circuits. The borrowed witnessed two-argument method is parsed and its witness bound checked in a renderer test; a real witnessed two-argument external consumer remains to be exercised. `runtime-rs` changes only its compatibility constant from ABI 20 to 21, because newly generated crates expose a larger API. There is no new transaction, VM, prover or wallet runtime logic, and private IR schema remains 8. The map Boolean fixture grows from 582 to 606 lines (+24); the entire 132-fixture regeneration is net +48 lines. Compile-time impact was not isolated, so no size or speed optimization is claimed.

#### Local verification and remaining gates

The 132-fixture freshness gate, 37 pinned-oracle inventory, 57 renderer tests, seven runtime encoding tests, separate generated-crate consumer and negative type check, full `check_compactc_target.py --consumer --proof`, formatting, syntax and staged-diff checks pass. The local macro/runtime package write and verify rehearsal passed for ABI 21 at `target/rust-runtime-release-abi21.json`; it includes an uncommitted user documentation edit in its dirty-source metadata, so it is not a clean release candidate. The compiler/proof gate used the current local ABI-21 `compactc` in an already-built Nix compiler shell because rebuilding the Nix CLI exhausted local disk; it did not certify a newly packaged Nix CLI. Local target incremental caches were cleared to recover space.

Follow-up within [#143](https://github.com/MediaNoxLabs/compact/issues/143): independently measure nested tuple/struct/vector input layouts and three-plus-argument behavior before expanding the emitter; add a real witnessed two-argument external consumer; measure generated compile time; establish same-head remote CI and clean signed package provenance. Caller-supplied observations still require trusted node/indexer provenance and stale-state handling. Keep the issue open until those acceptance gates are met.


### Research amendment — 2026-10-03: witnessed two-argument consumer

The first delivery's renderer parses a synthetic borrowed witness method, but no generated-crate-only consumer or ledger proof calls a witnessed method with two public parameters. This leaves a developer-facing risk: `Contract<W>::recording().<name>_call` may fail to borrow `W` correctly or may reorder public inputs when witness recording mutates private state. The next bounded probe will extend the existing `witness_cell_write.compact` source with a real two-Field circuit using `secret(seed)` and a second public offset. Before, that circuit has no generated typed call; callers would manually construct `CallSpec` with the entry name and two Field atoms. After, its generated `*_call(&observed, private, seed, offset)` should own those facts and borrow the witness implementation. The emitter and ABI-21 runtime need no new rule if the current two-argument method is sound; this change adds source and acceptance evidence. Capture TypeScript input order, compile a consumer with only the generated crate, compare manual/generated complete prototype and exact pre-proof bytes, independently prove/apply, and keep any unsupported witness shape explicit. This probe continues focused issue #143 and does not change its milestone or close it.


### Delivery amendment — 2026-10-03: real witnessed two-argument call

The prior delivery proved a Boolean/Field Map method and parsed a synthetic borrowed witness method. This amendment closes that specific evidence gap with `examples/rust_backend/witness_cell_write.compact` `write_offset(seed: Field, offset: Field)`, whose witness `secret(seed)` reads a Cell and advances private state. Local conventional GPG-verified/DCO commit: `bd2415e156e0f63189aebba8258aff43eeba6545` (`test(rust-backend): prove witnessed two-argument observed calls`). This is additional acceptance evidence for [#143](https://github.com/MediaNoxLabs/compact/issues/143), not a new emitter design decision.

Before, a caller of this new circuit would need to borrow the witness-backed recording handle, provide the entry point and encode two Field atoms manually:

```rust
let recorded = contract.recording().write_offset(
    observed.circuit_context(7_u64), Field::from(2), Field::from(5))?;
let manual = prepare_call(recorded, CallSpec::new(
    "write_offset", verifier,
    AlignedValue::from((Field::from(2), Field::from(5))), randomness))?;
```

After, the generated crate carries the source identity and ordered input through the borrowed witness handle:

```rust
let typed = contract.recording()
    .write_offset_call(&observed, 7_u64, Field::from(2), Field::from(5))?
    .prepare(verifier, randomness)?;
```

`BorrowedContract<'_, W>` retains `&W`; the method requires `W: TryWitnesses<Private>`, clones both typed arguments for FAB input, then calls the same recorded body. A separate Cargo consumer with only `compact_contract_witness_cell_write` as a direct dependency calls it and verifies private state 7→8, one private output and the checked `write_offset` entry identity. No emitter, transaction runtime, macro, private IR or ABI edit was needed; schema 8 and ABI 21 remain.

The reproducible independent TypeScript capture `capture-witness-offset-input.mjs` for `write_offset(2n,5n)` records public Field atoms `[2]`, `[5]`, two Field alignments, one private output atom `[9]`, and ordered VM `Push(storage=false)`, `Push(storage=true)`, `Ins(cached=false,n=1)`. Rust's FAB test matches the public atoms/alignment; the proof smoke checks the private output and typed upstream VM operation shape. Manual and generated complete ledger-8 prototypes and exact tagged 501-byte pre-proof payloads agree; each path independently proves, verifies, validates and applies to equal state, with final Cell value 14. The existing 58-call offline gate gains this case as call 59.

Verification: independent TypeScript capture reproduced its committed JSON; eight runtime encoding tests, ten witness fixture tests, 132 fresh generated fixtures, 37 pinned oracle sources, external generated-crate-only consumer, complete `check_compactc_target.py --consumer --proof`, formatting, syntax and staged-diff checks passed. The generated witness fixture grows 501→607 lines (+106) because the source gained a full exported circuit; isolated compile time was not measured, and no optimization is claimed. The full gate ran current local ABI-21 `compactc` in the existing Nix compiler shell. During the run, this checkout's completed consumer dependency cache was removed to recover disk space; source and user-owned `doc/ledger-adt.mdx` were untouched.

Issue #143 remains open for nested tuple/struct/vector public input parity, three-plus parameter support, compile-time measurement, same-head remote CI and clean release provenance. The prior proposal's general acceptance list is not declared complete by this two-Field proof.


### Research amendment — 2026-10-03: three-plus parameters and alignment composition

The accepted ABI-21 emitter still suppresses observed call methods above two declared parameters. Upstream pinned `midnight-base-crypto 1.0.0` provides tuple `From` implementations only through 11 elements, but `AlignedValue::concat` extends each input's `Value` and `Alignment` segments in order without a tuple arity limit. Generated TypeScript public input for prior Map and witnessed probes concatenates each parameter descriptor's value/alignment in declared order. The next decision is whether the AST emitter should build an array of separately aligned typed parameter values and call upstream `AlignedValue::concat`, rather than represent all parameters as one Rust tuple. This is a proposal, not parity evidence for three or nested shapes.

Before for a real three-parameter exported circuit, the developer must select its string and encode public input with `CallSpec`; no `*_call` method is emitted. Proposed after measured TypeScript parity: `contract.recording.<name>_call(&observed, private, first, second, third)?.prepare(verifier, randomness)?`. The emitter would derive each typed argument and ordered alignment from `StatefulCircuit.parameters`; runtime would reuse upstream FAB concatenation, `RecordedCall` and `prepare_call` without a new VM or prover rule. If the public generated surface expands, bump ABI while keeping private IR schema 8. Verify an actual three-parameter source, independent TypeScript atoms/alignment and VM shape, complete manual/generated prototype and exact pre-proof bytes, independent proof/application, one-dependency consumer, wrong-typed arguments, fixture freshness and release compatibility. Nested tuple/struct/vector shapes remain a separate parity claim. Continue issue #143 in `rust-backend-v2`; preserve this proposal if evidence requires another approach.


#### Three-parameter source selection and pre-implementation evidence — 2026-10-03

The real `map_boolean_field.compact` probe now includes `put_pair(left: Boolean, right: Boolean, value: Field)`, all three used through `table.insert(disclose(left && right), disclose(value))`. Current ABI-21 Rust compilation emits a complete `recording.put_pair` method but suppresses `put_pair_call`. Fresh TypeScript compilation for `(true,false,42n)` constructs input by concatenating each descriptor in declared order: atoms `[1]`, `[]`, `[42]` and alignments Boolean byte, Boolean byte, Field. The independently executed TypeScript capture also gives five upstream Map operations with checked flags. This establishes the source and expected public encoding before changing the emitter; complete Rust prototype/pre-proof parity and application remain unproven at this checkpoint.


#### Source correction before parity — 2026-10-03

The initial `put_pair` probe combined `left && right` in one Map key. The compiler generated a native method but did not mark that body as complete recorded output, so the observed-call emitter correctly had no eligible method to extend. The probe now uses two explicit `table.insert` actions, one for each Boolean key, with the Field value in both. Fresh ABI-22 Rust compilation emits `recording.put_pair_call` with upstream `AlignedValue::concat`. The prior TypeScript capture for the single combined key is superseded; it must be regenerated from this revised source before parity claims. This correction narrows the evidence to a supported recorded body without changing the three-parameter design question.


### Delivery amendment — 2026-10-03: ordered FAB composition for three-plus arguments

**Decision:** for complete recorded exports with three or more declared parameters, derive one upstream `AlignedValue` from each typed argument and compose them in IR declaration order with upstream `AlignedValue::concat`. Keep the previously proven zero-, one- and two-argument encodings unchanged. This removes the observed-method arity gate, not the requirement to prove each new value shape. Pinned `midnight-base-crypto 1.0.0` tuple conversions stop at 11 elements; direct aligned-value concatenation has no tuple-arity dependency and follows the Compact TypeScript descriptor concatenation. A local helper, proc macro, and manual `Value`/`Alignment` flattening were unnecessary. Local conventional GPG-verified/DCO commit: `947a40df818083eea36f9c3574f9a3d3ca63a87f` (`feat(rust-backend): compose observed inputs for larger signatures`). Branch remains local/unpushed.

#### Actual before/after Rust

The real `Map.put_pair(left: Boolean, right: Boolean, value: Field)` source performs two ordered Map insertions. Before ABI 22, a developer had a complete recorded method but no `put_pair_call`; they had to supply both the entry point and public input:

```rust
let recorded = contract.recording.put_pair(
    observed.circuit_context(()), true, false, Field::from(42))?;
let manual = prepare_call(recorded, CallSpec::new(
    "put_pair", verifier,
    AlignedValue::from((true, false, Field::from(42))), randomness))?;
```

After ABI 22, the generated crate owns these source facts:

```rust
let typed = contract.recording
    .put_pair_call(&observed, (), true, false, Field::from(42))?
    .prepare(verifier, randomness)?;
```

The emitted method constructs `AlignedValue::concat(&[AlignedValue::from(arg0.clone()), AlignedValue::from(arg1.clone()), AlignedValue::from(arg2.clone())])` before invoking the existing recorded body. The separate consumer depends directly only on the generated crate; a wrong-typed middle argument fails Rust checking with E0308. The renderer also parses a synthetic 12-parameter method, which demonstrates the emitter does not require an upstream 12-tuple implementation; it does **not** prove 12-parameter runtime parity.

#### Independent encoding and ledger evidence

Fresh TypeScript compilation and the reproducible `capture-map-three-parameter-input.mjs` capture for `(true,false,42n)` give public atoms `[1]`, `[]`, `[42]` with Boolean byte, Boolean byte, Field alignment. The empty byte atom is the upstream representation of `false`, and the committed JSON preserves it. TypeScript records two five-operation Map insertion sequences: for each key, `Idx(cached=false,pushPath=true,pathLength=1)`, `Push(storage=false)`, `Push(storage=true)`, `Ins(cached=false,n=1)`, `Ins(cached=true,n=1)`. The nine runtime encoding tests compare Rust concat to those atoms/alignment; proof smoke checks the two typed upstream VM sequences. It compares the complete manually prepared and generated `ContractCallPrototype`, exact tagged pre-proof transaction bytes, and independently proves, verifies, validates and applies both paths to equal ledger-8 state. The normal generated call leaves two Map entries. The rebuilt offline gate now covers 60 call cases. Independently sealed proof bytes remain nondeterministic under the pinned PLONK `OsRng` blinding; the exact deterministic boundary is pre-proof bytes (ADR-0043).

#### Ownership, compatibility and limits

The `syn`/`quote` AST emitter removes only the parameter-count condition and builds the ordered concat expression for arity three-plus. Exported `<name>_call` collision suppression and complete-recording eligibility remain. Runtime changes only `RUST_RUNTIME_ABI` from 21 to 22 because newly generated crates expose a larger API; no transaction, VM, proof, wallet or macro behavior changes, and private IR schema stays 8. `Map.put_pair` adds a real source circuit and grows that generated fixture from 606 to 708 lines (+102). The 132-fixture refresh also carries the ABI assertion; isolated compile-time impact was not measured, and no optimization is claimed.

The local 57 renderer tests, nine runtime encoding tests, two focused Map fixture tests, 132 fixture outputs, 37 pinned oracle sources, all-features workspace library check, one-dependency positive/negative consumer, and complete `check_compactc_target.py --consumer --proof` pass. ABI-22 macro/runtime archives package, compile unpacked, and match `target/rust-runtime-release-abi22.json` after commit. That local manifest is dirty because user-owned `doc/ledger-adt.mdx` is unstaged, so it is not a clean signed release candidate. The compiler/proof gate used current local ABI-22 `compactc` in an existing Nix compiler shell, not a newly packaged Nix CLI.

Issue [#143](https://github.com/MediaNoxLabs/compact/issues/143) stays open: independently establish nested tuple/struct/vector argument alignment, test an actual signature above 11 parameters, measure compile time, and pass same-head remote CI, clean signed release, registry consumer and branch publication. Caller-supplied observation trust and concurrent state limits also remain. This amendment supersedes the earlier arity-suppression proposal for complete recorded exports while preserving its rationale and the initial source correction history.


### Evidence amendment — 2026-10-03: current-head packaged Darwin compiler

The preceding ABI-22 delivery correctly said its proof gate used a local compiler in an existing Nix shell and had not yet certified a newly packaged Nix CLI. A fresh `aarch64-darwin` `nix build .#compactc --no-link --print-out-paths` now succeeds on local commit `947a40df818083eea36f9c3574f9a3d3ca63a87f`, producing `${HISTORICAL_NIX_STORE}/mvhv5543q2pw5zrvf515bs11davmj5cc-compactc`. That package's `compactc --version` reports 0.31.133; `--target rust --skip-zk` emits a Counter crate asserting ABI 22, and an external copied crate passes `cargo check --features ledger-transaction --offline`. The Nix source was dirty from the user-owned unstaged documentation edit, and this is local Darwin evidence, not remote CI, a clean signed release, or a proof gate rerun using the packaged CLI. The original caveat remains above as the state at delivery time; this amendment records the later verification.



### Related nested-input slice — 2026-10-03, ABI 23

ADR-0045 / [#144](https://github.com/MediaNoxLabs/compact/issues/144) now establishes concrete Vector, tuple, and nested generated-struct public FAB parity for **root Set insertion calls**. The TypeScript capture, local signed/DCO `66348cd7`, one-dependency consumer and 63-call proof/application gate provide this bounded evidence. It does not establish arbitrary nested Compact input shapes, other ledger containers, tuple/struct Set membership/removal, or actual >11-parameter runtime parity. ADR-0044/#143 stays open for those broader observed-input limits and production release gates. ABI 23 expands the generated surface through typed recording; schema 8 and transaction/VM/prover logic remain unchanged.



### Related composite Set roundtrip proof — 2026-10-03

ADR-0045 / [#144](https://github.com/MediaNoxLabs/compact/issues/144) adds local signed/DCO `3125f60c`: tuple and nested-struct root Set calls now exercise membership and removal as well as insertion, with TypeScript 14-op and `false` output parity, exact manual/generated pre-proof bytes (694/906), independent proof/application, and empty final Sets in the 65-call gate. This bounded evidence advances nested input support but does not establish arbitrary nested locations or actual >11-parameter runtime calls. ADR-0044/#143 remains open. ABI 23 and schema 8 are unchanged.



### Actual twelve-argument acceptance — 2026-10-03

**Decision and scope.** The existing ABI-22/23 `AlignedValue::concat` design is now exercised by a real exported Compact circuit beyond the upstream FAB tuple-conversion limit of eleven elements. Local conventional GPG-verified/DCO commit `4afca58eb3e7357d18f897e3de4c7eb07a9cc28a` (`test(rust-backend): prove twelve-argument observed calls`) adds this bounded acceptance. ADR-0044 remains **accepted-partial**: one real twelve-Field signature is proven, not every combination of twelve nested shapes or arbitrary ledger locations. The branch remains local and unpushed.

#### Problem and developer-facing before/after

The emitter had a synthetic twelve-parameter renderer assertion, but no independent TypeScript capture, generated-only consumer, or ledger proof for a real call over eleven inputs. A consumer could not rely on the arity claim as a tested contract. The new `record_twelve(a: Field, ..., l: Field)` circuit uses every argument in a sum and inserts 78 into a root `Set<Field>` for inputs 1 through 12.

Before the generated observed-call surface, the consumer would repeat the entry point and concatenate every public input manually:

```rust
let recorded = contract.recording.record_twelve(
    observed.circuit_context(()), a, b, c, d, e, f, g, h, i, j, k, l)?;
let input = AlignedValue::concat(&[
    AlignedValue::from(a), /* ... ten ordered Field values ... */,
    AlignedValue::from(l),
]);
let manual = prepare_call(recorded,
    CallSpec::new("record_twelve", verifier, input, randomness))?;
```

The generated-only consumer now invokes the typed method with twelve ordinary `Field` arguments:

```rust
let call = contract.recording.record_twelve_call(
    &observed, (),
    1_u64.into(), 2_u64.into(), 3_u64.into(), 4_u64.into(),
    5_u64.into(), 6_u64.into(), 7_u64.into(), 8_u64.into(),
    9_u64.into(), 10_u64.into(), 11_u64.into(), 12_u64.into(),
)?.prepare(verifier, randomness)?;
```

#### Emitter, runtime, source and compatibility

No emitter or runtime implementation changed: the existing `syn`/`quote` AST emitter derives the twelve typed parameters and emits upstream `AlignedValue::concat` in declared order; the generated method reuses the recorded body. The runtime still owns `ObservedContractState`, verifier decoding, transaction preparation and upstream ledger-8 proof/application. No new primitive, macro or local FAB encoder was introduced. Public generated/runtime ABI stays 23; private IR schema stays 8. The existing composite-key example gains one `Set<Field>` and one complete circuit. Its generated fixture grows 456→673 lines (+217, driven by a new circuit), with isolated compile-time impact unmeasured.

#### Independent evidence and gates

The reproducible TypeScript capture in `capture-observed-composite-keys.mjs` gives twelve ordered Field atoms 1…12 and twelve Field alignment segments, with one five-operation `Idx/Push/Push/Ins/Ins` Set insert transcript. All five prior capture entries remain byte-identical. Rust fixture tests compare exact input atoms/alignment and native versus recorded result, gas, effects, state, replay and operation shape. A one-dependency external consumer compiles and invokes the twelve-argument generated method.

The local packaged ABI-23 Darwin `compactc` plus ledger-8 ZKIR compiles all six circuits. The full `check_compactc_target.py --consumer --proof` gate passes **66** offline proof/verification/validation/application calls. For `record_twelve`, manual and generated calls have equal complete `ContractCallPrototype` and exact 546-byte tagged pre-proof payloads; both independently prove and apply the same state containing Set key 78. Two focused fixture tests, 133 refreshed fixtures (one changed), `cargo fmt --all -- --check`, and staged diff checks pass. GPG signature and DCO are verified on the local commit. The user-owned `doc/ledger-adt.mdx` was not included.

#### Remaining limits

This establishes one all-Field arity-twelve path. Mixed or nested values across a high-arity signature, nested ledger locations, compile-time measurement, same-head remote CI, clean signed release provenance, registry consumer, wallet/node submission and branch publication remain open. Caller-supplied observation trust and concurrent state limits remain as in ADR-0043. Focused tracking stays [#143](https://github.com/MediaNoxLabs/compact/issues/143) in `rust-backend-v2`; ADR-0045 / #144 covers composite root Set keys.
